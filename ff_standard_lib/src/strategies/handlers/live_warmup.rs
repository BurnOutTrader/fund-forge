use std::sync::Arc;
use std::time::Duration;
use chrono::{DateTime, NaiveTime, TimeZone, Utc};
use lazy_static::lazy_static;
use tokio::sync::broadcast;
use tokio::sync::mpsc::Sender;
use crate::standardized_types::base_data::history::{get_compressed_historical_data};
use crate::standardized_types::time_slices::TimeSlice;
use crate::strategies::handlers::indicator_handler::IndicatorHandler;
use crate::strategies::handlers::market_handler::price_service::{get_price_service_sender, PriceServiceMessage};
use crate::strategies::handlers::subscription_handler::SubscriptionHandler;
use crate::strategies::handlers::timed_events_handler::TimedEventHandler;
use crate::strategies::historical_time::update_backtest_time;
use crate::strategies::ledgers::ledger_service::LedgerService;
use crate::strategies::strategy_events::StrategyEvent;

lazy_static! {
    pub(crate) static ref WARMUP_COMPLETE_BROADCASTER: broadcast::Sender<DateTime<Utc>> = {
        let (tx, _) = broadcast::channel(1);
        tx
    };
}

pub(crate) async fn live_warm_up(
    warm_up_start_time: DateTime<Utc>,
    buffer_duration: Duration,
    subscription_handler: Arc<SubscriptionHandler>,
    strategy_event_sender: Sender<StrategyEvent>,
    timed_event_handler: Arc<TimedEventHandler>,
    ledger_service: Arc<LedgerService>,
    indicator_handler: Arc<IndicatorHandler>
) {
    tokio::task::spawn(async move {
        println!("Live Warmup: Warming up the strategy...");
        let market_price_sender = get_price_service_sender();

        // Get initial subscriptions
        let mut primary_subscriptions = loop {
            let subscriptions = subscription_handler.primary_subscriptions().await;
            if !subscriptions.is_empty() {
                break subscriptions;
            }
            println!("Live Warmup: Waiting for primary subscriptions...");
            tokio::time::sleep(Duration::from_millis(200)).await;
        };

        let mut primary_subscription_update_receiver = subscription_handler.subscribe_primary_subscription_updates();

        // Log subscriptions only in debug/development
        #[cfg(debug_assertions)] {
            for subscription in &primary_subscriptions {
                println!("Live Warmup: Primary Subscription: {}", subscription);
            }
            let strategy_subscriptions = subscription_handler.strategy_subscriptions().await;
            for subscription in &strategy_subscriptions {
                println!("Live Warmup: Strategy Subscription: {}", subscription);
            }
        }

        let mut last_time = warm_up_start_time;
        let mut first_iteration = true;

        'main_loop: loop {
            if !first_iteration {
                last_time += Duration::from_nanos(1)
            }

            let to_time = {
                let end_of_day_naive = last_time.date_naive().and_time(NaiveTime::from_hms_nano_opt(23, 59, 59, 999_999_999).unwrap());
                Utc.from_utc_datetime(&end_of_day_naive).max(last_time)
            };
            first_iteration = false;

            if last_time >= Utc::now() {
                WARMUP_COMPLETE_BROADCASTER.send(last_time).unwrap();
                if let Err(e) = strategy_event_sender.send(StrategyEvent::WarmUpComplete).await {
                    eprintln!("Live Warmup: Failed to send event: {}", e);
                }
                break 'main_loop;
            }

            let mut time_slices = match get_compressed_historical_data(primary_subscriptions.clone(), last_time, to_time, 900).await {
                Ok(time_slices) => {
                    if time_slices.is_empty() {
                        println!("Live Warmup: No data period, weekend or holiday: skipping to next day");
                        last_time = to_time + buffer_duration;
                        continue 'main_loop
                    }
                    time_slices
                },
                Err(_) => continue 'main_loop
            };

            let mut time = last_time;
            'day_loop: while time <= to_time {
                time += buffer_duration;

                // Early exit check
                if time >= Utc::now() {
                    WARMUP_COMPLETE_BROADCASTER.send(time).unwrap();
                    if let Err(e) = strategy_event_sender.send(StrategyEvent::WarmUpComplete).await {
                        eprintln!("Live Warmup: Failed to send event: {}", e);
                    }
                    break 'main_loop;
                }

                // Check subscription updates without blocking
                if let Ok(updates) = primary_subscription_update_receiver.try_recv() {
                    if updates != primary_subscriptions {
                        primary_subscriptions = updates;
                        break 'day_loop;
                    }
                }

                // Update time handlers
                timed_event_handler.update_time(time).await;
                update_backtest_time(time);

                // Extract data for current time window
                let time_range = last_time.timestamp_nanos_opt().unwrap()..=time.timestamp_nanos_opt().unwrap();
                let mut time_slice = TimeSlice::new();
                let keys_to_remove: Vec<i64> = time_slices.range(time_range).map(|(k, _)| *k).collect();

                // Process time slice data in batch
                if !keys_to_remove.is_empty() {
                    for key in keys_to_remove {
                        if let Some(data) = time_slices.remove(&key) {
                            time_slice.extend(data);
                        }
                    }

                    let arc_slice = Arc::new(time_slice.clone());

                    // Send updates in parallel using join
                    let (market_result, _ledger_result, subscription_result) = tokio::join!(
                        market_price_sender.send(PriceServiceMessage::TimeSliceUpdate(arc_slice.clone())),
                        ledger_service.timeslice_updates(arc_slice.clone()),
                        subscription_handler.update_time_slice(arc_slice)
                    );

                    if let Err(e) = market_result {
                        panic!("Live Warmup: Error sending backtest message: {}", e);
                    }

                    let mut strategy_time_slice = TimeSlice::new();
                    if let Some(consolidated_data) = subscription_result {
                        strategy_time_slice.extend(consolidated_data);
                    }
                    strategy_time_slice.extend(time_slice);

                    // Process consolidated data if any
                    if let Some(consolidated_data) = subscription_handler.update_consolidators_time(time).await {
                        strategy_time_slice.extend(consolidated_data);
                    }

                    // Send events if we have data
                    if !strategy_time_slice.is_empty() {
                        // Update indicators and send events in parallel
                        if let Some(events) = indicator_handler.update_time_slice(&strategy_time_slice).await {
                            if let Err(e) = strategy_event_sender.send(StrategyEvent::IndicatorEvent(events)).await {
                                eprintln!("Live Warmup: Failed to send indicator event: {}", e);
                            }
                        }

                        if let Err(e) = strategy_event_sender.send(StrategyEvent::TimeSlice(strategy_time_slice)).await {
                            eprintln!("Live Warmup: Failed to send time slice event: {}", e);
                        }
                    }
                }

                last_time = time;
            }
        }
    });
}
use std::sync::Arc;
use std::time::Duration;
use chrono::{DateTime, Timelike, Utc};
use dashmap::DashMap;
use tokio::sync::broadcast;
use ff_standard_lib::standardized_types::accounts::AccountId;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;
use crate::oanda_api::api_client::OandaClient;

// allows us to subscribe to quote bars by manually requesting bar updates every 5 seconds
pub fn handle_quotebar_subscribers(
    client: Arc<OandaClient>,
    account_id: AccountId,
) {
    tokio::spawn(async move {
        let last_closed_time: DashMap<DataSubscription, DateTime<Utc>> = DashMap::new();
        let quotebar_broadcasters: Arc<DashMap<DataSubscription, broadcast::Sender<BaseDataEnum>>> = client.quotebar_broadcasters.clone();

        loop {
            // Calculate delay until next 5-second boundary + 10ms
            let now = Utc::now();
            let next_five_seconds = now
                .with_nanosecond(0).unwrap()
                .checked_add_signed(chrono::Duration::seconds((5 - (now.second() % 5)) as i64))
                .unwrap();
            let target_time = next_five_seconds + chrono::Duration::milliseconds(10);
            let delay = target_time.signed_duration_since(now);

            // Sleep until the next tick
            if delay.num_milliseconds() > 0 {
                tokio::time::sleep(Duration::from_millis(delay.num_milliseconds() as u64)).await;
            }

            let mut to_remove = Vec::new();
            for broadcaster in quotebar_broadcasters.iter() {
                let bars = match client.get_latest_bars(
                    &broadcaster.key().symbol,
                    broadcaster.key().base_data_type,
                    broadcaster.key().resolution,
                    &account_id,
                    2
                ).await {
                    Ok(bars) => bars,
                    Err(e) => {
                        eprintln!("Failed to get_requests latest bars for quotebar subscriber: {}", e);
                        continue
                    }
                };

                for bar in bars {
                    if let Some(last_time) = last_closed_time.get(&broadcaster.key()) {
                        if bar.time_closed_utc() > *last_time.value() || !bar.is_closed() {
                            match broadcaster.value().send(bar.clone()) {
                                Ok(_) => {}
                                Err(_) => {
                                    if broadcaster.receiver_count() == 0 {
                                        to_remove.push(broadcaster.key().clone())
                                    }
                                }
                            }
                            last_closed_time.insert(bar.subscription(), bar.time_closed_utc());
                        }
                    }
                }
            }
            for key in to_remove {
                quotebar_broadcasters.remove(&key);
            }
        }
    });
}
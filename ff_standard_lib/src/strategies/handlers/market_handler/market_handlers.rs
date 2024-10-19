use chrono::{Utc};
use crate::strategies::ledgers::{LEDGER_SERVICE};
use crate::standardized_types::enums::{StrategyMode};
use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc;
use crate::standardized_types::broker_enum::Brokerage;
use crate::strategies::client_features::server_connections::{is_warmup_complete};
use crate::standardized_types::time_slices::TimeSlice;
use rust_decimal::Decimal;
use tokio::sync::mpsc::error::SendError;
use crate::standardized_types::accounts::{AccountId, AccountInfo, Currency};
use crate::standardized_types::orders::{Order, OrderId, OrderRequest};
use crate::strategies::handlers::market_handler::backtest_matching_engine;
use crate::strategies::handlers::market_handler::backtest_matching_engine::BackTestEngineMessage;
use crate::strategies::handlers::market_handler::price_service::{get_price_service_sender, PriceServiceMessage};
use crate::strategies::historical_time::get_backtest_time;

#[derive(Clone, Debug)]
pub enum MarketMessageEnum {
    TimeSliceUpdate(TimeSlice),
    OrderRequest(OrderRequest),
    InitLedger{ brokerage: Brokerage, account_id: AccountId, strategy_mode: StrategyMode, synchronize_accounts: bool, starting_cash: Decimal, currency: Currency },
    LiveLedgerSnapShot{ brokerage: Brokerage, account_id: AccountId, synchronize_accounts: bool, account_info: AccountInfo, strategy_mode: StrategyMode}
}

pub(crate) async fn market_handler(
    mode: StrategyMode,
    open_order_cache: Arc<DashMap<OrderId, Order>>,
    closed_order_cache: Arc<DashMap<OrderId, Order>>,
) -> Sender<MarketMessageEnum> {
    let (market_event_sender, mut market_event_receiver) = mpsc::channel(1000);
    tokio::task::spawn(async move{
        let backtest_order_sender = match mode {
            StrategyMode::Live => None,
            StrategyMode::LivePaperTrading | StrategyMode::Backtest => {
                let sender = backtest_matching_engine::backtest_matching_engine(open_order_cache.clone(), closed_order_cache.clone()).await;
                Some(sender)
            }
        };
        let market_price_sender = get_price_service_sender();
        while let Some(message) = market_event_receiver.recv().await {
            let time = match mode {
                StrategyMode::Backtest => get_backtest_time(),
                StrategyMode::Live | StrategyMode::LivePaperTrading => Utc::now()
            };
            match message {
                MarketMessageEnum::TimeSliceUpdate(time_slice) => {
                    match market_price_sender.send(PriceServiceMessage::TimeSliceUpdate(time_slice.clone())).await {
                        Ok(_) => {}
                        Err(e) => panic!("Market Handler: Error sending backtest message: {}", e)
                    }
                    LEDGER_SERVICE.timeslice_updates(time, time_slice).await;
                    if let Some(backtest_engine_sender) = &backtest_order_sender {
                        let message = BackTestEngineMessage::Time(time);
                        match backtest_engine_sender.send(message).await {
                            Ok(_) => {}
                            Err(e) => panic!("Market Handler: Error sending backtest message: {}", e)
                        }
                    }
                }
                MarketMessageEnum::OrderRequest(order_request) => {
                    if !is_warmup_complete() {
                        panic!("Market Handler: Warning: Attempted to place order during warm up!");
                    }
                    match mode {
                        StrategyMode::Live => {
                            panic!("Market Handler does not manage live orders")
                        },
                        StrategyMode::LivePaperTrading | StrategyMode::Backtest => {
                            if let Some(order_sender) = &backtest_order_sender {
                                //println!("sending order: {:?}", order_request);
                                match order_sender.send(BackTestEngineMessage::OrderRequest(time, order_request)).await {
                                    Ok(_) => {}
                                    Err(e) => panic!("Market Handler: Error sending order to backtest matching engine: {}", e)
                                }
                            }
                        }
                    }
                }
                MarketMessageEnum::InitLedger { brokerage, account_id, strategy_mode, synchronize_accounts, starting_cash, currency } => {
                    LEDGER_SERVICE.init_ledger(brokerage, account_id, strategy_mode, synchronize_accounts, starting_cash, currency).await;
                }
                MarketMessageEnum::LiveLedgerSnapShot { brokerage, account_id, synchronize_accounts, account_info, strategy_mode} => {
                    LEDGER_SERVICE.account_snapshot(brokerage, account_id, synchronize_accounts, account_info, strategy_mode).await;
                }
            }
        };
    });
    market_event_sender
}







use chrono::{Duration, NaiveDate};
use chrono_tz::Australia;
use rust_decimal_macros::dec;
use tokio::sync::mpsc;
use crate::standardized_types::accounts::{Account, Currency};
use crate::standardized_types::broker_enum::Brokerage;
use crate::standardized_types::datavendor_enum::DataVendor;
use crate::standardized_types::enums::{MarketType, StrategyMode};
use crate::standardized_types::resolution::Resolution;
use crate::standardized_types::subscriptions::{CandleType, DataSubscription, SymbolName};
use crate::strategies::fund_forge_strategy::FundForgeStrategy;

#[allow(dead_code)]
pub(crate) fn initialize_tests() -> FundForgeStrategy {
    // Set up strategy event channel
    let (strategy_event_sender, _strategy_event_receiver) = mpsc::channel(100);

    // Use `block_on` to call the asynchronous `initialize` method
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        FundForgeStrategy::initialize(
            StrategyMode::Backtest,
            dec!(100000),
            Currency::USD,
            NaiveDate::from_ymd_opt(2024, 6, 5).unwrap().and_hms_opt(0, 0, 0).unwrap(),
            NaiveDate::from_ymd_opt(2024, 6, 15).unwrap().and_hms_opt(0, 0, 0).unwrap(),
            Australia::Sydney,
            Duration::hours(1),
            vec![
                DataSubscription::new_custom(
                    SymbolName::from("EUR-USD"),
                    DataVendor::Test,
                    Resolution::Minutes(3),
                    MarketType::Forex,
                    CandleType::HeikinAshi,
                ),
            ],
            false,
            100,
            strategy_event_sender,
            core::time::Duration::from_millis(100),
            false,
            false,
            false,
            vec![
                Account::new(Brokerage::Test, "Test_Account_1".to_string()),
                Account::new(Brokerage::Test, "Test_Account_2".to_string()),
            ],
        )
            .await
    })
}
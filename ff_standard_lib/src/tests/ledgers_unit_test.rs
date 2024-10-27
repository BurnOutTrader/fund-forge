//todo, these will always fail because server connection is not maintained. need to create a way to actually run tests like this.
#[allow(dead_code, unused_imports, unused_variables)]
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{self, Duration};
    use chrono::{Utc};
    use lazy_static::lazy_static;
    use rust_decimal_macros::dec;
    use crate::standardized_types::accounts::{Account, Currency};
    use crate::standardized_types::orders::{OrderState, OrderType, TimeInForce};
    use crate::strategies::fund_forge_strategy::FundForgeStrategy;
    use crate::tests::initialize_connections_faux_strategy::initialize_tests;
    use crate::standardized_types::broker_enum::Brokerage;
    use crate::standardized_types::enums::{OrderSide, PositionSide, StrategyMode};
    use crate::standardized_types::orders::Order;
    use crate::standardized_types::position::Position;
    use crate::standardized_types::symbol_info::SymbolInfo;
    use crate::strategies::ledgers::LEDGER_SERVICE;


    lazy_static! {
        static ref STRATEGY: FundForgeStrategy = initialize_tests();
    }

    impl Default for Position {
        fn default() -> Self {
            Position {
                symbol_name: "".to_string(),
                symbol_code: "".to_string(),
                account: Account::new(Brokerage::Test, "test_account_id".to_string()),
                side: PositionSide::Long,
                open_time: Utc::now().to_string(),
                quantity_open: dec!(0),
                quantity_closed: dec!(0),
                close_time: None,
                average_price: dec!(0),
                open_pnl: dec!(0),
                booked_pnl: dec!(0),
                highest_recoded_price: dec!(0),
                lowest_recoded_price: dec!(0),
                average_exit_price: None,
                is_closed: false,
                position_id: "".to_string(),
                symbol_info: SymbolInfo::new("".to_string(), Currency::USD, dec!(1), dec!(0.01), 2),
                pnl_currency: Currency::USD,
                tag: "".to_string(),
            }
        }
    }

    #[tokio::test]
    async fn test_synchronized_mode_with_order_and_position_updates() {

    }

    #[tokio::test]
    async fn test_concurrent_updates_synchronized() {

    }

    #[tokio::test]
    async fn test_concurrent_updates_unsynchronized() {

    }
}
//todo, these will always fail because server connection is not maintained. need to create a way to actually run tests like this.
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
        let account = Account::new(Brokerage::Test, "test_account_id".to_string());
        let symbol_name = "AAPL".to_string();
        let symbol_code = "AAPL".to_string();
        let initial_quantity = dec!(100.0);
        let order_id = "order_1".to_string();
        let price = dec!(150.0);
        let side = OrderSide::Buy;
        let time = Utc::now();
        let tag = "test_tag".to_string();

        // Initialize ledger
        LEDGER_SERVICE.init_ledger(&account, StrategyMode::Live, true, dec!(10000.0), Currency::USD).await;

        // First update: position created based on an order fill
        let order_event = LEDGER_SERVICE.update_or_create_paper_position(
            &account,
            symbol_name.clone(),
            symbol_code.clone(),
            order_id.clone(),
            initial_quantity,
            side.clone(),
            time,
            price,
            tag.clone()
        ).await;

        // Assert that the position was created
        assert!(order_event.is_ok());

        // Trigger synchronized position update
        let position_update_event = LEDGER_SERVICE.synchronize_live_position(account.clone(), Position {
            symbol_name: symbol_name.clone(),
            symbol_code: symbol_code.clone(),
            account: account.clone(),
            side: PositionSide::Long,
            quantity_open: initial_quantity,
            average_price: price,
            ..Default::default()
        });

        assert!(position_update_event.is_some());

        // Print ledger state for visual verification
        LEDGER_SERVICE.print_ledger(&account).await;
    }

    #[tokio::test]
    async fn test_concurrent_updates_synchronized() {
        let account = Account::new(Brokerage::Test, "test_account_id".to_string());
        let symbol_name = "AAPL".to_string();
        let symbol_code = "AAPL".to_string();
        let initial_quantity = dec!(100.0);
        let price = dec!(150.0);
        let tag = "test_tag".to_string();

        LEDGER_SERVICE.init_ledger(&account, StrategyMode::Live, true, dec!(10000.0), Currency::USD).await;

        // Send order updates concurrently
        let order_tasks: Vec<_> = (0..5)
            .map(|i| {
                let account = account.clone();
                let symbol_name = symbol_name.clone();
                let symbol_code = symbol_code.clone();
                let tag = tag.clone();
                tokio::spawn(async move {
                    let order_event = LEDGER_SERVICE.update_or_create_paper_position(
                        &account,
                        symbol_name.clone(),
                        symbol_code.clone(),
                        format!("order_{}", i),
                        initial_quantity,
                        OrderSide::Buy,
                        Utc::now(),
                        price,
                        tag.clone()
                    ).await;

                    assert!(order_event.is_ok(), "Expected successful position creation.");
                })
            })
            .collect();

        // Await all tasks
        futures::future::join_all(order_tasks).await;

        // Verify ledger state consistency
        LEDGER_SERVICE.print_ledger(&account).await;
        assert!(LEDGER_SERVICE.position_size(&account, &symbol_name) > dec!(0), "Expected positions in ledger.");
    }

    #[tokio::test]
    async fn test_concurrent_updates_unsynchronized() {
        let account = Account::new(Brokerage::Test, "test_account_id".to_string());
        let symbol_name = "AAPL".to_string();
        let symbol_code = "AAPL".to_string();
        let initial_quantity = dec!(100.0);
        let reduced_quantity = dec!(50.0);
        let price = dec!(150.0);
        let tag = "test_tag".to_string();

        LEDGER_SERVICE.init_ledger(&account, StrategyMode::Live, false, dec!(10000.0), Currency::USD).await;

        // Clone all variables used within each task to avoid move errors
        let position_account = account.clone();
        let position_symbol_name = symbol_name.clone();
        let position_symbol_code = symbol_code.clone();
        let position_tag = tag.clone();
        let position_price = price;

        let order_account = account.clone();
        let order_symbol_name = symbol_name.clone();
        let order_symbol_code = symbol_code.clone();
        let order_tag = tag.clone();
        let order_reduced_quantity = reduced_quantity;

        // Spawn tasks for out-of-order position updates
        let position_task = tokio::spawn(async move {
            LEDGER_SERVICE.update_or_create_live_position(
                &position_account,
                position_symbol_name,
                position_symbol_code,
                initial_quantity,
                OrderSide::Buy,
                Utc::now(),
                position_price,
                position_tag,
            ).await;
        });

        time::sleep(Duration::from_millis(50)).await; // Simulate out-of-order processing delay

        let order_task = tokio::spawn(async move {
            LEDGER_SERVICE.process_synchronized_orders(Order {
                symbol_name: order_symbol_name,
                symbol_code: Some(order_symbol_code),
                account: order_account,
                quantity_open: order_reduced_quantity,
                quantity_filled: order_reduced_quantity,
                average_fill_price: None,
                limit_price: None,
                trigger_price: None,
                side: OrderSide::Sell,
                order_type: OrderType::Market,
                time_in_force: TimeInForce::GTC,
                tag: order_tag,
                id: "order_1".to_string(),
                time_created_utc: "".to_string(),
                time_filled_utc: None,
                state: OrderState::Created,
                fees: Default::default(),
                value: Default::default(),
                exchange: None,
            }, order_reduced_quantity).await;
        });

        // Await both tasks
        let (position_update, order_update) = tokio::join!(position_task, order_task);

        // Verify outcomes
        assert!(position_update.is_ok(), "Expected successful position update.");
        assert!(order_update.is_ok(), "Expected successful order update.");

        // Validate final ledger state
        LEDGER_SERVICE.print_ledger(&account).await;
    }
}
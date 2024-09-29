use std::sync::Arc;
use ahash::AHashMap;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::apis::brokerage::broker_enum::Brokerage;
use crate::market_handler::market_handlers::{SYMBOL_INFO};
use crate::server_connections::add_buffer;
use crate::standardized_types::accounts::ledgers::AccountId;
use crate::standardized_types::accounts::position::{Position, PositionId};
use crate::standardized_types::enums::{OrderSide, PositionSide};
use crate::standardized_types::orders::orders::{OrderUpdateEvent, ProtectiveOrder};
use crate::standardized_types::strategy_events::StrategyEvent;
use crate::standardized_types::subscriptions::SymbolName;
use crate::standardized_types::{Price, Volume};
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::time_slices::TimeSlice;

// Type alias for mapping positions by symbol
type SymbolPositionMap = AHashMap<SymbolName, Position>;
type SymbolClosedPositionMap = AHashMap<SymbolName, Vec<Position>>;

// Structure for managing backtest positions
pub struct PositionHandler {
    /// Open positions: Map of Brokerage -> AccountId -> SymbolName -> Position
    pub open: DashMap<Brokerage, DashMap<AccountId, SymbolPositionMap>>,
    /// Closed positions: Map of Brokerage -> AccountId -> SymbolName -> Vec<Position>
    pub closed: DashMap<Brokerage, DashMap<AccountId, SymbolClosedPositionMap>>,
    position_counter: DashMap<Brokerage, DashMap<AccountId, AHashMap<SymbolName, u64>>> 
}

impl PositionHandler {
    pub fn new() -> Self {
        PositionHandler {
            open: DashMap::new(),
            closed: DashMap::new(),
            position_counter: Default::default(),
        }
    }

    // Insert or update an open position
    pub fn insert_open_position(
        &self,
        brokerage: Brokerage,
        account_id: AccountId,
        symbol: SymbolName,
        position: Position,
    ) {
        let account_map = self
            .open
            .entry(brokerage)
            .or_insert_with(DashMap::new);
        let mut symbol_map = account_map
            .entry(account_id)
            .or_insert_with(AHashMap::new);
        symbol_map.insert(symbol, position);
    }

    pub async fn update_brackets(
        &self,
        brokerage: &Brokerage,
        account_id: &AccountId,
        symbol_name: &SymbolName,
        brackets: Vec<ProtectiveOrder>,
        time: DateTime<Utc>,
    ) -> Result<(), String> {
        // Find the position using the nested structure of `Brokerage`, `AccountId`, and `SymbolName`
        if let Some(account_map) = self.open.get(brokerage) {
            if let Some(mut symbol_map) = account_map.get_mut(account_id) {
                if let Some(mut position) = symbol_map.get_mut(symbol_name) {
                    // Update the position with new brackets
                    position.brackets = Some(brackets.clone());

                    // Create the event
                    let event = StrategyEvent::OrderEvents(OrderUpdateEvent::Updated {
                        brokerage: brokerage.clone(),
                        account_id: account_id.clone(),
                        order_id: position.id.clone(),
                    });

                    // Add the event to the buffer (assuming `add_buffer` is async)
                    add_buffer(time, event).await;

                    return Ok(());
                }
            }
        }

        // Return an error if the position wasn't found
        Err(format!("Position for brokerage {:?}, account {:?}, symbol {:?} not found", brokerage, account_id, symbol_name))
    }

    pub fn is_short(&self, brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
        if let Some(account_map) = self.open.get(brokerage) {
            if let Some(symbol_map) = account_map.get(account_id) {
                if let Some(position) = symbol_map.get(symbol_name) {
                    return match position.side {
                        PositionSide::Long => false,
                        PositionSide::Short => true
                    }
                }
            }
        }
        false
    }

    pub fn is_long(&self, brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
        if let Some(account_map) = self.open.get(brokerage) {
            if let Some(symbol_map) = account_map.get(account_id) {
                if let Some(position) = symbol_map.get(symbol_name) {
                    return match position.side {
                        PositionSide::Long => true,
                        PositionSide::Short => false
                    }
                }
            }
        }
        false
    }

    pub fn is_flat(&self, brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
        if let Some(account_map) = self.open.get(brokerage) {
            if let Some(symbol_map) = account_map.get(account_id) {
                if let Some(_) = symbol_map.get(symbol_name) {
                    return false
                }
            }
        }
        true
    }

    pub async fn update_or_create_paper_position(
        &self,
        symbol_name: &SymbolName,
        brokerage: Brokerage,
        account_id: &AccountId,
        quantity: Volume,
        side: OrderSide,
        time: &DateTime<Utc>,
        brackets: Option<Vec<ProtectiveOrder>>,
        market_price: Price // we use the passed in price because we dont know what sort of order was filled, limit or market
    ) -> Decimal {
        // Check if there's an existing position for the given symbol
        if let Some(account_map) = self.open.get(&brokerage) {
            if let Some(mut symbol_map) = account_map.get_mut(account_id) {
                if let Some(mut existing_position) = symbol_map.get_mut(symbol_name) {
                    let is_reducing = (existing_position.side == PositionSide::Long && side == OrderSide::Sell)
                        || (existing_position.side == PositionSide::Short && side == OrderSide::Buy);

                    if is_reducing {
                        if let Some(new_brackets) = brackets {
                            existing_position.brackets = Some(new_brackets);
                        }
                        existing_position.reduce_position_size(market_price, quantity, &time);
                    } else {
                        existing_position.add_to_position(market_price, quantity, time);
                        if let Some(new_brackets) = brackets {
                            existing_position.brackets = Some(new_brackets);
                        }
                        existing_position.add_to_position(market_price, quantity, &time);
                       return  dec!(0.0)
                    }
                }
            }
            dec!(0)
        } else {
            // Fetch symbol info if it doesn't exist
            if !SYMBOL_INFO.contains_key(symbol_name) {
                match brokerage.symbol_info(symbol_name.clone()).await {
                    Ok(info) => {
                        SYMBOL_INFO.insert(symbol_name.clone(), info.clone());
                    },
                    Err(_) => panic!("Unable to retrieve symbol info")
                };
            }

            // Determine the side of the position based on the order side
            let position_side = match side {
                OrderSide::Buy => PositionSide::Long,
                OrderSide::Sell => PositionSide::Short,
            };

            let info = SYMBOL_INFO.get(symbol_name).unwrap().value().clone();

            // Create a new position
            let position = Position::enter(
                symbol_name.clone(),
                brokerage.clone(),
                account_id.clone(),
                position_side,
                quantity,
                market_price,
                self.generate_id(symbol_name, brokerage, account_id, time.clone(), position_side),
                info.clone(),
                info.pnl_currency,
                brackets
            );

            // Insert the new position into the positions map
            self.insert_open_position(brokerage, account_id.clone(), symbol_name.clone(), position);
            dec!(0)
        }
    }

    pub fn generate_id(
        &self,
        symbol_name: &SymbolName,
        brokerage: Brokerage,
        account_id: &AccountId,
        time: DateTime<Utc>,
        side: PositionSide
    ) -> PositionId {
        // Get or insert the brokerage map
        let broker_count_map = self.position_counter.entry(brokerage)
            .or_insert_with(DashMap::new);

        // Get or insert the account map
        let mut account_map = broker_count_map.entry(account_id.clone())
            .or_insert_with(AHashMap::new);

        // Increment the counter for the symbol, or insert it if it doesn't exist
        let counter = account_map.entry(symbol_name.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1);

        // Return the generated position ID
        format!("{}-{}-{}-{}", symbol_name, counter, time.timestamp_nanos_opt().unwrap(), side)
    }

    pub async fn exit_position_paper( &self, brokerage: Brokerage, account_id: &AccountId, symbol_name: &SymbolName, time: DateTime<Utc>, market_price: Price) -> Decimal {
        if let Some(account_map) = self.open.get(&brokerage) {
            if let Some(mut symbol_map) = account_map.get_mut(account_id) {
                if let Some(mut existing_position) = symbol_map.remove(symbol_name) {
                    let side = match existing_position.side {
                        PositionSide::Long => OrderSide::Sell,
                        PositionSide::Short => OrderSide::Buy
                    };

                    existing_position.is_closed = true;
                    let booked_profit = existing_position.reduce_position_size(market_price, existing_position.quantity_open, &time);

                    let closed_account_map = self
                        .closed
                        .entry(brokerage)
                        .or_insert_with(DashMap::new);
                    let mut closed_symbol_map = closed_account_map
                        .entry(account_id.clone())
                        .or_insert_with(AHashMap::new);
                    closed_symbol_map
                        .entry(symbol_name.clone())
                        .or_insert_with(Vec::new)
                        .push(existing_position);

                    return booked_profit
                }
            }
        }
        dec!(0.0)
    }

    pub fn on_timeslice_updates(&self, time_slice: TimeSlice, time: &DateTime<Utc>) -> AHashMap<Brokerage, AHashMap<AccountId, Decimal>> {
        let mut booked_profits: AHashMap<Brokerage, AHashMap<AccountId, Decimal>> = AHashMap::new(); // Initialize booked profits

        // Iterate over base data enums in the time slice
        for base_data_enum in time_slice.iter() {
            for mut map in self.open.iter_mut() {
                for mut account_positions in map.iter_mut() {
                    // Check for an open position with the symbol name
                    if let Some(mut position) = account_positions.get_mut(&base_data_enum.symbol().name) {
                        // Update booked profits based on the base data
                        let profit = position.backtest_update_base_data(base_data_enum, time);

                        // Insert or update the booked profits for the brokerage and account
                        let account_profits = booked_profits
                            .entry(map.key().clone())
                            .or_insert_with(AHashMap::new);
                        *account_profits.entry(account_positions.key().clone()).or_insert(Decimal::ZERO) += profit; // Accumulate booked profit
                    }
                }
            }
        }

        booked_profits // Return the accumulated booked profits
    }

    pub fn on_data_updates(&self, base_data_enum: &BaseDataEnum, time: &DateTime<Utc>) -> AHashMap<Brokerage, AHashMap<AccountId, Decimal>> {
        let mut booked_profits: AHashMap<Brokerage, AHashMap<AccountId, Decimal>> = AHashMap::new(); // Initialize booked profits

        for map in self.open.iter() {
            for mut account_positions in map.iter_mut() {
                // Check for an open position with the symbol name
                if let Some(mut position) = account_positions.get_mut(&base_data_enum.symbol().name) {
                    // Update booked profits based on the base data
                    let profit = position.backtest_update_base_data(base_data_enum, time);

                    // Insert or update the booked profits for the brokerage and account
                    let account_profits = booked_profits
                        .entry(map.key().clone())
                        .or_insert_with(AHashMap::new);
                    *account_profits.entry(account_positions.key().clone()).or_insert(Decimal::ZERO) += profit; // Accumulate booked profit
                }
            }
        }

        booked_profits // Return the accumulated booked profits
    }
}
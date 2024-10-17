use std::fs::create_dir_all;
use std::path::Path;
use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;
use crate::standardized_types::enums::{PositionSide, StrategyMode};
use crate::standardized_types::subscriptions::SymbolName;
use dashmap::DashMap;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use csv::Writer;
use lazy_static::lazy_static;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal_macros::dec;
use crate::standardized_types::broker_enum::Brokerage;
use crate::standardized_types::position::{Position, PositionId};
use crate::standardized_types::symbol_info::SymbolInfo;
use serde_derive::{Deserialize, Serialize};
use crate::standardized_types::new_types::{Price, Volume};
use crate::strategies::handlers::market_handlers::SYMBOL_INFO;

lazy_static! {
    //todo get low res historical data and build currency conversion api
    static ref EARLIEST_CURRENCY_CONVERSIONS: DateTime<Utc> = Utc.with_ymd_and_hms(2010, 1, 1, 0, 0, 0).unwrap();
}

pub type AccountId = String;
pub type AccountName = String;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Serialize, Deserialize, PartialOrd, Copy)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum Currency {
    AUD,
    USD,
    CAD,
    EUR,
    JPY,
    USDT
}

impl Currency {
    pub fn from_str(string: &str) -> Self {
        match string {
            "AUD" => Currency::AUD,
            "USD" => Currency::USD,
            "CAD" => Currency::CAD,
            "EUR" => Currency::EUR,
            "JPY" => Currency::JPY,
            "USDT" => Currency::USDT,
            _ => panic!("No currency matching string, please implement")
        }
    }
}

/*todo,
   Make ledger take functions as params, create_order_reduce_position, increase_position, reduce_position, we will pass these in based on the type of positions we want.
   1. we can have a cumulative model, like we have now, for position building.
   2. FIFO: Each time we open an opposing order, the first position is cancelled out and returned as closed
   3. FILO: Each time we open a position, it is kept until we are flat or reversed and each counter order closes the order entry directly before.
   To achieve this ledgers will need to be able to hold multiple sub position details or positions themselves generate 'trade' objects
 */

pub struct AccountSetup {
    pub account_id: AccountId,
    pub brokerage: Brokerage,
    pub cash_value: Price,
    pub currency: Currency,
    pub size_limit: Option<Volume>,
    pub max_orders: Option<Volume>,
    pub daily_max_loss: Option<Price>,
    pub daily_max_loss_reset_hour: Option<u32>, //the hour of the day that the dail max loss resets
    pub daily_max_loss_reset_time_zone: Tz,
    leverage: Option<u32>
}
/// A ledger specific to the strategy which will ignore positions not related to the strategy but will update its balances relative to the actual account balances for live trading.
#[derive(Debug)]
pub struct Ledger {
    pub account_id: AccountId,
    pub brokerage: Brokerage,
    pub cash_value: Price,
    pub cash_available: Price,
    pub currency: Currency,
    pub cash_used: Price,
    pub positions: DashMap<SymbolName, Position>,
    pub margin_used: DashMap<SymbolName, Price>,
    pub positions_closed: DashMap<SymbolName, Vec<Position>>,
    pub positions_counter: DashMap<SymbolName, u64>,
    pub symbol_info: DashMap<SymbolName, SymbolInfo>,
    pub open_pnl: DashMap<SymbolName, Price>,
    pub booked_pnl: Price,
    pub mode: StrategyMode,
    pub leverage: u32,
    //todo, add daily max loss, max order size etc to ledger
}
impl Ledger {
    pub fn new(account_info: AccountInfo, strategy_mode: StrategyMode) -> Self {
        let positions = account_info
            .positions
            .into_iter()
            .map(|position| (position.symbol_name.clone(), position))
            .collect();

        let ledger = Self {
            account_id: account_info.account_id,
            brokerage: account_info.brokerage,
            cash_value: account_info.cash_value,
            cash_available: account_info.cash_available,
            currency: account_info.currency,
            cash_used: account_info.cash_used,
            positions,
            margin_used: Default::default(),
            positions_closed: DashMap::new(),
            positions_counter: DashMap::new(),
            symbol_info: DashMap::new(),
            open_pnl: DashMap::new(),
            booked_pnl: dec!(0.0),
            mode: strategy_mode,
            leverage: account_info.leverage
        };
        ledger
    }

    pub fn user_initiated(account_setup: AccountSetup, strategy_mode: StrategyMode) -> Self {
        //todo, add daily max loss, max order size etc to ledger
        let leverage = account_setup.leverage.unwrap_or_else(|| 1);
        let ledger = Self {
            account_id: account_setup.account_id,
            brokerage: account_setup.brokerage,
            cash_value: account_setup.cash_value,
            cash_available: account_setup.cash_value,
            currency: account_setup.currency,
            cash_used: dec!(0.0),
            positions: DashMap::new(),
            margin_used: DashMap::new(),
            positions_closed: DashMap::new(),
            positions_counter: DashMap::new(),
            symbol_info: DashMap::new(),
            open_pnl: DashMap::new(),
            booked_pnl: dec!(0.0),
            mode: strategy_mode,
            leverage
        };
        ledger
    }

    pub fn add_live_position(&self, mut position: Position) {
        //todo, check ledger max order etc before placing orders
        if let Some((symbol_name, mut existing_position)) = self.positions.remove(&position.symbol_name) {
            existing_position.is_closed = true;
            self.positions_closed
                .entry(symbol_name)
                .or_insert_with(Vec::new)
                .push(existing_position);
        }
        let id = self.generate_id(&position.symbol_name, position.side);
        position.position_id = id;
        self.positions.insert(position.symbol_name.clone(), position);
    }

    pub fn in_profit(&self, symbol_name: &SymbolName) -> bool {
        if let Some(position) = self.positions.get(symbol_name) {
            if position.value().open_pnl > dec!(0.0) {
                return true
            }
        }
        false
    }

    pub fn in_drawdown(&self, symbol_name: &SymbolName) -> bool {
        if let Some(position) = self.positions.get(symbol_name) {
            if position.value().open_pnl < dec!(0.0) {
                return true
            }
        }
        false
    }

    pub fn pnl(&self, symbol_name: &SymbolName) -> Decimal {
        if let Some(position) = self.positions.get(symbol_name) {
            return position.value().open_pnl.clone()
        }
        dec!(0)
    }

    pub fn position_size(&self, symbol_name: &SymbolName) -> Decimal {
        if let Some(position) = self.positions.get(symbol_name) {
            return position.value().quantity_open.clone()
        }
        dec!(0)
    }

    pub fn booked_pnl(&self, symbol_name: &SymbolName) -> Decimal {
        if let Some(position) = self.positions.get(symbol_name) {
            return position.value().booked_pnl.clone()
        }
        dec!(0)
    }

    // Function to export closed positions to CSV
    pub fn export_positions_to_csv(&self, folder: &str) {
        // Create the folder if it does not exist
        if let Err(e) = create_dir_all(folder) {
            eprintln!("Failed to create directory {}: {}", folder, e);
            return;
        }

        // Get current date in desired format
        let date = Utc::now().format("%Y%m%d_%H%M").to_string();

        // Use brokerage and account ID to format the filename
        let brokerage = self.brokerage.to_string(); // Assuming brokerage can be formatted as string
        let file_name = format!("{}/{:?}_Results_{}_{}_{}.csv", folder, self.mode ,brokerage, self.account_id, date);

        // Create a writer for the CSV file
        let file_path = Path::new(&file_name);
        match Writer::from_path(file_path) {
            Ok(mut wtr) => {
                // Iterate over all closed positions and write their data
                for entry in self.positions_closed.iter() {
                    for position in entry.value() {
                        let export = position.to_export(); // Assuming `to_export` provides a suitable data representation
                        if let Err(e) = wtr.serialize(export) {
                            eprintln!("Failed to write position data to {}: {}", file_path.display(), e);
                        }
                    }
                }

                // Ensure all data is flushed to the file
                if let Err(e) = wtr.flush() {
                    eprintln!("Failed to flush CSV writer for {}: {}", file_path.display(), e);
                } else {
                    println!("Successfully exported all positions to {}", file_path.display());
                }
            }
            Err(e) => {
                eprintln!("Failed to create CSV writer for {}: {}", file_path.display(), e);
            }
        }
    }

    pub async fn symbol_info(&self, symbol_name: &SymbolName) -> SymbolInfo {
        match self.symbol_info.get(symbol_name) {
            None => {
                match SYMBOL_INFO.get(symbol_name) {
                    None => {
                        let info =self.brokerage.symbol_info(symbol_name.clone()).await.unwrap();
                        self.symbol_info.insert(symbol_name.clone(), info.clone());
                        SYMBOL_INFO.insert(symbol_name.clone(), info.clone());
                        info
                    }
                    Some(info) => {
                        self.symbol_info.insert(symbol_name.clone(), info.clone());
                        info.value().clone()
                    }
                }
            }
            Some(info) => {
                info.value().clone()
            }
        }
    }

    pub fn get_open_pnl(&self) -> Price {
        let mut pnl = dec!(0);
        for price in self.open_pnl.iter() {
            pnl += price.value().clone()
        }
        pnl
    }

    pub fn is_long(&self, symbol_name: &SymbolName) -> bool {
        if let Some(position) = self.positions.get(symbol_name) {
            if position.value().side == PositionSide::Long {
                return true;
            }
        }
        false
    }

    pub fn is_short(&self, symbol_name: &SymbolName) -> bool {
        if let Some(position) = self.positions.get(symbol_name) {
            if position.value().side == PositionSide::Short {
                return true;
            }
        }
        false
    }

    pub fn is_flat(&self, symbol_name: &SymbolName) -> bool {
        if let Some(_) = self.positions.get(symbol_name) {
            false
        } else {
            true
        }
    }

    pub fn print(&self) -> String {
        let mut total_trades: usize = 0;
        let mut losses: usize = 0;
        let mut wins: usize = 0;
        let mut win_pnl = dec!(0.0);
        let mut loss_pnl = dec!(0.0);
        let mut pnl = dec!(0.0);

        for trades in self.positions_closed.iter() {
            total_trades += trades.value().len();
            for position in trades.value() {
                if position.booked_pnl > dec!(0.0) {
                    wins += 1;
                    win_pnl += position.booked_pnl;
                } else if position.booked_pnl < dec!(0.0) {
                    losses += 1;
                    loss_pnl += position.booked_pnl;
                }
                pnl += position.booked_pnl;
            }
        }

        // Calculate average win and average loss
        let avg_win_pnl = if wins > 0 {
            win_pnl / Decimal::from(wins) // Convert to Decimal type
        } else {
            dec!(0.0)
        };

        let avg_loss_pnl = if losses > 0 {
            loss_pnl / Decimal::from(losses) // Convert to Decimal type
        } else {
            dec!(0.0)
        };

        // Calculate risk-reward ratio
        let risk_reward = if losses == 0 && wins > 0 {
            Decimal::from(avg_win_pnl) // No losses, so risk/reward is considered infinite
        } else if wins == 0 && losses > 0 {
            dec!(0.0) // No wins, risk/reward is zero
        } else if avg_loss_pnl < dec!(0.0) && avg_win_pnl > dec!(0.0) {
            avg_win_pnl / -avg_loss_pnl // Negate loss_pnl for correct calculation
        } else {
            dec!(0.0)
        };

        let profit_factor = if loss_pnl != dec!(0.0) {
            win_pnl / -loss_pnl
        } else if win_pnl > dec!(0.0) {
            dec!(1000.0) // or use a defined constant for infinity
        } else {
            dec!(0.0) // when both win_pnl and loss_pnl are zero
        };

        let win_rate = if total_trades > 0 {
            (Decimal::from_usize(wins).unwrap() / Decimal::from_usize(total_trades).unwrap()) * dec!(100.0)
        } else {
            dec!(0.0)
        };

        let break_even = total_trades - wins - losses;
        format!("Brokerage: {}, Account: {}, Balance: {}, Win Rate: {}%, Average Risk Reward: {}, Profit Factor {}, Total profit: {}, Total Wins: {}, Total Losses: {}, Break Even: {}, Total Trades: {}, Open Positions: {}, Cash Used: {}, Cash Available: {}",
                self.brokerage, self.account_id, self.cash_value.round_dp(2), win_rate.round_dp(2), risk_reward.round_dp(2), profit_factor.round_dp(2), pnl.round_dp(2), wins, losses, break_even, total_trades, self.positions.len(), self.cash_used.round_dp(2), self.cash_available.round_dp(2))
    }

    pub fn generate_id(
        &self,
        symbol_name: &SymbolName,
        side: PositionSide
    ) -> PositionId {
        // Increment the counter for the symbol, or insert it if it doesn't exist
        let counter = self.positions_counter.entry(symbol_name.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1).value().clone();

        // Return the generated position ID
        format!("{}-{}-{}-{}-{}", self.brokerage, self.account_id, symbol_name, side, counter)
    }
}

pub(crate) mod historical_ledgers {
    use chrono::{DateTime, Utc};
    use dashmap::DashMap;
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromPrimitive;
    use rust_decimal_macros::dec;
    use crate::standardized_types::broker_enum::Brokerage;
    use crate::strategies::ledgers::{AccountId, Currency, Ledger};
    use crate::messages::data_server_messaging::FundForgeError;
    use crate::standardized_types::enums::{OrderSide, PositionSide, StrategyMode};
    use crate::standardized_types::position::{Position, PositionUpdateEvent};
    use crate::standardized_types::base_data::traits::BaseData;
    use crate::standardized_types::new_types::{Price, Volume};
    use crate::standardized_types::orders::{OrderId, OrderUpdateEvent};
    use crate::standardized_types::subscriptions::SymbolName;
    use crate::standardized_types::time_slices::TimeSlice;

    impl Ledger {
        pub async fn release_margin_used(&mut self, symbol_name: &SymbolName) {
            if let Some((_, margin_used)) = self.margin_used.remove(symbol_name) {
                self.cash_available += margin_used;
                self.cash_used -= margin_used;
            }
        }

        pub async fn commit_margin(&mut self, symbol_name: &SymbolName, quantity: Volume, market_price: Price) -> Result<(), FundForgeError> {
            //todo match time to market close and choose between intraday or overnight margin request.
            let margin = self.brokerage.intraday_margin_required(symbol_name.clone(), quantity).await?;
            let margin =match margin {
                None => {
                    (quantity * market_price) / Decimal::from_u32(self.leverage).unwrap()
                }
                Some(margin) => margin
            };
            // Check if the available cash is sufficient to cover the margin
            if self.cash_available < margin {
                return Err(FundForgeError::ClientSideErrorDebug("Insufficient funds".to_string()));
            }
            self.cash_available -= margin;
            self.cash_used += margin;
            Ok(())
        }

        pub fn paper_account_init(
            strategy_mode: StrategyMode,
            account_id: AccountId,
            brokerage: Brokerage,
            cash_value: Price,
            currency: Currency,
            leverage: u32
        ) -> Self {
            let account = Self {
                account_id,
                brokerage,
                cash_value,
                cash_available: cash_value,
                currency,
                leverage,
                cash_used: dec!(0.0),
                positions: DashMap::new(),
                positions_closed: DashMap::new(),
                positions_counter: DashMap::new(),
                margin_used: DashMap::new(),
                symbol_info: DashMap::new(),
                open_pnl: DashMap::new(),
                booked_pnl: dec!(0.0),
                mode: strategy_mode,
            };
            account
        }

        pub fn process_closed_position(&self, position: Position) {
            if !self.positions_closed.contains_key(&position.symbol_name) {
                self.positions_closed.insert(position.symbol_name.clone(), vec![position]);
            } else {
                if let Some(mut positions) = self.positions_closed.get_mut(&position.symbol_name) {
                    positions.value_mut().push(position);
                }
            }
        }

        pub fn timeslice_update(&mut self, time_slice: TimeSlice, time: DateTime<Utc>) {
            for base_data_enum in time_slice.iter() {
                let data_symbol_name = &base_data_enum.symbol().name;
                if let Some(mut position) = self.positions.get_mut(data_symbol_name) {
                    if position.is_closed {
                        continue
                    }
                    //returns booked pnl if exit on brackets
                    position.backtest_update_base_data(&base_data_enum, time, self.currency);
                    self.open_pnl.insert(data_symbol_name.clone(), position.open_pnl.clone());

                    if position.is_closed {
                        // Move the position to the closed positions map
                        let (symbol_name, position) = self.positions.remove(data_symbol_name).unwrap();
                        self.positions_closed
                            .entry(symbol_name)
                            .or_insert_with(Vec::new)
                            .push(position);
                    }
                }
            }
            if self.mode != StrategyMode::Live {
                self.cash_value = self.cash_used + self.cash_available;
            }
        }

        /// If Ok it will return a Position event for the successful position update, if the ledger rejects the order it will return an Err(OrderEvent)
        /// todo Need to handle creating a new opposing position when
        ///todo, check ledger max order etc before placing orders
        pub async fn update_or_create_position(
            &mut self,
            symbol_name: &SymbolName,
            order_id: OrderId,
            quantity: Volume,
            side: OrderSide,
            time: DateTime<Utc>,
            market_fill_price: Price, // we use the passed in price because we dont know what sort of order was filled, limit or market
            tag: String
        ) -> Result<Vec<PositionUpdateEvent>, OrderUpdateEvent> {
            let mut updates = vec![];
            // Check if there's an existing position for the given symbol
            let mut remaining_quantity = quantity;
            if let Some((symbol_name, mut existing_position)) = self.positions.remove(symbol_name) {

                let is_reducing = (existing_position.side == PositionSide::Long && side == OrderSide::Sell)
                    || (existing_position.side == PositionSide::Short && side == OrderSide::Buy);

                if is_reducing {
                    remaining_quantity -= existing_position.quantity_open;
                    let event= existing_position.reduce_paper_position_size(market_fill_price, quantity, time, tag.clone(), self.currency).await;

                    if self.mode != StrategyMode::Live {
                        self.release_margin_used(&symbol_name).await;
                    }

                    match &event {
                        PositionUpdateEvent::PositionReduced { booked_pnl, .. } => {
                            self.positions.insert(symbol_name, existing_position);

                            if self.mode != StrategyMode::Live {
                                self.cash_available += booked_pnl;
                            }
                        }
                        PositionUpdateEvent::PositionClosed { booked_pnl, .. } => {
                            self.booked_pnl += booked_pnl;

                            if self.mode != StrategyMode::Live {
                                self.cash_available += booked_pnl;
                            }
                            if !self.positions_closed.contains_key(&symbol_name) {
                                self.positions_closed.insert(symbol_name.clone(), vec![]);
                            }
                            if let Some(mut positions_closed) = self.positions_closed.get_mut(&symbol_name){
                                positions_closed.value_mut().push(existing_position);
                            }
                        }
                        _ => panic!("This shouldn't happen")
                    }

                    if self.mode != StrategyMode::Live {
                        self.cash_value = self.cash_used + self.cash_available;
                    }
                    updates.push(event);
                } else {
                    match self.commit_margin(&symbol_name, quantity, market_fill_price).await {
                        Ok(_) => {}
                        Err(e) => {
                            return Err(OrderUpdateEvent::OrderRejected {
                                brokerage: self.brokerage,
                                account_id: self.account_id.clone(),
                                order_id,
                                reason: e.to_string(),
                                tag,
                                time: time.to_string()
                            })
                        }
                    }
                    let event = existing_position.add_to_position(market_fill_price, quantity, time, tag.clone(), self.currency).await;
                    self.positions.insert(existing_position.symbol_name.clone(), existing_position);

                    if self.mode != StrategyMode::Live {
                        self.cash_value = self.cash_used + self.cash_available;
                    }

                    updates.push(event);
                    remaining_quantity = dec!(0.0);
                }
            }
            if remaining_quantity > dec!(0.0) {
                match self.commit_margin(&symbol_name, quantity, market_fill_price).await {
                    Ok(_) => {}
                    Err(e) => {
                        return Err(OrderUpdateEvent::OrderRejected {
                            brokerage: self.brokerage,
                            account_id: self.account_id.clone(),
                            order_id,
                            reason: e.to_string(),
                            tag: tag,
                            time: time.to_string()
                        })
                    }
                }

                // Determine the side of the position based on the order side
                let position_side = match side {
                    OrderSide::Buy => PositionSide::Long,
                    OrderSide::Sell => PositionSide::Short,
                };

                let info = self.symbol_info(symbol_name).await;
                let id = self.generate_id(symbol_name, position_side);
                // Create a new position
                let position = Position::new(
                    symbol_name.clone(),
                    self.brokerage.clone(),
                    self.account_id.clone(),
                    position_side,
                    remaining_quantity,
                    market_fill_price,
                    id.clone(),
                    info.clone(),
                    info.pnl_currency,
                    tag.clone(),
                    time,
                );

                // Insert the new position into the positions map
                self.positions.insert(symbol_name.clone(), position);
                if !self.positions_closed.contains_key(symbol_name) {
                    self.positions_closed.insert(symbol_name.clone(), vec![]);
                }

                let event = PositionUpdateEvent::PositionOpened{
                    position_id: id,
                    account_id: self.account_id.clone(),
                    brokerage: self.brokerage.clone(),
                    originating_order_tag: tag,
                    time: time.to_string()
                };

                if self.mode != StrategyMode::Live {
                    self.cash_value = self.cash_used + self.cash_available;
                }
                updates.push(event);
            }
            Ok(updates)
        }

        pub async fn exit_position_paper(
            &mut self,
            symbol_name: &SymbolName,
            time: DateTime<Utc>,
            market_price: Price,
            tag: String
        ) -> Option<PositionUpdateEvent>  {
            if let Some((symbol_name, mut existing_position)) = self.positions.remove(symbol_name) {
                // Mark the position as closed
                existing_position.is_closed = true;

                // Calculate booked profit by reducing the position size
                self.release_margin_used(&symbol_name).await;
                let event = existing_position.reduce_paper_position_size(market_price, existing_position.quantity_open, time, tag, self.currency).await;
                match &event {
                    PositionUpdateEvent::PositionClosed { booked_pnl,.. } => {
                        self.booked_pnl += booked_pnl;
                        self.cash_available += booked_pnl;
                    }
                    _ => panic!("this shouldn't happen")
                }

                self.cash_value = self.cash_used + self.cash_available;
                // Add the closed position to the positions_closed DashMap
                self.positions_closed
                    .entry(symbol_name.clone())                  // Access the entry for the symbol name
                    .or_insert_with(Vec::new)                    // If no entry exists, create a new Vec
                    .push(existing_position);     // Push the closed position to the Vec

                return Some(event)
            }
            None
        }
    }
}

pub enum RiskLimitStatus {
    Ok,
    Hit
}
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct AccountInfo {
    pub account_id: AccountId,
    pub brokerage: Brokerage,
    pub cash_value: Price,
    pub cash_available: Price,
    pub currency: Currency,
    pub open_pnl: Decimal,
    pub booked_pnl: Decimal,
    pub day_open_pnl: Decimal,
    pub day_booked_pnl: Decimal,
    pub cash_used: Price,
    pub positions: Vec<Position>,
    pub is_hedging: bool,
    pub leverage: u32,
    pub buy_limit: Option<Volume>,
    pub sell_limit: Option<Volume>,
    pub max_orders: Option<Volume>,
    pub daily_max_loss: Option<Price>,
    pub daily_max_loss_reset_time: Option<String>
}
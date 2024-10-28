use dashmap::DashMap;
use tokio::sync::Mutex;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};
use std::fs::create_dir_all;
use std::path::Path;
use csv::Writer;
use std::sync::Arc;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal_macros::dec;
use crate::standardized_types::accounts::{Account, AccountInfo, Currency};
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::broker_enum::Brokerage;
use crate::standardized_types::enums::{OrderSide, PositionSide, StrategyMode};
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::orders::Order;
use crate::standardized_types::position::{Position, PositionId, PositionUpdateEvent};
use crate::standardized_types::subscriptions::{SymbolCode, SymbolName};
use crate::standardized_types::symbol_info::SymbolInfo;
use crate::standardized_types::time_slices::TimeSlice;

/// A ledger specific to the strategy which will ignore positions not related to the strategy but will update its balances relative to the actual account balances for live trading.
#[derive(Debug)]
pub struct Ledger {
    pub account: Account,
    pub cash_value: Mutex<Price>,
    pub cash_available: Mutex<Price>,
    pub currency: Currency,
    pub cash_used: Mutex<Price>,
    pub positions: DashMap<SymbolCode, Position>,
    pub last_update: DashMap<SymbolCode, DateTime<Utc>>,
    pub symbol_code_map: DashMap<SymbolName, Vec<String>>,
    pub margin_used: DashMap<SymbolCode, Price>,
    pub positions_closed: DashMap<SymbolCode, Vec<Position>>,
    pub symbol_closed_pnl: DashMap<SymbolCode, Decimal>,
    pub positions_counter: DashMap<SymbolName, u64>,
    pub(crate) symbol_info: DashMap<SymbolName, SymbolInfo>,
    pub open_pnl: DashMap<SymbolCode, Price>,
    pub total_booked_pnl: Mutex<Price>,
    pub mode: StrategyMode,
    pub leverage: u32,
    pub is_simulating_pnl: bool,
    //todo, add daily max loss, max order size etc to ledger
}

impl Ledger {
    pub(crate) fn new(
        account_info: AccountInfo,
        mode: StrategyMode,
        synchronise_accounts: bool,
    ) -> Self {
        let is_simulating_pnl = match synchronise_accounts {
            true => false,
            false => true
        };
        println!("Ledger Created: {} {}", account_info.brokerage, account_info.account_id);
        let positions: DashMap<SymbolName, Position> = account_info
            .positions
            .into_iter()
            .map(|position| (position.symbol_code.clone(), position))
            .collect();

        let contract_map: DashMap<SymbolName, Vec<String>> = DashMap::new();
        for position in positions.iter() {
            contract_map
                .entry(position.value().symbol_name.clone())
                .or_insert_with(Vec::new)
                .push(position.value().symbol_code.clone());
        }

        let ledger = Self {
            account: Account::new(account_info.brokerage, account_info.account_id),
            cash_value: Mutex::new(account_info.cash_value),
            cash_available: Mutex::new(account_info.cash_available),
            currency: account_info.currency,
            cash_used: Mutex::new(account_info.cash_used),
            positions,
            last_update: Default::default(),
            symbol_code_map: contract_map,
            margin_used: Default::default(),
            positions_closed: DashMap::new(),
            symbol_closed_pnl: Default::default(),
            positions_counter: DashMap::new(),
            symbol_info: DashMap::new(),
            open_pnl: DashMap::new(),
            total_booked_pnl: Mutex::new(dec!(0)),
            mode,
            leverage: account_info.leverage,
            is_simulating_pnl
        };
        ledger
    }

    pub async fn update(&mut self, cash_value: Decimal, cash_available: Decimal, cash_used: Decimal) {
        let mut account_cash_value= self.cash_value.lock().await;
        *account_cash_value = cash_value;

        let mut account_cash_used= self.cash_used.lock().await;
        *account_cash_used = cash_used;

        let mut account_cash_available= self.cash_available.lock().await;
        *account_cash_available = cash_available;
    }

    pub fn synchronize_live_position(&self, position: Position, time: DateTime<Utc>) -> Option<PositionUpdateEvent> {
        if let Some(last_update) = self.last_update.get(&position.symbol_code) {
            if last_update.value() > &time {
                return None;
            }
        }
        self.last_update.insert(position.symbol_code.clone(), time);

        if position.is_closed {
            // If position is closed, remove it and don't try to create a new one
            self.positions.remove(&position.symbol_code);
            self.positions_closed.entry(position.symbol_code.clone())
                .or_insert(vec![])
                .push(position.clone());

            let close_time = position.close_time.unwrap_or_else(|| Utc::now().to_string());

            println!("Average Exit Price: {:?}", position.average_exit_price);
            return Some(PositionUpdateEvent::PositionClosed {
                symbol_name: position.symbol_name.clone(),
                side: position.side.clone(),
                symbol_code: position.symbol_code.clone(),
                position_id: position.position_id.clone(),
                total_quantity_open: dec!(0),
                total_quantity_closed: position.quantity_closed,
                average_price: position.average_price,
                booked_pnl: Default::default(),
                average_exit_price: position.average_exit_price,
                account: self.account.clone(),
                originating_order_tag: position.tag,
                time: close_time
            });
        }

        // Only handle open position updates if quantity > 0
        if position.quantity_open > dec!(0) {
            if let Some(mut existing) = self.positions.get_mut(&position.symbol_code) {
                // Update existing position
                existing.quantity_open = position.quantity_open;
                existing.open_pnl = position.open_pnl;
                existing.side = position.side;
                existing.average_price = position.average_price;
                None
            } else {
                // Create new position
                self.positions.insert(position.symbol_code.clone(), position.clone());
                Some(PositionUpdateEvent::PositionOpened {
                    side: position.side,
                    symbol_name: position.symbol_name,
                    symbol_code: position.symbol_code,
                    position_id: position.position_id,
                    account: self.account.clone(),
                    originating_order_tag: position.tag,
                    time: position.open_time
                })
            }
        } else {
            None
        }
    }

    pub async fn process_synchronized_orders(&self, order: Order, quantity: Decimal, time: DateTime<Utc>) {
        let symbol = match order.symbol_code {
            None => order.symbol_name,
            Some(code) => code
        };

        if let Some(last_update) = self.last_update.get(&symbol) {
            if last_update.value() > &time {
                return;
            }
        }
        self.last_update.insert(symbol.clone(), time);

        self.last_update.insert(symbol.clone(), time);

        if let Some(mut position) = self.positions.get_mut(&symbol) {
            let is_reducing = (position.side == PositionSide::Long && order.side == OrderSide::Sell)
                || (position.side == PositionSide::Short && order.side == OrderSide::Buy);

            if !is_reducing {
                return;
            }

            let market_fill_price = match order.average_fill_price {
                None => return,
                Some(price) => price
            };

            position.reduce_position_size(market_fill_price, quantity, Utc::now(), order.tag.clone(), self.currency).await;
        } else if let Some(mut position_vec) = self.positions_closed.get_mut(&symbol) {
            if let Some(last_position) = position_vec.last_mut() {
                let is_reducing = (last_position.side == PositionSide::Long && order.side == OrderSide::Sell)
                    || (last_position.side == PositionSide::Short && order.side == OrderSide::Buy);

                if !is_reducing {
                    return;
                }

                let market_fill_price = match order.average_fill_price {
                    None => return,
                    Some(price) => price
                };

                last_position.reduce_position_size(market_fill_price, quantity, Utc::now(), order.tag.clone(), self.currency).await;
            }
        }
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
        let brokerage = self.account.brokerage.to_string(); // Assuming brokerage can be formatted as string
        let file_name = format!("{}/{:?}_Results_{}_{}_{}.csv", folder, self.mode ,brokerage, self.account.account_id, date);

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

    pub async fn symbol_info(&self, brokerage: Brokerage, symbol_name: &SymbolName) -> SymbolInfo {
        match self.symbol_info.get(symbol_name) {
            None => match brokerage.symbol_info(symbol_name.clone()).await {
                Ok(info) => info,
                Err(e) => panic!("Ledgers: Error getting symbol infor: {}, {}: {}", brokerage, symbol_name, e)
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
                return true
            }
        }
       false
    }

    pub fn is_flat(&self, symbol_name: &SymbolName) -> bool {
        if let Some(_) = self.positions.get(symbol_name) {
            return false;
        }
        true
    }

    pub async fn print(&self) -> String {
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
        let cash_value = self.cash_value.lock().await.clone();
        let cash_used = self.cash_used.lock().await.clone();
        let cash_available = self.cash_available.lock().await.clone();
        format!("Account: {}, Balance: {}, Win Rate: {}%, Average Risk Reward: {}, Profit Factor {}, Total profit: {}, Total Wins: {}, Total Losses: {}, Break Even: {}, Total Trades: {}, Open Positions: {}, Cash Used: {}, Cash Available: {}",
                self.account, cash_value.round_dp(2), win_rate.round_dp(2), risk_reward.round_dp(2), profit_factor.round_dp(2), pnl.round_dp(2), wins, losses, break_even, total_trades, self.positions.len(), cash_used.round_dp(2), cash_available.round_dp(2))
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
        format!("{}-{}-{}-{}-{}", self.account.brokerage, self.account.account_id, counter, symbol_name, side)
    }

    pub async fn timeslice_update(&self, time_slice: Arc<TimeSlice>, time: DateTime<Utc>) {
        for base_data_enum in time_slice.iter() {
            let data_symbol_name = &base_data_enum.symbol().name;
            if let Some(codes) = self.symbol_code_map.get(data_symbol_name) {
                for code in codes.value() {
                    if let Some(mut position) = self.positions.get_mut(code) {
                        let open_pnl = position.update_base_data(&base_data_enum, time, self.currency);
                        self.open_pnl.insert(data_symbol_name.clone(), open_pnl);
                    }
                }
            }
            if let Some(mut position) = self.positions.get_mut(data_symbol_name) {
                if position.is_closed {
                    continue
                }

                if self.mode != StrategyMode::Live || self.is_simulating_pnl {
                    let open_pnl = position.update_base_data(&base_data_enum, time, self.currency);
                    self.open_pnl.insert(data_symbol_name.clone(), open_pnl);


                }

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
            let mut cash_value = self.cash_value.lock().await;
            let cash_used = self.cash_used.lock().await;
            let cash_available = self.cash_available.lock().await;
            *cash_value = *cash_used + *cash_available;
        }
    }

    pub(crate) async fn update_or_create_live_position(
        &self,
        symbol_name: SymbolName,
        symbol_code: SymbolCode,
        quantity: Volume,
        side: OrderSide,
        time: DateTime<Utc>,
        market_fill_price: Price,
        tag: String
    ) -> Vec<PositionUpdateEvent> {
        if let Some(last_update) = self.last_update.get(&symbol_code) {
            if last_update.value() > &time {
                return vec![];
            }
        }
        self.last_update.insert(symbol_code.clone(), time);

        let mut position_events = vec![];
        // Check if there's an existing position for the given symbol
        let mut remaining_quantity = quantity;
        if let Some((_, mut existing_position)) = self.positions.remove(&symbol_code) {
            let is_reducing = (existing_position.side == PositionSide::Long && side == OrderSide::Sell)
                || (existing_position.side == PositionSide::Short && side == OrderSide::Buy);

            if is_reducing {
                remaining_quantity -= existing_position.quantity_open;
                let event= existing_position.reduce_position_size(market_fill_price, quantity, time, tag.clone(), self.currency).await;
                match &event {
                    PositionUpdateEvent::PositionReduced { booked_pnl, .. } => {
                        self.positions.insert(symbol_code.clone(), existing_position);

                        // TODO[Strategy]: Add option to mirror account position or use internal position curating.
                        if self.is_simulating_pnl {
                            self.symbol_closed_pnl
                                .entry(symbol_code.clone())
                                .and_modify(|pnl| *pnl += booked_pnl)
                                .or_insert(booked_pnl.clone());
                            let mut total_booked_pnl = self.total_booked_pnl.lock().await;
                            *total_booked_pnl += booked_pnl;
                        }
                        //println!("Reduced Position: {}", symbol_name);
                    }
                    PositionUpdateEvent::PositionClosed { booked_pnl, .. } => {
                        // TODO[Strategy]: Add option to mirror account position or use internal position curating.
                        if self.is_simulating_pnl {
                            self.symbol_closed_pnl
                                .entry(symbol_code.clone())
                                .and_modify(|pnl| *pnl += booked_pnl)
                                .or_insert(booked_pnl.clone());

                            let mut total_booked_pnl = self.total_booked_pnl.lock().await;
                            *total_booked_pnl += booked_pnl;
                        }
                        if !self.positions_closed.contains_key(&symbol_code) {
                            self.positions_closed.insert(symbol_code.clone(), vec![]);
                        }
                        if let Some(mut positions_closed) = self.positions_closed.get_mut(&symbol_code){
                            positions_closed.value_mut().push(existing_position);
                        }
                        //println!("Closed Position: {}", symbol_name);
                    }
                    _ => panic!("This shouldn't happen")
                }
                position_events.push(event);
            } else {
                let event = existing_position.add_to_position(self.mode, self.is_simulating_pnl, market_fill_price, quantity, time, tag.clone(), self.currency).await;
                self.positions.insert(symbol_code.clone(), existing_position);

                position_events.push(event);
                remaining_quantity = dec!(0.0);
            }
        }
        if remaining_quantity > dec!(0.0) {
            // Determine the side of the position based on the order side
            let position_side = match side {
                OrderSide::Buy => PositionSide::Long,
                OrderSide::Sell => PositionSide::Short,
            };

            let info = self.symbol_info(self.account.brokerage, &symbol_name).await;
            if symbol_name != symbol_code && !self.symbol_code_map.contains_key(&symbol_name)   {
                self.symbol_code_map.insert(symbol_name.clone(), vec![]);
            };
            if symbol_name != symbol_code {
                if let Some(mut code_map) = self.symbol_code_map.get_mut(&symbol_name) {
                    if !code_map.contains(&symbol_code) {
                        code_map.value_mut().push(symbol_code.clone());
                    }
                }
            }

            let id = self.generate_id(&symbol_name, position_side);
            // Create a new position
            let position = Position::new(
                symbol_code.clone(),
                symbol_code.clone(),
                self.account.clone(),
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
            self.positions.insert(symbol_code.clone(), position);
            if !self.positions_closed.contains_key(&symbol_code) {
                self.positions_closed.insert(symbol_code.clone(), vec![]);
            }
            if symbol_name != symbol_code {
                self.symbol_code_map.entry(symbol_name.clone()).or_insert(vec![]).push(symbol_code.clone());
            }

            let event = PositionUpdateEvent::PositionOpened {
                symbol_name: symbol_name.clone(),
                symbol_code: symbol_code.clone(),
                position_id: id,
                side: position_side,
                account: self.account.clone(),
                originating_order_tag: tag,
                time: time.to_string()
            };

            //println!("{:?}", event);
            position_events.push(event);
        }
        position_events
    }
}
use std::collections::HashMap;
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
use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;
use crate::product_maps::oanda::maps::OANDA_SYMBOL_INFO;
use crate::product_maps::rithmic::maps::{find_base_symbol, get_symbol_info};
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
use crate::strategies::client_features::other_requests::get_exchange_rate;
use crate::strategies::strategy_events::StrategyEvent;

/*
 The ledger could be split into event driven components
 - We could have an static dashmap symbol name atomic bool, for is long is short etc, this can be updated in an event loop that manages position updates
 - The ledger itself doesnt need to exist,
*/

pub enum LedgerMessage {
    SyncPosition{position: Position, time: DateTime<Utc>},
    ProcessOrder{order: Order, quantity: Decimal, time: DateTime<Utc>},
    UpdateOrCreatePosition{symbol_name: SymbolName, symbol_code: SymbolCode, quantity: Volume, side: OrderSide, time: DateTime<Utc>, market_fill_price: Price, tag: String}
}
/// A ledger specific to the strategy which will ignore positions not related to the strategy but will update its balances relative to the actual account balances for live trading.
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
    pub(crate) symbol_info: DashMap<SymbolName, SymbolInfo>,
    pub open_pnl: DashMap<SymbolCode, Price>,
    pub total_booked_pnl: Mutex<Price>,
    pub mode: StrategyMode,
    pub is_simulating_pnl: bool,
    pub(crate) strategy_sender: Sender<StrategyEvent>,
    pub leverage: Decimal,
    pub rates: Arc<DashMap<Currency, Decimal>>,
    //todo, add daily max loss, max order size etc to ledger
}

impl Ledger {
    pub(crate) fn new(
        account_info: AccountInfo,
        mode: StrategyMode,
        synchronise_accounts: bool,
        strategy_sender: Sender<StrategyEvent>
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
            symbol_info: DashMap::new(),
            open_pnl: DashMap::new(),
            total_booked_pnl: Mutex::new(dec!(0)),
            mode,
            is_simulating_pnl,
            strategy_sender,
            leverage: Decimal::from(account_info.leverage),
            rates: Arc::new(Default::default()),
        };
        ledger
    }

    pub fn update_rates(&self, rates: &HashMap<Currency, Decimal>) {
        for (c1, c2) in rates.iter() {
            self.rates.insert(c1.clone(), c2.clone());
        }
    }

    pub fn get_exchange_multiplier(&self, to_currency: Currency) -> Decimal {
        if self.currency == to_currency {
            return dec!(1.0);
        }

        // Try direct rate
        if let Some(rate) = self.rates.get(&to_currency) {
            return *rate;
        }

        // Try inverse rate
        if let Some(rate) = self.rates.get(&to_currency) {
            return dec!(1.0) / *rate;
        }

        // Default to 1.0 if rate not found
        dec!(1.0)
    }

    pub async fn update(&mut self, cash_value: Decimal, cash_available: Decimal, cash_used: Decimal) {
        let mut account_cash_value= self.cash_value.lock().await;
        *account_cash_value = cash_value;

        let mut account_cash_used= self.cash_used.lock().await;
        *account_cash_used = cash_used;

        let mut account_cash_available= self.cash_available.lock().await;
        *account_cash_available = cash_available;
    }


    pub fn live_ledger_updates(ledger: Arc<Ledger>, mut receiver: Receiver<LedgerMessage>) {
        tokio::spawn(async move {
            let mut last_time = Utc::now();
            while let Some(message) = receiver.recv().await {
                match message {
                    LedgerMessage::SyncPosition { position, time } => {
                        if time < last_time {
                            continue;
                        }
                        if !ledger.is_simulating_pnl {
                            ledger.synchronize_live_position(position, time).await;
                        }
                        last_time = time;
                    }
                    LedgerMessage::ProcessOrder { .. } => {
                        //ledger.process_synchronized_orders(order, quantity, time).await; //todo, doesnt work
                    }
                    LedgerMessage::UpdateOrCreatePosition { symbol_name, symbol_code, quantity, side, time, market_fill_price, tag } => {
                        let updates = ledger.update_or_create_live_position(symbol_name, symbol_code, quantity, side, time, market_fill_price, tag).await;
                        for update in updates {
                            match ledger.strategy_sender.send(StrategyEvent::PositionEvents(update)).await {
                                Ok(_) => {}
                                Err(e) => eprintln!("Error sending position event: {}", e)
                            }
                        }
                        last_time = time;
                    }
                }
            }
        });
    }

    async fn synchronize_live_position(&self, position: Position, time: DateTime<Utc>) {
        if let Some(mut existing_position) = self.positions.get_mut(&position.symbol_code) {
            if existing_position.side != position.side {
                let side = match existing_position.side {
                    PositionSide::Long => OrderSide::Buy,
                    PositionSide::Short => OrderSide::Sell,
                };
                let exchange_rate = if self.currency != existing_position.symbol_info.pnl_currency {
                    match get_exchange_rate(self.currency, existing_position.symbol_info.pnl_currency, time, side).await {
                        Ok(rate) => {
                            self.rates.insert(existing_position.symbol_info.pnl_currency, rate);
                            rate
                        },
                        Err(_e) => self.get_exchange_multiplier(existing_position.symbol_info.pnl_currency)
                    }
                } else {
                    dec!(1.0)
                };
                existing_position.reduce_position_size(position.average_price, existing_position.quantity_open, exchange_rate, time, "Synchronizing".to_string()).await;
                let closed_event = StrategyEvent::PositionEvents(PositionUpdateEvent::PositionClosed {
                    position_id: existing_position.position_id.clone(),
                    side: existing_position.side,
                    symbol_name: existing_position.symbol_name.clone(),
                    symbol_code: existing_position.symbol_code.clone(),
                    total_quantity_open: existing_position.quantity_open.clone(),
                    total_quantity_closed: existing_position.quantity_closed.clone(),
                    average_price: existing_position.average_price.clone(),
                    booked_pnl: existing_position.booked_pnl.clone(),
                    average_exit_price: existing_position.average_exit_price,
                    account: existing_position.account.clone(),
                    originating_order_tag: "".to_string(),
                    time: "".to_string(),
                });
                match self.strategy_sender.send(closed_event).await {
                    Ok(_) => {}
                    Err(e) => eprintln!("Error sending position event: {}", e)
                }

                self.positions_closed
                    .entry(position.symbol_code.clone())
                    .or_insert_with(Vec::new)
                    .push(existing_position.value().clone());
            }
        }
        self.positions.insert(position.symbol_code.clone(), position.clone());
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
        match self.mode {
            StrategyMode::Backtest => {
                match brokerage {
                    Brokerage::Rithmic(_) => {
                        if let Some(symbol) = find_base_symbol(symbol_name.clone()) {
                            return match get_symbol_info(&symbol) {
                                Ok(info) => info,
                                Err(_) => panic!("Ledgers: Error getting symbol info: {}, {}", brokerage, symbol_name),
                            };
                        }
                    }
                    Brokerage::Oanda => {
                        if let Some(info) = OANDA_SYMBOL_INFO.get(symbol_name) {
                            return info.clone();
                        } else {
                            panic!("Ledgers: Error getting symbol info: {}, {}", brokerage, symbol_name);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        match self.symbol_info.get(symbol_name) {
            None => match brokerage.symbol_info(symbol_name.clone()).await {
                Ok(info) => info,
                Err(e) => panic!("Ledgers: Error getting symbol info: {}, {}: {}", brokerage, symbol_name, e),
            },
            Some(info) => info.value().clone(),
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
        side: PositionSide
    ) -> PositionId {
        // Generate a UUID v4 (random)
        let guid = Uuid::new_v4();

        // Return the generated position ID with GUID
        format!(
            "{}-{}",
            side,
            guid.as_simple()
        )
    }

    pub async fn timeslice_update(&self, time_slice: Arc<TimeSlice>) {
        for base_data_enum in time_slice.iter() {
            let data_symbol_name = &base_data_enum.symbol().name;
            if let Some(codes) = self.symbol_code_map.get(data_symbol_name) {
                for code in codes.value() {
                    if let Some(mut position) = self.positions.get_mut(code) {
                        let open_pnl = position.update_base_data(&base_data_enum);
                        self.open_pnl.insert(data_symbol_name.clone(), open_pnl);
                    }
                }
            }
            if let Some(mut position) = self.positions.get_mut(data_symbol_name) {
                if position.is_closed {
                    continue
                }

                if self.mode != StrategyMode::Live || self.is_simulating_pnl {
                    let open_pnl = position.update_base_data(&base_data_enum);
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

    async fn update_or_create_live_position(
        &self,
        symbol_name: SymbolName,
        symbol_code: SymbolCode,
        quantity: Volume,
        side: OrderSide,
        time: DateTime<Utc>,
        market_fill_price: Price,
        tag: String
    ) -> Vec<PositionUpdateEvent> {
    /*    if let Some(last_update) = self.last_update.get(&symbol_code) {
            if last_update.value() > &time {
                return vec![];
            }
        }*/
        self.last_update.insert(symbol_code.clone(), time);

        let mut position_events = vec![];
        // Check if there's an existing position for the given symbol
        let mut remaining_quantity = quantity;
        if let Some((_, mut existing_position)) = self.positions.remove(&symbol_code) {
            let is_reducing = (existing_position.side == PositionSide::Long && side == OrderSide::Sell)
                || (existing_position.side == PositionSide::Short && side == OrderSide::Buy);

            if is_reducing {
                remaining_quantity -= existing_position.quantity_open;
                let exchange_rate = if self.currency != existing_position.symbol_info.pnl_currency {
                    match get_exchange_rate(self.currency, existing_position.symbol_info.pnl_currency, time, side).await {
                        Ok(rate) => {
                            self.rates.insert(existing_position.symbol_info.pnl_currency, rate);
                            rate
                        },
                        Err(_e) => self.get_exchange_multiplier(existing_position.symbol_info.pnl_currency)
                    }
                } else {
                    dec!(1.0)
                };
                let event= existing_position.reduce_position_size(market_fill_price, quantity, exchange_rate, time, tag.clone()).await;
                match &event {
                    PositionUpdateEvent::PositionReduced { booked_pnl, .. } => {
                        self.positions.insert(symbol_code.clone(), existing_position);
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
                let event = existing_position.add_to_position(self.mode, self.is_simulating_pnl, market_fill_price, quantity, time, tag.clone()).await;
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
            let exchange_rate = if self.currency != info.pnl_currency {
                match get_exchange_rate(self.currency, info.pnl_currency, time, side).await {
                    Ok(rate) => {
                        self.rates.insert(info.pnl_currency, rate);
                        rate
                    },
                    Err(_e) => self.get_exchange_multiplier(info.pnl_currency)
                }
            } else {
                dec!(1.0)
            };

            let id = self.generate_id(position_side);
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
                exchange_rate,
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
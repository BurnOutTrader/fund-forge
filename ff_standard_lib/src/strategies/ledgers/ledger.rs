use dashmap::DashMap;
use tokio::sync::{oneshot};
use rust_decimal::Decimal;
use chrono::{DateTime, Duration, Utc};
use std::fs::create_dir_all;
use std::path::Path;
use std::str::FromStr;
use csv::Writer;
use std::sync::Arc;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal_macros::dec;
use serde_derive::Serialize;
use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;
use crate::helpers::converters::format_duration;
use crate::product_maps::oanda::maps::OANDA_SYMBOL_INFO;
use crate::product_maps::rithmic::maps::{find_base_symbol, get_futures_symbol_info};
use crate::standardized_types::accounts::{Account, AccountInfo, Currency};
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::broker_enum::Brokerage;
use crate::standardized_types::enums::{OrderSide, PositionSide, StrategyMode};
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::orders::{OrderId, OrderUpdateEvent};
use crate::standardized_types::position::{Position, PositionCalculationMode, PositionId, PositionUpdateEvent};
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

#[derive(Debug)]
pub enum LedgerMessage {
    SyncPosition{position: Position, time: DateTime<Utc>},
    UpdateOrCreatePosition{symbol_name: SymbolName, symbol_code: SymbolCode, quantity: Volume, side: OrderSide, time: DateTime<Utc>, market_fill_price: Price, tag: String, paper_response_sender: Option<oneshot::Sender<Option<OrderUpdateEvent>>>, order_id: Option<OrderId>},
    TimeSliceUpdate{time_slice: Arc<TimeSlice>},
    LiveAccountUpdate{cash_value: Decimal, cash_available: Decimal, cash_used: Decimal},
    ExitPaperPosition{symbol_code: SymbolCode, time: DateTime<Utc>, market_fill_price: Price, tag: String},
    PaperFlattenAll{time: DateTime<Utc>},
}

/// A ledger specific to the strategy which will ignore positions not related to the strategy but will update its balances relative to the actual account balances for live trading.
pub struct Ledger {
    pub account: Account,
    pub cash_value: Price,
    pub cash_available: Price,
    pub currency: Currency,
    pub cash_used: Price,
    pub positions: DashMap<SymbolCode, Position>,
    pub last_update: DashMap<SymbolCode, DateTime<Utc>>,
    pub symbol_code_map: DashMap<SymbolName, Vec<String>>,
    pub margin_used: DashMap<SymbolCode, Price>,
    pub positions_closed: DashMap<SymbolCode, Vec<Position>>,
    pub symbol_closed_pnl: DashMap<SymbolCode, Decimal>,
    pub(crate) symbol_info: DashMap<SymbolName, SymbolInfo>,
    pub open_pnl: DashMap<SymbolCode, Price>,
    pub total_booked_pnl: Price,
    pub commissions_paid: Decimal,
    pub mode: StrategyMode,
    pub is_simulating_pnl: bool,
    pub(crate) strategy_sender: Sender<StrategyEvent>,
    pub rates: Arc<DashMap<Currency, Decimal>>,
    pub position_calculation_mode: PositionCalculationMode
    //todo, add daily max loss, max order size etc to ledger
}

impl Ledger {
    pub(crate) fn new(
        account_info: AccountInfo,
        mode: StrategyMode,
        synchronise_accounts: bool,
        strategy_sender: Sender<StrategyEvent>,
        position_calculation_mode: PositionCalculationMode
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
            cash_value: account_info.cash_value,
            cash_available: account_info.cash_available,
            currency: account_info.currency,
            cash_used: account_info.cash_used,
            positions,
            last_update: Default::default(),
            symbol_code_map: contract_map,
            margin_used: Default::default(),
            positions_closed: DashMap::new(),
            symbol_closed_pnl: Default::default(),
            symbol_info: DashMap::new(),
            open_pnl: DashMap::new(),
            total_booked_pnl: dec!(0),
            commissions_paid: Default::default(),
            mode,
            is_simulating_pnl,
            strategy_sender,
            rates: Arc::new(Default::default()),
            position_calculation_mode
        };
        ledger
    }

    /// Used to save positions to disk in json format
    /// Useful for machine learning
    pub fn save_positions_to_file(&self, file: &str) {
        let mut positions = vec![];
        for position in self.positions.iter() {
            positions.push(position.value().clone());
        }
        let positions = serde_json::to_string(&positions).unwrap();
        std::fs::write(file, positions).unwrap();
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

    pub fn balance(&self) -> Decimal {
        self.cash_used + self.cash_available
    }

    pub fn update(&mut self, cash_value: Decimal, cash_available: Decimal, cash_used: Decimal) {
        self.cash_value = cash_value;
        self.cash_used = cash_used;
        self.cash_available = cash_available;
    }

    pub fn ledger_updates(&mut self, mut receiver: Receiver<LedgerMessage>, mode: StrategyMode) {
        let static_self = unsafe {
            &mut *(self as *mut Ledger)
        };
        tokio::spawn(async move {
            while let Some(message) = receiver.recv().await {
                match message {
                    LedgerMessage::SyncPosition { position, time } => {
                        if !static_self.is_simulating_pnl {
                            static_self.synchronize_live_position(position, time).await;
                        }
                    }
                    LedgerMessage::UpdateOrCreatePosition { symbol_name, symbol_code, quantity, side, time, market_fill_price, tag , paper_response_sender, order_id} => {
                        match mode {
                            StrategyMode::Backtest | StrategyMode::LivePaperTrading => static_self.update_or_create_paper_position(symbol_name, symbol_code, quantity, side, time, market_fill_price, tag, order_id.unwrap(), paper_response_sender.unwrap()).await,
                            StrategyMode::Live => {
                                if static_self.is_simulating_pnl {
                                    static_self.update_or_create_live_position(symbol_name, symbol_code, quantity, side, time, market_fill_price, tag).await
                                }
                            }
                        };
                    }
                    LedgerMessage::TimeSliceUpdate { time_slice } => {
                        static_self.timeslice_update(time_slice).await;
                    }
                    LedgerMessage::LiveAccountUpdate { cash_value, cash_available, cash_used } => {
                        static_self.update(cash_value, cash_available, cash_used);
                    }
                    LedgerMessage::ExitPaperPosition { symbol_code, time, market_fill_price, tag } => {
                        static_self.paper_exit_position(&symbol_code, time, market_fill_price, tag).await;
                    }
                    LedgerMessage::PaperFlattenAll { time } => {
                        static_self.flatten_all_for_paper_account(time).await;
                    }
                }
            }
        });
    }

    async fn synchronize_live_position(&self, mut position: Position, time: DateTime<Utc>) {
        if let Some((_, mut existing_position)) = self.positions.remove(&position.symbol_code) {
            if existing_position.side == position.side && existing_position.quantity_open == existing_position.quantity_open {
                return;
            }
            // Check if the position side has changed
            if existing_position.side != position.side {
                // Close the existing position
                let side = match existing_position.side {
                    PositionSide::Long => OrderSide::Buy,
                    PositionSide::Short => OrderSide::Sell,
                };
                let exchange_rate = if self.currency != existing_position.symbol_info.pnl_currency {
                    match get_exchange_rate(
                        self.currency,
                        existing_position.symbol_info.pnl_currency,
                        time,
                        side,
                    )
                        .await
                    {
                        Ok(rate) => {
                            self.rates.insert(existing_position.symbol_info.pnl_currency, rate);
                            rate
                        }
                        Err(_) => self.get_exchange_multiplier(existing_position.symbol_info.pnl_currency),
                    }
                } else {
                    dec!(1.0)
                };

                let event = existing_position
                    .reduce_position_size(
                        position.average_price,
                        existing_position.quantity_open,
                        self.currency,
                        exchange_rate,
                        time,
                        "Synchronizing".to_string(),
                    )
                    .await;

                if let Err(e) = self.strategy_sender.send(StrategyEvent::PositionEvents(event)).await {
                    eprintln!("Error sending position event: {}", e);
                }

                self.positions_closed
                    .entry(position.symbol_code.clone())
                    .or_insert_with(Vec::new)
                    .push(existing_position);
            } else {
                // Same side, update existing position
                let update_event = match position.quantity_open.cmp(&existing_position.quantity_open) {
                    std::cmp::Ordering::Greater => Some(StrategyEvent::PositionEvents(
                        PositionUpdateEvent::Increased {
                            position_id: position.position_id.clone(),
                            side: position.side,
                            total_quantity_open: position.quantity_open,
                            average_price: position.average_price,
                            symbol_name: position.symbol_name.clone(),
                            symbol_code: position.symbol_code.clone(),
                            open_pnl: position.open_pnl,
                            booked_pnl: position.booked_pnl,
                            account: position.account.clone(),
                            originating_order_tag: "Synchronized Increase".to_string(),
                            time: Utc::now().to_string(),
                        },
                    )),
                    std::cmp::Ordering::Less => Some(StrategyEvent::PositionEvents(
                        PositionUpdateEvent::PositionReduced {
                            position_id: position.position_id.clone(),
                            side: position.side,
                            total_quantity_open: position.quantity_open,
                            average_price: position.average_price,
                            symbol_name: position.symbol_name.clone(),
                            symbol_code: position.symbol_code.clone(),
                            open_pnl: position.open_pnl,
                            booked_pnl: position.booked_pnl,
                            average_exit_price: position.average_exit_price.unwrap(),
                            account: position.account.clone(),
                            originating_order_tag: "Synchronized Decrease".to_string(),
                            time: Utc::now().to_string(),
                            total_quantity_closed: position.quantity_closed,
                        },
                    )),
                    std::cmp::Ordering::Equal => None,
                };

                // Preserve position metadata
                position.highest_recoded_price = existing_position.highest_recoded_price;
                position.lowest_recoded_price = existing_position.lowest_recoded_price;
                position.booked_pnl = existing_position.booked_pnl;

                self.positions.insert(position.symbol_code.clone(), position);

                if let Some(event) = update_event {
                    if let Err(e) = self.strategy_sender.send(event).await {
                        eprintln!("Error sending position event: {}", e);
                    }
                }
            }
        } else if !position.is_closed {
            // New position
            self.positions.insert(position.symbol_code.clone(), position.clone());
            let position_event = StrategyEvent::PositionEvents(PositionUpdateEvent::PositionOpened {
                position_id: position.position_id.clone(),
                side: position.side,
                symbol_name: position.symbol_name.clone(),
                symbol_code: position.symbol_code.clone(),
                account: position.account.clone(),
                originating_order_tag: position.tag,
                time: position.open_time.to_string(),
            });

            if let Err(e) = self.strategy_sender.send(position_event).await {
                eprintln!("Error sending position event: {}", e);
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
        match self.mode {
            StrategyMode::Backtest => {
                match brokerage {
                    Brokerage::Rithmic(_) => {
                        if let Some(symbol) = find_base_symbol(symbol_name) {
                            return match get_futures_symbol_info(&symbol) {
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
        let cash_value = self.cash_value.clone();
        let cash_used = self.cash_used.clone();
        let cash_available = self.cash_available.clone();
        let commission_paid = self.commissions_paid.clone();
        let pnl = self.total_booked_pnl.clone();
        format!("Account: {}, Balance: {} {}, Win Rate: {}%, Average Risk Reward: {}, Profit Factor {}, Total profit: {}, Total Wins: {}, Total Losses: {}, Break Even: {}, Total Trades: {}, Open Positions: {}, Cash Used: {}, Cash Available: {}, Commission Paid: {}",
                self.account, cash_value.round_dp(2), self.currency, win_rate.round_dp(2), risk_reward.round_dp(2), profit_factor.round_dp(2), pnl.round_dp(2), wins, losses, break_even, total_trades, self.positions.len(), cash_used.round_dp(2), cash_available.round_dp(2), commission_paid)
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

    pub async fn timeslice_update(&mut self, time_slice: Arc<TimeSlice>) {
        for base_data_enum in time_slice.iter() {
            let data_symbol_name = &base_data_enum.symbol().name;
            if let Some(codes) = self.symbol_code_map.get(data_symbol_name) {
                for code in codes.value() {
                    if let Some(mut position) = self.positions.get_mut(code) {
                        let open_pnl = position.update_base_data(&base_data_enum, self.currency);
                        self.open_pnl.insert(data_symbol_name.clone(), open_pnl);
                    }
                }
            } else if let Some(mut position) = self.positions.get_mut(data_symbol_name) {
                if position.is_closed {
                    continue
                }

                if self.mode != StrategyMode::Live || self.is_simulating_pnl {
                    let open_pnl = position.update_base_data(&base_data_enum, self.currency);
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
            self.cash_value = self.cash_used + self.cash_available;
        }
    }

    async fn update_or_create_live_position(
        &mut self,
        symbol_name: SymbolName,
        symbol_code: SymbolCode,
        quantity: Volume,
        side: OrderSide,
        time: DateTime<Utc>,
        market_fill_price: Price,
        tag: String
    ) {
    /*    if let Some(last_update) = self.last_update.get_requests(&symbol_code) {
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
                let event= existing_position.reduce_position_size(market_fill_price, quantity, self.currency, exchange_rate, time, tag.clone()).await;
                match &event {
                    PositionUpdateEvent::PositionReduced { booked_pnl, .. } => {
                        self.positions.insert(symbol_code.clone(), existing_position);
                        if self.is_simulating_pnl {
                            self.symbol_closed_pnl
                                .entry(symbol_code.clone())
                                .and_modify(|pnl| *pnl += booked_pnl)
                                .or_insert(booked_pnl.clone());
                            self.total_booked_pnl += booked_pnl;
                        }
                        //println!("Reduced Position: {}", symbol_name);
                    }
                    PositionUpdateEvent::PositionClosed { booked_pnl, .. } => {
                        if self.is_simulating_pnl {
                            self.symbol_closed_pnl
                                .entry(symbol_code.clone())
                                .and_modify(|pnl| *pnl += booked_pnl)
                                .or_insert(booked_pnl.clone());

                            self.total_booked_pnl += booked_pnl;
                        }
                        if !self.positions_closed.contains_key(&symbol_code) {
                            self.positions_closed.insert(symbol_code.clone(), vec![]);
                        }
                        self.positions_closed
                            .entry(symbol_code.clone())
                            .or_insert_with(Vec::new)
                            .push(existing_position);
                        //println!("Closed Position: {}", symbol_name);
                    }
                    _ => panic!("This shouldn't happen")
                }
                position_events.push(event);
            } else {
                let event = existing_position.add_to_position(self.mode, self.is_simulating_pnl, self.currency, market_fill_price, quantity, time, tag.clone()).await;
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
                self.position_calculation_mode.clone()
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
        for event in position_events {
            match self.strategy_sender.send(StrategyEvent::PositionEvents(event)).await {
                Ok(_) => {}
                Err(e) => eprintln!("Error sending position event: {}", e)
            }
        }
    }

    // Function to export individual trades to CSV
    pub fn export_trades_to_csv(&self, folder: &str) {
        // Create the folder if it does not exist
        if let Err(e) = create_dir_all(folder) {
            eprintln!("Failed to create directory {}: {}", folder, e);
            return;
        }

        // Get current date in desired format
        let date = Utc::now().format("%Y%m%d_%H%M").to_string();

        // Use brokerage and account ID to format the filename
        let brokerage = self.account.brokerage.to_string();
        let file_name = format!("{}/{:?}_TradeResults_{}_{}_{}.csv", folder, self.mode, brokerage, self.account.account_id, date);

        // Create a writer for the CSV file
        let file_path = Path::new(&file_name);
        match Writer::from_path(file_path) {
            Ok(mut wtr) => {
                // Iterate over all closed positions and their trades
                for entry in self.positions_closed.iter() {
                    for position in entry.value() {
                        for trade in &position.completed_trades {
                            let export = TradeExport {
                                symbol_code: position.symbol_code.clone(),
                                position_id: position.position_id.clone(),
                                side: position.side.to_string(),
                                entry_price: trade.entry_price,
                                entry_quantity: trade.entry_quantity,
                                exit_price: trade.exit_price,
                                exit_quantity: trade.exit_quantity,
                                entry_time: trade.entry_time.clone(),
                                exit_time: trade.exit_time.clone(),
                                pnl: trade.profit,
                                tag: position.tag.clone(),
                            };

                            if let Err(e) = wtr.serialize(export) {
                                eprintln!("Failed to write trade data to {}: {}", file_path.display(), e);
                            }
                        }
                    }
                }

                if let Err(e) = wtr.flush() {
                    eprintln!("Failed to flush CSV writer for {}: {}", file_path.display(), e);
                } else {
                    println!("Successfully exported all trades to {}", file_path.display());
                }
            }
            Err(e) => {
                eprintln!("Failed to create CSV writer for {}: {}", file_path.display(), e);
            }
        }
    }

    pub fn print_trade_statistics(&self) -> String {
        let mut total_trades: usize = 0;
        let mut wins: usize = 0;
        let mut losses: usize = 0;
        let mut break_even: usize = 0;
        let mut total_pnl = dec!(0.0);
        let mut win_pnl = dec!(0.0);
        let mut loss_pnl = dec!(0.0);
        let mut longest_hold = Duration::zero();
        let mut shortest_hold = Duration::max_value();
        let mut total_hold_time = Duration::zero();
        let mut largest_win = dec!(0.0);
        let mut largest_loss = dec!(0.0);

        // Collect statistics for each individual trade
        for entry in self.positions_closed.iter() {
            for position in entry.value() {
                for trade in &position.completed_trades {
                    total_trades += 1;

                    let pnl = trade.profit;

                    if pnl > dec!(0.0) {
                        wins += 1;
                        win_pnl += pnl;
                        largest_win = largest_win.max(pnl);
                    } else if pnl < dec!(0.0) {
                        losses += 1;
                        loss_pnl += pnl;
                        largest_loss = largest_loss.min(pnl);
                    } else {
                        break_even += 1;
                    }

                    // Calculate hold time
                    let entry_time = DateTime::<Utc>::from_str(&trade.entry_time).unwrap();
                    let exit_time = DateTime::<Utc>::from_str(&trade.exit_time).unwrap();
                    let hold_duration = exit_time - entry_time;

                    longest_hold = longest_hold.max(hold_duration);
                    shortest_hold = shortest_hold.min(hold_duration);
                    total_hold_time = total_hold_time + hold_duration;
                }
            }
        }

        // Calculate derived statistics
        let win_rate = if total_trades > 0 {
            (wins as f64 / total_trades as f64 * 100.0).round()
        } else {
            0.0
        };

        let avg_win = if wins > 0 {
            win_pnl / Decimal::from(wins)
        } else {
            dec!(0.0)
        };

        let avg_loss = if losses > 0 {
            loss_pnl / Decimal::from(losses)
        } else {
            dec!(0.0)
        };

        let profit_factor = if loss_pnl.abs() > dec!(0.0) {
            win_pnl / loss_pnl.abs()
        } else if win_pnl > dec!(0.0) {
            dec!(1000.0)
        } else {
            dec!(0.0)
        };

        let avg_hold_time = if total_trades > 0 {
            total_hold_time / total_trades as i32
        } else {
            Duration::zero()
        };

        format!(
            "\nDetailed Trade Statistics:\n\
            Total Trades: {}\n\
            Win Rate: {}%\n\
            Wins: {}\n\
            Losses: {}\n\
            Break Even: {}\n\
            Total PnL: {}\n\
            Win PnL: {}\n\
            Loss PnL: {}\n\
            Average Win: {}\n\
            Average Loss: {}\n\
            Largest Win: {}\n\
            Largest Loss: {}\n\
            Profit Factor: {}\n\
            Average Hold Time: {}\n\
            Shortest Hold: {}\n\
            Longest Hold: {}\n\
            Commission Paid: {}\n",
            total_trades,
            win_rate,
            wins,
            losses,
            break_even,
            total_pnl.round_dp(2),
            win_pnl.round_dp(2),
            loss_pnl.round_dp(2),
            avg_win.round_dp(2),
            avg_loss.round_dp(2),
            largest_win.round_dp(2),
            largest_loss.round_dp(2),
            profit_factor.round_dp(2),
            format_duration(avg_hold_time),
            format_duration(shortest_hold),
            format_duration(longest_hold),
            self.commissions_paid.round_dp(2)
        )
    }
}

#[derive(Debug, Serialize)]
struct TradeExport {
    symbol_code: String,
    position_id: String,
    side: String,
    entry_price: Decimal,
    entry_quantity: Decimal,
    exit_price: Decimal,
    exit_quantity: Decimal,
    entry_time: String,
    exit_time: String,
    pnl: Decimal,
    tag: String,
}
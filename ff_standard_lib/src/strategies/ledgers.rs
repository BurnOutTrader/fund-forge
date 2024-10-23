use std::fs::create_dir_all;
use std::path::Path;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use crate::standardized_types::enums::{OrderSide, PositionSide, StrategyMode};
use crate::standardized_types::subscriptions::{SymbolCode, SymbolName};
use dashmap::DashMap;
use csv::Writer;
use lazy_static::lazy_static;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal_macros::dec;
use tokio::sync::{Mutex};
use crate::standardized_types::broker_enum::Brokerage;
use crate::standardized_types::position::{Position, PositionId, PositionUpdateEvent};
use crate::standardized_types::symbol_info::SymbolInfo;
use crate::standardized_types::accounts::{Account, AccountInfo, Currency};
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::orders::{OrderId, OrderUpdateEvent};
use crate::standardized_types::time_slices::TimeSlice;

lazy_static! {
    pub(crate) static ref LEDGER_SERVICE: LedgerService = LedgerService::new();
}

pub(crate) struct LedgerService {
    pub (crate) ledgers: DashMap<Account, Arc<Ledger>>,
}
impl LedgerService {
    pub fn new() -> Self {
        LedgerService {
            ledgers: Default::default(),
        }
    }

    pub(crate) async fn update_or_create_position(
        &self,
        account: &Account,
        symbol_name: SymbolName,
        symbol_code: SymbolCode,
        order_id: OrderId,
        quantity: Volume,
        side: OrderSide,
        time: DateTime<Utc>,
        market_fill_price: Price, // we use the passed in price because we don't know what sort of order was filled, limit or market
        tag: String
    ) -> Result<Vec<PositionUpdateEvent>, OrderUpdateEvent> {
        //println!("Create Position: Ledger Service: {}, {}, {}, {}", account, symbol_name, market_fill_price, quantity);
        if let Some(ledger_ref) = self.ledgers.get(account) {
            return ledger_ref.update_or_create_position(symbol_name, symbol_code, order_id, quantity, side, time, market_fill_price, tag).await;
        } else {
            panic!("No ledger for account: {}", account);
        }
    }

    pub(crate) async fn update_or_create_live_position(
        &self,
        account: &Account,
        symbol_name: SymbolName,
        symbol_code: SymbolCode,
        quantity: Volume,
        side: OrderSide,
        time: DateTime<Utc>,
        market_fill_price: Price, // we use the passed in price because we don't know what sort of order was filled, limit or market
        tag: String
    ) -> Vec<PositionUpdateEvent> {
        //println!("Create Position: Ledger Service: {}, {}, {}, {}", account, symbol_name, market_fill_price, quantity);
        if let Some(ledger_ref) = self.ledgers.get(account) {
            ledger_ref.update_or_create_live_position(symbol_name, symbol_code, quantity, side, time, market_fill_price, tag).await
        } else {
            panic!("No ledger for account: {}", account);
        }
    }

    pub fn synchronize_live_position(&self, account: Account, position: Position) -> Option<PositionUpdateEvent> {
        if let Some(account_ledger) = self.ledgers.get(&account) {
            return account_ledger.value().synchronize_live_position(position)
        }
        None
    }

    pub(crate) async fn paper_exit_position(
        &self,
        account: &Account,
        symbol_name: &SymbolName,
        time: DateTime<Utc>,
        market_price: Price,
        tag: String
    ) -> Option<PositionUpdateEvent> {
        if let Some(ledger_ref) = self.ledgers.get(account) {
            ledger_ref.paper_exit_position(symbol_name, time, market_price, tag).await
        } else {
            panic!("No ledger for account: {}", account);
        }
    }

    pub async fn live_account_updates(&self, account: &Account, cash_value: Decimal, cash_available: Decimal, cash_used: Decimal) {
        if let Some(ledger_ref) = self.ledgers.get(account) {
            let mut ledger_cash_value = ledger_ref.cash_value.lock().await;
            *ledger_cash_value = cash_value;
            let mut ledger_cash_available = ledger_ref.cash_available.lock().await;
            *ledger_cash_available = cash_available;
            let mut ledger_cash_used = ledger_ref.cash_used.lock().await;
            *ledger_cash_used = cash_used;
        }
    }

    pub async fn print_ledgers(&self) {
        for ledger in self.ledgers.iter() {
            ledger.value().print().await;
        }
    }

    pub fn export_trades(&self, account: &Account, directory: &str) {
        if let Some(ledger) = self.ledgers.get(account) {
            ledger.export_positions_to_csv(directory);
        }
    }

    pub async fn print_ledger(&self, account: &Account) {
       if let Some(ledger) = self.ledgers.get(account) {
           let string = ledger.value().print().await; //todo need to return the string here
           println!("{}", string);
       }
    }

    pub async fn timeslice_updates(&self, time: DateTime<Utc>, time_slice: Arc<TimeSlice>) {
        for ledger in self.ledgers.iter() {
            ledger.timeslice_update(time_slice.clone(), time).await;
        }
    }

    pub async fn init_ledger(&self, account: &Account, strategy_mode: StrategyMode, synchronize_accounts: bool, starting_cash: Decimal, currency: Currency) {
        if !self.ledgers.contains_key(account) {
            let ledger: Ledger = match strategy_mode {
                StrategyMode::Live => {
                    let account_info = match account.brokerage.account_info(account.account_id.clone()).await {
                        Ok(ledger) => ledger,
                        Err(e) => {
                            panic!("LEDGER_SERVICE: Error initializing account: {}", e);
                        }
                    };
                    Ledger::new(account_info, strategy_mode, synchronize_accounts)
                },
                StrategyMode::Backtest | StrategyMode::LivePaperTrading => {
                    match account.brokerage.paper_account_init(strategy_mode, starting_cash, currency, account.account_id.clone()).await {
                        Ok(ledger) => ledger,
                        Err(e) => {
                            panic!("LEDGER_SERVICE: Error initializing account: {}", e);
                        }
                    }
                }
            };
            let ledger = Arc::new(ledger);

            self.ledgers.insert(account.clone(), ledger);
        }
    }
    pub fn is_long(&self, account: &Account, symbol_name: &SymbolName) -> bool {
        self.ledgers.get(account)
             .map(|ledger| ledger.is_long(symbol_name))
            .unwrap_or(false)
    }

    pub fn is_short(&self, account: &Account, symbol_name: &SymbolName) -> bool {
        self.ledgers.get(account)
             .map(|ledger| ledger.is_short(symbol_name))
            .unwrap_or(false)
    }

    pub fn is_flat(&self, account: &Account, symbol_name: &SymbolName) -> bool {
        self.ledgers.get(account)
            .map(|ledger| ledger.is_flat(symbol_name))
            .unwrap_or(true)
    }

    pub fn position_size(&self, account: &Account, symbol_name: &SymbolName) -> Decimal {
        self.ledgers.get(account)
             .map(|ledger| ledger.position_size(symbol_name))
            .unwrap_or_else(|| dec!(0))
    }

    #[allow(dead_code)]
    pub fn open_pnl(&self, account: &Account) -> Decimal {
        self.ledgers.get(account)
             .map(|ledger| ledger.get_open_pnl())
            .unwrap_or_else(|| dec!(0))
    }

    pub fn open_pnl_symbol(&self, account: &Account, symbol_name: &SymbolName) -> Decimal {
        self.ledgers.get(account)
             .map(|ledger| ledger.pnl(symbol_name))
            .unwrap_or_else(|| dec!(0))
    }

    pub fn booked_pnl(&self, account: &Account, symbol_name: &SymbolName) -> Decimal {
        self.ledgers.get(account)
             .map(|ledger| ledger.booked_pnl(symbol_name))
            .unwrap_or_else(|| dec!(0))
    }

    pub fn in_profit(&self, account: &Account, symbol_name: &SymbolName) -> bool {
        self.ledgers.get(account)
             .map(|ledger| ledger.in_profit(symbol_name))
            .unwrap_or(false)
    }

    pub fn in_drawdown(&self, account: &Account, symbol_name: &SymbolName) -> bool {
        self.ledgers.get(account)
             .map(|ledger| ledger.in_drawdown(symbol_name))
            .unwrap_or(false)
    }
}

/// A ledger specific to the strategy which will ignore positions not related to the strategy but will update its balances relative to the actual account balances for live trading.
#[derive(Debug)]
pub struct Ledger {
    pub account: Account,
    pub cash_value: Mutex<Price>,
    pub cash_available: Mutex<Price>,
    pub currency: Currency,
    pub cash_used: Mutex<Price>,
    pub positions: DashMap<SymbolName, Position>,
    pub symbol_code_map: DashMap<SymbolName, Vec<String>>,
    pub margin_used: DashMap<SymbolName, Price>,
    pub positions_closed: DashMap<SymbolName, Vec<Position>>,
    pub symbol_closed_pnl: DashMap<SymbolName, Decimal>,
    pub positions_counter: DashMap<SymbolName, u64>,
    pub(crate) symbol_info: DashMap<SymbolName, SymbolInfo>,
    pub open_pnl: DashMap<SymbolName, Price>,
    pub total_booked_pnl: Mutex<Price>,
    pub mode: StrategyMode,
    pub leverage: u32,
    pub is_simulating_pnl: bool,
    //todo, add daily max loss, max order size etc to ledger
}
impl Ledger {
    fn new(
        account_info: AccountInfo,
        mode: StrategyMode,
        synchronise_accounts: bool,
    ) -> Self {
        let is_simulating_pnl = match synchronise_accounts {
            true => false,
            false => true
        };
        println!("Ledger Created: {}", account_info.account_id);
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

    pub fn synchronize_live_position(&self, position: Position) -> Option<PositionUpdateEvent> {
        if position.is_closed {
            self.positions.remove(&position.symbol_code);
            let average_exit_price = match position.average_exit_price {
                None => dec!(0.0),
                Some(price) => price
            };
            self.positions_closed.entry(position.symbol_code.clone()).or_insert(vec![]).push(position.clone());

            let close_time = position.close_time.unwrap_or_else(|| Utc::now().to_string());
            Some(PositionUpdateEvent::PositionClosed {
                position_id: position.position_id.clone(),
                total_quantity_open: dec!(0),
                total_quantity_closed: position.quantity_closed,
                average_price: average_exit_price,
                booked_pnl: Default::default(),
                average_exit_price: position.average_exit_price,
                account: self.account.clone(),
                originating_order_tag: position.tag,
                time: close_time
            })
        }
        else {
            if let Some(mut existing_position) = self.positions.get_mut(&position.symbol_code) {
                existing_position.open_pnl = position.open_pnl;
                existing_position.booked_pnl = position.booked_pnl;
                existing_position.quantity_open = position.quantity_open;
                existing_position.average_price = position.average_price;
                existing_position.is_closed = position.is_closed;
                None
            } else {
                self.positions.insert(position.symbol_code.clone(), position.clone());
                Some(PositionUpdateEvent::PositionOpened {
                    position_id: position.position_id.clone(),
                    account: self.account.clone(),
                    originating_order_tag: position.tag,
                    time: position.open_time
                })
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

    async fn update_or_create_live_position(
        &self,
        symbol_name: SymbolName,
        symbol_code: SymbolCode,
        quantity: Volume,
        side: OrderSide,
        time: DateTime<Utc>,
        market_fill_price: Price, // we use the passed in price because we don't know what sort of order was filled, limit or market
        tag: String
    ) -> Vec<PositionUpdateEvent> {
        if self.mode != StrategyMode::Live {
            panic!("Incorrect strategy mode for ledger update_or_create_live_position()");
        }
        let mut position_events = vec![];
        // Check if there's an existing position for the given symbol
        let mut remaining_quantity = quantity;
        if let Some((_, mut existing_position)) = self.positions.remove(&symbol_code) {
            let is_reducing = (existing_position.side == PositionSide::Long && side == OrderSide::Sell)
                || (existing_position.side == PositionSide::Short && side == OrderSide::Buy);

            if is_reducing {
                remaining_quantity -= existing_position.quantity_open;
                let event= existing_position.reduce_position_size(self.mode, self.is_simulating_pnl, market_fill_price, quantity, time, tag.clone(), self.currency).await;
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
            eprintln!("Symbol Code {}", symbol_code);
            self.positions.insert(symbol_code.clone(), position);
            if !self.positions_closed.contains_key(&symbol_code) {
                self.positions_closed.insert(symbol_code.clone(), vec![]);
            }
            if symbol_name != symbol_code {
                self.symbol_code_map.entry(symbol_name).or_insert(vec![]).push(symbol_code);
            }

            let event = PositionUpdateEvent::PositionOpened {
                position_id: id,
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

mod historical_ledgers {
    use chrono::{DateTime, Utc};
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromPrimitive;
    use rust_decimal_macros::dec;
    use crate::strategies::ledgers::Ledger;
    use crate::messages::data_server_messaging::FundForgeError;
    use crate::standardized_types::enums::{OrderSide, PositionSide, StrategyMode};
    use crate::standardized_types::new_types::{Price, Volume};
    use crate::standardized_types::orders::{OrderId, OrderUpdateEvent};
    use crate::standardized_types::position::{Position, PositionUpdateEvent};
    use crate::standardized_types::subscriptions::{SymbolCode, SymbolName};

    impl Ledger {
        pub(crate) async fn release_margin_used(&self, symbol_name: &SymbolName) {
            // First get the margin amount without removing it
            if let Some((_,margin_used)) = self.margin_used.remove(symbol_name) {
                let margin_amount = margin_used;

                // Update cash values first
                {
                    let mut account_cash_used = self.cash_used.lock().await;
                    *account_cash_used -= margin_amount;
                }
                {
                    let mut account_cash_available = self.cash_available.lock().await;
                    *account_cash_available += margin_amount;
                }
            }
        }

        pub(crate) async fn commit_margin(&self, symbol_name: &SymbolName, quantity: Volume, market_price: Price) -> Result<(), FundForgeError> {
            let margin = self.account.brokerage.intraday_margin_required(symbol_name.clone(), quantity).await?
                .unwrap_or_else(|| (quantity * market_price) / Decimal::from_u32(self.leverage).unwrap());

            // Check available cash first
            {
                let cash_available = self.cash_available.lock().await;
                if *cash_available < margin {
                    return Err(FundForgeError::ClientSideErrorDebug(format!(
                        "Insufficient funds: Required {}, Available {}",
                        margin,
                        *cash_available
                    )));
                }
            }

            // Update margin tracking before updating cash
            self.margin_used
                .entry(symbol_name.clone())
                .and_modify(|existing| *existing += margin)
                .or_insert(margin);

            // Update cash values
            {
                let mut account_cash_used = self.cash_used.lock().await;
                *account_cash_used += margin;
            }

            {
                let mut account_cash_available = self.cash_available.lock().await;
                *account_cash_available -= margin;
            }

            Ok(())
        }

        pub(crate) async fn paper_exit_position(
            &self,
            symbol_name: &SymbolName,
            time: DateTime<Utc>,
            market_price: Price,
            tag: String
        ) -> Option<PositionUpdateEvent> {
            if let Some((symbol_name, mut existing_position)) = self.positions.remove(symbol_name) {
                // Mark the position as closed
                existing_position.is_closed = true;
                {
                    self.release_margin_used(&symbol_name).await;
                }
                let event = existing_position.reduce_position_size(self.mode, self.is_simulating_pnl, market_price, existing_position.quantity_open, time, tag, self.currency).await;
                let mut cash_available = self.cash_available.lock().await;
                let mut total_booked_pnl = self.total_booked_pnl.lock().await;
                match &event {
                    PositionUpdateEvent::PositionClosed { booked_pnl, .. } => {
                        // TODO[Strategy]: Add option to mirror account position or use internal position curating.
                        self.symbol_closed_pnl
                            .entry(symbol_name.clone())
                            .and_modify(|pnl| *pnl += booked_pnl)
                            .or_insert(booked_pnl.clone());
                        *total_booked_pnl += booked_pnl;

                        *cash_available += booked_pnl;
                    }
                    _ => panic!("this shouldn't happen")
                }

                let mut cash_value = self.cash_value.lock().await;
                let cash_used = self.cash_used.lock().await;
                *cash_value = *cash_used + *cash_available;

                // Add the closed position to the positions_closed DashMap
                self.positions_closed
                    .entry(symbol_name.clone())                  // Access the entry for the symbol name
                    .or_insert_with(Vec::new)                    // If no entry exists, create a new Vec
                    .push(existing_position);     // Push the closed position to the Vec

                return Some(event)
            }
            None
        }

        /// If Ok it will return a Position event for the successful position update, if the ledger rejects the order it will return an Err(OrderEvent)
        ///todo, check ledger max order etc before placing orders
        pub(crate) async fn update_or_create_position(
            &self,
            symbol_name: SymbolName,
            symbol_code: SymbolCode,
            order_id: OrderId,
            quantity: Volume,
            side: OrderSide,
            time: DateTime<Utc>,
            market_fill_price: Price, // we use the passed in price because we don't know what sort of order was filled, limit or market
            tag: String
        ) -> Result<Vec<PositionUpdateEvent>, OrderUpdateEvent> {
            if self.mode == StrategyMode::Live {
                panic!("Incorrect mode for update_or_create_position()");
            }
            let mut updates = vec![];
            // Check if there's an existing position for the given symbol
            let mut remaining_quantity = quantity;
            if let Some((symbol_name, mut existing_position)) = self.positions.remove(&symbol_code) {
                let is_reducing = (existing_position.side == PositionSide::Long && side == OrderSide::Sell)
                    || (existing_position.side == PositionSide::Short && side == OrderSide::Buy);

                if is_reducing {
                    remaining_quantity -= existing_position.quantity_open;
                    let event = existing_position.reduce_position_size(self.mode, self.is_simulating_pnl, market_fill_price, quantity, time, tag.clone(), self.currency).await;

                    self.release_margin_used(&symbol_name).await;

                    match &event {
                        PositionUpdateEvent::PositionReduced { booked_pnl, .. } => {
                            self.commit_margin(&symbol_name, existing_position.quantity_open, existing_position.average_price).await.unwrap();
                            self.positions.insert(symbol_code.clone(), existing_position);

                            // TODO[Strategy]: Add option to mirror account position or use internal position curating.
                            self.symbol_closed_pnl
                                .entry(symbol_code.clone())
                                .and_modify(|pnl| *pnl += booked_pnl)
                                .or_insert(booked_pnl.clone());
                            {
                                let mut total_booked_pnl = self.total_booked_pnl.lock().await;
                                *total_booked_pnl += booked_pnl;

                                let mut cash_available = self.cash_available.lock().await;
                                *cash_available += booked_pnl;
                            }
                            //println!("Reduced Position: {}", symbol_name);
                        }
                        PositionUpdateEvent::PositionClosed { booked_pnl, .. } => {
                            // TODO[Strategy]: Add option to mirror account position or use internal position curating.
                            self.symbol_closed_pnl
                                .entry(symbol_code.clone())
                                .and_modify(|pnl| *pnl += booked_pnl)
                                .or_insert(booked_pnl.clone());

                            {
                                let mut total_booked_pnl = self.total_booked_pnl.lock().await;
                                *total_booked_pnl += booked_pnl;

                                let mut cash_available = self.cash_available.lock().await;
                                *cash_available += booked_pnl;
                            }

                            if !self.positions_closed.contains_key(&symbol_code) {
                                self.positions_closed.insert(symbol_code.clone(), vec![]);
                            }
                            if let Some(mut positions_closed) = self.positions_closed.get_mut(&symbol_code) {
                                positions_closed.value_mut().push(existing_position);
                            }
                            //println!("Closed Position: {}", symbol_name);
                        }
                        _ => panic!("This shouldn't happen")
                    }

                    {
                        let mut cash_value = self.cash_value.lock().await;
                        let cash_used = self.cash_used.lock().await;
                        let cash_available = self.cash_available.lock().await;
                        *cash_value = *cash_used + *cash_available;
                    }

                    updates.push(event);
                } else {
                    match self.commit_margin(&symbol_name, quantity, market_fill_price).await {
                        Ok(_) => {}
                        Err(e) => {
                            //todo this now gets added directly to buffer
                            let event = OrderUpdateEvent::OrderRejected {
                                account: self.account.clone(),
                                symbol_name: symbol_name.clone(),
                                symbol_code: symbol_code.clone(),
                                order_id,
                                reason: e.to_string(),
                                tag,
                                time: time.to_string()
                            };
                            return Err(event)
                        }
                    }
                    let event = existing_position.add_to_position(self.mode, self.is_simulating_pnl, market_fill_price, quantity, time, tag.clone(), self.currency).await;
                    self.positions.insert(symbol_code.clone(), existing_position);

                    {
                        let mut cash_value = self.cash_value.lock().await;
                        let cash_used = self.cash_used.lock().await;
                        let cash_available = self.cash_available.lock().await;
                        *cash_value = *cash_used + *cash_available;
                    }

                    updates.push(event);
                    remaining_quantity = dec!(0.0);
                }
            }
            if remaining_quantity > dec!(0.0) {
                match self.commit_margin(&symbol_name, quantity, market_fill_price).await {
                    Ok(_) => {}
                    Err(e) => {
                        let event = OrderUpdateEvent::OrderRejected {
                            account: self.account.clone(),
                            symbol_name: symbol_name.clone(),
                            symbol_code: symbol_code.clone(),
                            order_id,
                            reason: e.to_string(),
                            tag,
                            time: time.to_string()
                        };
                        return Err(event)
                    }
                }

                // Determine the side of the position based on the order side
                let position_side = match side {
                    OrderSide::Buy => PositionSide::Long,
                    OrderSide::Sell => PositionSide::Short,
                };

                let info = self.symbol_info(self.account.brokerage, &symbol_name).await;
                if symbol_name != symbol_code && !self.symbol_code_map.contains_key(&symbol_name) {
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
                eprintln!("Symbol Code {}", symbol_code);
                self.positions.insert(symbol_code.clone(), position);
                if !self.positions_closed.contains_key(&symbol_code) {
                    self.positions_closed.insert(symbol_code.clone(), vec![]);
                }
                if symbol_name != symbol_code {
                    self.symbol_code_map.entry(symbol_name).or_insert(vec![]).push(symbol_code);
                }

                let event = PositionUpdateEvent::PositionOpened {
                    position_id: id,
                    account: self.account.clone(),
                    originating_order_tag: tag,
                    time: time.to_string()
                };

                {
                    let mut cash_value = self.cash_value.lock().await;
                    let cash_used = self.cash_used.lock().await;
                    let cash_available = self.cash_available.lock().await;
                    *cash_value = *cash_used + *cash_available;
                }

                //println!("{:?}", event);
                updates.push(event);
            }
            Ok(updates)
        }
    }
}
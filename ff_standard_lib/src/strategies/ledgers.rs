use std::fs::create_dir_all;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use chrono::{DateTime, Utc};
use crate::standardized_types::enums::{OrderSide, PositionSide, StrategyMode};
use crate::standardized_types::subscriptions::{SymbolCode, SymbolName};
use dashmap::DashMap;
use csv::Writer;
use lazy_static::lazy_static;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal_macros::dec;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot;
use crate::standardized_types::broker_enum::Brokerage;
use crate::standardized_types::position::{Position, PositionId, PositionUpdateEvent};
use crate::standardized_types::symbol_info::SymbolInfo;
use crate::standardized_types::accounts::{AccountId, AccountInfo, Currency};
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::orders::{OrderId, OrderUpdateEvent};
use crate::standardized_types::time_slices::TimeSlice;
use crate::strategies::client_features::server_connections::add_buffer;
use crate::strategies::handlers::market_handler::price_service::price_service_request_market_fill_price;
use crate::strategies::strategy_events::StrategyEvent;
/*todo,
   Make ledger take functions as params, create_order_reduce_position, increase_position, reduce_position, we will pass these in based on the type of positions we want.
   1. we can have a cumulative model, like we have now, for position building.
   2. FIFO: Each time we open an opposing order, the first position is cancelled out and returned as closed
   3. FILO: Each time we open a position, it is kept until we are flat or reversed and each counter order closes the order entry directly before.
   To achieve this ledgers will need to be able to hold multiple sub position details or positions themselves generate 'trade' objects
 */

lazy_static! {
    pub(crate) static ref LEDGER_SERVICE: LedgerService = LedgerService::new();
}

pub(crate) struct LedgerService {
    callback_senders: DashMap<u64, oneshot::Sender<LedgerResponse>>,
    pub(crate) ledger_senders: DashMap<Brokerage, DashMap<AccountId, Sender<LedgerMessage>>>,
    symbol_info: Arc<DashMap<SymbolName, SymbolInfo>>,
    id: AtomicU64,
    strategy_mode: StrategyMode,
    synchronize_accounts: bool,
    starting_cash: Decimal,
    currency: Currency
}
impl LedgerService {
    pub fn new() -> Self {
        LedgerService {
            callback_senders: Default::default(),
            ledger_senders: Default::default(),
            symbol_info: Arc::new(Default::default()),
            id: AtomicU64::new(0),
            strategy_mode: StrategyMode::Backtest,
            synchronize_accounts: false,
            starting_cash: dec!(100000),
            currency: Currency::USD,
        }
    }

    pub fn get_next_id(&self) -> u64 {
        self.id.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| {
            Some((x + 1) % u64::MAX)
        }).unwrap_or(0)
    }

    pub async fn request_callback(&self, brokerage: Brokerage, account_id: &AccountId, ledger_request: LedgerRequest) -> oneshot::Receiver<LedgerResponse> {
        let callback_id = self.get_next_id();
        let (sender, receiver) = oneshot::channel();
        self.callback_senders.insert(callback_id, sender);
        self.send_message(brokerage, account_id, LedgerMessage::LedgerCallBackRequest(callback_id, ledger_request)).await;
        receiver
    }

    pub async fn send_message(&self, brokerage: Brokerage, account_id: &AccountId, ledger_message: LedgerMessage) {
        if let Some(broker_map) = self.ledger_senders.get(&brokerage) {
            if let Some(account_sender) = broker_map.get(account_id) {
                match account_sender.send(ledger_message).await {
                    Ok(_) => {}
                    Err(e) => eprintln!("LEDGER_SERVICE: Failed to send message: {}", e)
                }
            }
        }
    }

    pub async fn handle_response(&self, callback_id: u64, response: LedgerResponse) {
        if let Some((_, sender)) = self.callback_senders.remove(&callback_id) {
            let _ = sender.send(response);
        }
    }

    pub async fn timeslice_updates(&self, time: DateTime<Utc>, time_slice: TimeSlice) {
        let request = LedgerMessage::TimeSlice(time, time_slice);
        for broker_map in self.ledger_senders.iter() {
            for account_map in broker_map.iter() {
                match account_map.value().send(request.clone()).await {
                    Ok(_) => {}
                    Err(_e) => panic!("LEDGER_SERVICE: Ledger timeSlice update request failed")
                }
            }
        }
    }

    pub fn has_ledger(&self, brokerage: Brokerage, account_id: &AccountId) -> bool {
        if let Some(broker_map) = self.ledger_senders.get(&brokerage) {
           return broker_map.contains_key(account_id)
        }
        false
    }

    pub async fn init_ledger(&self, brokerage: Brokerage, account_id: AccountId, strategy_mode: StrategyMode, synchronize_accounts: bool, starting_cash: Decimal, currency: Currency) {
        if !self.ledger_senders.contains_key(&brokerage) {
            let broker_map = DashMap::new();
            self.ledger_senders.insert(brokerage.clone(), broker_map);
        }
        if !self.ledger_senders.get(&brokerage).unwrap().contains_key(&account_id) {
            let ledger: Ledger = match strategy_mode {
                StrategyMode::Live => {
                    let account_info = match brokerage.account_info(account_id.clone()).await {
                        Ok(ledger) => ledger,
                        Err(e) => {
                            panic!("LEDGER_SERVICE: Error initializing account: {}", e);
                        }
                    };
                    Ledger::new(account_info, strategy_mode, synchronize_accounts, self.symbol_info.clone())
                },
                StrategyMode::Backtest | StrategyMode::LivePaperTrading => {
                    match brokerage.paper_account_init(strategy_mode, starting_cash, currency, account_id.clone(), self.symbol_info.clone()).await {
                        Ok(ledger) => ledger,
                        Err(e) => {
                            panic!("LEDGER_SERVICE: Error initializing account: {}", e);
                        }
                    }
                }
            };
            let (sender, receiver) = tokio::sync::mpsc::channel(100);
            Ledger::start_receive(ledger, receiver);
            self.ledger_senders.get(&brokerage).unwrap().insert(account_id.clone(), sender);
        }
    }

    pub async fn account_snapshot(&self, brokerage: Brokerage, account_id: AccountId, synchronize_accounts: bool, account_info: AccountInfo, strategy_mode: StrategyMode) {
        if !self.ledger_senders.contains_key(&brokerage) {
            let broker_map = DashMap::new();
            self.ledger_senders.insert(brokerage.clone(), broker_map);
        }
        if !self.ledger_senders.get(&brokerage).unwrap().contains_key(&account_id) {
            let ledger: Ledger = match strategy_mode {
                StrategyMode::Live => {
                    Ledger::new(account_info, strategy_mode, synchronize_accounts, self.symbol_info.clone())
                },
                _ => panic!("LEDGER_SERVICE: should not update accounts with snapshots")
            };
            let (sender, receiver) = tokio::sync::mpsc::channel(100);
            Ledger::start_receive(ledger, receiver);
            self.ledger_senders.get(&brokerage).unwrap().insert(account_id.clone(), sender);
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum LedgerRequest {
    IsLongRequest(SymbolName),
    IsShortRequest(SymbolName),
    IsFlatRequest(SymbolName),
    PositionSizeRequest(SymbolName),
    BookedPnlRequest(SymbolName),
    OpenPnlRequest(SymbolName),
    InDrawDownRequest(SymbolName),
    InProfitRequest(SymbolName),
    PaperExitPosition {
        symbol_name: SymbolName,
        side: PositionSide,
        symbol_code: Option<SymbolCode>,
        order_id: OrderId,
        time: DateTime<Utc>,
        tag: String
    },
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum LedgerResponse {
    IsLongResponse(bool),
    IsShortResponse(bool),
    IsFlatResponse(bool),
    PositionSizeResponse(Decimal),
    BookedPnlResponse(Decimal),
    OpenPnlResponse(Decimal),
    InDrawDownResponse(bool),
    InProfitResponse(bool),
    ProcessLedgerResponse(String),
    FlattenAccount(bool),
    PaperExitPosition{had_position: bool}
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum LedgerMessage {
    TimeSlice(DateTime<Utc>, TimeSlice),
    LedgerCallBackRequest(u64, LedgerRequest),
    PrintLedgerRequest,
    ExportTrades(String),
    UpdateOrCreatePosition {
        symbol_name: SymbolName,
        symbol_code: SymbolCode,
        order_id: OrderId,
        quantity: Volume,
        side: OrderSide,
        time: DateTime<Utc>,
        market_fill_price: Price,
        tag: String
    },
    FlattenAccount(DateTime<Utc>),
    LiveAccountUpdates { brokerage: Brokerage, account_id: AccountId, cash_value: Decimal, cash_available: Decimal, cash_used: Decimal },
    LivePositionUpdates { brokerage: Brokerage, account_id: AccountId,  symbol_name: SymbolName, symbol_code: SymbolCode, open_pnl: Decimal, open_quantity: Decimal, side: Option<PositionSide> }
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
    pub symbol_code_map: DashMap<SymbolName, Vec<String>>,
    pub margin_used: DashMap<SymbolName, Price>,
    pub positions_closed: DashMap<SymbolName, Vec<Position>>,
    pub symbol_closed_pnl: DashMap<SymbolName, Decimal>,
    pub positions_counter: DashMap<SymbolName, u64>,
    pub(crate) symbol_info: Arc<DashMap<SymbolName, SymbolInfo>>,
    pub open_pnl: DashMap<SymbolName, Price>,
    pub total_booked_pnl: Price,
    pub mode: StrategyMode,
    pub leverage: u32,
    pub is_simulating_pnl: bool,
    //todo, add daily max loss, max order size etc to ledger
}
impl Ledger {
    pub fn new(
        account_info: AccountInfo,
        mode: StrategyMode,
        synchronise_accounts: bool,
        symbol_info: Arc<DashMap<SymbolName, SymbolInfo>>
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
            account_id: account_info.account_id,
            brokerage: account_info.brokerage,
            cash_value: account_info.cash_value,
            cash_available: account_info.cash_available,
            currency: account_info.currency,
            cash_used: account_info.cash_used,
            positions,
            symbol_code_map: contract_map,
            margin_used: Default::default(),
            positions_closed: DashMap::new(),
            symbol_closed_pnl: Default::default(),
            positions_counter: DashMap::new(),
            symbol_info,
            open_pnl: DashMap::new(),
            total_booked_pnl: dec!(0.0),
            mode,
            leverage: account_info.leverage,
            is_simulating_pnl
        };
        ledger
    }

    pub(crate) fn start_receive(ledger: Ledger, mut request_receiver: Receiver<LedgerMessage>) {
        tokio::task::spawn(async move{
            let mut ledger = ledger;
            while let Some(request) = request_receiver.recv().await {
                match request {
                    LedgerMessage::TimeSlice(time, slice) =>  {
                        ledger.timeslice_update(slice, time);
                    },
                    LedgerMessage::LedgerCallBackRequest(callback_id, request) => {
                        match request {
                            LedgerRequest::IsLongRequest(symbol) => {
                                LEDGER_SERVICE.handle_response(callback_id, LedgerResponse::IsLongResponse(ledger.is_long(&symbol))).await;
                            }
                            LedgerRequest::IsShortRequest(symbol) => {
                                LEDGER_SERVICE.handle_response(callback_id, LedgerResponse::IsShortResponse(ledger.is_short(&symbol))).await;
                            }
                            LedgerRequest::IsFlatRequest(symbol) => {
                                LEDGER_SERVICE.handle_response(callback_id, LedgerResponse::IsFlatResponse(ledger.is_flat(&symbol))).await;
                            }
                            LedgerRequest::PositionSizeRequest(symbol) => {
                                LEDGER_SERVICE.handle_response(callback_id, LedgerResponse::PositionSizeResponse(ledger.position_size(&symbol))).await;
                            }
                            LedgerRequest::BookedPnlRequest(symbol) => {
                                LEDGER_SERVICE.handle_response(callback_id, LedgerResponse::BookedPnlResponse(ledger.booked_pnl(&symbol))).await;
                            }
                            LedgerRequest::OpenPnlRequest(symbol) => {
                                let open_pnl = match ledger.open_pnl.get(&symbol) {
                                    None => dec!(0),
                                    Some(pnl) => pnl.value().clone()
                                };
                                LEDGER_SERVICE.handle_response(callback_id, LedgerResponse::OpenPnlResponse(open_pnl)).await;
                            }
                            LedgerRequest::InDrawDownRequest(symbol) => {
                                LEDGER_SERVICE.handle_response(callback_id, LedgerResponse::InDrawDownResponse(ledger.in_drawdown(&symbol))).await;
                            }
                            LedgerRequest::InProfitRequest(symbol) => {
                                LEDGER_SERVICE.handle_response(callback_id, LedgerResponse::InProfitResponse(ledger.in_profit(&symbol))).await;
                            }
                            LedgerRequest::PaperExitPosition { symbol_name, symbol_code: _, order_id: _, side, time, tag: _ } => {
                                if ledger.mode == StrategyMode::Backtest || ledger.mode == StrategyMode::LivePaperTrading {
                                    if let Some((symbol_name, position)) = ledger.positions.remove(&symbol_name) {
                                        if position.side == side {
                                            let side = match position.side {
                                                PositionSide::Long => OrderSide::Sell,
                                                PositionSide::Short => OrderSide::Buy
                                            };
                                            let market_price = match price_service_request_market_fill_price(side, symbol_name.clone(), position.quantity_open).await {
                                                Ok(price) => price.price(),
                                                Err(e) => panic!("Unable to get fill price for {}: {}", position.symbol_name, e) //todo, we shou
                                            };
                                            ledger.paper_exit_position(&position.symbol_name, time, market_price.unwrap(), "Flatten All".to_string()).await;
                                            ledger.positions_closed.entry(symbol_name.clone()).or_insert(vec![]).push(position.clone());
                                        }
                                    }
                                }
                            }
                        }
                    },
                    LedgerMessage::PrintLedgerRequest => {
                        ledger.print();
                    },
                    LedgerMessage::UpdateOrCreatePosition { symbol_name, symbol_code, order_id, quantity, side, time, market_fill_price, tag } => {
                        ledger.update_or_create_position(symbol_name, symbol_code, order_id, quantity, side, time, market_fill_price, tag).await;
                    }
                    LedgerMessage::FlattenAccount(time) => {
                        if ledger.mode == StrategyMode::Backtest || ledger.mode == StrategyMode::LivePaperTrading {
                            // Collect keys (symbol names) to process
                            let symbols_to_process: Vec<String> = ledger.positions.iter()
                                .map(|entry| entry.key().clone())
                                .collect();

                            for symbol_name in symbols_to_process {
                                if let Some((_, position)) = ledger.positions.remove(&symbol_name) {
                                    let side = match position.side {
                                        PositionSide::Long => OrderSide::Sell,
                                        PositionSide::Short => OrderSide::Buy
                                    };
                                    let market_price = match price_service_request_market_fill_price(
                                        side,
                                        symbol_name.clone(),
                                        position.quantity_open,
                                    ).await {
                                        Ok(price) => price,
                                        Err(e) => {
                                            eprintln!("Unable to get fill price for {}: {}. Skipping this position.", position.symbol_name, e);
                                            continue;
                                        }
                                    };
                                    ledger.paper_exit_position(&position.symbol_name, time, market_price.price().unwrap(), "Flatten All".to_string()).await;
                                    ledger.positions_closed.entry(symbol_name).or_insert(vec![]).push(position);
                                }
                            }
                        }
                    }
                    LedgerMessage::ExportTrades(directory) => {
                        ledger.export_positions_to_csv(&directory);
                    }
                    LedgerMessage::LiveAccountUpdates { .. } => {
                        todo!()
                    }
                    LedgerMessage::LivePositionUpdates { .. } => {
                        todo!()
                    }
                }
            }
        });
    }

    pub fn update(&mut self, cash_value: Decimal, cash_available: Decimal, cash_used: Decimal) {
        self.cash_value = cash_value;
        self.cash_used = cash_used;
        self.cash_available = cash_available;
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

    // TODO[Strategy]: Add option to mirror account position or use internal position curating.
    #[allow(unused)]
    pub fn update_live_position(&self, symbol_name: SymbolName, product_code: Option<String>, open_pnl: Decimal, open_quantity: Decimal, side: Option<PositionSide>) {
        //todo, check ledger max order etc before placing orders
        if let Some((_symbol_name, mut existing_position)) = self.positions.remove(&symbol_name) {
            match existing_position.side {
                PositionSide::Long => {

                }
                PositionSide::Short => {

                }
            }
        }
        else if let Some(product_code) = product_code {
            if let Some(existing_position) = self.positions.get_mut(&product_code) {
                match existing_position.side {
                    PositionSide::Long => {

                    }
                    PositionSide::Short => {

                    }
                }
            }
        }

        /* existing_position.is_closed = true;
           self.positions_closed
               .entry(symbol_name)
               .or_insert_with(Vec::new)
               .push(existing_position);*/
     /*   let id = self.generate_id(&symbol_name, position.side);
        position.position_id = id;
        self.positions.insert(position.symbol_name.clone(), position);*/
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
            None => panic!("this should be here before orders or positions exist"),
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
        format!("{}-{}-{}-{}-{}", self.brokerage, self.account_id, counter, symbol_name, side)
    }

    pub fn timeslice_update(&mut self, time_slice: TimeSlice, time: DateTime<Utc>) {
        for base_data_enum in time_slice.iter() {
            let data_symbol_name = &base_data_enum.symbol().name;
            if let Some(mut position) = self.positions.get_mut(data_symbol_name) {
                if position.is_closed {
                    continue
                }
                //returns booked pnl if exit on brackets
                position.update_base_data(&base_data_enum, time, self.currency);

                // TODO[Strategy]: Add option to mirror account position or use internal position curating.
                if self.mode != StrategyMode::Live || self.is_simulating_pnl {
                    self.open_pnl.insert(data_symbol_name.clone(), position.open_pnl.clone());
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

    /// If Ok it will return a Position event for the successful position update, if the ledger rejects the order it will return an Err(OrderEvent)
    ///todo, check ledger max order etc before placing orders
    pub async fn update_or_create_position(
        &mut self,
        symbol_name: SymbolName,
        symbol_code: SymbolCode,
        order_id: OrderId,
        quantity: Volume,
        side: OrderSide,
        time: DateTime<Utc>,
        market_fill_price: Price, // we use the passed in price because we dont know what sort of order was filled, limit or market
        tag: String
    ) {
        let mut updates = vec![];
        // Check if there's an existing position for the given symbol
        let mut remaining_quantity = quantity;
        if let Some((symbol_name, mut existing_position)) = self.positions.remove(&symbol_code) {
            let is_reducing = (existing_position.side == PositionSide::Long && side == OrderSide::Sell)
                || (existing_position.side == PositionSide::Short && side == OrderSide::Buy);

            if is_reducing {
                remaining_quantity -= existing_position.quantity_open;
                let event= existing_position.reduce_position_size(market_fill_price, quantity, time, tag.clone(), self.currency).await;

                if self.mode != StrategyMode::Live {
                    self.release_margin_used(&symbol_name).await;
                }

                match &event {
                    PositionUpdateEvent::PositionReduced { booked_pnl, .. } => {
                        if self.mode != StrategyMode::Live {
                            //this should never panic since we just freed the margin temp
                            self.commit_margin(&symbol_name, existing_position.quantity_open, existing_position.average_price).await.unwrap();
                        }
                        self.positions.insert(symbol_code.clone(), existing_position);

                        // TODO[Strategy]: Add option to mirror account position or use internal position curating.
                        if self.mode != StrategyMode::Live || self.is_simulating_pnl {
                            self.symbol_closed_pnl
                                .entry(symbol_code.clone())
                                .and_modify(|pnl| *pnl += booked_pnl)
                                .or_insert(booked_pnl.clone());
                            self.total_booked_pnl += booked_pnl;
                        }
                        if self.mode != StrategyMode::Live {
                            self.cash_available += booked_pnl;
                        }
                        //println!("Reduced Position: {}", symbol_name);
                    }
                    PositionUpdateEvent::PositionClosed { booked_pnl, .. } => {
                        // TODO[Strategy]: Add option to mirror account position or use internal position curating.
                        if self.mode != StrategyMode::Live || self.is_simulating_pnl {
                            self.symbol_closed_pnl
                                .entry(symbol_code.clone())
                                .and_modify(|pnl| *pnl += booked_pnl)
                                .or_insert(booked_pnl.clone());
                            self.total_booked_pnl += booked_pnl;
                        }
                        if self.mode != StrategyMode::Live {
                            self.cash_available += booked_pnl;
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

                if self.mode != StrategyMode::Live {
                    self.cash_value = self.cash_used + self.cash_available;
                }
                updates.push(event);
            } else {
                if self.mode != StrategyMode::Live {
                    match self.commit_margin(&symbol_name, quantity, market_fill_price).await {
                        Ok(_) => {}
                        Err(e) => {
                            //todo this now gets added directly to buffer
                            add_buffer(time, StrategyEvent::OrderEvents(OrderUpdateEvent::OrderRejected {
                                brokerage: self.brokerage,
                                account_id: self.account_id.clone(),
                                symbol_name: symbol_name.clone(),
                                symbol_code: symbol_code.clone(),
                                order_id,
                                reason: e.to_string(),
                                tag,
                                time: time.to_string()
                            })).await;
                            return
                        }
                    }
                }
                let event = existing_position.add_to_position(market_fill_price, quantity, time, tag.clone(), self.currency).await;
                self.positions.insert(symbol_code.clone(), existing_position);

                if self.mode != StrategyMode::Live {
                    self.cash_value = self.cash_used + self.cash_available;
                }

                updates.push(event);
                remaining_quantity = dec!(0.0);
            }
        }
        if remaining_quantity > dec!(0.0) {
            if self.mode != StrategyMode::Live {
                match self.commit_margin(&symbol_name, quantity, market_fill_price).await {
                    Ok(_) => {}
                    Err(e) => {
                        add_buffer(time, StrategyEvent::OrderEvents(OrderUpdateEvent::OrderRejected {
                            brokerage: self.brokerage,
                            account_id: self.account_id.clone(),
                            symbol_name: symbol_name.clone(),
                            symbol_code: symbol_code.clone(),
                            order_id,
                            reason: e.to_string(),
                            tag,
                            time: time.to_string()
                        })).await;
                        return
                    }
                }
            }

            // Determine the side of the position based on the order side
            let position_side = match side {
                OrderSide::Buy => PositionSide::Long,
                OrderSide::Sell => PositionSide::Short,
            };

            eprintln!("Symbol Name for info request: {}", symbol_name);
            let info = self.symbol_info(&symbol_name).await;
            if !self.symbol_code_map.contains_key(&symbol_name) && symbol_name != symbol_code  {
                self.symbol_code_map
                    .entry(symbol_name.clone())
                    .or_insert_with(Vec::new)
                    .push(symbol_code.clone());
            };

            let id = self.generate_id(&symbol_name, position_side);
            // Create a new position
            let position = Position::new(
                symbol_code.clone(),
                symbol_code.clone(),
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
            self.positions.insert(symbol_code.clone(), position);
            if !self.positions_closed.contains_key(&symbol_code) {
                self.positions_closed.insert(symbol_code.clone(), vec![]);
            }

            let event = PositionUpdateEvent::PositionOpened {
                position_id: id,
                account_id: self.account_id.clone(),
                brokerage: self.brokerage.clone(),
                originating_order_tag: tag,
                time: time.to_string()
            };

            if self.mode != StrategyMode::Live {
                self.cash_value = self.cash_used + self.cash_available;
            }
            //println!("Position Created: {}", symbol_name);
            updates.push(event);
        }
        //todo this now gets added directly to buffer
        for event in updates {
            add_buffer(time, StrategyEvent::PositionEvents(event)).await;
        }
    }
}

pub(crate) mod historical_ledgers {
    use chrono::{DateTime, Utc};
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromPrimitive;
    use crate::strategies::ledgers::Ledger;
    use crate::messages::data_server_messaging::FundForgeError;
    use crate::standardized_types::enums::StrategyMode;
    use crate::standardized_types::new_types::{Price, Volume};
    use crate::standardized_types::position::PositionUpdateEvent;
    use crate::standardized_types::subscriptions::SymbolName;
    use crate::strategies::client_features::server_connections::add_buffer;
    use crate::strategies::strategy_events::StrategyEvent;

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

        pub async fn paper_exit_position(
            &mut self,
            symbol_name: &SymbolName,
            time: DateTime<Utc>,
            market_price: Price,
            tag: String
        ) {
            if let Some((symbol_name, mut existing_position)) = self.positions.remove(symbol_name) {
                // Mark the position as closed
                existing_position.is_closed = true;

                // Calculate booked profit by reducing the position size
                if self.mode != StrategyMode::Live {
                    self.release_margin_used(&symbol_name).await;
                }
                let event = existing_position.reduce_position_size(market_price, existing_position.quantity_open, time, tag, self.currency).await;
                match &event {
                    PositionUpdateEvent::PositionClosed { booked_pnl,.. } => {
                        // TODO[Strategy]: Add option to mirror account position or use internal position curating.
                        if self.mode != StrategyMode::Live || self.is_simulating_pnl {
                            self.symbol_closed_pnl
                                .entry(symbol_name.clone())
                                .and_modify(|pnl| *pnl += booked_pnl)
                                .or_insert(booked_pnl.clone());
                            self.total_booked_pnl += booked_pnl;
                        }
                        if self.mode != StrategyMode::Live {
                            self.cash_available += booked_pnl;
                        }
                    }
                    _ => panic!("this shouldn't happen")
                }

                if self.mode != StrategyMode::Live {
                    self.cash_value = self.cash_used + self.cash_available;
                }
                // Add the closed position to the positions_closed DashMap
                self.positions_closed
                    .entry(symbol_name.clone())                  // Access the entry for the symbol name
                    .or_insert_with(Vec::new)                    // If no entry exists, create a new Vec
                    .push(existing_position);     // Push the closed position to the Vec

                add_buffer(time, StrategyEvent::PositionEvents(event)).await;
            }
        }
    }
}
use std::cmp::min;
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
use crate::standardized_types::position::{Position, PositionCalculationMode, PositionId, PositionUpdateEvent, TradeResult};
use crate::standardized_types::subscriptions::{SymbolCode, SymbolName};
use crate::standardized_types::symbol_info::SymbolInfo;
use crate::standardized_types::time_slices::TimeSlice;
use crate::strategies::client_features::other_requests::get_exchange_rate;
use crate::strategies::handlers::market_handler::price_service::MarketPriceService;
use crate::strategies::strategy_events::StrategyEvent;

/*
 The ledger could be split into event driven components
 - We could have an static dashmap symbol name atomic bool, for is long is short etc, this can be updated in an event loop that manages position updates
 - The ledger itself doesnt need to exist,
*/

#[derive(Debug)]
pub enum LedgerMessage {
    SyncPosition{symbol_name: SymbolName, symbol_code: SymbolCode, account: Account, open_quantity: f64, average_price: f64, side: PositionSide, open_pnl: f64, time: String},
    UpdateOrCreatePosition{symbol_name: SymbolName, symbol_code: SymbolCode, quantity: Volume, side: OrderSide, time: DateTime<Utc>, market_fill_price: Price, tag: String, paper_response_sender: Option<oneshot::Sender<Option<OrderUpdateEvent>>>, order_id: OrderId},
    TimeSliceUpdate{time_slice: Arc<TimeSlice>},
    LiveAccountUpdate{cash_value: Decimal, cash_available: Decimal, cash_used: Decimal},
    ExitPaperPosition{symbol_code: SymbolCode, order_id: OrderId, time: DateTime<Utc>, market_fill_price: Price, tag: String},
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
    pub position_calculation_mode: PositionCalculationMode,
    pub market_price_service: Arc<MarketPriceService>
    //todo, add daily max loss, max order size etc to ledger
}

impl Ledger {
    pub(crate) fn new(
        account_info: AccountInfo,
        mode: StrategyMode,
        synchronise_accounts: bool,
        strategy_sender: Sender<StrategyEvent>,
        position_calculation_mode: PositionCalculationMode,
        market_price_service: Arc<MarketPriceService>

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
            position_calculation_mode,
            market_price_service,
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
               /* match &message {
                    LedgerMessage::SyncPosition { .. } => eprintln!("{:?}", message),
                    LedgerMessage::UpdateOrCreatePosition { .. } => eprintln!("{:?}", message),
                    _ => {}
                }*/
                //let mut position_locks = AHashMap::new();
                match message {
                    #[allow(unused_variables)]
                    LedgerMessage::SyncPosition { symbol_name, symbol_code, account, open_quantity, average_price, side, open_pnl, time } => {
                      /*  if !static_self.is_simulating_pnl {
                            if !position_locks.contains_key(&symbol_code) {
                                position_locks.insert(symbol_code.clone(), Mutex::new(()));
                            }
                            if let Some(lock) = position_locks.get(&symbol_code) {
                                let _guard = lock.lock().await;
                                static_self.synchronize_live_position(symbol_name, symbol_code, account, open_quantity, average_price, side, open_pnl, time).await;
                            }
                        }*/
                    }
                    LedgerMessage::UpdateOrCreatePosition { symbol_name, symbol_code, quantity, side, time, market_fill_price, tag , paper_response_sender, order_id} => {
                        match mode {
                            StrategyMode::Backtest | StrategyMode::LivePaperTrading => static_self.update_or_create_paper_position(symbol_name, symbol_code, quantity, side, time, market_fill_price, tag, order_id.clone(), paper_response_sender.unwrap()).await,
                            StrategyMode::Live => {
                                if static_self.is_simulating_pnl {
                                    static_self.update_or_create_live_position(symbol_name, symbol_code, order_id, quantity, side, time, market_fill_price, tag).await
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
                    LedgerMessage::ExitPaperPosition { symbol_code, order_id,time, market_fill_price, tag } => {
                        static_self.paper_exit_position(order_id, &symbol_code, time, market_fill_price, tag).await;
                    }
                    LedgerMessage::PaperFlattenAll { time } => {
                        static_self.flatten_all_for_paper_account(time).await;
                    }
                }
            }
        });
    }

    /*async fn synchronize_live_position(&mut self, symbol_name: SymbolName, symbol_code: SymbolCode, account: Account, open_quantity: f64, average_price: f64, side: PositionSide, open_pnl: f64, time: String) {
        //sleep(std::time::Duration::from_millis(100)).await;
        let mut to_remove = false;
        let mut to_create = true;
        let pnl = match Decimal::from_f64(open_pnl) {
            Some(pnl) => pnl,
            None => return
        };
        let quantity = match Decimal::from_f64(open_quantity) {
            Some(quantity) => quantity,
            None => return
        };

        let average_price = match Decimal::from_f64(average_price) {
            Some(price) => price,
            None => return
        };

        let position_exists = self.positions.contains_key(&symbol_code);
        if position_exists {
            if let Some(mut position) = self.positions.get_mut(&symbol_code) {
                position.open_pnl = pnl;
                if position.quantity_open == quantity && position.side == side {
                    return
                }
                if position.side == side {
                    if position.quantity_open == quantity {
                        return;
                    }
                    if quantity > position.quantity_open {
                        let event = PositionUpdateEvent::Increased {
                            position_id: position.position_id.clone(),
                            side,
                            total_quantity_open: quantity,
                            average_price,
                            symbol_name: position.symbol_name.clone(),
                            symbol_code: symbol_code.clone(),
                            open_pnl: pnl,
                            booked_pnl: position.booked_pnl.clone(),
                            account: account.clone(),
                            originating_order_tag: position.tag.clone(),
                            time: time.clone(),
                        };
                        position.quantity_open = quantity;
                        position.average_price = average_price;
                        self.strategy_sender.send(StrategyEvent::PositionEvents(event)).await.unwrap();
                    } else if quantity < position.quantity_open {
                        let order_side = match side {
                            PositionSide::Long => OrderSide::Buy,
                            PositionSide::Short => OrderSide::Sell,
                            PositionSide::Flat => OrderSide::Sell //this is only for market price so it's not critical
                        };
                        let reduced_size = position.quantity_open - quantity;
                        let exchange_rate = if self.currency != position.symbol_info.pnl_currency {
                            match get_exchange_rate(self.currency, position.symbol_info.pnl_currency, Utc::now(), order_side).await {
                                Ok(rate) => {
                                    self.rates.insert(position.symbol_info.pnl_currency, rate);
                                    rate
                                },
                                Err(_e) => self.get_exchange_multiplier(position.symbol_info.pnl_currency)
                            }
                        } else {
                            dec!(1.0)
                        };
                        let market_price = match average_price > dec!(0) {
                            false => match price_service_request_market_price(order_side, position.symbol_name.clone(), symbol_code.clone()).await {
                                Ok(price) => match price {
                                    PriceServiceResponse::MarketPrice(price) => match price {
                                        None => return,
                                        Some(p) => p
                                    },
                                    _ => return
                                },
                                Err(_) => return
                            },
                            true => average_price
                        };
                        let event = position.reduce_position_size(market_price, reduced_size, "NULL".to_string(), self.currency, exchange_rate, Utc::now(), "Synchronizing Position: Reduce Size".to_string()).await;
                        self.strategy_sender.send(StrategyEvent::PositionEvents(event)).await.unwrap();
                        if position.is_closed {
                            to_remove = true;
                        }
                    }
                    to_create = false;
                } else if side != position.side {
                    let order_side = match side {
                        PositionSide::Long => OrderSide::Sell,
                        PositionSide::Short => OrderSide::Buy,
                        PositionSide::Flat => OrderSide::Sell //this is only for market price so it's not critical
                    };
                    let market_price = match price_service_request_market_price(order_side, position.symbol_name.clone(), symbol_code.clone()).await {
                        Ok(price) => match price {
                            PriceServiceResponse::MarketPrice(price) => match price {
                                None => return,
                                Some(p) => p
                            }
                            _ => return
                        },
                        Err(_) => return
                    };
                    let exchange_rate = if self.currency != position.symbol_info.pnl_currency {
                        match get_exchange_rate(self.currency, position.symbol_info.pnl_currency, Utc::now(), order_side).await {
                            Ok(rate) => {
                                self.rates.insert(position.symbol_info.pnl_currency, rate);
                                rate
                            },
                            Err(_e) => self.get_exchange_multiplier(position.symbol_info.pnl_currency)
                        }
                    } else {
                        dec!(1.0)
                    };
                    let quantity = position.quantity_open.clone();
                    let event = position.reduce_position_size(market_price, quantity, "NULL".to_string(), self.currency, exchange_rate, Utc::now(), "Synchronizing Position: Reduce Size".to_string()).await;
                    self.strategy_sender.send(StrategyEvent::PositionEvents(event)).await.unwrap();
                    to_remove = true;
                    to_create = true;
                }
                if to_remove {
                    match self.positions.remove(&symbol_code) {
                        Some((_,p)) => self.positions_closed.entry(symbol_code.clone()).or_insert_with(Vec::new).push(p),
                        None => {}
                    }
                }
            }
        }
        if to_create && quantity > dec!(0) {
            let order_side = match side {
                PositionSide::Long => OrderSide::Buy,
                PositionSide::Short => OrderSide::Sell,
                PositionSide::Flat => return
            };
            self.update_or_create_live_position(symbol_name, symbol_code.clone(), "NULL".to_string(), quantity, order_side, Utc::now(), average_price, "Synchronizing Position: Position Opened".to_string()).await;
            if let Some(position) = self.positions.get(&symbol_code) {
                let event = PositionUpdateEvent::PositionOpened {
                    average_price,
                    position_id: position.position_id.clone(),
                    side,
                    account,
                    symbol_name: position.symbol_name.clone(),
                    symbol_code: position.symbol_code.clone(),
                    originating_order_tag: position.tag.clone(),
                    time,
                };
                self.strategy_sender.send(StrategyEvent::PositionEvents(event)).await.unwrap();
            }
        }
    }*/

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

    pub fn ledger_statistics_to_string(&self) -> String {
        let mut total_trades: usize = 0;
        let mut losses: usize = 0;
        let mut wins: usize = 0;
        let mut win_pnl = dec!(0.0);
        let mut loss_pnl = dec!(0.0);
        let mut pnl = dec!(0.0);
        let mut max_drawdown = dec!(0.0);
        let mut peak = dec!(0.0);
        let mut running_pnl = dec!(0.0);

        // Track running PNL and maximum drawdown
        for trades in self.positions_closed.iter() {
            total_trades += trades.value().len();
            for position in trades.value() {
                running_pnl += position.booked_pnl;
                if running_pnl > peak {
                    peak = running_pnl;
                }

                let drawdown = peak - running_pnl;
                if drawdown > max_drawdown {
                    max_drawdown = drawdown;
                }

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
            win_pnl / Decimal::from(wins)
        } else {
            dec!(0.0)
        };

        let avg_loss_pnl = if losses > 0 {
            loss_pnl / Decimal::from(losses)
        } else {
            dec!(0.0)
        };

        // Calculate win rate
        let win_rate = if total_trades > 0 {
            (Decimal::from_usize(wins).unwrap() / Decimal::from_usize(total_trades).unwrap()) * dec!(100.0)
        } else {
            dec!(0.0)
        };

        // Calculate risk-reward ratio
        let risk_reward = if losses == 0 && wins > 0 {
            avg_win_pnl
        } else if wins == 0 && losses > 0 {
            dec!(0.0)
        } else if avg_loss_pnl < dec!(0.0) && avg_win_pnl > dec!(0.0) {
            avg_win_pnl / -avg_loss_pnl
        } else {
            dec!(0.0)
        };

        // Calculate profit factor
        let profit_factor = if loss_pnl != dec!(0.0) {
            win_pnl / -loss_pnl
        } else if win_pnl > dec!(0.0) {
            dec!(1000.0)
        } else {
            dec!(0.0)
        };

        // Calculate Quality Ratio (win rate as decimal × profit factor)
        // Provides a combined measure of consistency and profitability
        let quality_ratio = (win_rate / dec!(100.0)) * profit_factor;

        // Calculate Reward to Drawdown Ratio (total profit / maximum drawdown)
        let pain_2_gain = if max_drawdown > dec!(0.0) {
            pnl / max_drawdown
        } else if pnl > dec!(0.0) {
            dec!(1000.0) // When profitable with no drawdown
        } else {
            dec!(0.0)
        };

        let break_even = total_trades - wins - losses;
        let cash_value = self.cash_value.clone();
        let cash_used = self.cash_used.clone();
        let cash_available = self.cash_available.clone();
        let commission_paid = self.commissions_paid.clone();
        let pnl = self.total_booked_pnl.clone();

        format!(
            "Account: {}, Balance: {} {}, Win Rate: {}%, Average Risk Reward: {}, \
         Profit Factor: {}, Quality Ratio: {},  Pain to Gain Ratio: {}, \
         Max Drawdown: {}, Total profit: {}, Total Wins: {}, Total Losses: {}, \
         Break Even: {}, Total Positions: {}, Open Positions: {}, \
         Cash Used: {}, Cash Available: {}, Commission Paid: {}",
            self.account,
            cash_value.round_dp(2),
            self.currency,
            win_rate.round_dp(2),
            risk_reward.round_dp(2),
            profit_factor.round_dp(2),
            quality_ratio.round_dp(2),
            pain_2_gain.round_dp(2),
            max_drawdown.round_dp(2),
            pnl.round_dp(2),
            wins,
            losses,
            break_even,
            total_trades,
            self.positions.len(),
            cash_used.round_dp(2),
            cash_available.round_dp(2),
            commission_paid
        )
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
        order_id: OrderId,
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
                let quantity = min(remaining_quantity, existing_position.quantity_open);
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
                let event= existing_position.reduce_position_size(market_fill_price, quantity, order_id.clone(), self.currency, exchange_rate, time, tag.clone()).await;
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
                let event = existing_position.add_to_position(self.mode, self.is_simulating_pnl, order_id.clone(), self.currency, market_fill_price, quantity, time, tag.clone()).await;
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
                order_id,
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
                average_price: market_fill_price,
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
                                result: trade.result.to_string()
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

    pub fn trade_statistics_to_string(&self) -> String {
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
                    total_pnl += trade.profit;

                    match trade.result {
                        TradeResult::Win => {
                            wins += 1;
                            win_pnl += trade.profit;
                            largest_win = largest_win.max(trade.profit);
                        }
                        TradeResult::Loss => {
                            losses += 1;
                            loss_pnl += trade.profit;
                            largest_loss = largest_loss.min(trade.profit);
                        },
                        TradeResult::BreakEven => break_even += 1,
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

        // Calculate average risk/reward
        let risk_reward = if avg_loss.abs() > dec!(0.0) {
            avg_win / avg_loss.abs()
        } else if avg_win > dec!(0.0) {
            dec!(1000.0) // No losses, so effectively infinite R:R
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
        Average Risk/Reward: {}\n\
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
            risk_reward.round_dp(2),
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
    result: String,
}

#[cfg(test)]
mod test {
    use super::*;
    use rust_decimal_macros::dec;
    use crate::apis::rithmic::rithmic_systems::RithmicSystem;

    async fn setup_test_ledger() -> (Ledger, tokio::sync::mpsc::Receiver<StrategyEvent>) {
        let (strategy_sender, strategy_receiver) = tokio::sync::mpsc::channel(100);

        let account_info = AccountInfo {
            brokerage: Brokerage::Rithmic(RithmicSystem::Rithmic01),
            account_id: "TEST-123".to_string(),
            cash_value: dec!(100000),
            cash_available: dec!(100000),
            cash_used: dec!(0),
            currency: Currency::USD,
            open_pnl: Default::default(),
            booked_pnl: Default::default(),
            day_open_pnl: Default::default(),
            positions: vec![],
            is_hedging: false,
            buy_limit: None,
            sell_limit: None,
            max_orders: None,
            daily_max_loss: None,
            daily_max_loss_reset_time: None,
            day_booked_pnl: Default::default(),
            leverage: 0,
        };

        let ledger = Ledger::new(
            account_info,
            StrategyMode::Backtest,
            false,
            strategy_sender,
            PositionCalculationMode::FIFO,
        );

        (ledger, strategy_receiver)
    }

    //todo, total profit is wrong, somewhere ledger calulates final proft wrong
    #[tokio::test]
    async fn test_position_pnl_calculation() {
        let (mut ledger, mut strategy_receiver) = setup_test_ledger().await;

        // Spawn a task to consume strategy events
        let event_handler = tokio::spawn(async move {
            while let Some(event) = strategy_receiver.recv().await {
                println!("Received strategy event: {:?}", event);
            }
        });

        let symbol_name = "NQ".to_string();
        let symbol_code = "NQZ4".to_string();
        let tag = "test".to_string();
        let time = Utc::now();

        // Create a long position
        println!("\nOpening long position...");
        let (tx, rx) = tokio::sync::oneshot::channel();
        ledger.update_or_create_paper_position(
            symbol_name.clone(),
            symbol_code.clone(),
            dec!(2.0),             // quantity
            OrderSide::Buy,        // side
            time,                  // time
            dec!(17500.0),         // entry price
            tag.clone(),
            "order1".to_string(),
            tx,
        ).await;
        println!("Ledger total profit: {}", ledger.total_booked_pnl);
        let quantity_open = ledger.position_size(&symbol_code);
        println!("Position size: {}", quantity_open);
        assert_eq!(quantity_open, dec!(2.0));
        assert_eq!(ledger.total_booked_pnl, dec!(0.0));
        let _ = rx.await;

        // Add to the position
        println!("\nAdding to position...");
        let (tx, rx) = tokio::sync::oneshot::channel();
        ledger.update_or_create_paper_position(
            symbol_name.clone(),
            symbol_code.clone(),
            dec!(1.0),             // quantity
            OrderSide::Buy,        // side
            time + Duration::minutes(5),
            dec!(17525.0),         // entry price
            "add".to_string(),
            "order2".to_string(),
            tx,
        ).await;
        println!("Ledger total profit: {}", ledger.total_booked_pnl);
        let quantity_open = ledger.position_size(&symbol_code);
        println!("Position size: {}", quantity_open);
        assert_eq!(quantity_open, dec!(3.0));
        assert_eq!(ledger.total_booked_pnl, dec!(0.0));
        let _ = rx.await;

        // Partial exit
        println!("\nPartial exit...");
        let (tx, rx) = tokio::sync::oneshot::channel();
        ledger.update_or_create_paper_position(
            symbol_name.clone(),
            symbol_code.clone(),
            dec!(1.0),             // quantity
            OrderSide::Sell,       // side
            time + Duration::minutes(10),
            dec!(17550.0),         // exit price
            "partial_exit".to_string(),
            "order3".to_string(),
            tx,
        ).await;
        println!("Ledger total profit: {}", ledger.total_booked_pnl);
        let quantity_open = ledger.position_size(&symbol_code);
        println!("Position size: {}", quantity_open);
        assert_eq!(quantity_open, dec!(2.0));
        assert_eq!(ledger.total_booked_pnl, dec!(1000.0));
        let _ = rx.await;

        // Final exit
        println!("\nFinal exit...");
        ledger.paper_exit_position(
            "Test".to_string(),
            &symbol_code,
            time + Duration::minutes(15),
            dec!(17575.0),         // exit price
            "final_exit".to_string(),
        ).await;
        println!("Ledger total profit: {}", ledger.total_booked_pnl);
        let quantity_open = ledger.position_size(&symbol_code);
        assert_eq!(quantity_open, dec!(0.0));
        assert_eq!(ledger.total_booked_pnl, dec!(3500.0));
        println!("Position size: {}", quantity_open);

        // Print detailed position information
        {
            let closed_positions = ledger.positions_closed.get(&symbol_code);
            if let Some(positions) = closed_positions {
                for position in positions.value() {
                    println!("\nClosed Position Details:");
                    println!("  Total Booked PnL: {}", position.booked_pnl);
                    println!("  Number of trades: {}", position.completed_trades.len());

                    println!("\nIndividual Trades:");
                    for trade in &position.completed_trades {
                        println!("  Entry Price: {}, Exit Price: {}, Quantity: {}, PnL: {}, Result: {:?}",
                                 trade.entry_price,
                                 trade.exit_price,
                                 trade.entry_quantity,
                                 trade.profit,
                                 trade.result);
                    }
                }
            }
        }

        println!("\nLedger Statistics:");
        let ledger_stats = ledger.ledger_statistics_to_string();
        println!("{}", ledger_stats);

        println!("\nTrade Statistics:");
        let trade_stats = ledger.trade_statistics_to_string();
        println!("{}", trade_stats);

        let expected_first_trade_pnl = dec!(1000.0);
        let expected_second_trade_pnl = dec!(1500.0);
        let expected_third_trade_pnl = dec!(1000.0);
        let expected_total_pnl = expected_first_trade_pnl + expected_second_trade_pnl + expected_third_trade_pnl;

        {
            let closed_positions = ledger.positions_closed.get(&symbol_code);
            if let Some(positions) = closed_positions {
                let position = &positions.value()[0];

                // Verify individual trade PnL
                assert_eq!(
                    position.completed_trades[0].profit,
                    expected_first_trade_pnl,
                    "First trade PnL mismatch: expected {}, got {}",
                    expected_first_trade_pnl,
                    position.completed_trades[0].profit
                );

                assert_eq!(
                    position.completed_trades[1].profit,
                    expected_second_trade_pnl,
                    "Second trade PnL mismatch: expected {}, got {}",
                    expected_second_trade_pnl,
                    position.completed_trades[1].profit
                );

                assert_eq!(
                    position.completed_trades[2].profit,
                    expected_third_trade_pnl,
                    "Third trade PnL mismatch: expected {}, got {}",
                    expected_third_trade_pnl,
                    position.completed_trades[2].profit
                );

                // Verify total PnL
                assert_eq!(
                    position.booked_pnl,
                    expected_total_pnl,
                    "Total PnL mismatch: expected {}, got {}",
                    expected_total_pnl,
                    position.booked_pnl
                );
            }
        }

        // Wait for all events to be processed
        event_handler.abort();
    }

    #[tokio::test]
    async fn test_multiple_positions_pnl_calculation() {
        let (mut ledger, mut strategy_receiver) = setup_test_ledger().await;

        // Spawn a task to consume strategy events
        let event_handler = tokio::spawn(async move {
            while let Some(event) = strategy_receiver.recv().await {
                println!("Received strategy event: {:?}", event);
            }
        });

        let symbol1 = "NQ".to_string();
        let symbol1_code = "NQZ4".to_string();
        let symbol2 = "ES".to_string();
        let symbol2_code = "ESZ4".to_string();
        let time = Utc::now();

        // Opening first position for Symbol1
        println!("\nOpening long position for Symbol1...");
        let (tx, rx) = tokio::sync::oneshot::channel();
        ledger
            .update_or_create_paper_position(
                symbol1.clone(),
                symbol1_code.clone(),
                dec!(2.0),             // quantity
                OrderSide::Buy,        // side
                time,                  // time
                dec!(17500.0),         // entry price
                "test".to_string(),
                "order1".to_string(),
                tx,
            )
            .await;
        let _ = rx.await;

        println!("Ledger total profit: {}", ledger.total_booked_pnl);
        let qty_symbol1 = ledger.position_size(&symbol1_code);
        println!("Position size Symbol1: {}", qty_symbol1);
        assert_eq!(qty_symbol1, dec!(2.0));

        // Opening position for Symbol2
        println!("\nOpening long position for Symbol2...");
        let (tx, rx) = tokio::sync::oneshot::channel();
        ledger
            .update_or_create_paper_position(
                symbol2.clone(),
                symbol2_code.clone(),
                dec!(1.0),             // quantity
                OrderSide::Buy,        // side
                time + Duration::minutes(2),
                dec!(4500.0),          // entry price
                "test".to_string(),
                "order2".to_string(),
                tx,
            )
            .await;
        let _ = rx.await;

        println!("Ledger total profit: {}", ledger.total_booked_pnl);
        let qty_symbol2 = ledger.position_size(&symbol2_code);
        println!("Position size Symbol2: {}", qty_symbol2);
        assert_eq!(qty_symbol2, dec!(1.0));

        // Adding to Symbol1 position
        println!("\nAdding to Symbol1 position...");
        let (tx, rx) = tokio::sync::oneshot::channel();
        ledger
            .update_or_create_paper_position(
                symbol1.clone(),
                symbol1_code.clone(),
                dec!(1.0),             // quantity
                OrderSide::Buy,        // side
                time + Duration::minutes(5),
                dec!(17525.0),         // entry price
                "add".to_string(),
                "order3".to_string(),
                tx,
            )
            .await;
        let _ = rx.await;

        println!("Ledger total profit: {}", ledger.total_booked_pnl);
        let qty_symbol1 = ledger.position_size(&symbol1_code);
        println!("Position size Symbol1: {}", qty_symbol1);
        assert_eq!(qty_symbol1, dec!(3.0));

        // Partial exit for Symbol1
        println!("\nPartial exit for Symbol1...");
        let (tx, rx) = tokio::sync::oneshot::channel();
        ledger
            .update_or_create_paper_position(
                symbol1.clone(),
                symbol1_code.clone(),
                dec!(1.0),             // quantity
                OrderSide::Sell,       // side
                time + Duration::minutes(10),
                dec!(17550.0),         // exit price
                "partial_exit".to_string(),
                "order4".to_string(),
                tx,
            )
            .await;
        let _ = rx.await;

        println!("Ledger total profit: {}", ledger.total_booked_pnl);
        let qty_symbol1 = ledger.position_size(&symbol1_code);
        println!("Position size Symbol1: {}", qty_symbol1);
        assert_eq!(qty_symbol1, dec!(2.0));

        // Exiting Symbol2 completely
        println!("\nFinal exit for Symbol2...");
        ledger
            .paper_exit_position(
                "Test".to_string(),
                &symbol2_code,
                time + Duration::minutes(15),
                dec!(4550.0),         // exit price
                "final_exit".to_string(),
            )
            .await;

        println!("Ledger total profit: {}", ledger.total_booked_pnl);
        let qty_symbol2 = ledger.position_size(&symbol2_code);
        println!("Position size Symbol2: {}", qty_symbol2);
        assert_eq!(qty_symbol2, dec!(0.0));

        // Final exit for Symbol1
        println!("\nFinal exit for Symbol1...");
        ledger
            .paper_exit_position(
                "Test".to_string(),
                &symbol1_code,
                time + Duration::minutes(20),
                dec!(17575.0),        // exit price
                "final_exit".to_string(),
            )
            .await;

        println!("Ledger total profit: {}", ledger.total_booked_pnl);
        let qty_symbol1 = ledger.position_size(&symbol1_code);
        assert_eq!(qty_symbol1, dec!(0.0));

        let expected_pnl_symbol1 = dec!(3500.0); // NQ
        let expected_pnl_symbol2 = dec!(2500.0); // ES
        let expected_total_pnl = expected_pnl_symbol1 + expected_pnl_symbol2;

        assert_eq!(
            ledger.total_booked_pnl,
            expected_total_pnl,
            "Total PnL mismatch: expected {}, got {}",
            expected_total_pnl,
            ledger.total_booked_pnl
        );

        // Print detailed information
        println!("\nFinal Ledger Statistics:");
        println!("{}", ledger.ledger_statistics_to_string());

        println!("\nTrade Statistics:");
        println!("{}", ledger.trade_statistics_to_string());

        // Wait for all events to be processed
        event_handler.abort();
    }

    #[tokio::test]
    async fn test_short_positions_pnl_calculation() {
        let (mut ledger, mut strategy_receiver) = setup_test_ledger().await;

        let event_handler = tokio::spawn(async move {
            while let Some(event) = strategy_receiver.recv().await {
                println!("Received strategy event: {:?}", event);
            }
        });

        let symbol1 = "NQ".to_string();
        let symbol1_code = "NQZ4".to_string();
        let symbol2 = "ES".to_string();
        let symbol2_code = "ESZ4".to_string();
        let time = Utc::now();

        // Opening first short position for Symbol1
        println!("\nOpening short position for Symbol1...");
        let (tx, rx) = tokio::sync::oneshot::channel();
        ledger
            .update_or_create_paper_position(
                symbol1.clone(),
                symbol1_code.clone(),
                dec!(2.0),             // quantity stays positive
                OrderSide::Sell,       // side (SHORT)
                time,
                dec!(17500.0),         // entry price
                "test".to_string(),
                "order1".to_string(),
                tx,
            )
            .await;
        let _ = rx.await;

        assert_eq!(ledger.position_size(&symbol1_code), dec!(2.0)); // Position size stays positive

        // Opening short position for Symbol2
        println!("\nOpening short position for Symbol2...");
        let (tx, rx) = tokio::sync::oneshot::channel();
        ledger
            .update_or_create_paper_position(
                symbol2.clone(),
                symbol2_code.clone(),
                dec!(1.0),             // quantity stays positive
                OrderSide::Sell,       // side (SHORT)
                time + Duration::minutes(2),
                dec!(4500.0),          // entry price
                "test".to_string(),
                "order2".to_string(),
                tx,
            )
            .await;
        let _ = rx.await;

        assert_eq!(ledger.position_size(&symbol2_code), dec!(1.0)); // Position size stays positive

        // Covering half of Symbol1 position
        println!("\nPartial cover for Symbol1...");
        let (tx, rx) = tokio::sync::oneshot::channel();
        ledger
            .update_or_create_paper_position(
                symbol1.clone(),
                symbol1_code.clone(),
                dec!(1.0),             // quantity to reduce
                OrderSide::Buy,        // side (COVER)
                time + Duration::minutes(10),
                dec!(17450.0),         // exit price (profit - price went down)
                "partial_cover".to_string(),
                "order3".to_string(),
                tx,
            )
            .await;
        let _ = rx.await;

        assert_eq!(ledger.position_size(&symbol1_code), dec!(1.0)); // Should be reduced by 1.0

        // Cover Symbol2 completely
        println!("\nFinal cover for Symbol2...");
        ledger
            .paper_exit_position(
                "Test".to_string(),
                &symbol2_code,
                time + Duration::minutes(15),
                dec!(4450.0),          // exit price (profit - price went down)
                "final_cover".to_string(),
            )
            .await;

        // Final cover for Symbol1
        println!("\nFinal cover for Symbol1...");
        ledger
            .paper_exit_position(
                "Test".to_string(),
                &symbol1_code,
                time + Duration::minutes(20),
                dec!(17400.0),         // exit price (profit - price went down)
                "final_cover".to_string(),
            )
            .await;

        let expected_pnl_symbol1 = dec!(3000.0); // (17500 - 17450) * 1 + (17500 - 17400) * 1
        let expected_pnl_symbol2 = dec!(2500.0); // (4500 - 4450) * 1
        let expected_total_pnl = expected_pnl_symbol1 + expected_pnl_symbol2;

        assert_eq!(
            ledger.total_booked_pnl,
            expected_total_pnl,
            "Total PnL mismatch: expected {}, got {}",
            expected_total_pnl,
            ledger.total_booked_pnl
        );

        println!("\nFinal Ledger Statistics:");
        println!("{}", ledger.ledger_statistics_to_string());

        println!("\nTrade Statistics:");
        println!("{}", ledger.trade_statistics_to_string());

        event_handler.abort();
    }

    #[tokio::test]
    async fn test_losing_trades_pnl_calculation() {
        let (mut ledger, mut strategy_receiver) = setup_test_ledger().await;

        let event_handler = tokio::spawn(async move {
            while let Some(event) = strategy_receiver.recv().await {
                println!("Received strategy event: {:?}", event);
            }
        });

        let symbol1 = "NQ".to_string();
        let symbol1_code = "NQZ4".to_string();
        let symbol2 = "ES".to_string();
        let symbol2_code = "ESZ4".to_string();
        let time = Utc::now();

        // Opening long position for Symbol1 (will be a losing trade)
        println!("\nOpening long position for Symbol1...");
        let (tx, rx) = tokio::sync::oneshot::channel();
        ledger
            .update_or_create_paper_position(
                symbol1.clone(),
                symbol1_code.clone(),
                dec!(2.0),             // quantity
                OrderSide::Buy,        // side
                time,
                dec!(17500.0),         // entry price
                "test".to_string(),
                "order1".to_string(),
                tx,
            )
            .await;
        let _ = rx.await;

        // Opening short position for Symbol2 (will be a losing trade)
        println!("\nOpening short position for Symbol2...");
        let (tx, rx) = tokio::sync::oneshot::channel();
        ledger
            .update_or_create_paper_position(
                symbol2.clone(),
                symbol2_code.clone(),
                dec!(1.0),             // quantity
                OrderSide::Sell,       // side
                time + Duration::minutes(2),
                dec!(4500.0),          // entry price
                "test".to_string(),
                "order2".to_string(),
                tx,
            )
            .await;
        let _ = rx.await;

        // Partial exit for Symbol1 at a loss
        println!("\nPartial exit for Symbol1...");
        let (tx, rx) = tokio::sync::oneshot::channel();
        ledger
            .update_or_create_paper_position(
                symbol1.clone(),
                symbol1_code.clone(),
                dec!(1.0),             // quantity
                OrderSide::Sell,       // side
                time + Duration::minutes(10),
                dec!(17450.0),         // exit price (LOSS - price went down)
                "partial_exit".to_string(),
                "order3".to_string(),
                tx,
            )
            .await;
        let _ = rx.await;

        // Exit Symbol2 at a loss
        println!("\nFinal exit for Symbol2...");
        ledger
            .paper_exit_position(
                "Test".to_string(),
                &symbol2_code,
                time + Duration::minutes(15),
                dec!(4550.0),          // exit price (LOSS - price went up)
                "final_exit".to_string(),
            )
            .await;

        // Final exit for Symbol1 at a further loss
        println!("\nFinal exit for Symbol1...");
        ledger
            .paper_exit_position(
                "Test".to_string(),
                &symbol1_code,
                time + Duration::minutes(20),
                dec!(17400.0),         // exit price (LOSS - price went down more)
                "final_exit".to_string(),
            )
            .await;

        let expected_pnl_symbol1 = dec!(-3000.0); // (17450 - 17500) * 1 + (17400 - 17500) * 1
        let expected_pnl_symbol2 = dec!(-2500.0); // (4500 - 4550) * 1
        let expected_total_pnl = expected_pnl_symbol1 + expected_pnl_symbol2;

        assert_eq!(
            ledger.total_booked_pnl,
            expected_total_pnl,
            "Total PnL mismatch: expected {}, got {}",
            expected_total_pnl,
            ledger.total_booked_pnl
        );

        println!("\nFinal Ledger Statistics:");
        println!("{}", ledger.ledger_statistics_to_string());

        println!("\nTrade Statistics:");
        println!("{}", ledger.trade_statistics_to_string());

        event_handler.abort();
    }
}

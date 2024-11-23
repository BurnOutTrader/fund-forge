use std::sync::Arc;
use chrono::{DateTime, Utc};
use crate::standardized_types::enums::{OrderSide, PositionSide, StrategyMode};
use crate::standardized_types::subscriptions::{SymbolCode, SymbolName};
use dashmap::DashMap;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tokio::sync::{oneshot};
use crate::standardized_types::position::{Position, PositionCalculationMode};
use crate::standardized_types::accounts::{Account, Currency};
use crate::standardized_types::broker_enum::Brokerage;
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::orders::{OrderId, OrderUpdateEvent};
use crate::standardized_types::time_slices::TimeSlice;
use crate::strategies::ledgers::ledger::{Ledger, LedgerMessage};
use crate::strategies::strategy_events::StrategyEvent;

pub(crate) struct LedgerService {
    pub (crate) ledgers: DashMap<Account, &'static Ledger>,
    ledger_senders: DashMap<Account, tokio::sync::mpsc::Sender<LedgerMessage>>,
    strategy_sender: tokio::sync::mpsc::Sender<StrategyEvent>,
}

impl LedgerService {
    pub fn new(strategy_sender: tokio::sync::mpsc::Sender<StrategyEvent>) -> Self {
        LedgerService {
            ledgers: Default::default(),
            ledger_senders: Default::default(),
            strategy_sender,
        }
    }

    pub async fn synchronize_live_position(&self, symbol_name: SymbolName, symbol_code: SymbolCode, account: Account, open_quantity: f64, average_price: f64, side: PositionSide, open_pnl: f64, time: String) {
        if let Some(sender) = self.ledger_senders.get(&account) {
            let msg = LedgerMessage::SyncPosition{symbol_name, symbol_code, account, open_quantity, average_price, side, open_pnl, time};
            sender.send(msg).await.unwrap();
        }
    }

    pub async fn flatten_all_for_paper_account(&self, account: Account, time: DateTime<Utc>) {
        if let Some(sender) = self.ledger_senders.get(&account) {
            let msg = LedgerMessage::PaperFlattenAll{time};
            sender.send(msg).await.unwrap();
        }
    }

    pub fn get_positions(&self, account: &Account) -> DashMap<SymbolCode, Vec<Position>> {
        if let Some(ledger) = self.ledgers.get(account) {
            ledger.value().positions_closed.clone()
        } else {
            Default::default()
        }
    }

    pub fn save_positions_to_file(&self, account: &Account, file_path: &str) {
        if let Some(ledger) = self.ledgers.get(account) {
            ledger.value().save_positions_to_file(file_path);
        }
    }

    pub fn balance(&self, account: &Account) -> Decimal {
        self.ledgers.get(account)
            .map(|ledger| ledger.balance())
            .unwrap_or_else(|| dec!(0))
    }

    pub(crate) async fn update_or_create_position(
        &self,
        account: &Account,
        symbol_name: SymbolName,
        symbol_code: SymbolCode,
        quantity: Volume,
        side: OrderSide,
        time: DateTime<Utc>,
        market_fill_price: Price, // we use the passed in price because we don't know what sort of order was filled, limit or market
        tag: String,
        paper_response_sender: Option<oneshot::Sender<Option<OrderUpdateEvent>>>,
        order_id: OrderId
    ) {
        if let Some(sender) = self.ledger_senders.get(account) {
            let msg = LedgerMessage::UpdateOrCreatePosition{symbol_name, symbol_code, quantity, side, time, market_fill_price, tag, paper_response_sender, order_id};
            sender.send(msg).await.unwrap();
        }
    }

    pub(crate) async fn paper_exit_position(
        &self,
        account: &Account,
        symbol_code: SymbolCode,
        order_id: OrderId,
        time: DateTime<Utc>,
        market_price: Price,
        tag: String
    ) {
        if let Some(ledger_sender) = self.ledger_senders.get(account) {
            let msg = LedgerMessage::ExitPaperPosition {
                symbol_code,
                order_id,
                time,
                tag,
                market_fill_price: market_price,
            };
            ledger_sender.send(msg).await.unwrap();
        }
    }

    pub async fn live_account_updates(&self, account: &Account, cash_value: Decimal, cash_available: Decimal, cash_used: Decimal) {
        if let Some(ledger_sender) = self.ledger_senders.get(account) {
            let msg = LedgerMessage::LiveAccountUpdate{cash_value, cash_available, cash_used};
            ledger_sender.send(msg).await.unwrap();
        }
    }

    pub fn print_ledgers(&self) {
        for ledger in self.ledgers.iter() {
            let msg = ledger.value().ledger_statistics_to_string();
            println!("{}", msg);
        }
    }

    pub fn export_trades_to_csv(&self, account: &Account, directory: &str) {
        if let Some(ledger) = self.ledgers.get(account) {
            ledger.export_trades_to_csv(directory);
        }
    }

    pub fn export_positions_to_csv(&self, account: &Account, directory: &str) {
        if let Some(ledger) = self.ledgers.get(account) {
            ledger.export_positions_to_csv(directory);
        }
    }

    pub fn print_trade_statistics(&self, account: &Account) {
        if let Some(ledger) = self.ledgers.get(account) {
            let msg = ledger.trade_statistics_to_string();
            println!("{}", msg);
        }
    }

    pub fn booked_pnl_account(&self, account: &Account) -> Decimal {
        if let Some(ledger) = self.ledgers.get(account) {
            ledger.total_booked_pnl.clone()
        } else {
            dec!(0)
        }
    }

    pub fn print_ledger(&self, account: &Account) {
       if let Some(ledger) = self.ledgers.get(account) {
           let string = ledger.value().ledger_statistics_to_string(); //todo need to return the string here
           println!("{}", string);
       }
    }

    pub async fn timeslice_updates(&self, time_slice: Arc<TimeSlice>) {
        for ledger in self.ledger_senders.iter() {
            let update_message = LedgerMessage::TimeSliceUpdate{time_slice: time_slice.clone()};
            ledger.value().send(update_message).await.unwrap();
        }
    }

    pub async fn init_ledger(&self, account: &Account, strategy_mode: StrategyMode, synchronize_accounts: bool, starting_cash: Decimal, currency: Currency) {
        let position_calculation_mode = match account.brokerage {
            Brokerage::Rithmic(_) => PositionCalculationMode::LIFO,
            _ => PositionCalculationMode::LIFO,
        };
        if !self.ledgers.contains_key(account) {
            match strategy_mode {
                StrategyMode::Live => {
                    let account_info = match account.brokerage.account_info(account.account_id.clone()).await {
                        Ok(ledger) => ledger,
                        Err(e) => {
                            panic!("LEDGER_SERVICE: Error initializing account: {}", e);
                        }
                    };
                    // Convert the Ledger to a static reference using Box::leak
                    let ledger = Box::new(Ledger::new(
                        account_info,
                        strategy_mode,
                        synchronize_accounts,
                        self.strategy_sender.clone(),
                        position_calculation_mode
                    ));
                    let static_ledger: &'static Ledger = Box::leak(ledger);

                    // Store the static reference
                    self.ledgers.insert(account.clone(), static_ledger);

                    // Create channel
                    let (sender, receiver) = tokio::sync::mpsc::channel(100);

                    // Get mutable access for updates
                    let mutable_ledger = unsafe { &mut *(static_ledger as *const Ledger as *mut Ledger) };
                    Ledger::ledger_updates(mutable_ledger, receiver, strategy_mode);

                    self.ledger_senders.insert(account.clone(), sender);
                },
                StrategyMode::Backtest | StrategyMode::LivePaperTrading => {
                    let ledger = Box::new(Ledger {
                        account: account.clone(),
                        cash_value: starting_cash.clone(),
                        cash_available: starting_cash.clone(),
                        currency,
                        cash_used: dec!(0),
                        positions: Default::default(),
                        last_update: Default::default(),
                        symbol_code_map: Default::default(),
                        margin_used: Default::default(),
                        positions_closed: Default::default(),
                        symbol_closed_pnl: Default::default(),
                        symbol_info: Default::default(),
                        open_pnl: Default::default(),
                        total_booked_pnl: dec!(0),
                        commissions_paid: Default::default(),
                        mode: strategy_mode.clone(),
                        is_simulating_pnl: true,
                        strategy_sender: self.strategy_sender.clone(),
                        rates: Arc::new(DashMap::new()),
                        position_calculation_mode,

                    });
                    let static_ledger: &'static Ledger = Box::leak(ledger);

                    // Store the static reference
                    self.ledgers.insert(account.clone(), static_ledger);

                    // Create channel
                    let (sender, receiver) = tokio::sync::mpsc::channel(100);

                    // Get mutable access for updates
                    let mutable_ledger = unsafe { &mut *(static_ledger as *const Ledger as *mut Ledger) };
                    Ledger::ledger_updates(mutable_ledger, receiver, strategy_mode);

                    self.ledger_senders.insert(account.clone(), sender);
                }
            }
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


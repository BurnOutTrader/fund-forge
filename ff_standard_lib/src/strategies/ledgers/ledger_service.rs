use std::sync::Arc;
use chrono::{DateTime, Utc};
use crate::standardized_types::enums::{OrderSide, PositionSide, StrategyMode};
use crate::standardized_types::subscriptions::{SymbolCode, SymbolName};
use dashmap::DashMap;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tokio::sync::Mutex;
use crate::standardized_types::position::{Position, PositionUpdateEvent};
use crate::standardized_types::accounts::{Account, Currency};
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::orders::{Order, OrderId, OrderUpdateEvent};
use crate::standardized_types::time_slices::TimeSlice;
use crate::strategies::handlers::market_handler::price_service::price_service_request_market_fill_price;
use crate::strategies::ledgers::ledger::{Ledger, LedgerMessage};
use crate::strategies::strategy_events::StrategyEvent;

pub(crate) struct LedgerService {
    pub (crate) ledgers: DashMap<Account, Arc<Ledger>>,
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

    pub async fn synchronize_live_position(&self, account: Account, position: Position, time: DateTime<Utc>) {
        if let Some(sender) = self.ledger_senders.get(&account) {
            let msg = LedgerMessage::SyncPosition{position, time};
            sender.send(msg).await.unwrap();
        }
    }

    pub(crate) async fn update_or_create_paper_position(
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
            ledger_ref.update_or_create_paper_position(symbol_name, symbol_code, order_id, quantity, side, time, market_fill_price, tag).await
        } else {
            panic!("No ledger for account: {}, Please Initialize accounts in strategy initialize", account);
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
    ) {
        if let Some(sender) = self.ledger_senders.get(account) {
            let msg = LedgerMessage::UpdateOrCreatePosition{symbol_name, symbol_code, quantity, side, time, market_fill_price, tag};
            sender.send(msg).await.unwrap();
        }
    }

    #[allow(dead_code)]
    pub async fn process_synchronized_orders(&self, order: Order, quantity: Decimal, time: DateTime<Utc>) {
        if let Some(sender) = self.ledger_senders.get(&order.account) {
            let msg = LedgerMessage::ProcessOrder{order, quantity, time};
            sender.send(msg).await.unwrap();
        }
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

    pub async fn timeslice_updates(&self, time_slice: Arc<TimeSlice>) {
        for ledger in self.ledgers.iter() {
            ledger.timeslice_update(time_slice.clone()).await;
        }
    }

    pub async fn init_ledger(&self, account: &Account, strategy_mode: StrategyMode, synchronize_accounts: bool, starting_cash: Decimal, currency: Currency) {
        if !self.ledgers.contains_key(account) {
            let ledger: Arc<Ledger> = match strategy_mode {
                StrategyMode::Live => {
                    let account_info = match account.brokerage.account_info(account.account_id.clone()).await {
                        Ok(ledger) => ledger,
                        Err(e) => {
                            panic!("LEDGER_SERVICE: Error initializing account: {}", e);
                        }
                    };
                    let ledger = Arc::new(Ledger::new(account_info, strategy_mode, synchronize_accounts, self.strategy_sender.clone()));
                    let (sender, receiver) = tokio::sync::mpsc::channel(100);
                    Ledger::live_ledger_updates(ledger.clone(), receiver);
                    self.ledger_senders.insert(account.clone(), sender);
                    ledger
                },
                StrategyMode::Backtest | StrategyMode::LivePaperTrading => {
                    Arc::new(Ledger {
                        account: account.clone(),
                        cash_value: Mutex::new(starting_cash.clone()),
                        cash_available: Mutex::new(starting_cash.clone()),
                        currency,
                        cash_used: Mutex::new(dec!(0)),
                        positions: Default::default(),
                        last_update: Default::default(),
                        symbol_code_map: Default::default(),
                        margin_used: Default::default(),
                        positions_closed: Default::default(),
                        symbol_closed_pnl: Default::default(),
                        symbol_info: Default::default(),
                        open_pnl: Default::default(),
                        total_booked_pnl: Mutex::new(dec!(0)),
                        commissions_paid: Default::default(),
                        mode: strategy_mode.clone(),
                        is_simulating_pnl: true,
                        strategy_sender: self.strategy_sender.clone(),
                        rates: Arc::new(DashMap::new()),
                    })
                }
            };

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

    pub async fn flatten_all_for_paper_account(&self, account: &Account, time: DateTime<Utc>) {
        if let Some(ledger) = self.ledgers.get(account) {
            for position in ledger.positions.iter() {
                let order_side = match position.side {
                    PositionSide::Long => OrderSide::Sell,
                    PositionSide::Short => OrderSide::Buy,
                };
                let market_price = match price_service_request_market_fill_price(order_side, position.symbol_name.clone(), position.symbol_code.clone(), position.quantity_open).await {
                    Ok(price) => {
                        match price.price() {
                            None =>  continue,
                            Some(price) => price
                        }
                    },
                    Err(_) => continue
                };
                let tag = "Flatten All".to_string();
                let _ = ledger.paper_exit_position(position.key(), time, market_price, tag).await;
            }
        }
    }
}


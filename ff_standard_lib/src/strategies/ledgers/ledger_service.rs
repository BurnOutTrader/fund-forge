use std::sync::Arc;
use chrono::{DateTime, Utc};
use crate::standardized_types::enums::{OrderSide, StrategyMode};
use crate::standardized_types::subscriptions::{SymbolCode, SymbolName};
use dashmap::DashMap;
use lazy_static::lazy_static;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::standardized_types::position::{Position, PositionUpdateEvent};
use crate::standardized_types::accounts::{Account, Currency};
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::orders::{Order, OrderId, OrderUpdateEvent};
use crate::standardized_types::time_slices::TimeSlice;
use crate::strategies::ledgers::ledger::Ledger;

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

    pub fn synchronize_live_position(&self, account: Account, position: Position, time: DateTime<Utc>) -> Option<PositionUpdateEvent> {
        if let Some(account_ledger) = self.ledgers.get(&account) {
            return account_ledger.value().synchronize_live_position(position, time)
        }
        None
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
        if let Some(ledger_ref) = self.ledgers.get(account) {
            ledger_ref.update_or_create_live_position(symbol_name, symbol_code, quantity, side, time, market_fill_price, tag).await
        } else {
            panic!("No ledger for account: {}", account);
        }
    }

    pub async fn process_synchronized_orders(&self, order: Order, quantity: Decimal, time: DateTime<Utc>) {
        if let Some(account_ledger) = self.ledgers.get(&order.account) {
            account_ledger.value().process_synchronized_orders(order, quantity, time).await;
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

    pub async fn flatten_all_positions(&self, account: &Account) {
        if let Some(_) = self.ledgers.get(account) {
            todo!();
        }
    }
}


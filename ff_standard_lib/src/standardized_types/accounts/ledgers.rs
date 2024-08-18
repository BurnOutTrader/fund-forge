use std::collections::HashMap;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use crate::apis::brokerage::Brokerage;
use crate::standardized_types::enums::{PositionSide};
use crate::standardized_types::subscriptions::{Symbol};
use std::fmt::Display;
use tokio::sync::RwLock;
use crate::apis::brokerage::client_requests::ClientSideBrokerage;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::data_server_messaging::FundForgeError;
use crate::standardized_types::time_slices::TimeSlice;
use crate::traits::bytes::Bytes;


// B
pub type AccountId = String;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
compare(PartialEq),
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub enum AccountCurrency {
    AUD,
    NZD,
    EUR,
    GBP,
    CHF,
    JPY,
    SGD,
    HKD,
    USD,
    BTC,
    ETH,
    USDT,
}

/// A ledger specific to the strategy which will ignore positions not related to the strategy but will update its balances relative to the actual account balances for live trading.
#[derive(Debug)]
pub struct Ledger {
    pub account_id: AccountId,
    pub brokerage: Brokerage,
    pub cash_value: f64,
    pub cash_available: f64,
    pub currency: AccountCurrency,
    pub cash_used: f64,
    pub positions: RwLock<HashMap<Symbol, Position>>,
    pub positions_closed: RwLock<HashMap<Symbol, Position>>,
}

impl Ledger {
    pub fn new(account_info: AccountInfo) -> Self {
        let positions = account_info.positions.into_iter().map(|position| (position.symbol_name.clone(), position)).collect();
        let ledger = Self {
            account_id: account_info.account_id,
            brokerage: account_info.brokerage,
            cash_value: account_info.cash_value,
            cash_available: account_info.cash_available,
            currency: account_info.currency,
            cash_used: account_info.cash_used,
            positions: RwLock::new(positions),
            positions_closed: RwLock::new(HashMap::new()),
        };
        ledger
    }

    pub fn paper_account_init(account_id: AccountId, brokerage: Brokerage, cash_value: f64, currency: AccountCurrency) -> Self {
        let account = Self {
            account_id,
            brokerage,
            cash_value,
            cash_available: cash_value,
            currency,
            cash_used: 0.0,
            positions: RwLock::new(HashMap::new()),
            positions_closed: RwLock::new(HashMap::new()),
        };
        account
    }

    pub async fn exit_position_paper(&mut self, symbol_name: &Symbol) {
        let mut positions = self.positions.write().await;
        if !positions.contains_key(symbol_name) {
            return
        }
        let position = positions.remove(symbol_name).unwrap();
        let margin_freed = self.brokerage.margin_required(&symbol_name, position.quantity).await;
        self.cash_available += margin_freed;
        self.cash_used -= margin_freed;
        self.positions_closed.write().await.insert(symbol_name.clone(), position);
    }

    pub async fn enter_short_paper(&mut self, symbol_name: Symbol, quantity: u64, price: f64) -> Result<(), FundForgeError> {
        let mut positions = self.positions.write().await;
        if let Some(position) = positions.get(&symbol_name) {
            if position.side == PositionSide::Short {
                // we will need to modify our average price and quantity
            }
        }
        let margin_required = self.brokerage.margin_required(&symbol_name, quantity).await;
        if self.cash_available < margin_required {
            return Err(FundForgeError::ClientSideErrorDebug("Insufficient funds".to_string()));
        }
        let position = Position::enter(symbol_name.clone(), self.brokerage.clone(), PositionSide::Long, quantity, price);
        positions.insert(symbol_name.clone(), position);
        self.cash_available -= margin_required;
        self.cash_used += margin_required;
        Ok(())
    }

    pub async fn enter_long_paper(&mut self, symbol_name: Symbol, quantity: u64, price: f64) -> Result<(), FundForgeError> {
        let mut positions = self.positions.write().await;
        if let Some(position) = positions.get(&symbol_name) {
            if position.side == PositionSide::Long {
                // we will need to modify our average price and quantity
            }
        }
        let margin_required = self.brokerage.margin_required(&symbol_name, quantity).await;
        if self.cash_available < margin_required {
            return Err(FundForgeError::ClientSideErrorDebug("Insufficient funds".to_string()));
        }
        let position: Position = Position::enter(symbol_name.clone(), self.brokerage.clone(), PositionSide::Long, quantity, price);
        positions.insert(symbol_name.clone(), position);
        self.cash_available -= margin_required;
        self.cash_used += margin_required;
        Ok(())
    }

    pub async fn account_init(account_id: AccountId, brokerage: Brokerage) -> Self {
        match brokerage.init_ledger(account_id).await {
            Ok(ledger) => ledger,
            Err(e) => panic!("Failed to initialize account: {:?}", e),
        }
    }

    pub async fn is_long(&self, symbol: &Symbol) -> bool {
        let positions = self.positions.read().await;
        if let Some(position) = positions.get(symbol) {
            if position.side == PositionSide::Long {
                return true;
            }
        }
        false
    }

    pub async fn is_short(&self, symbol: &Symbol) -> bool {
        let positions = self.positions.read().await;
        if let Some(position) = positions.get(symbol) {
            if position.side == PositionSide::Short {
                return true;
            }
        }
        false
    }

    pub async fn is_flat(&self, symbol: &Symbol) -> bool {
        let positions = self.positions.read().await;
        if let Some(_) = positions.get(symbol) {
            false
        } else {
            true
        }
    }

    pub async fn on_data_update(&self, time_slice: TimeSlice) {
        let mut positions = self.positions.write().await;
        for position in positions.values_mut() {
            position.update_base_data(&time_slice);
        }
    }
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug)]
#[archive(
    compare(PartialEq),
    check_bytes,
)]
#[archive_attr(derive(Debug))]
pub struct Position {
    pub symbol_name: Symbol,
    pub brokerage: Brokerage,
    pub side: PositionSide,
    pub quantity: u64,
    pub average_price: f64,
    pub pnl_raw: Option<f64>,
    pub highest_recoded_price: Option<f64>,
    pub lowest_recoded_price: Option<f64>,
    pub is_closed: bool,
}

impl Position {
    pub fn enter(symbol_name: Symbol, brokerage: Brokerage, side: PositionSide, quantity: u64, average_price: f64) -> Self {
        Self {
            symbol_name,
            brokerage,
            side,
            quantity,
            average_price,
            pnl_raw: None,
            highest_recoded_price: None,
            lowest_recoded_price: None,
            is_closed: false,
        }
    }

    pub fn update_base_data(&mut self, time_slice: &TimeSlice) {
        if self.is_closed {
            return;
        }
        for base_data in time_slice {
            if &self.symbol_name != base_data.symbol() {
                continue;
            }
            match base_data {
                BaseDataEnum::Price(price) => {
                    if let Some(highest_recoded_price) = self.highest_recoded_price {
                        if price.price > highest_recoded_price {
                            self.highest_recoded_price = Some(price.price);
                        }
                    } else {
                        self.highest_recoded_price = Some(price.price);
                    }
                    if let Some(lowest_recoded_price) = self.lowest_recoded_price {
                        if price.price < lowest_recoded_price {
                            self.lowest_recoded_price = Some(price.price);
                        }
                    } else {
                        self.lowest_recoded_price = Some(price.price);
                    }
                }
                BaseDataEnum::Candle(candle) => {
                    if let Some(highest_recoded_price) = self.highest_recoded_price {
                        if candle.high > highest_recoded_price {
                            self.highest_recoded_price = Some(candle.high);
                        }
                    } else {
                        self.highest_recoded_price = Some(candle.high);
                    }

                    if let Some(lowest_recoded_price) = self.lowest_recoded_price {
                        if candle.low < lowest_recoded_price {
                            self.lowest_recoded_price = Some(candle.low);
                        }
                    } else {
                        self.lowest_recoded_price = Some(candle.low);
                    }
                }
                BaseDataEnum::QuoteBar(bar) => {
                    if let Some(highest_recoded_price) = self.highest_recoded_price {
                        if bar.ask_high > highest_recoded_price {
                            self.highest_recoded_price = Some(bar.ask_high);
                        }
                    } else {
                        self.highest_recoded_price = Some(bar.ask_high);
                    }

                    if let Some(lowest_recoded_price) = self.lowest_recoded_price {
                        if bar.bid_low < lowest_recoded_price {
                            self.lowest_recoded_price = Some(bar.bid_low);
                        }
                    } else {
                        self.lowest_recoded_price = Some(bar.bid_low);
                    }
                }
                BaseDataEnum::Tick(tick) => {
                    if let Some(highest_recoded_price) = self.highest_recoded_price {
                        if tick.price > highest_recoded_price {
                            self.highest_recoded_price = Some(tick.price);
                        }
                    } else {
                        self.highest_recoded_price = Some(tick.price);
                    }

                    if let Some(lowest_recoded_price) = self.lowest_recoded_price {
                        if tick.price < lowest_recoded_price {
                            self.lowest_recoded_price = Some(tick.price);
                        }
                    } else {
                        self.lowest_recoded_price = Some(tick.price);
                    }
                }
                BaseDataEnum::Quote(quote) => {
                    if let Some(highest_recoded_price) = self.highest_recoded_price {
                        if quote.ask > highest_recoded_price {
                            self.highest_recoded_price = Some(quote.ask);
                        }
                    } else {
                        self.highest_recoded_price = Some(quote.ask);
                    }

                    if let Some(lowest_recoded_price) = self.lowest_recoded_price {
                        if quote.bid < lowest_recoded_price {
                            self.lowest_recoded_price = Some(quote.bid);
                        }
                    } else {
                        self.lowest_recoded_price = Some(quote.bid);
                    }
                }
                BaseDataEnum::Fundamental(_) => (),
            }
        }
    }
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug)]
#[archive(
    compare(PartialEq),
    check_bytes,
)]
#[archive_attr(derive(Debug))]
pub struct AccountInfo {
    pub account_id: AccountId,
    pub brokerage: Brokerage,
    pub cash_value: f64,
    pub cash_available: f64,
    pub currency: AccountCurrency,
    pub cash_used: f64,
    pub positions: Vec<Position>,
    pub positions_closed: Vec<Position>,
    pub is_hedging: bool,
}
impl Bytes<Self> for AccountInfo {
    fn to_bytes(&self) -> Vec<u8> {
        let vec = rkyv::to_bytes::<_, 256>(self).unwrap();
        vec.into()
    }

    fn from_bytes(archived: &[u8]) -> Result<AccountInfo, FundForgeError> {
        // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<AccountInfo>(archived) { //Ignore this warning: Trait `Deserialize<ResponseType, SharedDeserializeMap>` is not implemented for `ArchivedRequestType` [E0277]
            Ok(response) => Ok(response),
            Err(e) => {
                Err(FundForgeError::ClientSideErrorDebug(e.to_string()))
            }
        }
    }
}


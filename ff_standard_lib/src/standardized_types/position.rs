use std::fmt;
use std::str::FromStr;
use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use rust_decimal_macros::dec;
use serde_derive::{Deserialize, Serialize};
use crate::standardized_types::broker_enum::Brokerage;
use crate::standardized_types::enums::PositionSide;
use crate::standardized_types::subscriptions::SymbolName;
use crate::strategies::ledgers::{AccountId, Currency};
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::symbol_info::SymbolInfo;

pub type PositionId = String;
#[derive(Serialize)]
pub(crate) struct PositionExport {
    entry_time: String,
    symbol_name: String,
    side: String,
    quantity: Volume,
    average_price: Price,
    exit_price: Price,
    booked_pnl: Price,
    open_pnl: Price,
    highest_recoded_price: Price,
    lowest_recoded_price: Price,
    exit_time: String,
    tag: String
}

#[derive(Clone, rkyv::Serialize, rkyv::Deserialize, Archive, Debug, PartialEq)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum PositionUpdateEvent {
    PositionOpened {
        position_id: PositionId,
        account_id: AccountId,
        brokerage: Brokerage,
        originating_order_tag: String,
        time: String
    },
    Increased {
        position_id: PositionId,
        total_quantity_open: Volume,
        average_price: Price,
        open_pnl: Price,
        booked_pnl: Price,
        account_id: AccountId,
        brokerage: Brokerage,
        originating_order_tag: String,
        time: String
    },
    PositionReduced {
        position_id: PositionId,
        total_quantity_open: Volume,
        total_quantity_closed: Volume,
        average_price: Price,
        open_pnl: Price,
        booked_pnl: Price,
        average_exit_price: Price,
        account_id: AccountId,
        brokerage: Brokerage,
        originating_order_tag: String,
        time: String
    },
    PositionClosed {
        position_id: PositionId,
        total_quantity_open: Volume,
        total_quantity_closed: Volume,
        average_price: Price,
        booked_pnl: Price,
        average_exit_price: Option<Price>,
        account_id: AccountId,
        brokerage: Brokerage,
        originating_order_tag: String,
        time: String
    },
}

impl PositionUpdateEvent {
    pub fn brokerage(&self) -> &Brokerage {
        match self {
            PositionUpdateEvent::PositionOpened{brokerage,..} => brokerage,
            PositionUpdateEvent::Increased{brokerage,..} => brokerage,
            PositionUpdateEvent::PositionReduced {brokerage,..} => brokerage,
            PositionUpdateEvent::PositionClosed {brokerage,..} => brokerage,
        }
    }

    pub fn account_id(&self) -> &AccountId {
        match self {
            PositionUpdateEvent::PositionOpened{account_id,..} => account_id,
            PositionUpdateEvent::Increased{account_id,..} => account_id,
            PositionUpdateEvent::PositionReduced {account_id,..} => account_id,
            PositionUpdateEvent::PositionClosed {account_id,..} => account_id,
        }
    }

    pub fn time_local(&self, time_zone: &Tz) -> DateTime<Tz> {
        let utc_time: DateTime<Utc> = self.time_utc();
        time_zone.from_utc_datetime(&utc_time.naive_utc())
    }

    pub fn time_utc(&self) -> DateTime<Utc> {
        match self {
            PositionUpdateEvent::PositionOpened{time,..} => DateTime::from_str(time).unwrap(),
            PositionUpdateEvent::Increased{time,..} =>  DateTime::from_str(time).unwrap(),
            PositionUpdateEvent::PositionReduced {time,..} =>  DateTime::from_str(time).unwrap(),
            PositionUpdateEvent::PositionClosed {time,..} =>  DateTime::from_str(time).unwrap(),
        }
    }
}

impl fmt::Display for PositionUpdateEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PositionUpdateEvent::PositionOpened{position_id, brokerage, account_id, originating_order_tag: tag,.. } => {
                write!(f, "PositionOpened: Position ID = {}, Brokerage: {}, Account: {}, Originating Order Tag: {}", position_id, brokerage, account_id, tag)
            }
            PositionUpdateEvent::Increased {
                position_id,
                total_quantity_open,
                average_price,
                open_pnl,
                booked_pnl,
                account_id,
                brokerage,
                originating_order_tag: tag,
                ..
            } => {
                write!(
                    f,
                    "PositionIncreased: Position ID = {}, Brokerage: {}, Account: {}, Total Quantity Open = {}, Average Price = {}, Open PnL = {}, Booked PnL = {}, Originating Order Tag: {}",
                    position_id, brokerage, account_id, total_quantity_open, average_price, open_pnl, booked_pnl, tag
                )
            }
            PositionUpdateEvent::PositionReduced {
                position_id,
                total_quantity_open,
                total_quantity_closed,
                average_price,
                open_pnl,
                booked_pnl,
                average_exit_price,
                account_id,
                brokerage,
                originating_order_tag: tag,
                ..
            } => {
                write!(
                    f,
                    "PositionReduced: Position ID = {}, Brokerage: {}, Account: {}, Total Quantity Open = {}, Total Quantity Closed = {}, Average Price = {}, Open PnL = {}, Booked PnL = {}, Average Exit Price = {}, Originating Order Tag: {}",
                    position_id, brokerage, account_id, total_quantity_open, total_quantity_closed, average_price, open_pnl, booked_pnl, average_exit_price, tag
                )
            }
            PositionUpdateEvent::PositionClosed {
                position_id,
                total_quantity_open,
                total_quantity_closed,
                average_price,
                booked_pnl,
                average_exit_price,
                account_id,
                brokerage,
                originating_order_tag: tag,
                ..
            } => {
                write!(
                    f,
                    "PositionClosed: Position ID = {}, Brokerage: {}, Account: {}, Total Quantity Open = {}, Total Quantity Closed = {}, Average Price = {}, Booked PnL = {}, Average Exit Price = {}, Originating Order Tag: {}",
                    position_id, brokerage, account_id, total_quantity_open, total_quantity_closed, average_price, booked_pnl,
                    match average_exit_price {
                        Some(price) => price.to_string(),
                        None => "None".to_string(),
                    },
                    tag
                )
            }
        }
    }
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug, PartialEq, Serialize, Deserialize,     PartialOrd,)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct Position {
    pub symbol_name: SymbolName,
    pub brokerage: Brokerage,
    pub account_id: AccountId,
    pub side: PositionSide,
    pub open_time: String,
    pub quantity_open: Volume,
    pub quantity_closed: Volume,
    pub close_time: Option<String>,
    pub average_price: Price,
    pub open_pnl: Price,
    pub booked_pnl: Price,
    pub highest_recoded_price: Price,
    pub lowest_recoded_price: Price,
    pub average_exit_price: Option<Price>,
    pub is_closed: bool,
    pub position_id: PositionId,
    pub symbol_info: SymbolInfo,
    pub pnl_currency: Currency,
    pub tag: String
}

impl Position {
    pub fn new (
        symbol_name: SymbolName,
        brokerage: Brokerage,
        account_id: AccountId,
        side: PositionSide,
        quantity: Volume,
        average_price: Price,
        id: PositionId,
        symbol_info: SymbolInfo,
        account_currency: Currency,
        tag: String,
        time: DateTime<Utc>
    ) -> Self {
        Self {
            symbol_name,
            brokerage,
            account_id,
            side,
            open_time: time.to_string(),
            quantity_open: quantity,
            quantity_closed: dec!(0.0),
            close_time: None,
            average_price,
            open_pnl: dec!(0.0),
            booked_pnl: dec!(0.0),
            highest_recoded_price: average_price,
            lowest_recoded_price: average_price,
            average_exit_price: None,
            is_closed: false,
            position_id: id,
            symbol_info,
            pnl_currency: account_currency,
            tag,
        }
    }

    pub(crate) fn to_export(&self) -> PositionExport {
        let exit_time = match &self.close_time {
            None => "None".to_string(),
            Some(time) => time.to_string()
        };
        PositionExport {
            symbol_name: self.symbol_name.to_string(),
            side: self.side.to_string(),
            quantity: self.quantity_closed,
            average_price: self.average_price,
            exit_price: self.average_exit_price.unwrap(),
            booked_pnl: self.booked_pnl.round_dp(2),
            open_pnl: self.open_pnl.round_dp(2),
            highest_recoded_price: self.highest_recoded_price,
            lowest_recoded_price: self.lowest_recoded_price,
            exit_time,
            entry_time: self.open_time.to_string(),
            tag: self.tag.clone()
        }
    }
}

pub(crate) mod historical_position {
    use chrono::{DateTime, Utc};
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use crate::helpers::decimal_calculators::{calculate_historical_pnl};
    use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
    use crate::standardized_types::enums::PositionSide;
    use crate::standardized_types::position::{Position, PositionUpdateEvent};
    use crate::standardized_types::new_types::{Price, Volume};
    use crate::strategies::ledgers::Currency;

    impl Position {

        /// Reduces paper position size a position event, this event will include a booked_pnl property
        pub(crate) async fn reduce_paper_position_size(&mut self, market_price: Price, quantity: Volume, time: DateTime<Utc>, tag: String, account_currency: Currency) -> PositionUpdateEvent {
            if quantity > self.quantity_open {
                panic!("Something wrong with logic, ledger should know this not to be possible")
            }
            // Calculate booked PnL
            let booked_pnl = calculate_historical_pnl(
                self.side,
                self.average_price,
                market_price,
                quantity,
                account_currency,
                time,
                &self.symbol_info
            );

            // Update position
            self.booked_pnl += booked_pnl;
            self.open_pnl -= booked_pnl;

            self.average_exit_price = match self.average_exit_price {
                Some(existing_exit_price) => {
                    let exited_quantity = Decimal::from(self.quantity_closed);
                    let new_exit_price = market_price;
                    let new_exit_quantity = quantity;
                    // Calculate the weighted average of the existing exit price and the new exit price
                    let total_exit_quantity = exited_quantity + new_exit_quantity;
                    let weighted_existing_exit = existing_exit_price * exited_quantity;
                    let weighted_new_exit = new_exit_price * new_exit_quantity;
                    Some(((weighted_existing_exit + weighted_new_exit) / total_exit_quantity).round_dp(self.symbol_info.decimal_accuracy))
                }
                None => Some(market_price)
            };

            self.quantity_open -= quantity;
            self.quantity_closed += quantity;
            self.is_closed = self.quantity_open <= dec!(0.0);

            // Reset open PnL if position is closed
            if self.is_closed {
                self.open_pnl = dec!(0);
                self.close_time = Some(time.to_string());
                PositionUpdateEvent::PositionClosed {
                    position_id: self.position_id.clone(),
                    total_quantity_open: self.quantity_open,
                    total_quantity_closed: self.quantity_closed,
                    average_price: self.average_price,
                    booked_pnl: self.booked_pnl,
                    average_exit_price: self.average_exit_price,
                    account_id: self.account_id.clone(),
                    brokerage: self.brokerage.clone(),
                    originating_order_tag: tag,
                    time: time.to_string()
                }
            } else {
                PositionUpdateEvent::PositionReduced {
                    position_id: self.position_id.clone(),
                    total_quantity_open: self.quantity_open,
                    total_quantity_closed: self.quantity_closed,
                    average_price: self.average_price,
                    open_pnl: self.open_pnl,
                    booked_pnl: self.booked_pnl,
                    average_exit_price: self.average_exit_price.unwrap(),
                    account_id: self.account_id.clone(),
                    brokerage: self.brokerage.clone(),
                    originating_order_tag: tag,
                    time: time.to_string()
                }
            }
        }

        /// Adds to the paper position
        pub(crate) async fn add_to_position(&mut self, market_price: Price, quantity: Volume, time: DateTime<Utc>, tag: String, account_currency: Currency) -> PositionUpdateEvent {
            // Correct the average price calculation with proper parentheses
            if self.quantity_open + quantity != Decimal::ZERO {
                self.average_price = ((self.quantity_open * self.average_price + quantity * market_price) / (self.quantity_open + quantity)).round_dp(self.symbol_info.decimal_accuracy);
            } else {
                panic!("Average price should not be 0");
            }

            // Update the total quantity
            self.quantity_open += quantity;

            // Recalculate open PnL
            self.open_pnl = calculate_historical_pnl(
                self.side,
                self.average_price,
                market_price,
                self.quantity_open,
                account_currency,
                time,
                &self.symbol_info
            );

            PositionUpdateEvent::Increased {
                position_id: self.position_id.clone(),
                total_quantity_open: self.quantity_open,
                average_price: self.average_price,
                open_pnl: self.open_pnl,
                booked_pnl: self.booked_pnl,
                account_id: self.account_id.clone(),
                brokerage: self.brokerage.clone(),
                originating_order_tag: tag,
                time: time.to_string()
            }
        }

        /// Returns the open pnl for the paper position
        pub(crate) fn backtest_update_base_data(&mut self, base_data: &BaseDataEnum, time: DateTime<Utc>, account_currency: Currency) -> Decimal {
            if self.is_closed {
                return dec!(0)
            }

            // Extract market price, highest price, and lowest price from base data
            let (market_price, highest_price, lowest_price) = match base_data {
                BaseDataEnum::Candle(candle) => (candle.close, candle.high, candle.low),
                BaseDataEnum::Tick(tick) => (tick.price, tick.price, tick.price),
                BaseDataEnum::QuoteBar(bar) => match self.side {
                    PositionSide::Long => (bar.ask_close, bar.ask_high, bar.ask_low),
                    PositionSide::Short => (bar.bid_close, bar.bid_high, bar.bid_low),
                },
                BaseDataEnum::Quote(quote) => match self.side {
                    PositionSide::Long => (quote.ask, quote.ask, quote.ask),
                    PositionSide::Short => (quote.bid, quote.bid, quote.bid),
                },
                BaseDataEnum::Fundamental(_) => panic!("Fundamentals should not be here"),
            };

            // Update highest and lowest recorded prices
            self.highest_recoded_price = self.highest_recoded_price.max(highest_price);
            self.lowest_recoded_price = self.lowest_recoded_price.min(lowest_price);

            // Calculate the open PnL
            self.open_pnl = calculate_historical_pnl(
                self.side,
                self.average_price,
                market_price,
                self.quantity_open,
                account_currency,
                time,
                &self.symbol_info
            );

            self.open_pnl.clone()
        }
    }
}
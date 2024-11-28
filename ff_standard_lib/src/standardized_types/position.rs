use std::collections::VecDeque;
use std::fmt;
use std::fmt::Display;
use std::str::FromStr;
use chrono::{DateTime, Duration, TimeZone, Utc};
use chrono_tz::Tz;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal_macros::dec;
use serde_derive::{Deserialize, Serialize};
use crate::helpers::converters::format_duration;
use crate::helpers::decimal_calculators::calculate_theoretical_pnl;
use crate::product_maps::rithmic::maps::get_futures_commissions_info;
use crate::standardized_types::accounts::{Account, AccountId, Currency};
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::broker_enum::Brokerage;
use crate::standardized_types::enums::{PositionSide, StrategyMode};
use crate::standardized_types::subscriptions::{SymbolCode, SymbolName};
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::orders::OrderId;
use crate::standardized_types::symbol_info::SymbolInfo;

pub type PositionId = String;
#[derive(Serialize)]
pub(crate) struct PositionExport {
    symbol_code: String,
    position_side: String,
    tag: String,
    quantity: Volume,
    average_entry_price: Price,
    average_exit_price: Price,
    booked_pnl: Price,
    open_pnl: Price,
    highest_recoded_price: Price,
    lowest_recoded_price: Price,
    entry_time: String,
    exit_time: String,
    hold_duration: String,
}

#[derive(Clone, rkyv::Serialize, rkyv::Deserialize, Archive, Debug, PartialEq)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum PositionUpdateEvent {
    PositionOpened {
        position_id: PositionId,
        side: PositionSide,
        account: Account,
        symbol_name: SymbolName,
        symbol_code: SymbolCode,
        average_price: Price,
        originating_order_tag: String,
        time: String
    },
    Increased {
        position_id: PositionId,
        side: PositionSide,
        total_quantity_open: Volume,
        average_price: Price,
        symbol_name: SymbolName,
        symbol_code: SymbolCode,
        open_pnl: Price,
        booked_pnl: Price,
        account: Account,
        originating_order_tag: String,
        time: String
    },
    PositionReduced {
        position_id: PositionId,
        side: PositionSide,
        total_quantity_open: Volume,
        symbol_name: SymbolName,
        symbol_code: SymbolCode,
        total_quantity_closed: Volume,
        average_price: Price,
        open_pnl: Price,
        booked_pnl: Price,
        average_exit_price: Price,
        account: Account,
        originating_order_tag: String,
        time: String
    },
    PositionClosed {
        position_id: PositionId,
        side: PositionSide,
        symbol_name: SymbolName,
        symbol_code: SymbolCode,
        total_quantity_open: Volume,
        total_quantity_closed: Volume,
        average_price: Price,
        booked_pnl: Price,
        average_exit_price: Price,
        account: Account,
        originating_order_tag: String,
        time: String
    },
}

impl PositionUpdateEvent {
    pub fn brokerage(&self) -> &Brokerage {
        match self {
            PositionUpdateEvent::PositionOpened{account,..} =>  &account.brokerage,
            PositionUpdateEvent::Increased{account,..} => &account.brokerage,
            PositionUpdateEvent::PositionReduced {account,..} => &account.brokerage,
            PositionUpdateEvent::PositionClosed {account,..} => &account.brokerage,
        }
    }

    pub fn account(&self) -> &Account {
        match self {
            PositionUpdateEvent::PositionOpened{account,..} =>  &account,
            PositionUpdateEvent::Increased{account,..} => &account,
            PositionUpdateEvent::PositionReduced {account,..} => &account,
            PositionUpdateEvent::PositionClosed {account,..} => &account,
        }
    }

    pub fn account_id(&self) -> &AccountId {
        match self {
            PositionUpdateEvent::PositionOpened{account,..} => &account.account_id,
            PositionUpdateEvent::Increased{account,..} =>  &account.account_id,
            PositionUpdateEvent::PositionReduced {account,..} => &account.account_id,
            PositionUpdateEvent::PositionClosed {account,..} =>  &account.account_id,
        }
    }

    pub fn time_local(&self, time_zone: &Tz) -> DateTime<Tz> {
        let utc_time: DateTime<Utc> = self.time_utc();
        time_zone.from_utc_datetime(&utc_time.naive_utc())
    }

    pub fn symbol_code(&self) -> &SymbolCode {
        match self {
            PositionUpdateEvent::PositionOpened{symbol_code,..} => symbol_code,
            PositionUpdateEvent::Increased{symbol_code,..} => symbol_code,
            PositionUpdateEvent::PositionReduced {symbol_code,..} => symbol_code,
            PositionUpdateEvent::PositionClosed {symbol_code,..} => symbol_code,
        }
    }

    pub fn symbol_name(&self) -> &SymbolName {
        match self {
            PositionUpdateEvent::PositionOpened{symbol_name,..} => symbol_name,
            PositionUpdateEvent::Increased{symbol_name,..} => symbol_name,
            PositionUpdateEvent::PositionReduced {symbol_name,..} => symbol_name,
            PositionUpdateEvent::PositionClosed {symbol_name,..} => symbol_name,
        }
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
            PositionUpdateEvent::PositionOpened{position_id, account, originating_order_tag: tag,.. } => {
                write!(f, "PositionOpened: Position ID = {}, Account: {}, Originating Order Tag: {}", position_id, account, tag)
            }
            PositionUpdateEvent::Increased {
                position_id,
                total_quantity_open,
                average_price,
                open_pnl,
                booked_pnl,
                account,
                originating_order_tag: tag,
                ..
            } => {
                write!(
                    f,
                    "PositionIncreased: Position ID = {}, Account: {}, Total Quantity Open = {}, Average Price = {}, Open PnL = {}, Booked PnL = {}, Originating Order Tag: {}",
                    position_id, account, total_quantity_open, average_price, open_pnl, booked_pnl, tag
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
                account,
                originating_order_tag: tag,
                ..
            } => {
                write!(
                    f,
                    "PositionReduced: Position ID = {}, Account: {}, Total Quantity Open = {}, Total Quantity Closed = {}, Average Price = {}, Open PnL = {}, Booked PnL = {}, Average Exit Price = {}, Originating Order Tag: {}",
                    position_id, account, total_quantity_open, total_quantity_closed, average_price, open_pnl, booked_pnl, average_exit_price, tag
                )
            }
            PositionUpdateEvent::PositionClosed {
                position_id,
                total_quantity_open,
                total_quantity_closed,
                average_price,
                booked_pnl,
                average_exit_price,
                account,
                originating_order_tag: tag,
                ..
            } => {
                write!(
                    f,
                    "PositionClosed: Position ID = {}, Account: {}, Total Quantity Open = {}, Total Quantity Closed = {}, Average Price = {}, Booked PnL = {}, Average Exit Price = {}, Originating Order Tag: {}",
                    position_id, account, total_quantity_open, total_quantity_closed, average_price, booked_pnl, average_exit_price, tag
                )
            }
        }
    }
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug, PartialEq, Serialize, Deserialize, PartialOrd,)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum PositionCalculationMode {
    FIFO,
    LIFO,
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug, PartialEq, Serialize, Deserialize, PartialOrd,)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct EntryPrice {
    pub volume: Volume,
    pub price: Price,
    pub order_id: OrderId
}

impl EntryPrice {
    pub fn new(volume: Volume, price: Price, order_id: OrderId) -> Self {
        Self { volume, price, order_id }
    }
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug, PartialEq, Serialize, Deserialize, PartialOrd,)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum TradeResult {
    Win,
    Loss,
    BreakEven,
}
impl Display for TradeResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            TradeResult::Win => "Win".to_string(),
            TradeResult::Loss => "Loss".to_string(),
            TradeResult::BreakEven => "BreakEven".to_string(),
        };
        write!(f, "{}", str)
    }
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug, PartialEq, Serialize, Deserialize, PartialOrd,)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct Trade {
    pub entry_price: Price,
    pub entry_order_id: OrderId,
    pub entry_quantity: Volume,
    pub exit_price: Price,
    pub exit_order_id: OrderId,
    pub exit_quantity: Volume,
    pub entry_time: String,
    pub exit_time: String,
    pub profit: Price,
    pub result: TradeResult,
    pub commissions: Decimal,
}

#[derive(Debug)]
pub struct PositionStatistics {
    pub total_trades: usize,
    pub average_entry_price: Decimal,
    pub average_exit_price: Decimal,
    pub best_trade_pnl: Decimal,
    pub worst_trade_pnl: Decimal,
    pub best_exit_price: Decimal,
    pub worst_exit_price: Decimal,
    pub total_quantity_traded: Decimal,
    pub weighted_avg_hold_time: Duration,
}



#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug, PartialEq, Serialize, Deserialize, PartialOrd,)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct Position {
    pub symbol_name: SymbolName,
    pub symbol_code: String,
    pub account: Account,
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
    pub exchange_rate_multiplier: Decimal,
    pub average_exit_price: Option<Price>,
    pub is_closed: bool,
    pub position_id: PositionId,
    pub symbol_info: SymbolInfo,
    pub tag: String,
    pub position_calculation_mode: PositionCalculationMode,
    pub open_entry_prices: VecDeque<EntryPrice>,
    pub completed_trades: Vec<Trade>,
}

impl Position {
    pub fn new (
        symbol_name: SymbolName,
        symbol_code: String,
        entry_order_id: OrderId,
        account: Account,
        side: PositionSide,
        quantity: Volume,
        average_price: Price,
        id: PositionId,
        symbol_info: SymbolInfo,
        exchange_rate_multiplier: Decimal,
        tag: String,
        time: DateTime<Utc>,
        position_calculation_mode: PositionCalculationMode
    ) -> Self {
        Self {
            symbol_name,
            symbol_code,
            account,
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
            exchange_rate_multiplier,
            tag,
            position_calculation_mode,
            open_entry_prices: VecDeque::from(vec![EntryPrice::new(quantity, average_price, entry_order_id)]),
            completed_trades: vec![]
        }
    }

    pub(crate) fn to_export(&self) -> PositionExport {
        let (exit_time, hold_duration) = match &self.close_time {
            None => ("None".to_string(), "N/A".to_string()),
            Some(time) => (time.to_string(), format_duration(DateTime::<Utc>::from_str(time).unwrap() - DateTime::<Utc>::from_str(&self.open_time).unwrap()))
        };

        // Calculate final stats from completed trades
        let (weighted_entry, weighted_exit, total_quantity) = self.completed_trades.iter()
            .fold((dec!(0.0), dec!(0.0), dec!(0.0)), |(entry_sum, exit_sum, qty_sum), trade| {
                (
                    entry_sum + trade.entry_price * trade.entry_quantity,
                    exit_sum + trade.exit_price * trade.exit_quantity,
                    qty_sum + trade.entry_quantity
                )
            });

        let final_entry_price = if total_quantity > dec!(0.0) {
            weighted_entry / total_quantity
        } else {
            self.average_price
        };

        let final_exit_price = if total_quantity > dec!(0.0) {
            weighted_exit / total_quantity
        } else {
            self.average_exit_price.unwrap_or(self.average_price)
        };

        PositionExport {
            symbol_code: self.symbol_code.to_string(),
            position_side: self.side.to_string(),
            quantity: self.quantity_closed,
            average_entry_price: final_entry_price,
            average_exit_price: final_exit_price,
            booked_pnl: self.booked_pnl.round_dp(2),
            open_pnl: self.open_pnl.round_dp(2),
            highest_recoded_price: self.highest_recoded_price,
            lowest_recoded_price: self.lowest_recoded_price,
            exit_time,
            entry_time: self.open_time.to_string(),
            hold_duration,
            tag: self.tag.clone()
        }
    }

    /// Returns the open pnl for the paper position
    pub(crate) fn update_base_data(&mut self, base_data: &BaseDataEnum, account_currency: Currency) -> Decimal {
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
                _ => unreachable!("This shouldn't be possible"),
            },
            BaseDataEnum::Quote(quote) => match self.side {
                PositionSide::Long => (quote.ask, quote.ask, quote.ask),
                PositionSide::Short => (quote.bid, quote.bid, quote.bid),
                _ => unreachable!("This shouldn't be possible"),
            },
            BaseDataEnum::Fundamental(_) => panic!("Fundamentals should not be here"),
        };

        // Update highest and lowest recorded prices
        self.highest_recoded_price = self.highest_recoded_price.max(highest_price);
        self.lowest_recoded_price = self.lowest_recoded_price.min(lowest_price);

        // Calculate the open PnL
        self.open_pnl = calculate_theoretical_pnl(
            self.account.brokerage,
            self.side,
            self.average_price,
            market_price,
            self.quantity_open,
            &self.symbol_info,
            self.exchange_rate_multiplier,
            account_currency
        );

        self.open_pnl.clone()
    }

    /// Reduces position size a position event, this event will include a booked_pnl property
    pub(crate) async fn reduce_position_size(&mut self, mode: StrategyMode, market_price: Price, quantity: Volume, order_id: OrderId, account_currency: Currency, exchange_rate: Decimal, time: DateTime<Utc>, tag: String) -> PositionUpdateEvent {
        if quantity > self.quantity_open {
            panic!("Something wrong with logic, ledger should know this not to be possible")
        }

        self.exchange_rate_multiplier = exchange_rate;
        let mut remaining_exit_quantity = quantity;
        let mut total_booked_pnl = dec!(0.0);

        // Create a temporary queue/stack for processing to avoid borrow checker issues
        let mut temp_entries = VecDeque::new();

        // Keep processing entry prices until we've covered the full exit quantity
        while remaining_exit_quantity > dec!(0.0) {
            let entry = match self.position_calculation_mode {
                PositionCalculationMode::FIFO => self.open_entry_prices.pop_front(),
                PositionCalculationMode::LIFO => self.open_entry_prices.pop_back(),
            }.expect("No entry prices available but position still open");

            // Calculate how much we can exit from this entry price level
            let exit_quantity = remaining_exit_quantity.min(entry.volume);

            // Calculate PnL for this portion
            let mut portion_booked_pnl = calculate_theoretical_pnl(
                self.account.brokerage,
                self.side,
                entry.price,
                market_price,
                exit_quantity,
                &self.symbol_info,
                self.exchange_rate_multiplier,
                account_currency
            );

            let commissions = if let Ok(commission_info) = get_futures_commissions_info(&self.symbol_name) {
                let commission = exit_quantity * commission_info.per_side * exchange_rate;
                portion_booked_pnl -= commission * dec!(2.0); // Subtract commission from both sides
                commission
            } else {
                dec!(0.0)
            };

            let result = match portion_booked_pnl {
                pnl if pnl > dec!(0.0) => TradeResult::Win,
                pnl if pnl < dec!(0.0) => TradeResult::Loss,
                _ => TradeResult::BreakEven,
            };

            // Record the trade
            self.completed_trades.push(Trade {
                entry_price: entry.price,
                entry_order_id: entry.order_id.clone(),
                entry_quantity: exit_quantity,
                exit_price: market_price,
                exit_quantity,
                entry_time: self.open_time.clone(), // We could add entry_time to EntryPrice if we need exact times
                exit_time: time.to_string(),
                profit: portion_booked_pnl,
                exit_order_id: order_id.clone(),
                result,
                commissions
            });

            // If we didn't use all of this entry, we need to put back the remainder
            let remaining_entry_volume = entry.volume - exit_quantity;
            if remaining_entry_volume > dec!(0.0) {
                let remaining_entry = EntryPrice::new(remaining_entry_volume, entry.price, entry.order_id.clone());
                match self.position_calculation_mode {
                    PositionCalculationMode::FIFO => temp_entries.push_back(remaining_entry),
                    PositionCalculationMode::LIFO => temp_entries.push_front(remaining_entry),
                }
            }

            total_booked_pnl += portion_booked_pnl;
            remaining_exit_quantity -= exit_quantity;
        }

        // Put any remaining entries back in the correct order
        // No need to reverse the order when restoring since we maintained it during temp storage
        while let Some(entry) = temp_entries.pop_front() {
            match self.position_calculation_mode {
                PositionCalculationMode::FIFO => self.open_entry_prices.push_front(entry),
                PositionCalculationMode::LIFO => self.open_entry_prices.push_back(entry),
            }
        }

        // Recalculate average entry price from remaining entries
        if !self.open_entry_prices.is_empty() {
            let (total_volume, total_weighted_price) = self.open_entry_prices.iter()
                .fold((dec!(0.0), dec!(0.0)), |(sum_vol, sum_price), entry| {
                    (
                        sum_vol + entry.volume,
                        sum_price + (entry.volume * entry.price)
                    )
                });

            self.average_price = (total_weighted_price / total_volume)
                .round_dp(self.symbol_info.decimal_accuracy);
        } else {
            self.average_price = dec!(0.0);
        }

        // Update position
        self.booked_pnl += total_booked_pnl;
        self.open_pnl -= total_booked_pnl;

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
                side: self.side.clone(),
                symbol_name: self.symbol_name.clone(),
                symbol_code: self.symbol_code.clone(),
                total_quantity_open: self.quantity_open,
                total_quantity_closed: self.quantity_closed,
                average_price: self.average_price,
                booked_pnl: total_booked_pnl,
                average_exit_price: self.average_exit_price.unwrap(),
                account: self.account.clone(),
                originating_order_tag: tag,
                time: time.to_string()
            }
        } else {
            PositionUpdateEvent::PositionReduced {
                position_id: self.position_id.clone(),
                side: self.side.clone(),
                symbol_name: self.symbol_name.clone(),
                symbol_code: self.symbol_code.clone(),
                total_quantity_open: self.quantity_open,
                total_quantity_closed: self.quantity_closed,
                average_price: self.average_price,
                open_pnl: self.open_pnl,
                booked_pnl: total_booked_pnl,
                average_exit_price: self.average_exit_price.unwrap(),
                account: self.account.clone(),
                originating_order_tag: tag,
                time: time.to_string()
            }
        }
    }

    pub(crate) async fn add_to_position(&mut self, mode: StrategyMode, is_simulating_pnl: bool, order_id: OrderId, account_currency: Currency, market_price: Price, quantity: Volume, time: DateTime<Utc>, tag: String) -> PositionUpdateEvent {
        // Add new entry price
        self.open_entry_prices.push_back(EntryPrice::new(quantity, market_price, order_id));

        // Recalculate average price from all entries
        let (total_volume, total_weighted_price) = self.open_entry_prices.iter()
            .fold((dec!(0.0), dec!(0.0)), |(sum_vol, sum_price), entry| {
                (
                    sum_vol + entry.volume,
                    sum_price + (entry.volume * entry.price)
                )
            });

        if total_volume == dec!(0.0) {
            panic!("Total volume should not be 0");
        }

        self.average_price = (total_weighted_price / total_volume)
            .round_dp(self.symbol_info.decimal_accuracy);

        // Update the total quantity
        self.quantity_open += quantity;

        if mode != StrategyMode::Live || is_simulating_pnl {
            // Recalculate open PnL
            self.open_pnl = calculate_theoretical_pnl(
                self.account.brokerage,
                self.side,
                self.average_price,
                market_price,
                self.quantity_open,
                &self.symbol_info,
                self.exchange_rate_multiplier,
                account_currency
            );
        }

        PositionUpdateEvent::Increased {
            symbol_name: self.symbol_name.clone(),
            side: self.side.clone(),
            symbol_code: self.symbol_code.clone(),
            position_id: self.position_id.clone(),
            total_quantity_open: self.quantity_open,
            average_price: self.average_price,
            open_pnl: self.open_pnl,
            booked_pnl: self.booked_pnl,
            account: self.account.clone(),
            originating_order_tag: tag,
            time: time.to_string()
        }
    }


    pub fn get_statistics(&self) -> PositionStatistics {
        let mut stats = PositionStatistics {
            total_trades: self.completed_trades.len(),
            average_entry_price: dec!(0.0),
            average_exit_price: dec!(0.0),
            best_trade_pnl: dec!(0.0),
            worst_trade_pnl: dec!(0.0),
            best_exit_price: dec!(0.0),
            worst_exit_price: dec!(0.0),
            total_quantity_traded: dec!(0.0),
            weighted_avg_hold_time: Duration::zero(),
        };

        if self.completed_trades.is_empty() {
            return stats;
        }

        let mut total_weighted_entry = dec!(0.0);
        let mut total_weighted_exit = dec!(0.0);
        let mut total_quantity = dec!(0.0);
        let mut total_weighted_duration = Duration::zero();

        for trade in &self.completed_trades {
            let trade_pnl = match self.side {
                PositionSide::Long => (trade.exit_price - trade.entry_price) * trade.entry_quantity,
                PositionSide::Short => (trade.entry_price - trade.exit_price) * trade.entry_quantity,
                _ => unreachable!("This shouldn't be possible"),
            };

            // Update best/worst trades
            stats.best_trade_pnl = stats.best_trade_pnl.max(trade_pnl);
            stats.worst_trade_pnl = stats.worst_trade_pnl.min(trade_pnl);

            // Track weighted prices
            total_weighted_entry += trade.entry_price * trade.entry_quantity;
            total_weighted_exit += trade.exit_price * trade.exit_quantity;
            total_quantity += trade.entry_quantity;

            // Calculate hold time
            let entry_time = DateTime::<Utc>::from_str(&trade.entry_time).unwrap();
            let exit_time = DateTime::<Utc>::from_str(&trade.exit_time).unwrap();
            let duration = exit_time - entry_time;
            total_weighted_duration = total_weighted_duration + (duration * trade.entry_quantity.to_i32().unwrap());

            // Track best/worst exits
            if stats.best_exit_price == dec!(0.0) {
                stats.best_exit_price = trade.exit_price;
                stats.worst_exit_price = trade.exit_price;
            } else {
                stats.best_exit_price = stats.best_exit_price.max(trade.exit_price);
                stats.worst_exit_price = stats.worst_exit_price.min(trade.exit_price);
            }
        }

        stats.average_entry_price = (total_weighted_entry / total_quantity).round_dp(self.symbol_info.decimal_accuracy);
        stats.average_exit_price = (total_weighted_exit / total_quantity).round_dp(self.symbol_info.decimal_accuracy);
        stats.total_quantity_traded = total_quantity;
        stats.weighted_avg_hold_time = total_weighted_duration / total_quantity.to_i32().unwrap();

        stats
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use rust_decimal_macros::dec;
    use crate::product_maps::rithmic::maps::get_futures_symbol_info;
    use crate::standardized_types::base_data::quote::Quote;
    use crate::standardized_types::datavendor_enum::DataVendor;
    use crate::standardized_types::enums::{FuturesExchange, MarketType};
    use crate::standardized_types::subscriptions::Symbol;

    fn setup_basic_position() -> Position {
        let symbol_info = get_futures_symbol_info("NQ").unwrap();
        Position::new(
            "NQ".to_string(),
            "NQ2403".to_string(), // Example futures contract
            "Test".to_string(),
            Account::new(Brokerage::Test, "test-account".to_string()),
            PositionSide::Long,
            dec!(1.0),
            dec!(17500.0), // More realistic NQ price
            "test-pos-1".to_string(),
            symbol_info,
            dec!(1.0),
            "test".to_string(),
            Utc::now(),
            PositionCalculationMode::FIFO,
        )
    }

    #[tokio::test]
    async fn test_fifo_position_handling() {
        let mut position = setup_basic_position();

        // Add multiple entries at different prices
        position.add_to_position(
            StrategyMode::Backtest,
            true,
            "Test".to_string(),
            Currency::USD,
            dec!(17525.0),
            dec!(0.5),
            Utc::now(),
            "test-add-1".to_string()
        ).await;

        position.add_to_position(
            StrategyMode::Backtest,
            true,
            "Test".to_string(),
            Currency::USD,
            dec!(17550.0),
            dec!(0.5),
            Utc::now(),
            "test-add-2".to_string()
        ).await;

        // Verify entry prices are in correct order
        assert_eq!(position.open_entry_prices.len(), 3);
        assert_eq!(position.open_entry_prices[0].price, dec!(17500.0));
        assert_eq!(position.open_entry_prices[1].price, dec!(17525.0));
        assert_eq!(position.open_entry_prices[2].price, dec!(17550.0));

        // Reduce position (should take from first entry - FIFO)
        let event = position.reduce_position_size(
            dec!(17575.0),
            dec!(1.0),
            "Test".to_string(),
            Currency::USD,
            dec!(1.0),
            Utc::now(),
            "test-reduce-1".to_string()
        ).await;

        // Verify FIFO behavior
        assert_eq!(position.open_entry_prices.len(), 2);
        assert_eq!(position.open_entry_prices[0].price, dec!(17525.0));
        assert_eq!(position.open_entry_prices[1].price, dec!(17550.0));

        match event {
            PositionUpdateEvent::PositionReduced { booked_pnl, .. } => {
                assert!(booked_pnl > dec!(0.0)); // Should be profitable
            },
            _ => panic!("Expected PositionReduced event"),
        }
    }

    #[tokio::test]
    async fn test_lifo_position_handling() {
        let symbol_info = get_futures_symbol_info("NQ").unwrap();
        let mut position = Position::new(
            "NQ".to_string(),
            "NQ2403".to_string(),
            "Test".to_string(),
            Account::new(Brokerage::Test, "test-account".to_string()),
            PositionSide::Long,
            dec!(1.0),
            dec!(17500.0),
            "test-pos-1".to_string(),
            symbol_info,
            dec!(1.0),
            "test".to_string(),
            Utc::now(),
            PositionCalculationMode::LIFO,
        );

        // Add multiple entries
        position.add_to_position(
            StrategyMode::Backtest,
            true,
            "Test".to_string(),
            Currency::USD,
            dec!(17525.0),
            dec!(1.0),
            Utc::now(),
            "test-add-1".to_string()
        ).await;

        position.add_to_position(
            StrategyMode::Backtest,
            true,
            "Test".to_string(),
            Currency::USD,
            dec!(17550.0),
            dec!(1.0),
            Utc::now(),
            "test-add-2".to_string()
        ).await;

        // Reduce position (should take from last entry - LIFO)
        let event = position.reduce_position_size(
            dec!(17575.0),
            dec!(1.0),
            "Test".to_string(),
            Currency::USD,
            dec!(1.0),
            Utc::now(),
            "test-reduce-1".to_string()
        ).await;

        // Verify LIFO behavior
        assert_eq!(position.open_entry_prices.len(), 2);
        assert_eq!(position.open_entry_prices[0].price, dec!(17500.0));
        assert_eq!(position.open_entry_prices[1].price, dec!(17525.0));

        match event {
            PositionUpdateEvent::PositionReduced { booked_pnl, .. } => {
                assert!(booked_pnl > dec!(0.0));
            },
            _ => panic!("Expected PositionReduced event"),
        }
    }

    #[tokio::test]
    async fn test_partial_position_reduction() {
        let mut position = setup_basic_position();

        // Add position
        position.add_to_position(
            StrategyMode::Backtest,
            true,
            "Test".to_string(),
            Currency::USD,
            dec!(17525.0),
            dec!(2.0),
            Utc::now(),
            "test-add-1".to_string()
        ).await;

        // Reduce position partially
        position.reduce_position_size(
            dec!(17550.0),
            dec!(1.5),
            "Test".to_string(),
            Currency::USD,
            dec!(1.0),
            Utc::now(),
            "test-reduce-1".to_string()
        ).await;

        // Verify remaining position
        assert_eq!(position.quantity_open, dec!(1.5));
        assert_eq!(position.quantity_closed, dec!(1.5));
        assert!(!position.is_closed);

        // Verify entry prices were properly adjusted
        let total_entry_volume: Decimal = position.open_entry_prices.iter()
            .map(|entry| entry.volume)
            .sum();
        assert_eq!(total_entry_volume, position.quantity_open);
    }

    #[tokio::test]
    async fn test_complete_position_closure() {
        let mut position = setup_basic_position();

        // Close entire position
        let event = position.reduce_position_size(
            dec!(17525.0),
            dec!(1.0),
            "Test".to_string(),
            Currency::USD,
            dec!(1.0),
            Utc::now(),
            "test-close".to_string()
        ).await;

        // Verify position state
        assert!(position.is_closed);
        assert_eq!(position.quantity_open, dec!(0.0));
        assert_eq!(position.open_pnl, dec!(0.0));
        assert!(position.close_time.is_some());
        assert!(position.open_entry_prices.is_empty());

        match event {
            PositionUpdateEvent::PositionClosed { .. } => {},
            _ => panic!("Expected PositionClosed event"),
        }
    }

    #[tokio::test]
    async fn test_average_price_calculation() {
        let mut position = setup_basic_position();

        // Initial average price should be the entry price
        assert_eq!(position.average_price, dec!(17500.0));

        // Add position at different price
        position.add_to_position(
            StrategyMode::Backtest,
            true,
            "Test".to_string(),
            Currency::USD,
            dec!(17600.0),
            dec!(1.0),
            Utc::now(),
            "test-add-1".to_string()
        ).await;

        // Verify weighted average price
        assert_eq!(position.average_price, dec!(17550.0));

        // Reduce position
        position.reduce_position_size(
            dec!(17650.0),
            dec!(1.0),
            "Test".to_string(),
            Currency::USD,
            dec!(1.0),
            Utc::now(),
            "test-reduce-1".to_string()
        ).await;

        // Verify average price is recalculated correctly
        assert_eq!(position.average_price, dec!(17600.0));
    }

    #[tokio::test]
    async fn test_pnl_calculation() {
        let mut position = setup_basic_position();

        // Add to position
        position.add_to_position(
            StrategyMode::Backtest,
            true,
            "Test".to_string(),
            Currency::USD,
            dec!(17525.0),
            dec!(1.0),
            Utc::now(),
            "test-add-1".to_string()
        ).await;

        // Update market price and check open PnL
        let mock_data = BaseDataEnum::Quote(Quote {
            symbol: Symbol::new("NQ".to_string(), DataVendor::Rithmic, MarketType::Futures(FuturesExchange::CME)),
            bid: dec!(17550.0),
            ask_volume: dec!(100),
            ask: dec!(17550.0),
            time: Utc::now().to_string(),
            bid_volume: dec!(100),
        });

        let open_pnl = position.update_base_data(&mock_data, Currency::USD);
        assert!(open_pnl > dec!(0.0));

        // Close half position and verify booked PnL
        let event = position.reduce_position_size(
            dec!(17550.0),
            dec!(1.0),
            "Test".to_string(),
            Currency::USD,
            dec!(1.0),
            Utc::now(),
            "test-reduce-1".to_string()
        ).await;

        match event {
            PositionUpdateEvent::PositionReduced { booked_pnl, open_pnl, .. } => {
                assert!(booked_pnl > dec!(0.0));
                assert!(open_pnl > dec!(0.0));
            },
            _ => panic!("Expected PositionReduced event"),
        }
    }

    #[tokio::test]
    #[should_panic(expected = "Something wrong with logic, ledger should know this not to be possible")]
    async fn test_invalid_reduction() {
        let mut position = setup_basic_position();

        // Try to reduce more than available
        position.reduce_position_size(
            dec!(17525.0),
            dec!(2.0), // More than position size
            "Test".to_string(),
            Currency::USD,
            dec!(1.0),
            Utc::now(),
            "test-invalid-reduce".to_string()
        ).await;
    }

    #[tokio::test]
    async fn test_average_price_calculation_from_entries() {
        let mut position = setup_basic_position();

        // Initial position
        let initial_quantity = dec!(1.0);
        let initial_price = dec!(17500.0);
        assert_eq!(position.average_price, initial_price);

        // Add second entry
        let second_quantity = dec!(2.0);
        let second_price = dec!(17600.0);
        position.add_to_position(
            StrategyMode::Backtest,
            true,
            "Test".to_string(),
            Currency::USD,
            second_price,
            second_quantity,
            Utc::now(),
            "test-add-1".to_string()
        ).await;

        // Calculate expected average: (17500 * 1 + 17600 * 2) / 3
        let expected_avg = (initial_price * initial_quantity + second_price * second_quantity) /
            (initial_quantity + second_quantity);
        assert_eq!(position.average_price.round_dp(2), expected_avg.round_dp(2));

        // Verify open_entry_prices matches average price calculation
        let (total_vol, total_weighted) = position.open_entry_prices.iter()
            .fold((dec!(0.0), dec!(0.0)), |(vol, weighted), entry| {
                (vol + entry.volume, weighted + entry.volume * entry.price)
            });

        assert_eq!(position.average_price.round_dp(2), (total_weighted / total_vol).round_dp(2));
    }

    #[tokio::test]
    async fn test_trade_statistics() {
        let mut position = setup_basic_position();
        let initial_time = Utc::now();

        // Add to position
        position.add_to_position(
            StrategyMode::Backtest,
            true,
            "Test".to_string(),
            Currency::USD,
            dec!(17525.0),
            dec!(2.0),
            initial_time + Duration::minutes(30),
            "add-1".to_string()
        ).await;

        // Reduce position in parts
        position.reduce_position_size(
            dec!(17550.0),
            dec!(1.5),
            "Test".to_string(),
            Currency::USD,
            dec!(1.0),
            initial_time + Duration::hours(1),
            "exit-1".to_string()
        ).await;

        position.reduce_position_size(
            dec!(17575.0),
            dec!(1.5),
            "Test".to_string(),
            Currency::USD,
            dec!(1.0),
            initial_time + Duration::hours(2),
            "exit-2".to_string()
        ).await;

        let stats = position.get_statistics();

        assert_eq!(stats.total_trades, 3);
        assert!(stats.best_trade_pnl > stats.worst_trade_pnl);
        assert!(stats.best_exit_price > stats.worst_exit_price);
        assert_eq!(stats.total_quantity_traded, dec!(3.0));
    }

    #[tokio::test]
    async fn test_position_export_with_trades() {
        let mut position = setup_basic_position();
        let initial_time = Utc::now();

        // Add to position
        position.add_to_position(
            StrategyMode::Backtest,
            true,
            "Test".to_string(),
            Currency::USD,
            dec!(17525.0),
            dec!(2.0),
            initial_time + Duration::minutes(30),
            "add-1".to_string()
        ).await;

        // Reduce position in parts
        position.reduce_position_size(
            dec!(17550.0),
            dec!(1.5),
            "Test".to_string(),
            Currency::USD,
            dec!(1.0),
            initial_time + Duration::hours(1),
            "exit-1".to_string()
        ).await;

        position.reduce_position_size(
            dec!(17575.0),
            dec!(1.5),
            "Test".to_string(),
            Currency::USD,
            dec!(1.0),
            initial_time + Duration::hours(2),
            "exit-2".to_string()
        ).await;

        let export = position.to_export();

        // Verify the exported data uses trade history for calculations
        assert!(export.average_entry_price > dec!(0.0));
        assert!(export.average_exit_price > export.average_entry_price); // Should be profitable
        assert_eq!(export.quantity, dec!(3.0));
    }
}
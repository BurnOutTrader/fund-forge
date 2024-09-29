use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use rust_decimal_macros::dec;
use serde_derive::{Deserialize, Serialize};
use crate::apis::brokerage::broker_enum::Brokerage;
use crate::standardized_types::enums::PositionSide;
use crate::standardized_types::subscriptions::SymbolName;
use crate::standardized_types::{Price, Volume};
use crate::standardized_types::accounts::ledgers::{AccountId, Currency};
use crate::standardized_types::orders::orders::{OrderId, ProtectiveOrder};
use crate::standardized_types::symbol_info::SymbolInfo;

pub type PositionId = String;
#[derive(Serialize)]
pub(crate) struct PositionExport {
    symbol_name: String,
    side: String,
    quantity: Volume,
    average_price: Price,
    exit_price: Price,
    booked_pnl: Price,
    highest_recoded_price: Price,
    lowest_recoded_price: Price,
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug, PartialEq, Serialize, Deserialize,     PartialOrd,)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct Position {
    pub symbol_name: SymbolName,
    pub brokerage: Brokerage,
    pub account_id: AccountId,
    pub side: PositionSide,
    pub quantity_open: Volume,
    pub quantity_closed: Volume,
    pub average_price: Price,
    pub open_pnl: Price,
    pub booked_pnl: Price,
    pub highest_recoded_price: Price,
    pub lowest_recoded_price: Price,
    pub average_exit_price: Option<Price>,
    pub is_closed: bool,
    pub position_id: PositionId,
    pub symbol_info: SymbolInfo,
    pub account_currency: Currency,
}

#[derive(
    Clone,
    rkyv::Serialize,
    rkyv::Deserialize,
    Archive,
    Debug,
    PartialEq,
)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum PositionUpdateEvent {
    Opened(PositionId),
    Increased {
        position_id: PositionId,
        total_quantity_open: Volume,
        average_price: Price,
        open_pnl: Price,
        booked_pnl: Price,
    },
    Reduced{
        position_id: PositionId,
        total_quantity_open: Volume,
        total_quantity_closed: Volume,
        average_price: Price,
        open_pnl: Price,
        booked_pnl: Price,
        average_exit_price: Price,
    },
    Closed {
        position_id: PositionId,
        total_quantity_open: Volume,
        total_quantity_closed: Volume,
        average_price: Price,
        booked_pnl: Price,
        average_exit_price: Option<Price>,
    },
    UpdateSnapShot {
        position_id: PositionId,
        position: Position
    },
}

//todo make it so stop loss and take profit can be attached to positions, then instead of updating in the market handler, those orders update in the position and auto cancel themselves when position closes
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
        account_currency: Currency
    ) -> Self {
        Self {
            symbol_name,
            brokerage,
            account_id,
            side,
            quantity_open: quantity,
            quantity_closed: dec!(0.0),
            average_price,
            open_pnl: dec!(0.0),
            booked_pnl: dec!(0.0),
            highest_recoded_price: average_price,
            lowest_recoded_price: average_price,
            average_exit_price: None,
            is_closed: false,
            position_id: id,
            symbol_info,
            account_currency,
        }
    }

    pub(crate) fn to_export(&self) -> PositionExport {
        PositionExport {
            symbol_name: self.symbol_name.to_string(),
            side: self.side.to_string(),
            quantity: self.quantity_closed,
            average_price: self.average_price,
            exit_price: self.average_exit_price.unwrap(),
            booked_pnl: self.booked_pnl,
            highest_recoded_price: self.highest_recoded_price,
            lowest_recoded_price: self.lowest_recoded_price,
        }
    }
}

pub(crate) mod historical_position {
    use chrono::{DateTime, Utc};
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use crate::helpers::decimal_calculators::round_to_tick_size;
    use crate::server_connections::add_buffer;
    use crate::standardized_types::accounts::ledgers::calculate_pnl;
    use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
    use crate::standardized_types::enums::PositionSide;
    use crate::standardized_types::orders::orders::ProtectiveOrder;
    use crate::standardized_types::{Price, Volume};
    use crate::standardized_types::accounts::position::{Position, PositionUpdateEvent};
    use crate::standardized_types::strategy_events::StrategyEvent;

    impl Position {

        pub(crate) async fn reduce_position_size(&mut self, market_price: Price, quantity: Volume, time: DateTime<Utc>) -> Price {
            // Ensure quantity does not exceed the open quantity
            let quantity = if quantity > self.quantity_open {
                self.quantity_open
            } else {
                quantity
            };

            // Calculate booked PnL
            let booked_pnl = calculate_pnl(
                &self.side,
                self.average_price,
                market_price,
                self.symbol_info.tick_size,
                self.symbol_info.value_per_tick,
                quantity,
                &self.symbol_info.pnl_currency,
                &self.account_currency,
                time,
            );

            // Update position
            self.booked_pnl += booked_pnl;
            self.quantity_open -= quantity;
            self.quantity_closed += quantity;
            self.is_closed = self.quantity_open <= dec!(0.0);
            self.average_exit_price = Some(market_price);

            // Reset open PnL if position is closed
            if self.is_closed {
                self.booked_pnl += self.open_pnl;
                self.open_pnl = dec!(0.0);
                let event = StrategyEvent::PositionEvents(PositionUpdateEvent::Closed {
                    position_id: self.position_id.clone(),
                    total_quantity_open: self.quantity_open,
                    total_quantity_closed: self.quantity_closed,
                    average_price: self.average_price,
                    booked_pnl: self.booked_pnl,
                    average_exit_price: self.average_exit_price,
                });
                add_buffer(time, event).await;
            } else {
                let event = StrategyEvent::PositionEvents(PositionUpdateEvent::Reduced {
                    position_id: self.position_id.clone(),
                    total_quantity_open: self.quantity_open,
                    total_quantity_closed: self.quantity_closed,
                    average_price: self.average_price,
                    open_pnl: self.open_pnl,
                    booked_pnl: self.booked_pnl,
                    average_exit_price: self.average_exit_price.unwrap(),
                });
                add_buffer(time, event).await;
            }
            booked_pnl
        }

        pub(crate) async fn add_to_position(&mut self, market_price: Price, quantity: Volume, time: DateTime<Utc>) {
            // Correct the average price calculation with proper parentheses
            self.average_price = round_to_tick_size(
                ((self.quantity_open * self.average_price) + (quantity * market_price))
                    / (self.quantity_open + quantity),
                self.symbol_info.tick_size,
            );

            // Update the total quantity
            self.quantity_open += quantity;

            // Recalculate open PnL
            self.open_pnl = calculate_pnl(
                &self.side,
                self.average_price,
                market_price,
                self.symbol_info.tick_size,
                self.symbol_info.value_per_tick,
                self.quantity_open,
                &self.symbol_info.pnl_currency,
                &self.account_currency,
                time,
            );

            let event = StrategyEvent::PositionEvents(PositionUpdateEvent::Increased {
                position_id: self.position_id.clone(),
                total_quantity_open: self.quantity_open,
                average_price: self.average_price,
                open_pnl: self.open_pnl,
                booked_pnl: self.booked_pnl,
            });

            add_buffer(time, event).await;
        }

        //todo, this needs to be ddone by passing in the position, so we can use the market price, it should return what kind of order was triggered
        pub(crate) fn backtest_update_base_data(&mut self, base_data: &BaseDataEnum, time: DateTime<Utc>) {
            if self.is_closed {
                return;
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
            self.open_pnl = calculate_pnl(
                &self.side,
                self.average_price,
                market_price,
                self.symbol_info.tick_size,
                self.symbol_info.value_per_tick,
                self.quantity_open,
                &self.symbol_info.pnl_currency,
                &self.account_currency,
                time,
            );
        }
    }
}
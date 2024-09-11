use std::fs::create_dir_all;
use std::path::Path;
use chrono::{DateTime, Utc};
use crate::apis::brokerage::Brokerage;
use crate::standardized_types::data_server_messaging::FundForgeError;
use crate::standardized_types::enums::PositionSide;
use crate::standardized_types::subscriptions::{SymbolName};
use crate::traits::bytes::Bytes;
use dashmap::DashMap;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use crate::helpers::decimal_calculators::{round_to_tick_size};
use crate::standardized_types::orders::orders::ProtectiveOrder;
use csv::Writer;
use rust_decimal_macros::dec;
use serde_derive::Serialize;
use crate::standardized_types::{Price, Volume};
// B
pub type AccountId = String;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum AccountCurrency {
    AUD,
    USD,
    BTC,
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct SymbolInfo {
    symbol_name: SymbolName,
    pnl_currency: AccountCurrency,
    value_per_tick: Price,
    tick_size: Price
}

impl SymbolInfo {
    pub fn new(symbol_name: SymbolName,
               pnl_currency: AccountCurrency,
               value_per_tick: Price,
               tick_size: Price) -> Self {
        Self {
            symbol_name,
            pnl_currency,
            value_per_tick,
            tick_size
        }
    }
}

/// A ledger specific to the strategy which will ignore positions not related to the strategy but will update its balances relative to the actual account balances for live trading.
#[derive(Debug)]
pub struct Ledger {
    pub account_id: AccountId,
    pub brokerage: Brokerage,
    pub cash_value: Price,
    pub cash_available: Price,
    pub currency: AccountCurrency,
    pub cash_used: Price,
    pub positions: DashMap<SymbolName, Position>,
    pub positions_closed: DashMap<SymbolName, Vec<Position>>,
    pub positions_counter: DashMap<SymbolName, u64>,
    pub symbol_info: DashMap<SymbolName, SymbolInfo>,
    pub open_pnl: Price,
    pub booked_pnl: Price,
}
impl Ledger {
    pub fn new(account_info: AccountInfo) -> Self {
        let positions = account_info
            .positions
            .into_iter()
            .map(|position| (position.symbol_name.clone(), position))
            .collect();

        let ledger = Self {
            account_id: account_info.account_id,
            brokerage: account_info.brokerage,
            cash_value: account_info.cash_value,
            cash_available: account_info.cash_available,
            currency: account_info.currency,
            cash_used: account_info.cash_used,
            positions,
            positions_closed: DashMap::new(),
            positions_counter: DashMap::new(),
            symbol_info: DashMap::new(),
            open_pnl: dec!(0.0),
            booked_pnl: dec!(0.0),
        };
        ledger
    }

    // Function to export closed positions to CSV
    pub fn export_positions_to_csv(&self, folder: &str) {
        // Create the folder if it does not exist
        if let Err(e) = create_dir_all(folder) {
            eprintln!("Failed to create directory {}: {}", folder, e);
            return;
        }

        // Get current date in desired format
        let date = Utc::now().format("%Y-%m-%d").to_string();

        // Use brokerage and account ID to format the filename
        let brokerage = self.brokerage.to_string(); // Assuming brokerage can be formatted as string
        let file_name = format!("{}/{}_{}_{}.csv", folder, brokerage, self.account_id, date);

        // Create a writer for the CSV file
        let file_path = Path::new(&file_name);
        match Writer::from_path(file_path) {
            Ok(mut wtr) => {
                // Write headers once
                if let Err(e) = wtr.write_record(&[
                    "Symbol Name",
                    "Side",
                    "Quantity",
                    "Average Price",
                    "Exit Price",
                    "Pnl by Quantity",
                    "Booked PnL",
                    "Highest Recorded Price",
                    "Lowest Recorded Price",
                ]) {
                    eprintln!("Failed to write headers to {}: {}", file_path.display(), e);
                    return;
                }

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
}

pub(crate) mod historical_ledgers {
    use chrono::{DateTime, Utc};
    use dashmap::DashMap;
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromPrimitive;
    use rust_decimal_macros::dec;
    use crate::apis::brokerage::Brokerage;
    use crate::apis::brokerage::client_requests::ClientSideBrokerage;
    use crate::standardized_types::accounts::ledgers::{AccountCurrency, AccountId, Ledger, Position, PositionId};
    use crate::standardized_types::data_server_messaging::FundForgeError;
    use crate::standardized_types::enums::{OrderSide, PositionSide};
    use crate::standardized_types::orders::orders::ProtectiveOrder;
    use crate::standardized_types::{Price, Volume};
    use crate::standardized_types::subscriptions::SymbolName;
    use crate::standardized_types::time_slices::TimeSlice;

    impl Ledger {
        pub fn update_brackets(&self, symbol_name: &SymbolName, brackets: Vec<ProtectiveOrder>) {
            if let Some(mut position) = self.positions.get_mut(symbol_name) {
                position.brackets = Some(brackets);
            }
        }

        pub async fn subtract_margin_used(&mut self, symbol_name: SymbolName, quantity: Volume) {
            let margin = self.brokerage.margin_required_historical(symbol_name, quantity).await;
            self.cash_available += margin;
            self.cash_used -= margin;
        }

        pub async fn add_margin_used(&mut self, symbol_name: SymbolName, quantity: Volume) -> Result<(), FundForgeError> {
            let margin = self.brokerage.margin_required_historical(symbol_name, quantity).await;
            // Check if the available cash is sufficient to cover the margin
            if self.cash_available < margin {
                return Err(FundForgeError::ClientSideErrorDebug("Insufficient funds".to_string()));
            }
            self.cash_available -= margin;
            self.cash_used += margin;
            Ok(())
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
                    } else {
                        loss_pnl += position.booked_pnl;
                        if position.booked_pnl < dec!(0.0) {
                            losses += 1;
                        }
                    }
                    pnl += position.booked_pnl;
                }
            }

            let risk_reward = if losses > 0 {
                win_pnl / -loss_pnl // negate loss_pnl for correct calculation
            } else {
                dec!(0.0)
            };

            // Calculate profit factor
            let profit_factor = if loss_pnl != dec!(0.0) {
                win_pnl / -loss_pnl // negate loss_pnl
            } else {
                dec!(0.0)
            };

            // Calculate win rate
            let win_rate = if total_trades > 0 {
                Decimal::from_usize(wins).unwrap() / Decimal::from_usize(total_trades).unwrap() * Decimal::from_f64(100.0).unwrap()
            } else {
                dec!(0.0)
            };

            let break_even = total_trades - wins - losses;
            format!("Brokerage: {}, Account: {}, Balance: {:.2}, Win Rate: {:.2}%, Risk Reward: {:.2}, Profit Factor {:.2}, Total profit: {:.2}, Total Wins: {}, Total Losses: {}, Break Even: {}, Total Trades: {}, Cash Used: {}, Cash Available: {}",
                    self.brokerage, self.account_id, self.cash_value, win_rate, risk_reward, profit_factor, pnl, wins, losses, break_even, total_trades, self.cash_used, self.cash_available) //todo, check against self.pnl

        }

        pub fn paper_account_init(
            account_id: AccountId,
            brokerage: Brokerage,
            cash_value: Price,
            currency: AccountCurrency,
        ) -> Self {
            let account = Self {
                account_id,
                brokerage,
                cash_value,
                cash_available: cash_value,
                currency,
                cash_used: dec!(0.0),
                positions: DashMap::new(),
                positions_closed: DashMap::new(),
                positions_counter: DashMap::new(),
                symbol_info: DashMap::new(),
                open_pnl: dec!(0.0),
                booked_pnl: dec!(0.0),
            };
            account
        }

        pub async fn generate_id(&self, symbol_name: &SymbolName, time: DateTime<Utc>, side: PositionSide) -> PositionId {
            self.positions_counter.entry(symbol_name.clone())
                .and_modify(|value| *value += 1)  // Increment the value if the key exists
                .or_insert(1);

            format!("{}-{}-{}-{}-{}-{}", self.brokerage, self.account_id, symbol_name, time.timestamp(), self.positions_counter.get(symbol_name).unwrap().value().clone(), side)
        }

        pub fn process_closed_position(&self, position: Position) {
            if !self.positions_closed.contains_key(&position.symbol_name) {
                self.positions_closed.insert(position.symbol_name.clone(), vec![position]);
            } else {
                if let Some(mut positions) = self.positions_closed.get_mut(&position.symbol_name) {
                    positions.value_mut().push(position);
                }
            }
        }



        pub async fn update_or_create_paper_position(
            &mut self,
            symbol_name: &SymbolName,
            quantity: Volume,
            market_price: Price,
            side: OrderSide,
            time: &DateTime<Utc>,
            brackets: Option<Vec<ProtectiveOrder>>
        ) -> Result<(), FundForgeError>
        {
            // Check if there's an existing position for the given symbol
            if self.positions.contains_key(symbol_name) {
                let (_, mut existing_position) = self.positions.remove(symbol_name).unwrap();
                let is_reducing = (existing_position.side == PositionSide::Long && side == OrderSide::Sell) || (existing_position.side == PositionSide::Short && side == OrderSide::Buy);
                if is_reducing {
                    if let Some(new_brackets) = brackets {
                        existing_position.brackets = Some(new_brackets);
                    }

                    let pnl = existing_position.reduce_position_size(market_price, quantity, time);
                    self.booked_pnl += pnl;

                    self.subtract_margin_used(symbol_name.clone(), quantity).await;
                    self.cash_value = self.cash_available + self.cash_used + self.booked_pnl + self.open_pnl;

                    if !existing_position.is_closed {
                        self.positions.insert(symbol_name.clone(), existing_position);
                    } else {
                        self.process_closed_position(existing_position);
                    }
                    Ok(())
                } else {
                    // Deduct margin from cash available
                    match self.add_margin_used(symbol_name.clone(), quantity).await {
                        Ok(_) => {}
                        Err(e) => return Err(e)
                    }

                    existing_position.add_to_position(market_price, quantity, time);
                    if let Some(new_brackets) = brackets {
                        existing_position.brackets = Some(new_brackets);
                    }


                    self.positions.insert(symbol_name.clone(), existing_position);
                    self.cash_value = self.cash_available + self.cash_used + self.booked_pnl + self.open_pnl;
                    Ok(())
                }
            } else {
                match self.add_margin_used(symbol_name.clone(), quantity).await {
                    Ok(_) => {}
                    Err(e) => return Err(e)
                }

                if !self.symbol_info.contains_key(symbol_name) {
                    let symbol_info = match self.brokerage.symbol_info(symbol_name.clone()).await {
                        Ok(info) => info,
                        Err(_) => return Err(FundForgeError::ClientSideErrorDebug(format!("No Symbol info for: {}", symbol_name)))
                    };
                    self.symbol_info.insert(symbol_name.clone(), symbol_info);
                }

                // Determine the side of the position based on the order side
                let side = match side {
                    OrderSide::Buy => PositionSide::Long,
                    OrderSide::Sell => PositionSide::Short
                };

                // Create a new position with the given details
                let position = Position::enter(
                    symbol_name.clone(),
                    self.brokerage.clone(),
                    side,
                    quantity,
                    market_price,
                    self.generate_id(&symbol_name, time.clone(), side).await,
                    self.symbol_info.get(symbol_name).unwrap().value().clone(),
                    self.currency.clone(),
                    brackets
                );

                // Insert the new position into the positions map
                self.positions.insert(position.symbol_name.clone(), position);

                self.cash_value = self.cash_available + self.cash_used + self.booked_pnl + self.open_pnl ;
                Ok(())
            }
        }

        pub async fn exit_position_paper(&mut self, symbol_name: &SymbolName, market_price: Price, time: DateTime<Utc>) {
            if !self.positions.contains_key(symbol_name) {
                return;
            }
            let (_, mut old_position) = self.positions.remove(symbol_name).unwrap();

            let booked_pnl =old_position.reduce_position_size(market_price, old_position.quantity_open, &time);
            self.booked_pnl += booked_pnl;

            old_position.open_pnl = dec!(0.0);
            self.process_closed_position(old_position);
            self.print();
        }

        pub async fn account_init(account_id: AccountId, brokerage: Brokerage) -> Self {
            match brokerage.init_ledger(account_id).await {
                Ok(ledger) => ledger,
                Err(e) => panic!("Failed to initialize account: {:?}", e),
            }
        }

        pub async fn is_long(&self, symbol_name: &SymbolName) -> bool {
            if let Some(position) = self.positions.get(symbol_name) {
                if position.value().side == PositionSide::Long {
                    return true;
                }
            }
            false
        }

        pub async fn is_short(&self, symbol_name: &SymbolName) -> bool {
            if let Some(position) = self.positions.get(symbol_name) {
                if position.value().side == PositionSide::Short {
                    return true;
                }
            }
            false
        }

        pub async fn is_flat(&self, symbol_name: &SymbolName) -> bool {
            if let Some(_) = self.positions.get(symbol_name) {
                false
            } else {
                true
            }
        }

        pub async fn on_data_update(&mut self, time_slice: TimeSlice, time: &DateTime<Utc>) {
            let mut open_pnl = dec!(0.0);
            let mut booked_pnl = dec!(0.0);
            for base_data_enum in time_slice {
                let data_symbol_name = &base_data_enum.symbol().name;
                if let Some(mut position) = self.positions.get_mut(data_symbol_name) {
                    booked_pnl += position.backtest_update_base_data(&base_data_enum, time);
                    open_pnl += position.open_pnl;

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
            self.open_pnl = open_pnl;
            self.booked_pnl += booked_pnl;
            self.cash_value += booked_pnl;
        }
    }
}

pub type PositionId = String;
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct Position {
    pub symbol_name: SymbolName,
    pub brokerage: Brokerage,
    pub side: PositionSide,
    pub quantity_open: Volume,
    pub quantity_closed: Volume,
    pub average_price: Price,
    pub open_pnl: Price,
    pub booked_pnl: Price,
    pub highest_recoded_price: Price,
    pub lowest_recoded_price: Price,
    pub exit_price: Option<Price>,
    pub is_closed: bool,
    pub id: PositionId,
    pub symbol_info: SymbolInfo,
    account_currency: AccountCurrency,
    brackets: Option<Vec<ProtectiveOrder>>
}
//todo make it so stop loss and take profit can be attached to positions, then instead of updating in the market handler, those orders update in the position and auto cancel themselves when position closes
impl Position {
    pub fn enter(
        symbol_name: SymbolName,
        brokerage: Brokerage,
        side: PositionSide,
        quantity: Volume,
        average_price: Price,
        id: PositionId,
        symbol_info: SymbolInfo,
        account_currency: AccountCurrency,
        brackets: Option<Vec<ProtectiveOrder>>
    ) -> Self {
        Self {
            symbol_name,
            brokerage,
            side,
            quantity_open: quantity,
            quantity_closed: dec!(0.0),
            average_price,
            open_pnl: dec!(0.0),
            booked_pnl: dec!(0.0),
            highest_recoded_price: average_price,
            lowest_recoded_price: average_price,
            exit_price: None,
            is_closed: false,
            id,
            symbol_info,
            account_currency,
            brackets
        }
    }

    fn to_export(&self) -> PositionExport {
        let pnl_by_quantity = match self.side {
            PositionSide::Long => {
                self.quantity_closed * (self.exit_price.unwrap() - self.average_price)
            }
            PositionSide::Short => {
                self.quantity_closed * (self.average_price - self.exit_price.unwrap())
            }
        };
        PositionExport {
            symbol_name: self.symbol_name.to_string(),
            side: self.side.to_string(),
            quantity: self.quantity_closed,
            average_price: self.average_price,
            exit_price: self.exit_price.unwrap(),
            pnl_by_quantity,
            booked_pnl: round_to_tick_size(self.booked_pnl, self.symbol_info.tick_size),
            highest_recoded_price: self.highest_recoded_price,
            lowest_recoded_price: self.lowest_recoded_price,
        }
    }
}

//todo, later I will move this to be for historical only
fn calculate_pnl(
    side: &PositionSide,
    entry_price: Price,
    market_price: Price,
    tick_size: Price,
    value_per_tick: Price,
    quantity: Volume,
    _base_currency: &AccountCurrency,
    _account_currency: &AccountCurrency,
    _time: &DateTime<Utc>,
) -> Price {

    // Calculate the price difference based on position side
    let price_difference = match side {
        PositionSide::Long => market_price - entry_price,   // Profit if market price > entry price
        PositionSide::Short => entry_price - market_price, // Profit if entry price > market price
    };

    // Round the price difference to the nearest tick size using Decimal
    let rounded_difference = (price_difference / tick_size).round() * tick_size;

    // Convert rounded difference back to f64 and calculate PnL
    let pnl = rounded_difference * value_per_tick * quantity;

    // Placeholder for currency conversion if the base currency differs from account currency
    // let pnl = convert_currency(pnl, base_currency, account_currency, time);

    pnl
}

pub(crate) mod historical_position {
    use chrono::{DateTime, Utc};
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromPrimitive;
    use rust_decimal_macros::dec;
    use crate::helpers::decimal_calculators::round_to_tick_size;
    use crate::standardized_types::accounts::ledgers::{calculate_pnl, Position};
    use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
    use crate::standardized_types::enums::PositionSide;
    use crate::standardized_types::orders::orders::ProtectiveOrder;
    use crate::standardized_types::{Price, Volume};

    impl Position {

        pub(crate) fn reduce_position_size(&mut self, market_price: Price, quantity: Volume, time: &DateTime<Utc>) -> Price {
            let quantity = match quantity > self.quantity_open {
                true => self.quantity_open,
                false => quantity
            };
            let booked_pnl = calculate_pnl(&self.side, self.average_price, market_price, self.symbol_info.tick_size, self.symbol_info.value_per_tick, quantity, &self.symbol_info.pnl_currency, &self.account_currency, time);
            self.booked_pnl += booked_pnl;
            self.quantity_open -= quantity;
            self.quantity_closed += quantity;
            self.is_closed = self.quantity_open <= dec!(0.0);
            self.exit_price = Some(market_price);
            if self.is_closed {
                self.open_pnl = dec!(0.0);
            }
            booked_pnl
        }

        pub(crate) fn add_to_position(&mut self, market_price: Price, quantity: Volume, time: &DateTime<Utc>) {
            // If adding to the short position, calculate the new average price
            self.average_price =
               round_to_tick_size((self.quantity_open* self.average_price) + (quantity * market_price)
                    / (self.quantity_open + quantity), self.symbol_info.tick_size);

            // Update the total quantity
            self.quantity_open += quantity;

            self.open_pnl =  calculate_pnl(&self.side, self.average_price, market_price, self.symbol_info.tick_size, self.symbol_info.value_per_tick, self.quantity_open, &self.symbol_info.pnl_currency, &self.account_currency, time);
        }

        pub(crate) fn backtest_update_base_data(&mut self, base_data: &BaseDataEnum, time: &DateTime<Utc>) -> Price {
            if self.is_closed {
                return dec!(0.0);
            }
            let (market_price, highest_price, lowest_price) = match base_data {
                BaseDataEnum::TradePrice(price) => (price.price, price.price, price.price),
                BaseDataEnum::Candle(candle) => (candle.close,candle.high,candle.low),
                BaseDataEnum::Tick(tick) => (tick.price,tick.price,tick.price),
                BaseDataEnum::QuoteBar(bar) => {
                    match self.side {
                        PositionSide::Long => (bar.ask_close, bar.ask_high, bar.ask_low),
                        PositionSide::Short => (bar.bid_close, bar.bid_high, bar.bid_low),
                    }
                }
                BaseDataEnum::Quote(quote) => {
                    match self.side {
                        PositionSide::Long => (quote.ask, quote.ask, quote.ask),
                        PositionSide::Short => (quote.bid, quote.bid, quote.bid)
                    }
                }
                BaseDataEnum::Fundamental(_) => panic!("Fundamentals shouldnt be here")
            };
            self.highest_recoded_price = self.highest_recoded_price.max(highest_price);
            self.lowest_recoded_price = self.lowest_recoded_price.min(lowest_price);
            self.open_pnl = calculate_pnl(&self.side, self.average_price, market_price, self.symbol_info.tick_size, self.symbol_info.value_per_tick, self.quantity_open, &self.symbol_info.pnl_currency, &self.account_currency, time);

            let mut bracket_triggered = false;
            if let Some(brackets) = &self.brackets {
                'bracket_loop: for bracket in brackets {
                    match bracket {
                        ProtectiveOrder::TakeProfit { price } => match self.side {
                            PositionSide::Long => {
                                if highest_price >= *price {
                                    bracket_triggered = true;
                                    break 'bracket_loop;
                                }
                            }
                            PositionSide::Short => {
                                if lowest_price <= *price {
                                    bracket_triggered = true;
                                    break 'bracket_loop;
                                }
                            }
                        },
                        ProtectiveOrder::StopLoss { price } => match self.side {
                            PositionSide::Long => {
                                if lowest_price <= *price {
                                    bracket_triggered = true;
                                    break 'bracket_loop;
                                }
                            }
                            PositionSide::Short => {
                                if highest_price >= *price {
                                    bracket_triggered = true;
                                    break 'bracket_loop;
                                }
                            }
                        },
                        ProtectiveOrder::TrailingStopLoss { mut price, trail_value } => match self.side {
                            PositionSide::Long => {
                                if lowest_price <= price {
                                    bracket_triggered = true;
                                    break 'bracket_loop;
                                }
                                if highest_price > price + trail_value {
                                    price += trail_value;
                                }
                            }
                            PositionSide::Short => {
                                if highest_price >= price {
                                    bracket_triggered = true;
                                    break 'bracket_loop;
                                }
                                if lowest_price < price + trail_value {
                                    price -= trail_value;
                                }
                            }
                        }
                    }
                }
            }
            if bracket_triggered {
                let new_pnl =  self.open_pnl;
                self.booked_pnl += new_pnl;
                self.open_pnl = dec!(0.0);
                self.is_closed = true;
                self.quantity_closed += self.quantity_open;
                self.quantity_open = dec!(0.0);
                return new_pnl;
            }
            dec!(0.0)
        }
    }
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct AccountInfo {
    pub account_id: AccountId,
    pub brokerage: Brokerage,
    pub cash_value: Price,
    pub cash_available: Price,
    pub currency: AccountCurrency,
    pub cash_used: Price,
    pub positions: Vec<Position>,
    pub positions_closed: Vec<Position>,
    pub is_hedging: bool,
}
impl Bytes<Self> for AccountInfo {
    fn from_bytes(archived: &[u8]) -> Result<AccountInfo, FundForgeError> {
        // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<AccountInfo>(archived) {
            //Ignore this warning: Trait `Deserialize<ResponseType, SharedDeserializeMap>` is not implemented for `ArchivedRequestType` [E0277]
            Ok(response) => Ok(response),
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(e.to_string())),
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let vec = rkyv::to_bytes::<_, 256>(self).unwrap();
        vec.into()
    }
}

#[derive(Serialize)]
struct PositionExport {
    symbol_name: String,
    side: String,
    quantity: Volume,
    average_price: Price,
    exit_price: Price,
    pnl_by_quantity: Price,
    booked_pnl: Price,
    highest_recoded_price: Price,
    lowest_recoded_price: Price,
}

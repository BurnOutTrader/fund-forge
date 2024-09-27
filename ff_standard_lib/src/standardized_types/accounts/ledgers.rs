use std::fs::create_dir_all;
use std::path::Path;
use chrono::{DateTime, Utc};
use crate::standardized_types::enums::PositionSide;
use crate::standardized_types::subscriptions::SymbolName;
use crate::traits::bytes::Bytes;
use dashmap::DashMap;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use csv::Writer;
use rust_decimal_macros::dec;
use crate::apis::brokerage::broker_enum::Brokerage;
use crate::standardized_types::{Price, Volume};
use crate::standardized_types::accounts::position::Position;
use crate::standardized_types::data_server_messaging::FundForgeError;
use crate::standardized_types::symbol_info::SymbolInfo;
use serde_derive::{Deserialize, Serialize};
pub type AccountId = String;
pub type AccountName = String;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Serialize, Deserialize, PartialOrd,)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum Currency {
    AUD,
    USD,
    BTC,
}

impl Currency {
    pub fn from_str(string: &str) -> Self {
        match string {
            "AUD" => Currency::AUD,
            "USD" => Currency::USD,
            "BTC" => Currency::BTC,
            _ => panic!("No currency matching string, please implement")
        }
    }
}

pub(crate) fn calculate_pnl(
    side: &PositionSide,
    entry_price: Price,
    market_price: Price,
    tick_size: Price,
    value_per_tick: Price,
    quantity: Volume,
    _base_currency: &Currency,
    _account_currency: &Currency,
    _time: &DateTime<Utc>,
) -> Price {
    // Calculate the price difference based on position side
    let raw_ticks = match side {
        PositionSide::Long => ((market_price - entry_price) / tick_size).round(),   // Profit if market price > entry price
        PositionSide::Short => ((entry_price - market_price) / tick_size).round(), // Profit if entry price > market price
    };

    // Calculate PnL by multiplying with value per tick and quantity
    let pnl = raw_ticks * value_per_tick * quantity;

    pnl
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
    use crate::apis::brokerage::broker_enum::Brokerage;
    use crate::standardized_types::accounts::ledgers::{AccountId, Currency, Ledger};
    use crate::standardized_types::data_server_messaging::FundForgeError;
    use crate::standardized_types::enums::{OrderSide, PositionSide};
    use crate::standardized_types::orders::orders::ProtectiveOrder;
    use crate::standardized_types::{Price, Volume};
    use crate::standardized_types::accounts::position::{Position, PositionId};
    use crate::standardized_types::base_data::traits::BaseData;
    use crate::standardized_types::subscriptions::SymbolName;
    use crate::standardized_types::time_slices::TimeSlice;

    impl Ledger {
        pub fn update_brackets(&self, symbol_name: &SymbolName, brackets: Vec<ProtectiveOrder>) {
            if let Some(mut position) = self.positions.get_mut(symbol_name) {
                position.brackets = Some(brackets);
            }
        }

        pub async fn subtract_margin_used(&mut self, symbol_name: SymbolName, quantity: Volume) {
            let margin = self.brokerage.margin_required_historical(symbol_name, quantity).await.unwrap();
            self.cash_available += margin;
            self.cash_used -= margin;
        }

        pub async fn add_margin_used(&mut self, symbol_name: SymbolName, quantity: Volume) -> Result<(), FundForgeError> {
            let margin = self.brokerage.margin_required_historical(symbol_name, quantity).await?;
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
                    } else if position.booked_pnl < dec!(0.0) {
                        loss_pnl += position.booked_pnl;
                        losses += 1;
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
            currency: Currency,
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
                    self.account_id.clone(),
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

                self.cash_value = self.cash_available + self.cash_used + self.booked_pnl + self.open_pnl;
                Ok(())
            }
        }

        pub async fn exit_position_paper(&mut self, symbol_name: &SymbolName, market_price: Price, time: DateTime<Utc>) {
            if !self.positions.contains_key(symbol_name) {
                return;
            }
            let (_, mut old_position) = self.positions.remove(symbol_name).unwrap();

            let booked_pnl = old_position.reduce_position_size(market_price, old_position.quantity_open, &time);
            self.booked_pnl += booked_pnl;

            old_position.open_pnl = dec!(0.0);
            self.cash_value = self.cash_available + self.cash_used + self.booked_pnl + self.open_pnl;
            self.process_closed_position(old_position);
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

        pub async fn on_historical_data_update(&mut self, time_slice: TimeSlice, time: &DateTime<Utc>) {
            let mut open_pnl = dec!(0.0);
            let mut brackets_booked_pnl = dec!(0.0);
            for base_data_enum in time_slice {
                let data_symbol_name = &base_data_enum.symbol().name;
                if let Some(mut position) = self.positions.get_mut(data_symbol_name) {
                    if position.is_closed {
                        continue
                    }

                    //returns booked pnl if exit on brackets
                    brackets_booked_pnl += position.backtest_update_base_data(&base_data_enum, time);
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
            self.booked_pnl += brackets_booked_pnl;
            self.cash_value = self.cash_used + self.cash_available + self.booked_pnl + self.open_pnl;
        }
    }
}

pub enum RiskLimitStatus {
    Ok,
    Hit
}
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct AccountInfo {
    pub account_id: AccountId,
    pub brokerage: Brokerage,
    pub cash_value: Price,
    pub cash_available: Price,
    pub currency: Currency,
    pub cash_used: Price,
    pub positions: Vec<Position>,
    pub is_hedging: bool,
    pub buy_limit: Option<u64>,
    pub sell_limit: Option<u64>,
    pub max_orders: Option<u64>,
    pub daily_max_loss: Option<u64>,
    pub daily_max_loss_reset_time: Option<String>
}

impl Bytes<Self> for AccountInfo {
    fn from_bytes(archived: &[u8]) -> Result<AccountInfo, FundForgeError> {
        // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<AccountInfo>(archived) {
            //Ignore this warning: Trait `Deserialize<ResponseType, SharedDeserializeMap>` is not implemented for `AccountInfoType` [E0277]
            Ok(response) => Ok(response),
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(e.to_string())),
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let vec = rkyv::to_bytes::<_, 256>(self).unwrap();
        vec.into()
    }
}
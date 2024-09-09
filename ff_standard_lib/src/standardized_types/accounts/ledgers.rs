use chrono::{DateTime, Utc};
use crate::apis::brokerage::client_requests::ClientSideBrokerage;
use crate::apis::brokerage::Brokerage;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::data_server_messaging::FundForgeError;
use crate::standardized_types::enums::{OrderSide, PositionSide};
use crate::standardized_types::subscriptions::{SymbolName};
use crate::standardized_types::time_slices::TimeSlice;
use crate::traits::bytes::Bytes;
use dashmap::DashMap;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use crate::helpers::decimal_calculators::round_to_decimals;
use crate::standardized_types::Price;

// B
pub type AccountId = String;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
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
    pub positions: DashMap<SymbolName, Position>,
    pub positions_closed: DashMap<SymbolName, Vec<Position>>,
    pub positions_counter: DashMap<SymbolName, u64>,
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
        };
        ledger
    }

    pub fn print(&self) -> String {
        let mut total_trades = 0;
        let mut wins = 0;
        let mut win_pnl = 0.0;
        let mut loss_pnl = 0.0;
        let mut pnl = 0.0;
        for trades in self.positions_closed.iter() {
            total_trades += trades.value().len();
            for position in trades.value() {
                if position.booked_pnl > 0.0 {
                    wins += 1;
                    win_pnl += position.booked_pnl;
                } else {
                    loss_pnl += position.booked_pnl;
                }
                pnl += position.booked_pnl;
            }
        }

        let (risk_reward, win_rate) = match total_trades > 0 && wins > 0 {
            true => (round_to_decimals(win_pnl / loss_pnl, 2), round_to_decimals((wins as f64 / total_trades as f64) * 100.0, 2)),
            false => (0.0, 0.0)
        };
        format!("Brokerage: {}, Account: {}, Win Rate: {}%, Risk Reward: {}, Total profit: {}, Total Wins: {}, Total Trades: {}", self.brokerage, self.account_id, win_rate, risk_reward, pnl, wins, total_trades)
    }

    pub fn paper_account_init(
        account_id: AccountId,
        brokerage: Brokerage,
        cash_value: f64,
        currency: AccountCurrency,
    ) -> Self {
        let account = Self {
            account_id,
            brokerage,
            cash_value,
            cash_available: cash_value,
            currency,
            cash_used: 0.0,
            positions: DashMap::new(),
            positions_closed: DashMap::new(),
            positions_counter: DashMap::new(),
        };
        account
    }

    pub async fn generate_id(&self, symbol_name: &SymbolName, time: DateTime<Utc>, side: PositionSide) -> PositionId {
        self.positions_counter.entry(symbol_name.clone())
            .and_modify(|value| *value += 1)  // Increment the value if the key exists
            .or_insert(1);

        format!("{}-{}-{}-{}-{}-{}", self.brokerage, self.account_id, symbol_name, time.timestamp(), self.positions_counter.get(symbol_name).unwrap().value().clone(), side)
    }

    pub async fn update_or_create_paper_position(
        &mut self,
        symbol_name: &SymbolName,
        quantity: u64,
        price: Price,
        side: OrderSide,
        time: DateTime<Utc>
    ) -> Result<(), FundForgeError>
    {
        // Check if there's an existing position for the given symbol
        if let Some(mut existing_position) = self.positions.get_mut(symbol_name) {
            let existing_quantity = existing_position.quantity;
            let existing_price = existing_position.average_price;

            match existing_position.side {
                PositionSide::Long => {
                    match side {
                        OrderSide::Buy => {
                            // Calculate the margin required for adding more to the position
                            let margin_required = self.brokerage.margin_required(symbol_name.clone(), quantity).await;
                            // Check if the available cash is sufficient to cover the margin
                            if self.cash_available < margin_required {
                                return Err(FundForgeError::ClientSideErrorDebug("Insufficient funds".to_string()));
                            }
                            // If adding to the position, calculate the new average price
                            existing_position.average_price =
                                ((existing_quantity as f64 * existing_price) + (quantity as f64 * price))
                                    / (existing_quantity + quantity) as f64;
                            // Update the total quantity
                            existing_position.quantity += quantity;
                            // Calculate the new open PnL based on the current price
                            existing_position.open_pnl = (price - existing_position.average_price)
                                * existing_position.quantity as f64;

                            // Deduct margin from cash available
                            self.cash_available -= margin_required;
                            self.cash_used += margin_required;

                            Ok(())
                        }
                        OrderSide::Sell => {
                            // Calculate the booked PnL from open PnL for the reduced position
                            let quantity_sold = quantity.min(existing_quantity);
                            let booked_pnl = (price - existing_price) * quantity_sold as f64;
                            existing_position.booked_pnl += booked_pnl;
                            // Update the position by reducing the quantity
                            existing_position.quantity -= quantity_sold;

                            // Update PnL: if the entire position is closed, set PnL to 0
                            if existing_position.quantity == 0 {
                                existing_position.open_pnl = 0.0;
                                existing_position.average_price = 0.0;
                            } else {
                                // Calculate remaining open PnL based on remaining quantity
                                existing_position.open_pnl = (price - existing_position.average_price)
                                    * existing_position.quantity as f64;
                            }

                            // Add booked PnL to account cash and adjust cash used
                            self.cash_available += booked_pnl;
                            self.cash_used -= booked_pnl;

                            Ok(())
                        }
                    }
                }
                PositionSide::Short => {
                    match side {
                        OrderSide::Buy => {
                            // Calculate the booked PnL from open PnL for the reduced position
                            let quantity_bought = quantity.min(existing_quantity);
                            let booked_pnl = (existing_price - price) * quantity_bought as f64;
                            existing_position.booked_pnl += booked_pnl;
                            // Update the position by reducing the quantity
                            existing_position.quantity -= quantity_bought;

                            // Update PnL: if the entire position is closed, set PnL to 0
                            if existing_position.quantity == 0 {
                                existing_position.open_pnl = 0.0;
                                existing_position.average_price = 0.0;
                            } else {
                                // Calculate remaining open PnL based on remaining quantity
                                existing_position.open_pnl = (existing_position.average_price - price)
                                    * existing_position.quantity as f64;
                            }

                            // Add booked PnL to account cash and adjust cash used
                            self.cash_available += booked_pnl;
                            self.cash_used -= booked_pnl;

                            Ok(())
                        }
                        OrderSide::Sell => {
                            // Calculate the margin required for adding more to the short position
                            let margin_required = self.brokerage.margin_required(symbol_name.clone(), quantity).await;
                            // Check if the available cash is sufficient to cover the margin
                            if self.cash_available < margin_required {
                                return Err(FundForgeError::ClientSideErrorDebug("Insufficient funds".to_string()));
                            }

                            // If adding to the short position, calculate the new average price
                            existing_position.average_price =
                                ((existing_quantity as f64 * existing_price) + (quantity as f64 * price))
                                    / (existing_quantity + quantity) as f64;
                            // Update the total quantity
                            existing_position.quantity += quantity;
                            // Calculate the new open PnL based on the current price
                            existing_position.open_pnl = (existing_position.average_price - price)
                                * existing_position.quantity as f64;

                            // Deduct margin from cash available
                            self.cash_available -= margin_required;
                            self.cash_used += margin_required;

                            Ok(())
                        }
                    }
                }
            }
        } else {
            // No existing position, create a new one
            let margin_required = self.brokerage.margin_required(symbol_name.clone(), quantity).await;
            // Check if the available cash is sufficient to cover the margin
            if self.cash_available < margin_required {
                return Err(FundForgeError::ClientSideErrorDebug("Insufficient funds".to_string()));
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
                price,
                self.generate_id(&symbol_name, time, side).await
            );

            // Insert the new position into the positions map
            self.positions.insert(symbol_name.clone(), position);

            // Adjust the account cash to reflect the new position's margin requirement
            self.cash_available -= margin_required;
            self.cash_used += margin_required;

            Ok(())
        }
    }

    pub async fn exit_position_paper(&mut self, symbol_name: &SymbolName) {
        if !self.positions.contains_key(symbol_name) {
            return;
        }
        let (_, mut old_position) = self.positions.remove(symbol_name).unwrap();
        let margin_freed = self
            .brokerage
            .margin_required(symbol_name.clone(), old_position.quantity)
            .await;

        old_position.booked_pnl += old_position.open_pnl;
        self.cash_available += margin_freed + old_position.booked_pnl;
        self.cash_used -= margin_freed + old_position.booked_pnl;
        self.cash_value +=  old_position.booked_pnl;
        old_position.is_closed = true;

        match old_position.booked_pnl > 0.0 {
            true => {}
            false => {}
        }
        if !self.positions_closed.contains_key(&old_position.symbol_name) {
            self.positions_closed.insert(old_position.symbol_name.clone(), vec![old_position]);
        } else {
            self.positions_closed.get_mut(&old_position.symbol_name).unwrap().value_mut().push(old_position);
        }
    }

    pub async fn enter_short_paper(
        &mut self,
        symbol_name: &SymbolName,
        quantity: u64,
        price: Price,
        time: DateTime<Utc>
    ) -> Result<(), FundForgeError> {
        if let Some(position) = self.positions.get(symbol_name) {
            if position.value().side == PositionSide::Short {
                // we will need to modify our average price and quantity
            } else if position.value().side == PositionSide::Long {
                let (_, mut old_position) = self.positions.remove(symbol_name).unwrap();
                let margin_freed = self
                    .brokerage
                    .margin_required(symbol_name.clone(), old_position.quantity)
                    .await;

                old_position.booked_pnl += old_position.open_pnl;
                self.cash_available += margin_freed + old_position.booked_pnl;
                self.cash_used -= margin_freed + old_position.booked_pnl;
                self.cash_value +=  old_position.booked_pnl;
                old_position.is_closed = true;
                if !self.positions_closed.contains_key(&old_position.symbol_name) {
                    self.positions_closed.insert(old_position.symbol_name.clone(), vec![old_position]);
                } else {
                    self.positions_closed.get_mut(&old_position.symbol_name).unwrap().value_mut().push(old_position);
                }
            }
        }
        let margin_required = self.brokerage.margin_required(symbol_name.clone(), quantity).await;
        if self.cash_available < margin_required {
            return Err(FundForgeError::ClientSideErrorDebug(
                "Insufficient funds".to_string(),
            ));
        }

        let position = Position::enter(
            symbol_name.clone(),
            self.brokerage.clone(),
            PositionSide::Long,
            quantity,
            price,
            self.generate_id(&symbol_name, time, PositionSide::Short).await
        );
        self.positions.insert(symbol_name.clone(), position);
        self.cash_available -= margin_required;
        self.cash_used += margin_required;
        Ok(())
    }

    pub async fn enter_long_paper(
        &mut self,
        symbol_name: &SymbolName,
        quantity: u64,
        price: Price,
        time: DateTime<Utc>
    ) -> Result<(), FundForgeError> {
        if let Some(position) = self.positions.get(symbol_name) {
            if position.value().side == PositionSide::Long {
                // we will need to modify our average price and quantity
            } else if position.value().side == PositionSide::Short {
                let (_, mut old_position) = self.positions.remove(symbol_name).unwrap();
                let margin_freed = self
                    .brokerage
                    .margin_required(symbol_name.clone(), old_position.quantity)
                    .await;
                old_position.booked_pnl += old_position.open_pnl;
                self.cash_available += margin_freed + old_position.booked_pnl;
                self.cash_used -= margin_freed + old_position.booked_pnl;
                self.cash_value +=  old_position.booked_pnl;
                old_position.is_closed = true;
                if !self.positions_closed.contains_key(&old_position.symbol_name) {
                    self.positions_closed.insert(old_position.symbol_name.clone(), vec![old_position]);
                } else {
                    self.positions_closed.get_mut(&old_position.symbol_name).unwrap().value_mut().push(old_position);
                }
            }
        }
        let margin_required = self.brokerage.margin_required(symbol_name.clone(), quantity).await;
        if self.cash_available < margin_required {
            return Err(FundForgeError::ClientSideErrorDebug(
                "Insufficient funds".to_string(),
            ));
        }
        let position: Position = Position::enter(
            symbol_name.clone(),
            self.brokerage.clone(),
            PositionSide::Long,
            quantity,
            price,
            self.generate_id(&symbol_name, time, PositionSide::Long).await
        );
        self.positions.insert(symbol_name.clone(), position);
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

    pub async fn is_long(&self, symbol_name: &SymbolName) -> bool {
        if let Some(position) = self.positions.get(symbol_name) {
            if position.side == PositionSide::Long {
                return true;
            }
        }
        false
    }

    pub async fn is_short(&self, symbol_name: &SymbolName) -> bool {
        if let Some(position) = self.positions.get(symbol_name) {
            if position.side == PositionSide::Short {
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

    pub async fn on_data_update(&self, time_slice: TimeSlice) {
        for mut position in self.positions.iter_mut() {
            position.value_mut().update_base_data(&time_slice);
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
    pub quantity: u64,
    pub average_price: Price,
    pub open_pnl: f64,
    pub booked_pnl: f64,
    pub highest_recoded_price: Price,
    pub lowest_recoded_price: Price,
    pub is_closed: bool,
    pub id: PositionId
}

impl Position {
    pub fn enter(
        symbol_name: SymbolName,
        brokerage: Brokerage,
        side: PositionSide,
        quantity: u64,
        average_price: f64,
        id: PositionId
    ) -> Self {
        Self {
            symbol_name,
            brokerage,
            side,
            quantity,
            average_price,
            open_pnl: 0.0,
            booked_pnl: 0.0,
            highest_recoded_price: average_price,
            lowest_recoded_price: average_price,
            is_closed: false,
            id,
        }
    }

    pub fn update_base_data(&mut self, time_slice: &TimeSlice) {
        if self.is_closed {
            return;
        }
        for base_data in time_slice {
            if self.symbol_name != base_data.symbol().name {
                continue;
            }
            match base_data {
                BaseDataEnum::Price(price) => {
                    self.highest_recoded_price = self.highest_recoded_price.max(price.price);
                    self.lowest_recoded_price = self.lowest_recoded_price.min(price.price);
                    match self.side {
                        PositionSide::Long => {
                            // For a long position, profit is gained when current price is above the average price
                            self.open_pnl = round_to_decimals((price.price - self.average_price) * self.quantity as f64, 2);
                        }
                        PositionSide::Short => {
                            // For a short position, profit is gained when current price is below the average price
                            self.open_pnl = round_to_decimals((self.average_price - price.price) * self.quantity as f64, 2);
                        }
                    }
                }
                BaseDataEnum::Candle(candle) => {
                    self.highest_recoded_price = self.highest_recoded_price.max(candle.high);
                    self.lowest_recoded_price = self.lowest_recoded_price.min(candle.low);
                    match self.side {
                        PositionSide::Long => {
                            // For a long position, profit is gained when current price is above the average price
                            self.open_pnl = round_to_decimals((candle.close - self.average_price) * self.quantity as f64,2);
                        }
                        PositionSide::Short => {
                            // For a short position, profit is gained when current price is below the average price
                            self.open_pnl = round_to_decimals((self.average_price - candle.close) * self.quantity as f64, 2);
                        }
                    }
                }
                BaseDataEnum::QuoteBar(bar) => {
                    match self.side {
                        PositionSide::Long => {
                            self.highest_recoded_price = self.highest_recoded_price.max(bar.ask_high);
                            self.lowest_recoded_price = self.lowest_recoded_price.min(bar.ask_low);
                            self.open_pnl = round_to_decimals((bar.ask_close - self.average_price) * self.quantity as f64, 2);
                        }
                        PositionSide::Short => {
                            self.highest_recoded_price = self.highest_recoded_price.max(bar.bid_high);
                            self.lowest_recoded_price = self.lowest_recoded_price.min(bar.bid_low);
                            self.open_pnl = round_to_decimals((self.average_price - bar.bid_close) * self.quantity as f64, 2);
                        }
                    }
                }
                BaseDataEnum::Tick(tick) => {
                    self.highest_recoded_price = self.highest_recoded_price.max(tick.price);
                    self.lowest_recoded_price = self.lowest_recoded_price.min(tick.price);

                    match self.side {
                        PositionSide::Long => {
                            // For a long position, profit is gained when current price is above the average price
                            self.open_pnl = round_to_decimals((tick.price - self.average_price) * self.quantity as f64,2);
                        }
                        PositionSide::Short => {
                            // For a short position, profit is gained when current price is below the average price
                            self.open_pnl = round_to_decimals((self.average_price - tick.price) * self.quantity as f64, 2);
                        }
                    }
                }
                BaseDataEnum::Quote(quote) => {
                    match self.side {
                        PositionSide::Long => {
                            self.highest_recoded_price = self.highest_recoded_price.max(quote.ask);
                            self.lowest_recoded_price = self.lowest_recoded_price.min(quote.ask);
                            self.open_pnl = round_to_decimals((quote.ask - self.average_price) * self.quantity as f64, 2);
                        }
                        PositionSide::Short => {
                            self.highest_recoded_price = self.highest_recoded_price.max(quote.bid);
                            self.lowest_recoded_price = self.lowest_recoded_price.min(quote.bid);
                            self.open_pnl = round_to_decimals((self.average_price - quote.bid) * self.quantity as f64, 2);
                        }
                    }
                }
                BaseDataEnum::Fundamental(_) => (),
            }
        }
    }
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug)]
#[archive(compare(PartialEq), check_bytes)]
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

use crate::strategies::ledgers::ledger::Ledger;
use chrono::{DateTime, Utc};
use rust_decimal_macros::dec;
use crate::messages::data_server_messaging::FundForgeError;
use crate::standardized_types::accounts::Currency;
use crate::standardized_types::enums::{OrderSide, PositionSide, StrategyMode};
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::orders::{OrderId, OrderUpdateEvent};
use crate::standardized_types::position::{Position, PositionUpdateEvent};
use crate::standardized_types::subscriptions::{SymbolCode, SymbolName};
use crate::strategies::client_features::other_requests::get_exchange_rate;

impl Ledger {
    pub(crate) async fn release_margin_used(&self, symbol_name: &SymbolName) {
        // First get the margin amount without removing it
        if let Some((_,margin_used)) = self.margin_used.remove(symbol_name) {
            let margin_amount = margin_used;

            // Update cash values first
            {
                let mut account_cash_used = self.cash_used.lock().await;
                *account_cash_used -= margin_amount;
            }
            {
                let mut account_cash_available = self.cash_available.lock().await;
                *account_cash_available += margin_amount;
            }
        }
    }

    pub(crate) async fn commit_margin(&self, symbol_name: &SymbolName, quantity: Volume, market_price: Price, time: DateTime<Utc>, side: OrderSide, base_currency: Option<Currency>, position_currency: Currency) -> Result<(), FundForgeError> {
        let rate = if position_currency == self.currency {
            dec!(1)
        } else {
            // We want rate from position_currency to account_currency
            match self.rates.get(&position_currency) {
                Some(rate) => rate.value().clone(),
                None => {
                    // Get position_currency to account_currency rate
                    let rate = get_exchange_rate(position_currency, self.currency, time, side).await.unwrap_or_else(|_e| dec!(1));
                    self.rates.insert(position_currency, rate);
                    rate
                }
            }
        };

        eprintln!("Account Currency: {}, Position Currency: {}, Base Currency: {:?}, Rate: {}",
                  self.currency, position_currency, base_currency, rate);

        // Now in intraday_margin_required:
        // If account_currency == base_currency (like USD account trading USD/JPY):
        //   - quantity is already in account currency
        // Else:
        //   - value (quantity * price) is in position_currency
        //   - multiply by rate to convert to account currency
        let margin = self.account.brokerage.intraday_margin_required(
            symbol_name, quantity, market_price,
            self.currency, base_currency, rate
        ).await?
            .unwrap_or_else(|| quantity * market_price * rate);

        // Check available cash first
        {
            let cash_available = self.cash_available.lock().await;
            if *cash_available < margin {
                return Err(FundForgeError::ClientSideErrorDebug(format!(
                    "Insufficient funds: Required {}, Available {}",
                    margin,
                    *cash_available
                )));
            }
        }

        // Update margin tracking before updating cash
        self.margin_used
            .entry(symbol_name.clone())
            .and_modify(|existing| *existing += margin)
            .or_insert(margin);

        // Update cash values
        {
            let mut account_cash_used = self.cash_used.lock().await;
            *account_cash_used += margin;
        }

        {
            let mut account_cash_available = self.cash_available.lock().await;
            *account_cash_available -= margin;
        }

        Ok(())
    }

    pub(crate) async fn paper_exit_position(
        &self,
        symbol_name: &SymbolName,
        time: DateTime<Utc>,
        market_price: Price,
        tag: String
    ) -> Option<PositionUpdateEvent> {
        if let Some((symbol_name, mut existing_position)) = self.positions.remove(symbol_name) {
            // Mark the position as closed
            existing_position.is_closed = true;
            {
                self.release_margin_used(&symbol_name).await;
            }
            let exchange_rate = if self.currency != existing_position.symbol_info.pnl_currency {
                let side = match existing_position.side {
                    PositionSide::Long => OrderSide::Buy,
                    PositionSide::Short => OrderSide::Sell
                };
                match get_exchange_rate(self.currency, existing_position.symbol_info.pnl_currency, time, side).await {
                    Ok(rate) => {
                        self.rates.insert(existing_position.symbol_info.pnl_currency, rate);
                        rate
                    },
                    Err(_e) => self.get_exchange_multiplier(existing_position.symbol_info.pnl_currency)
                }
            } else {
                dec!(1.0)
            };
            let event = existing_position.reduce_position_size(market_price, existing_position.quantity_open, self.currency, exchange_rate, time, tag).await;
            let mut cash_available = self.cash_available.lock().await;
            let mut total_booked_pnl = self.total_booked_pnl.lock().await;
            match &event {
                PositionUpdateEvent::PositionClosed { booked_pnl, .. } => {
                    // TODO[Strategy]: Add option to mirror account position or use internal position curating.
                    self.symbol_closed_pnl
                        .entry(symbol_name.clone())
                        .and_modify(|pnl| *pnl += booked_pnl)
                        .or_insert(booked_pnl.clone());
                    *total_booked_pnl += booked_pnl;

                    *cash_available += booked_pnl;
                }
                _ => panic!("this shouldn't happen")
            }

            let mut cash_value = self.cash_value.lock().await;
            let cash_used = self.cash_used.lock().await;
            *cash_value = *cash_used + *cash_available;

            // Add the closed position to the positions_closed DashMap
            self.positions_closed
                .entry(symbol_name.clone())                  // Access the entry for the symbol name
                .or_insert_with(Vec::new)                    // If no entry exists, create a new Vec
                .push(existing_position);     // Push the closed position to the Vec

            return Some(event)
        }
        None
    }

    /// If Ok it will return a Position event for the successful position update, if the ledger rejects the order it will return an Err(OrderEvent)
    ///todo, check ledger max order etc before placing orders
    pub(crate) async fn update_or_create_paper_position(
        &self,
        symbol_name: SymbolName,
        symbol_code: SymbolCode,
        order_id: OrderId,
        quantity: Volume,
        side: OrderSide,
        time: DateTime<Utc>,
        market_fill_price: Price, // we use the passed in price because we don't know what sort of order was filled, limit or market
        tag: String
    ) -> Result<Vec<PositionUpdateEvent>, OrderUpdateEvent> {
        if self.mode == StrategyMode::Live {
            panic!("Incorrect mode for update_or_create_position()");
        }
        //todo it is possible I might want to use the last updates time to prevent duplicate updates or out of order updates
        let mut updates = vec![];
        // Check if there's an existing position for the given symbol
        let mut remaining_quantity = quantity;
        if let Some((symbol_name, mut existing_position)) = self.positions.remove(&symbol_code) {
            let is_reducing = (existing_position.side == PositionSide::Long && side == OrderSide::Sell)
                || (existing_position.side == PositionSide::Short && side == OrderSide::Buy);

            if is_reducing {
                remaining_quantity -= existing_position.quantity_open;
                let exchange_rate = if self.currency != existing_position.symbol_info.pnl_currency {
                    match get_exchange_rate(self.currency, existing_position.symbol_info.pnl_currency, time, side).await {
                        Ok(rate) => {
                            self.rates.insert(existing_position.symbol_info.pnl_currency, rate);
                            rate
                        },
                        Err(_e) => self.get_exchange_multiplier(existing_position.symbol_info.pnl_currency)
                    }
                } else {
                    dec!(1.0)
                };
                let event = existing_position.reduce_position_size(market_fill_price, quantity, self.currency,exchange_rate, time, tag.clone()).await;

                self.release_margin_used(&symbol_name).await;

                match &event {
                    PositionUpdateEvent::PositionReduced { booked_pnl, .. } => {
                        self.commit_margin(&symbol_name, existing_position.quantity_open, existing_position.average_price, time, side, existing_position.symbol_info.base_currency, existing_position.symbol_info.pnl_currency).await.unwrap();
                        self.positions.insert(symbol_code.clone(), existing_position);

                        self.symbol_closed_pnl
                            .entry(symbol_code.clone())
                            .and_modify(|pnl| *pnl += booked_pnl)
                            .or_insert(booked_pnl.clone());
                        {
                            let mut total_booked_pnl = self.total_booked_pnl.lock().await;
                            *total_booked_pnl += booked_pnl;

                            let mut cash_available = self.cash_available.lock().await;
                            *cash_available += booked_pnl;
                        }
                        //println!("Reduced Position: {}", symbol_name);
                    }
                    PositionUpdateEvent::PositionClosed { booked_pnl, .. } => {
                        self.symbol_closed_pnl
                            .entry(symbol_code.clone())
                            .and_modify(|pnl| *pnl += booked_pnl)
                            .or_insert(booked_pnl.clone());

                        {
                            let mut total_booked_pnl = self.total_booked_pnl.lock().await;
                            *total_booked_pnl += booked_pnl;

                            let mut cash_available = self.cash_available.lock().await;
                            *cash_available += booked_pnl;
                        }

                        if !self.positions_closed.contains_key(&symbol_code) {
                            self.positions_closed.insert(symbol_code.clone(), vec![]);
                        }
                        if let Some(mut positions_closed) = self.positions_closed.get_mut(&symbol_code) {
                            positions_closed.value_mut().push(existing_position);
                        }
                        //println!("Closed Position: {}", symbol_name);
                    }
                    _ => panic!("This shouldn't happen")
                }

                {
                    let mut cash_value = self.cash_value.lock().await;
                    let cash_used = self.cash_used.lock().await;
                    let cash_available = self.cash_available.lock().await;
                    *cash_value = *cash_used + *cash_available;
                }

                updates.push(event);
            } else {
                match self.commit_margin(&symbol_name, quantity, market_fill_price, time, side, existing_position.symbol_info.base_currency, existing_position.symbol_info.pnl_currency).await {
                    Ok(_) => {}
                    Err(e) => {
                        //todo this now gets added directly to buffer
                        let event = OrderUpdateEvent::OrderRejected {
                            account: self.account.clone(),
                            symbol_name: symbol_name.clone(),
                            symbol_code: symbol_code.clone(),
                            order_id,
                            reason: e.to_string(),
                            tag,
                            time: time.to_string()
                        };
                        return Err(event)
                    }
                }
                let event = existing_position.add_to_position(self.mode, self.is_simulating_pnl, self.currency, market_fill_price, quantity, time, tag.clone()).await;
                self.positions.insert(symbol_code.clone(), existing_position);

                {
                    let mut cash_value = self.cash_value.lock().await;
                    let cash_used = self.cash_used.lock().await;
                    let cash_available = self.cash_available.lock().await;
                    *cash_value = *cash_used + *cash_available;
                }

                updates.push(event);
                remaining_quantity = dec!(0.0);
            }
        }
        if remaining_quantity > dec!(0.0) {
            let info = self.symbol_info(self.account.brokerage, &symbol_name).await;
            match self.commit_margin(&symbol_name, quantity, market_fill_price, time, side, info.base_currency, info.pnl_currency).await {
                Ok(_) => {}
                Err(e) => {
                    let event = OrderUpdateEvent::OrderRejected {
                        account: self.account.clone(),
                        symbol_name: symbol_name.clone(),
                        symbol_code: symbol_code.clone(),
                        order_id,
                        reason: e.to_string(),
                        tag,
                        time: time.to_string()
                    };
                    return Err(event)
                }
            }

            // Determine the side of the position based on the order side
            let position_side = match side {
                OrderSide::Buy => PositionSide::Long,
                OrderSide::Sell => PositionSide::Short,
            };


            let exchange_rate = if self.currency != info.pnl_currency {
                match get_exchange_rate(self.currency, info.pnl_currency, time, side).await {
                    Ok(rate) => {
                        self.rates.insert(info.pnl_currency, rate);
                        rate
                    },
                    Err(_e) => self.get_exchange_multiplier(info.pnl_currency)
                }
            } else {
                dec!(1.0)
            };

            if symbol_name != symbol_code && !self.symbol_code_map.contains_key(&symbol_name) {
                self.symbol_code_map.insert(symbol_name.clone(), vec![]);
            };
            if symbol_name != symbol_code {
                if let Some(mut code_map) = self.symbol_code_map.get_mut(&symbol_name) {
                    if !code_map.contains(&symbol_code) {
                        code_map.value_mut().push(symbol_code.clone());
                    }
                }
            }

            let id = self.generate_id(position_side);
            // Create a new position
            let position = Position::new(
                symbol_code.clone(),
                symbol_code.clone(),
                self.account.clone(),
                position_side.clone(),
                remaining_quantity,
                market_fill_price,
                id.clone(),
                info.clone(),
                exchange_rate,
                tag.clone(),
                time,
            );

            // Insert the new position into the positions map
            eprintln!("Symbol Code {}", symbol_code);
            self.positions.insert(symbol_code.clone(), position);
            if !self.positions_closed.contains_key(&symbol_code) {
                self.positions_closed.insert(symbol_code.clone(), vec![]);
            }
            if symbol_name != symbol_code {
                self.symbol_code_map.entry(symbol_name.clone()).or_insert(vec![]).push(symbol_code.clone());
            }

            let event = PositionUpdateEvent::PositionOpened {
                side: position_side,
                symbol_name: symbol_name.clone(),
                symbol_code: symbol_code.clone(),
                position_id: id,
                account: self.account.clone(),
                originating_order_tag: tag,
                time: time.to_string()
            };

            {
                let mut cash_value = self.cash_value.lock().await;
                let cash_used = self.cash_used.lock().await;
                let cash_available = self.cash_available.lock().await;
                *cash_value = *cash_used + *cash_available;
            }

            //println!("{:?}", event);
            updates.push(event);
        }
        Ok(updates)
    }
}
use crate::strategies::ledgers::ledger::Ledger;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::messages::data_server_messaging::FundForgeError;
use crate::product_maps::rithmic::maps::get_futures_commissions_info;
use crate::standardized_types::accounts::{Currency};
use crate::standardized_types::enums::{OrderSide, PositionSide, StrategyMode};
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::orders::{OrderId, OrderUpdateEvent};
use crate::standardized_types::position::{Position, PositionUpdateEvent};
use crate::standardized_types::subscriptions::{SymbolCode, SymbolName};
use crate::strategies::client_features::other_requests::get_exchange_rate;
use crate::strategies::handlers::market_handler::price_service::price_service_request_market_fill_price;
use crate::strategies::strategy_events::StrategyEvent;

impl Ledger {
    pub(crate) async fn charge_commission(&mut self, symbol_name: &SymbolName, contracts: Volume, exchange_rate: Decimal) {
        //todo, this fn get_futures_commissions_info() will need to be a more dynamic / generic function to cover all brokerages, we also need to use
        // commission info to get the exchange rate, for example pnl currency will not be exchange rate currency
        if let Ok(commission_info) = get_futures_commissions_info(&symbol_name) {
            let commission = contracts * commission_info.per_side * exchange_rate;
            self.cash_available -= commission;
            self.commissions_paid += commission;
        }
    }

    pub(crate) async fn release_margin_used(&mut self, symbol_code: &SymbolCode) {
        // First get_requests the margin amount without removing it
        if let Some((_,margin_used)) = self.margin_used.remove(symbol_code) {
            //eprintln!("release_margin_used: {}", symbol_code);
            // Update cash values first
            self.cash_used -= margin_used;
            self.cash_available += margin_used;
        }
    }

    pub(crate) async fn commit_margin(&mut self, symbol_name: &SymbolName, symbol_code: &SymbolCode, quantity: Volume, market_price: Price, time: DateTime<Utc>, side: OrderSide, base_currency: Option<Currency>, position_currency: Currency) -> Result<(), FundForgeError> {
        //eprintln!("commit_margin: {}", symbol_code);
        let rate = if position_currency == self.currency {
            dec!(1)
        } else {
            match self.rates.get(&position_currency) {
                Some(rate) => rate.value().clone(),
                None => {
                    let rate = get_exchange_rate(position_currency, self.currency, time, side).await.unwrap_or_else(|_e| dec!(1));
                    self.rates.insert(position_currency, rate);
                    rate
                }
            }
        };

        let margin = self.account.brokerage.intraday_margin_required(symbol_name, quantity, market_price, self.currency, base_currency, position_currency, rate).await?
            .unwrap_or_else(|| quantity * market_price * rate);

        // Check available cash first
        if  self.cash_available < margin {
            return Err(FundForgeError::ClientSideErrorDebug(format!(
                "Insufficient funds: Required {}, Available {}",
                margin,
                self.cash_available
            )));
        }

        // Update margin tracking before updating cash
        // Add to existing margin instead of replacing
        let total_margin = if let Some(existing_margin) = self.margin_used.get(symbol_code) {
            margin + existing_margin.value()
        } else {
            margin
        };
        self.margin_used.insert(symbol_code.clone(), total_margin);
        self.cash_used += margin;
        self.cash_available -= margin;
        //println!("Margin Used: {}", margin_used);

        Ok(())
    }

    pub(crate) async fn paper_exit_position(
        &mut self,
        symbol_code: &SymbolCode,
        time: DateTime<Utc>,
        market_price: Price,
        tag: String
    ) {
        if let Some((symbol_name, mut existing_position)) = self.positions.remove(symbol_code) {
            // Mark the position as closed
            existing_position.is_closed = true;
            self.release_margin_used(&symbol_code).await;
            let exchange_rate = if self.currency != existing_position.symbol_info.pnl_currency {
                let side = match existing_position.side {
                    PositionSide::Long => OrderSide::Buy,
                    PositionSide::Short => OrderSide::Sell,
                    _ => unreachable!("This shouldn't happen")
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
            self.charge_commission(&symbol_name, existing_position.quantity_open, exchange_rate).await;
            let event = existing_position.reduce_position_size(market_price, existing_position.quantity_open, self.currency, exchange_rate, time, tag).await;
            match &event {
                PositionUpdateEvent::PositionClosed { booked_pnl, .. } => {
                    // TODO[Strategy]: Add option to mirror account position or use internal position curating.
                    self.symbol_closed_pnl
                        .entry(symbol_name.clone())
                        .and_modify(|pnl| *pnl += booked_pnl)
                        .or_insert(booked_pnl.clone());
                    self.total_booked_pnl += booked_pnl;

                    self.cash_available += booked_pnl;
                }
                _ => panic!("this shouldn't happen")
            }

            self.cash_value = self.cash_used + self.cash_available;

            // Add the closed position to the positions_closed DashMap
            self.positions_closed
                .entry(symbol_name.clone())                  // Access the entry for the symbol name
                .or_insert_with(Vec::new)                    // If no entry exists, create a new Vec
                .push(existing_position);     // Push the closed position to the Vec

            self.strategy_sender.send(StrategyEvent::PositionEvents(event)).await.unwrap();
        }
    }

    /// If Ok it will return a Position event for the successful position update, if the ledger rejects the order it will return an Err(OrderEvent)
    ///todo, check ledger max order etc before placing orders
    pub(crate) async fn update_or_create_paper_position(
        &mut self,
        symbol_name: SymbolName,
        symbol_code: SymbolCode,
        quantity: Volume,
        side: OrderSide,
        time: DateTime<Utc>,
        market_fill_price: Price, // we use the passed in price because we don't know what sort of order was filled, limit or market
        tag: String,
        order_id: OrderId,
        paper_response_sender: tokio::sync::oneshot::Sender<Option<OrderUpdateEvent>>
    ) {
        if self.mode == StrategyMode::Live {
            panic!("Incorrect mode for update_or_create_position()");
        }
        //eprintln!("quantity: {}, side: {:?}, time: {}, market_fill_price: {}, tag: {}", quantity, side, time, market_fill_price, tag);
        //todo it is possible I might want to use the last updates time to prevent duplicate updates or out of order updates
        let mut position_events = vec![];
        // Check if there's an existing position for the given symbol
        let mut remaining_quantity = quantity;
        if let Some((_, mut existing_position)) = self.positions.remove(&symbol_code) {
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
                self.charge_commission(&symbol_name, existing_position.quantity_open, exchange_rate).await;
                let event = existing_position.reduce_position_size(market_fill_price, quantity, self.currency,exchange_rate, time, tag.clone()).await;

               // eprintln!("symbol_code: {}, existing_position: {:?}", symbol_code, existing_position);

                match &event {
                    PositionUpdateEvent::PositionReduced { booked_pnl, .. } => {
                        self.release_margin_used(&symbol_code).await;
                        self.commit_margin(&symbol_name, &symbol_code, existing_position.quantity_open, existing_position.average_price, time, side, existing_position.symbol_info.base_currency, existing_position.symbol_info.pnl_currency).await.unwrap();
                        self.positions.insert(symbol_code.clone(), existing_position);

                        self.symbol_closed_pnl
                            .entry(symbol_code.clone())
                            .and_modify(|pnl| *pnl += booked_pnl)
                            .or_insert(booked_pnl.clone());

                        self.total_booked_pnl += booked_pnl;

                        self.cash_available += booked_pnl;

                        //println!("Reduced Position: {}", symbol_name);
                    }
                    PositionUpdateEvent::PositionClosed { booked_pnl, .. } => {
                        self.release_margin_used(&symbol_code).await;
                        self.symbol_closed_pnl
                            .entry(symbol_code.clone())
                            .and_modify(|pnl| *pnl += booked_pnl)
                            .or_insert(booked_pnl.clone());

                        self.total_booked_pnl += booked_pnl;
                        self.cash_available += booked_pnl;

                        if !self.positions_closed.contains_key(&symbol_code) {
                            self.positions_closed.insert(symbol_code.clone(), vec![]);
                        }
                        self.positions_closed
                            .entry(symbol_code.clone())
                            .or_insert_with(Vec::new)
                            .push(existing_position);
                        //println!("Closed Position: {}", symbol_name);
                    }
                    _ => panic!("This shouldn't happen")
                }

                self.cash_value = self.cash_used + self.cash_available;

                position_events.push(event);
            } else {
                match self.commit_margin(&symbol_name, &symbol_code, quantity, market_fill_price, time, side, existing_position.symbol_info.base_currency, existing_position.symbol_info.pnl_currency).await {
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
                        paper_response_sender.send(Some(event)).unwrap();
                        return
                    }
                }
                let event = existing_position.add_to_position(self.mode, self.is_simulating_pnl, self.currency, market_fill_price, quantity, time, tag.clone()).await;
                self.positions.insert(symbol_code.clone(), existing_position);

                self.cash_value = self.cash_used + self.cash_available;

                position_events.push(event);
                remaining_quantity = dec!(0.0);
            }
        }
        if remaining_quantity > dec!(0.0) {
            let info = self.symbol_info(self.account.brokerage, &symbol_name).await;
            match self.commit_margin(&symbol_name, &symbol_code, quantity, market_fill_price, time, side, info.base_currency, info.pnl_currency).await {
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
                    paper_response_sender.send(Some(event)).unwrap();
                    return
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
            //eprintln!("symbol_code: {}, exchange_rate: {}, {}, {}", symbol_code, exchange_rate, self.currency, info.pnl_currency);
            //todo, we only need to do this for certain brokerages, I will need a better pattern..
            self.charge_commission(&symbol_name, remaining_quantity, exchange_rate).await;
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
                symbol_name.clone(),
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
                self.position_calculation_mode.clone()
            );

            // Insert the new position into the positions map
            //eprintln!("Symbol Code {}", symbol_code);
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

            self.cash_value = self.cash_used + self.cash_available;

            //println!("{:?}", event);
            position_events.push(event);
        }
        paper_response_sender.send(None).unwrap();
        for event in position_events {
            match self.strategy_sender.send(StrategyEvent::PositionEvents(event)).await {
                Ok(_) => {}
                Err(e) => eprintln!("Error sending position event: {}", e)
            }
        }
    }

    pub async fn flatten_all_for_paper_account(&mut self, time: DateTime<Utc>) {
        let positions_to_close: Vec<_> = self.positions.iter()
            .map(|position| {
                (
                    position.key().clone(),  // symbol_code
                    position.side,
                    position.symbol_name.clone(),
                    position.quantity_open
                )
            })
            .collect();

        // Then close each position
        for (symbol_code, side, symbol_name, quantity) in positions_to_close {
            let order_side = match side {
                PositionSide::Long => OrderSide::Sell,
                PositionSide::Short => OrderSide::Buy,
                _ => unreachable!("This shouldn't happen")
            };

            let market_price = match price_service_request_market_fill_price(
                order_side,
                symbol_name.clone(),
                symbol_code.clone(),
                quantity
            ).await {
                Ok(price) => match price.price() {
                    None => continue,
                    Some(price) => price
                },
                Err(_) => continue
            };

            let tag = "Flatten All".to_string();
            self.paper_exit_position(&symbol_code, time, market_price, tag).await;
        }
    }
}
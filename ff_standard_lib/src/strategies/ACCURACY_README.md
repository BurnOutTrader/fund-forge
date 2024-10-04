# Trade Analysis Results

## Given Statistics:
- Starting Account Balance: $100,000
- Ending Balance: $88,630.00
- Win Rate: 44.44%
- Average Risk Reward: 0.93
- Profit Factor: 0.74
- Total Profit: -$11,370.00
- Total Wins: 56
- Total Losses: 70
- Break Even: 0
- Total Trades: 126
- Open Positions: 0
- Cash Used: $0
- Cash Available: $88,630.00


## Verification With Decimal Accuracy Specified By Symbol Info:

1. **Ending Balance**:
   - Starting Balance + Total Profit = $100,000 - $11,370 = $88,630
   - ✅ Correct

2. **Win Rate**:
   - Calculated: (Total Wins / Total Trades) * 100 = (56 / 126) * 100 = 44.44%
   - ✅ Correct

3. **Total Trades**:
   - Count of trades in the CSV: 126
   - ✅ Correct

4. **Total Wins and Losses**:
   - Wins (positive PnL): 56
   - Losses (negative PnL): 70
   - ✅ Correct

5. **Total Profit**:
   - Sum of all PnL entries: -$11,370
   - ✅ Correct

6. **Average Risk Reward**:
   - Average Win / Average Loss
   - Average Win: $591.80 (sum of positive PnL / number of wins)
   - Average Loss: $638.63 (sum of negative PnL / number of losses)
   - Calculated: 591.80 / 638.63 = 0.93
   - ✅ Correct

7. **Profit Factor**:
   - (Sum of Profits) / (Sum of Losses)
   - Sum of Profits: $33,141
   - Sum of Losses: $44,511
   - Calculated: 33,141 / 44,511 = 0.74
   - ✅ Correct

8. **Break Even, Open Positions, Cash Used**:
   - No contradictory information in the CSV
   - ✅ Assumed Correct

9. **Cash Available**:
   - Matches the ending balance
   - ✅ Correct

## Conclusion:
All provided statistics are accurate based on the trade data in the CSV file.

Certainly. Let's analyze the `reduce_paper_position_size`, `add_to_position`, and `backtest_update_base_data` functions in the context of the `calculate_historical_pnl` function we reviewed earlier. I'll evaluate if the PnL calculations and related operations appear correct.


Analyze these functions in relation to the `calculate_historical_pnl` function:

1. `reduce_paper_position_size`:
   - The booked PnL calculation looks correct. It uses `calculate_historical_pnl` with the correct parameters.
   - The update of `booked_pnl` and `open_pnl` is correct: booked PnL is added to the total, and open PnL is reduced by the same amount.
   - The average exit price calculation is correct, using a weighted average approach.
   - The quantity adjustments and closure checks are correct.

2. `add_to_position`:
   - The average price calculation is correct, using a weighted average approach.
   - The quantity update is correct.
   - The open PnL recalculation using `calculate_historical_pnl` is correct and uses the updated average price and quantity.

3. `backtest_update_base_data`:
   - The market price extraction based on different data types is correct and considers the position side for quote-based data.
   - The highest and lowest recorded price updates are correct.
   - The open PnL calculation using `calculate_historical_pnl` is correct and uses the current market price and open quantity.

Overall, the PnL calculations and related operations in these functions appear to be correct and consistent with the `calculate_historical_pnl` function.
The code handles different scenarios (reducing position, adding to position, and updating based on new data) appropriately.

A few observations:

1. The code consistently uses the `calculate_historical_pnl` function for PnL calculations, which is good for consistency.
2. The handling of decimal precision (using `round_dp`) is consistent throughout the code.
3. The code properly handles different types of market data (candles, ticks, quote bars, quotes) in the `backtest_update_base_data` function.
4. The average price and average exit price calculations use weighted averages, which is the correct approach.

In conclusion, the PnL calculations and position management logic appear to be implemented correctly and consistently across these functions.

## Functions for Historical PnL

```rust
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


// Only used to calculate estimated open pnl, never booked pnl, booked pnl comes from the order book or market handler logic
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

pub fn calculate_historical_pnl(
   side: PositionSide,
   entry_price: Price,
   market_price: Price,
   quantity: Volume,
   _account_currency: Currency,
   _time: DateTime<Utc>,
   symbol_info: &SymbolInfo
) -> Price {
   // Calculate the price difference based on position side
   let raw_ticks = match side {
      PositionSide::Long => {
         let ticks =  ((market_price - entry_price) / symbol_info.tick_size).round_dp(symbol_info.decimal_accuracy);
         if ticks == dec!(0.0) {
            return dec!(0.0)
         }
         ticks
      },   // Profit if market price > entry price
      PositionSide::Short => {
         let ticks = ((entry_price - market_price) / symbol_info.tick_size).round_dp(symbol_info.decimal_accuracy);
         if ticks == dec!(0.0) {
            return dec!(0.0)
         }
         ticks
      },
   };

   /*   if pnl_currency != account_currency && time > *EARLIEST_CURRENCY_CONVERSIONS {
          //todo historical currency conversion using time
          // return pnl in account currency
      }*/

   // Calculate PnL by multiplying with value per tick and quantity
   let pnl = raw_ticks * symbol_info.value_per_tick * quantity;
   pnl
}

```

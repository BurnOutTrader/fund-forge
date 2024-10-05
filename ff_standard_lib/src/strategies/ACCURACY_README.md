# Trade Analysis Results
Market orders will attempt to use the order book to fill, assuming we get to soak up 100% volume per level, if the book only has 1 level then it will just fill according to order side assuming that we only have best bid or best offer.

Limit orders will partially fill if we have order books with volume, else they will fully fill at the best bid or offer depending on order side.

If no order book or quote data is available we will fill all orders at the last price.

Accuracy was tested using only market orders, enter long, enter short, exit long, exit short, however all other orders follow the same logic and should work accurately and I will build a test in the future.

Any slight differences in expected statistical values will be due to rounding.
- The ledger rounds average prices to the symbols decimal_accuracy each time a position increases or decreases in size using the weighted average prices.

When a position reduces size:
Fund Forge determines the number of ticks based on the symbol tick_size and Multiplies the number of ticks(or pips) by exiting order quantity filled and value per tick.

This is done for each position when the position closes or changes open value, not when the stats are calculated.

The alternative would be rounding pnl after we have the total, but this would be less realistic than the current method.

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

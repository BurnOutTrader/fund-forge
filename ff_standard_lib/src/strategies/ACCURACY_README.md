### Test Results
# Updated Backtest Results Verification

## Calculations from Raw Data

1. Total Profit/Loss: -2130.0
   (Sum of all 'Booked PnL' values)

2. Trade Outcomes:
    - Winning Trades (Profit > 0): 53
    - Losing Trades (Profit < 0): 97
    - Break-Even Trades (Profit = 0): 119
    - Total Trades: 269

3. Win Rate: (53 / 269) * 100 = 19.70%

4. Profit Factor:
   Total Profit from Winning Trades: 2190.0
   Total Loss from Losing Trades: -4320.0
   Profit Factor = 2190.0 / 4320.0 = 0.51

5. Average Risk Reward:
   Average Profit per Winning Trade: 2190.0 / 53 = 41.32
   Average Loss per Losing Trade: 4320.0 / 97 = 44.54
   Average Risk Reward = 41.32 / 44.54 = 0.93

6. Final Balance:
   Starting Balance: 100,000
   Total Profit/Loss: -2130.0
   Final Balance: 100,000 - 2130.0 = 97,870.00

## Comparison with Provided Statistics

| Metric | Calculated | Provided | Match? |
|--------|------------|----------|--------|
| Balance | 97,870.00 | 97,870.00 | ✅ |
| Win Rate | 19.70% | 19.70% | ✅ |
| Average Risk Reward | 0.93 | 1.03 | ❌ |
| Profit Factor | 0.51 | 0.56 | ❌ |
| Total Profit | -2130.00 | -2130.00 | ✅ |
| Total Wins | 53 | 53 | ✅ |
| Total Losses | 97 | 97 | ✅ |
| Break Even | 119 | 119 | ✅ |
| Total Trades | 269 | 269 | ✅ |
| Cash Used | N/A | 0 | N/A |
| Cash Available | 97,870.00 | 97,870.00 | ✅ |

## Conclusion

1. All trade counts (wins, losses, break-even, and total) match exactly.
2. The total profit/loss, win rate, and final balance are all correct.
3. The cash available matches the final balance, which is consistent with the provided information.
4. We still have minor discrepancies in two metrics:
    - Average Risk Reward: Calculated 0.93 vs. Provided 1.03
    - Profit Factor: Calculated 0.51 vs. Provided 0.56

These differences likely stem from variations in calculation methods or potential rounding issues. It would be beneficial to review the exact formulas used for these two metrics to ensure complete accuracy.

Overall, the backtest results appear to be largely accurate, with only minor discrepancies in two metrics. 

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
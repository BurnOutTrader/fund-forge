## DataBase and Ticks

### Historical Tick Data Time Accuracy
#### Timestamp Handling in Fund Forge Engine for Historical Data

The Fund Forge engine maintains nanosecond-level DateTime precision. When retrieving historical tick data from specific DataVendor implementations, there can be instances where multiple ticks share the same timestamp due to vendor-specific timestamp limitations or simply because 2 ticks were created by the same aggressor order at the same time. To prevent data duplication, the engine compares each tick’s timestamp with the last processed timestamp.

If a timestamp collision occurs, the engine adjusts the new tick’s timestamp by adding +1 nanosecond * number of consecutive collisions, ensuring each tick is uniquely stored.

#### Rationale
Since we buffer data in memory, we are not trading below a nanosecond accuracy, so we can safely adjust the timestamp to ensure uniqueness.

This approach strikes a balance between storage efficiency and data precision, avoiding the need for additional structures that could duplicate data unnecessarily. Although this adjustment alters the original timestamp slightly, the impact on practical backtesting is minimal.

Considerations for Supporting Identical Timestamps

Allowing identical timestamps would require extensive structural changes, including storing vectors of data points (e.g., Vec<Tick> or Vec<BaseDataType>) at each timestamp. This adjustment would increase both storage demands and computational load for all BaseDataTypes, not just ticks, complicating data processing and aggregation tasks such as timeslicing.

#### Conclusion

This solution offers an efficient balance by using a minor timestamp adjustment to ensure uniqueness while maintaining the engine’s performance and scalability, particularly when handling data from vendors with limited timestamp granularity.




## Oanda for currency conversion.
Currency conversion tool to estimate open pnl on active positions.
An initial exchange multiplier will be fetched from out historical data for and attached to a position object. (1 by default or for same pnl currency == account currency)
On closing a position where the pnl currency does not match the account currency we will again fetch the exchange rate and calculate the pnl in the account currency.
this could be configured as end of day convertion, where we keep all currencies held by an account in a map, then convert EOD, or we can convert on position close,
The level of detail would depend on the desired backtest accuracy.

## Stats
Statistics in synchronize accounts mode do not work, since orders arrive after the position is closed.
Need to retroactively update the statistics when the order updates arrive.

## Subscription handling
Subscriptions need to be handled by the broker enum, to many variables on the client side
- Live subscriptions will be done by
1. Starting the warm up while also buffering incoming new data
   - this will require a check to see if the individual subscription is warmed up
   - new data is fed from buffer into consolidators etc

Current setup has problems using ticks to build heikin ashi bars, maybe causing locking, the code here is just confusing and overcomplicated by 
accommodating all variants.

## OrderBooks
- Need to do order books after moving subscription handling to server.

## Add More info to updates
- Add price and quantity to order fill events
- Add price and quantity to position creation and reducing events

## Indicators Not fully tested
These will be tested with charting enabled
- Renko
- ATR



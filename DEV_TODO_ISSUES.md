## Put buffer on server side
- Consider moving buffer to server side to simplify engine logic

## Subscription handling
Need a new function for warming up new live subscriptions.
- Live subscription in symbol subscription handler could be simplified by just allowing the data server to determine the correct primary resolution.
- Live history requests will require the server to manage updating from the last serialized data point to fill forward until Utc::now().

## Build Tests
- Test live subscription warm up
- Test live indicator warm up

## Positions and Statistics
Currently, fund forge only implements 1 of the 3 intended position types that I intend to implement.
1. Cumulative, when a position is opened, it any additional entries in the same direction will be counted as part of the same position. (Implemented)
2. First in First Out: When a position is opened any order in the opposite direction will create a trade object based on the entry price and exit price (Not yet implemented)
3. First in Last Out: Recent entries are prioritized for closing, leaving the older entries as the last to be closed.

This will be done by passing in a set of 3 functions to the ledger in its initialization.

## TIF
Need a way to handle custom TIF timezones for 
1. TimeInForce::Day(TzString) 
2. TimeInForce::(TimeString, TzString)

This can be achieved by either: 
- Converting the time to market time, depending on the Brokerage used. Or
- Manually sending cancel order to the Brokerage at the specified time.

## Indicators Not fully tested
These will be tested with properly charting enabled
- Renko
- ATR



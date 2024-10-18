## Add More info to updates
- Add price and quantity to order fill events
- Add price and quantity to position creation and reducing events
## Fix server
- Need a better server architecture, consider using warp to manage server threads.
- Need to fix thread management for Rithmic, currently it has problems.

## Subscription handling
- Live subscriptions will be done by
1. Starting the warm up while also buffering incoming new data
   - this will require a check to see if the individual subscription is warmed up
   - new data is fed from buffer into consolidators etc

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
These will be tested with charting enabled
- Renko
- ATR



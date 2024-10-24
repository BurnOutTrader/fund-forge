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



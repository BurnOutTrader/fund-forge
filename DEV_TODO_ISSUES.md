
## Subscription handling
- Live subscription in symbol subscription handler could be simplified by just allowing the data server to determine the correct primary resolution.
- Live warm up will have to proceed while also collecting live bars in a buffer from the data server to get accurate warm up, BTreeMap<DateTime<Utc>, Data> 
using data time as key will correctly align the stream with the history available.


## Build Tests
- Test live subscription warm up
- Test live indicator warm up


## Positions and Statistics
Currently fund forge only implements 1 of the 3 intended position types that I intend to implement.
1. Cumulative, when a position is opened, it any additional entries in the same direction will be counted as part of the same position. (Implemented)
2. First in First Out: When a position is opened any order in the opposite direction will create a trade object based on the entry price and exit price (Not yet implemented)
3. First in Last Out: Recent entries are prioritized for closing, leaving the older entries as the last to be closed.

This will be done by passing in a set of 3 functions to the ledger in its initialization.


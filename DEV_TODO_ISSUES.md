
## Subscription handling
- Live subscription in symbol subscription handler could be simplified by just allowing the data server to determine the correct primary resolution.
- Live warm up will have to proceed while also collecting live bars in a buffer from the data server to get accurate warm up, BTreeMap<DateTime<Utc>, Data> 
using data time as key will correctly align the stream with the history available.


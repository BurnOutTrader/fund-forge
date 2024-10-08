## Using rkyv crate
rkyv implements total zero-copy deserialization, 
which guarantees that no data is copied during deserialization and no work is done to deserialize data. 
It achieves this by structuring its encoded representation so that it is the same as the in-memory representation of the source type.
see https://github.com/rkyv/rkyv and https://rkyv.org/

#### CSV TESTS
Serialize 121241 candles took: `69.326083ms` using csv format. \
Deserialize 121241 candles took: `74.376041ms` using csv format. \
File size: 7362642 bytes. 

#### RKYV TESTS
Serialize 121241 candles took: `6.317917ms` using rkyv format. \
Deserialize 121241 candles took: `3.690916ms` using rkyv format. \
Load as bytes 121241 candles took: `530.667µs` using rkyv format. \
File size: 8729360 bytes.

For an example of saving and loading types as .rkyv files and creating bytes from types see candles.rs and candle_tests.rs \
\
Data streamed from the historical_server and api_implementations will be in the form of `Vec<u8>` bytes that will be deserialized into the required type on the client side.






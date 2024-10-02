## Building Api's
See the [server_features](src/server_features) mod
Any mod inside this mod will be visible only to crates which enabled the 'server' feature flag for the 'ff_standard_lib'.
This helps to prevent direct api calls for `Brokerage` and `DataVednor` implementations on the client/strategy side.
The only crate that uses this feature by default is the `ff_data_server`.
see [DEV_README.ms](src/server_features/DEV_README.md)

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

For ser/de bytes from bytes just implement [Bytes](src/standardized_types/bytes_trait.rs) \
\
Data streamed from the historical_server and api_implementations will be in the form of `Vec<u8>` bytes that will be deserialized into the required type on the client side.





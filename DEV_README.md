# Development guide for adding to fund-forge

## A guide for adding new features to fund forge
At a glance it might seem complicated, but because of the use of traits and enums you can basically just implement a new enum and/or trait and attempt to compile and you will see all the tasks you need to complete.
Knowing the code base I can generally implement a new Request and Response in half an hour. Using rust rover IDE is advantageous as it takes you directly to each new task when compilation fails.

There are still a lot of Request and Response types to be implemented as I am currently implementing Rithmic Api.

Any Data transferred in a `DataServerResponse` enum and `DataServerRequest` enum must implement the rkyv traits.
rkyv is used for fast ser/de of data from bytes to types.
Some complex data is more difficult to serialize this way, but just remember you can always serialize or desrialize as strings or an array of bytes itself, native data types are no problem.
The only problem I have had was with DateTime object, which was easily overcome by using time strings or time stamps, rkyv now supports many more types, but I have not yet updated.
Most of our trading related data is of the native types and so I don't see any major issues moving forward as rkyv develops.
```rust
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
#[derive(Serialize_rkyv, Deserialize_rkyv, Archive)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
```

### Api's
`DataVendor` or `Brokerage` Api:
1. To create a new brokerage or data vendor we need to create an api object that:
- To place orders and manage accounts we need an api object that implements the trait [BrokerApiResponse](ff_standard_lib/src/server_features/server_side_brokerage.rs).
- To subscribe to data feeds, download historical data etc we need an api object that implements the trait [VendorApiResponse](ff_standard_lib/src/server_features/server_side_datavendor.rs).
- *A brokerage could implement both if you want to use data from the brokerage, if you only want to place orders, you only need to implement the `BrokerApiResponse`.*

2. Create a new enum variant
- If you are implementing a DataVendor create a new [DataVendor](ff_standard_lib/src/standardized_types/datavendor_enum.rs) variant 
- If you are implementing a Brokerage
  1. create a new [Brokerage](ff_standard_lib/src/standardized_types/broker_enum.rs) variant. and/or
  2. create a [DataVendor](ff_standard_lib/src/standardized_types/datavendor_enum.rs)
  

3. You will need to complete the matching statements for the new enum variant on the server side:
- for server side [Brokerage](ff_standard_lib/src/server_features/server_side_brokerage.rs)
- for server side [DataVendor](ff_standard_lib/src/server_features/server_side_datavendor.rs)

4. Since your object implements a trait of the same name as the server side implementation, 
you only need to be able to get your api object and you can directly return the required values when your new enum variant is called.
```rust
#[async_trait]
impl VendorApiResponse for DataVendor {
    async fn symbols_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        market_type: MarketType,
        callback_id: u64
    ) -> DataServerResponse {
        match self {
            DataVendor::Rithmic(_) => {
                if let Some(client) = get_rithmic_client(&self) {
                    return client.symbols_response(mode, stream_name, market_type, callback_id).await
                }
            }, 
            // todo see the test client is a static object, it is created when the server is launched. 
            DataVendor::Test => return TEST_CLIENT.symbols_response(mode, stream_name, market_type, callback_id).await,
            // DataVendor::{Your New Vendor} => YOUR_NEW_VENDOR.symbols_response(mode, stream_name, market_type, callback_id).await,
         }
        DataServerResponse::Error { callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self)) }
    }
}
```
How you actually return the data is up to you. if your api does not return the type of data just hard code the values, you don't have to support every symbol that your broker or vendor offers.
The static CLIENT is just an object that returns the trait implementations, you can build the api logic however you want.

You don't need to touch the client side implementations when implementing new DataVendor or Brokerage variants.


### Creating new Request or Response types
If you want to add a new DataVendor or Brokerage feature, like `get_example_data()`.

You will need to create a new `DataServerRequest` and `DataServerResponse`.
You will need to know if your request is blocking (requesting an object you need to continue a function) or non-blocking (like requesting a data stream to start)

#### Connection Type 
```rust
pub enum ConnectionType {
    Vendor(DataVendor),
    Broker(Brokerage),
}
 ```
The Connection type is just a wrapper for your DataVendor or Brokerage enum, to help the request handler find the correct address for the server.
Remember by default all connections use a single default server, so you don't need to worry about much regarding this.
however you should pass in your actual ConnectionType based on if it is a brokerage or data vendor implementation including wrapping you new enum variant.

There are only 2 request types, and to send a request we need to have the correct ConnectionType enum for your implementation.
```rust
fn example() {
  let broker = Brokerage::Test;
  let connection_type = ConnectionType::Broker(broker);
}
```

#### Blocking Requests
We create a one shot and send the Callback message with the one shot attached.
- Notice that this enum variant has a callback_id field, you will need the same field if you are expecting a callback.
- The callback_id value will not be set in your function, but you need the field, just set the callback_id to 0 in your function.
```rust
fn example() {
    let request = DataServerRequest::TickSize {
        callback_id: 0,
        data_vendor: self.data_vendor.clone(),
        symbol_name: self.name.clone(),
    };
    let (sender, receiver) = oneshot::channel();
    let msg = StrategyRequest::CallBack(ConnectionType::Vendor(self.data_vendor.clone()), request, sender);
    send_request(msg).await;
}
```

- You will need to add a matching statement to the `DataServerRequest` and `DataServerResponse`. implementations of `fn callback_id()` this allows the engine determine if the requests and response are callbacks.
The functions are found in the [data_server_messaging file](ff_standard_lib/src/messages/data_server_messaging.rs)

to send our DataServerRequest we create a oneshot sender and receiver and wrap them in `StrategyRequest::CallBack(ConnectionType::Vendor(self.data_vendor.clone()), DataServerRequest, sender);`

After we send the request we wait for the response on the receiver and handle it however we need.
You won't need to do anything with the client handlers, since it will return the data to your oneshot receiver as soon as it arrives.
```rust
impl Symbol {
    pub async fn tick_size(&self) -> Result<Price, FundForgeError> {
        let request = DataServerRequest::TickSize {
            callback_id: 0,
            data_vendor: self.data_vendor.clone(),
            symbol_name: self.name.clone(),
        };
        let (sender, receiver) = oneshot::channel();
        let msg = StrategyRequest::CallBack(ConnectionType::Vendor(self.data_vendor.clone()), request, sender);
        send_request(msg).await;
        match receiver.await {
            Ok(response) => {
                match response {
                    DataServerResponse::TickSize { tick_size, .. } => Ok(tick_size),
                    DataServerResponse::Error { error, .. } => Err(error),
                    _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                }
            },
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
        }
    }
}
```
You will then need to complete a matching statement for the server logic in ff_data_server handle_client function so the server knows what to do with the request type.
[manage_async_requests()](ff_data_server/src/request_handlers.rs)

#### Non Blocking Requests
For non-blocking messages like streams or orders first we send the request by wrapping it in a strategy request enum variant.

`let register_request = StrategyRequest::OneWay(connection_type.clone(), DataServerRequest::Register(mode.clone()));`

`send_request(StrategyRequest).await;`

this is a public fn that can be called from anywhere in our code. It will add your message to the buffer for the outgoing TLS stream.

Then we need to handle the response in both the client sides buffered and unbuffered response handlers below:
see the [live handlers](ff_standard_lib/src/strategies/client_features/server_connections.rs).
(at the time of writing I am considering simplifying into a single handler)

You will then need to complete a matching statement for the server logic in ff_data_server handle_client function so the server knows what to do with the request type.

#### Long Option A
If you want all implementations to return this kind of response then you will then need to add a new `VendorApiResponse` or `BrokerApiResponse` to the trait.
- trait [BrokerApiResponse](ff_data_server/src/server_side_brokerage.rs).
- trait [VendorApiResponse](ff_data_server/src/server_side_datavendor.rs).

you will then need to provide matching statements for all existing api objects for the enum type on the server side.
- for server side [Brokerage](ff_data_server/src/server_side_brokerage.rs)
- for server side [DataVendor](ff_data_server/src/server_side_datavendor.rs)

you might also need to provide [client side implementations](ff_standard_lib/src/strategies/client_features/client_side_vendor).
depending on how you want to access the data in your strategies.
You will then need to complete a matching statement for the server logic in `ff_data_server` function [manage_async_requests()](ff_data_server/src/request_handlers.rs) function so the server knows what to do with the request type.
This is quite easy as it is just another mathcing statement.

#### Short Option B
If you don't want other variants to return this response you can just move onto sending the response and receiving the message.
You will need to complete a matching statement for the server logic in ff_data_server handle_client function so the server knows what to do with the request type.
Then we need to send the request to the data server, via the public function `send_request(StrategyRequest).await;`

if You sent a `StrategyRequest::OneWay` message the server will handle the request and you can move on. 
if you expect a streaming response, handle the response in the [live handlers](ff_standard_lib/src/strategies/client_features/server_connections.rs).

If you sent a `StrategyRequest::CallBack`, then you just wait until the response arrives and proceed with handling the new data.


## Indicators
See [Indicators readme](ff_standard_lib/src/strategies/indicators/INDICATORS_README.md)

## Subscribing to Streams
We use a broadcaster to manage streams.
How you manage streams inside your api object is up to you, but the data will be received by the server through a `tokio::sync::broadcast::Receiver<BaseDataEnum>`.
All the logic is handled for you, you just need to call the subscribe_stream or unsubscribe_stream functions from inside your api logic and pass in the correct

Our api object will hold some map of `tokio::sync::broadcast::Sender<BaseDataEnum>`.

for example we might have: 
```rust
struct ClientExample {
  streams: DashMap<DataSubscription, tokio::sync::broadcast::Sender<BaseDataEnum> >
}
```

We need to create a broadcast receiver.
If we already have an active broadcaster for an existing stream we can just subscribe the new broadcaster to the new stream, 

```rust
fn example(client: ClientExample, subscription: DataSubscription) {
    // stream name is the port number of the incoming request, it is not related to your stream, it will be passed to your client in your `impl VendorApiResponse`
    // subscription will also be passed to you.
    // you will need to create a receiver by calling your clients broadcaster object 
    let receiver = match client.streams.get(&subscription) {
      Some(stream) => {
        let receiver = broadcaster.value().subscribe();
        receiver
      }
      None => {
        // you will have to handle how you intialize new streams with your client. you just need to get data from the api and convert it to base data enum
        // you need to create a new broadcaster for the subscription
        // you need to broadcast the base data enum to subscribers
      }
    };
    
    // Once we have a receiver we can send it to the handler by using this helper function and the data server will do the rest
    pub async fn subscribe_stream(stream_name: &StreamName, subscription: DataSubscription, receiver: broadcast::Receiver<BaseDataEnum>);

    // When you receive an unsubscribe request, use this function to stop the data server from trying to check the receiver before you drop it.
    pub async fn unsubscribe_stream(stream_name: &StreamName, subscription: &DataSubscription);
}
```

### Historical Tick Data Time Accuracy
### Timestamp Handling in Fund Forge Engine for Historical Data

The Fund Forge engine maintains nanosecond-level DateTime precision. When retrieving historical tick data from specific DataVendor implementations, there can be instances where multiple ticks share the same timestamp due to vendor-specific timestamp limitations or simply because 2 ticks were created by the same aggressor order at the same time. To prevent data duplication, the engine compares each tick’s timestamp with the last processed timestamp.

If a timestamp collision occurs, the engine adjusts the new tick’s timestamp by adding +1 nanosecond * number of consecutive collisions, ensuring each tick is uniquely stored.

### Rationale
Since we buffer data in memory, we are not trading below a nanosecond accuracy, so we can safely adjust the timestamp to ensure uniqueness.

This approach strikes a balance between storage efficiency and data precision, avoiding the need for additional structures that could duplicate data unnecessarily. Although this adjustment alters the original timestamp slightly, the impact on practical backtesting is minimal.

### Considerations for Supporting Identical Timestamps

Allowing identical timestamps would require extensive structural changes, including storing vectors of data points (e.g., `vec![BaseDataEnum]` and `vec![BaseDataEnum, BaseDataEnum, BaseDataEnum]`) at each timestamp. This adjustment would increase both storage demands and computational load for all BaseDataTypes, not just ticks, complicating data processing and aggregation tasks such as time-slicing.

### Conclusion

This solution offers an efficient balance by using a minor timestamp adjustment to ensure uniqueness while maintaining the engine’s performance and scalability, particularly when handling data from vendors with limited timestamp granularity.




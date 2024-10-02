# Development Guide

## A guide for adding new features to fund forge
At a glance it might seem complicated, but because of the use of traits and enums you can basically just implement a new enum and/or trait and attempt to compile and you will see all the tasks you need to complete.
Knowing the code base I can generally implement a new Request and Response in half an hour. Using rust rover IDE is advantageous as it takes you directly to each new task when compilation fails.

There are still a lot of Request and Response types to be implemented as I am currently implementing Rithmic Api.

### Api's
`DataVendor` or `Brokerage` Api:
1. To create a new brokerage or data vendor we need to create an api object that:
- To place orders and manage accounts we need an api object that implements the trait [BrokerApiResponse](server_side_brokerage.rs).
- To subscribe to data feeds, download historical data etc we need an api object that implements the trait [VendorApiResponse](server_side_datavendor.rs).
- *A brokerage could implement both if you want to use data from the brokerage, if you only want to place orders, you only need to implement the `BrokerApiResponse`.*

2. Create a new enum variant
- If you are implementing a DataVendor create a new [DataVendor](../standardized_types/datavendor_enum.rs) variant 
- If you are implementing a Brokerage
  1. create a new [Brokerage](../standardized_types/broker_enum.rs) variant. and/or
  2. create a [DataVendor](../standardized_types/datavendor_enum.rs)
  

3. You will need to complete the matching statements for the new enum variant on the server side:
- for server side [Brokerage](../server_features/server_side_brokerage.rs)
- for server side [DataVendor](../server_features/server_side_datavendor.rs)

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

#### Blocking
We create a one shot and send the Callback message with the one shot attached.
- Notice that this enum variant has a callback_id field, you will need the same field.
- You will need to add a matching statement to the `DataServerRequest` and `DataServerResponse`. implementations of `fn callback_id()` this allows the engine determine if the requests and response are callbacks.

- The callback_id will not be set in your function, but you need the field, just set the callback_id to 0 in your function.
After we send the request we wait for the response on the receiver and handle however we need.
You won't need to do anything with the client handlers, since it will return the data to your oneshot receiver as soon as it arrives.

However, you will then need to complete a matching statement for the server logic in ff_data_server handle_client function so the server knows what to do with the request type.
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
See [data server messages](../messages/data_server_messaging.rs).

#### Non Blocking
For non-blocking messages like streams or orders first we send the request by wrapping it in a strategy request enum variant.

`let register_request = StrategyRequest::OneWay(connection_type.clone(), DataServerRequest::Register(mode.clone()));`

`send_request(StrategyRequest).await;`

this is a public fn that can be called from anywhere in our code. It will add your message to the buffer for the outgoing TLS stream.

Then we need to handle the response in both the client sides buffered and unbuffered response handlers below:
see the [live handlers](../strategies/client_features/server_connections.rs).
(at the time of writing I am considering simplifying into a single handler)

You will then need to complete a matching statement for the server logic in ff_data_server handle_client function so the server knows what to do with the request type.

#### Long Option A
If you want all implementations to return this kind of response then you will then need to add a new `VendorApiResponse` or `BrokerApiResponse` to the trait.
- trait [BrokerApiResponse](server_side_brokerage.rs).
- trait [VendorApiResponse](server_side_datavendor.rs).

you will then need to provide matching statements for all existing api objects for the enum type on the server side.
- for server side [Brokerage](../server_features/server_side_brokerage.rs)
- for server side [DataVendor](../server_features/server_side_datavendor.rs)

you might also need to provide [client side implementations](../strategies/client_features/client_side_impl.rs).
depending on how you want to access the data in your strategies.
You will then need to complete a matching statement for the server logic in `ff_data_server` function [manage_async_requests()](../../../ff_data_server/src/request_handlers.rs) function so the server knows what to do with the request type.
This is quite easy as it is just another mathcing statement.

#### Short Option B
If you don't want other variants to return this response you can just move onto sending the response and receiving the message.
You will need to complete a matching statement for the server logic in ff_data_server handle_client function so the server knows what to do with the request type.
Then we need to send the request to the data server, via the public function `send_request(StrategyRequest).await;`

if You sent a `StrategyRequest::OneWay` message the server will handle the request and you can move on. 
if you expect a streaming response, handle the response in the [live handlers](../strategies/client_features/server_connections.rs).

If you sent a `StrategyRequest::CallBack`, then you just wait until the response arrives and proceed with handling the new data.






# Fund Forge
## Strategies
Once you have followed the setup instructions below, you can play with a test strategy by reviewing https://github.com/BurnOutTrader/fund-forge/blob/main/ff_strategies/README.md

## Architecture
First and foremost I have tried to keep the design as modular as possible to allow for upgrades and improvements to functions by more experienced programmers. 

I have opted for hard code using `impl` over `dyn` or dynamic dispatch, using enums instead of inheritance when possible for better run time performance. 

The architecture of fund forge designed to allow for "monolithic" or "micro-service" runtime depending on:
```rust
pub enum PlatformMode {
    SingleMachine,
    MultiMachine
}
```
Most of the complexity has been abstracted away from the end user and any switch to Multi-Machine mode is as simple as individually launching servers and specifying a few connection settings (ip address etc) in the `server_settings.toml` file.

In `PlatformMode::SingleMachine`: all api communication is done locally by launching the handlers in `ff_standard_lib::servers::request_handlers` which will receive requests from the strategy or ui and provide appropriate responses. In this mode all `DataVendor` and `Brokerage` Apis are launched locally.

In `PlatformMode::MultiMachine`: all api communication can be handled by launching the `ff_data_server` either on local host or on a remote server, the data server will listen for incoming TLS Connections and will launch new handlers in server_message_handler.rs
for each new connection. Further we can set a unique address for each `DataVendor` and `Brokerage` Api and have a separate server devoted to each api type. For info on MultiMachine setups see [[Multi Machine]]

If no settings are provided in the `resources/server_settings.toml` then that brokerage or vendor will use the default server addresses, this way we have the option for maximum and minimum segregation of api instances.

When running a Strategy or Ui each vendor or brokerage instance will automatically generate both a Synchronous and Asynchronous connection per individual specification, if no unique specs are input in the `server_settings.toml` then a default connection will be assumed to host all api instances on a single server or docker instance.

 - The synchronous connection uses a Restful, send request await response style, where one thread at a time takes control of the sending and receiving side of the communicator. This allows us to mimic the behaviour of rest apis and is used for all request types that need a synchronous response.

- The async connection is a split of the original communicator, whether that is a `TlsStream` or a mspc channel, this allows us to send requests that do not require a synchronous response, and receive all responses together as stream of messages. Allowing both streaming and rest style functionality which is common in modern apis.

Both local and Remote connections will be handled using the same logic by the engine, so there is no work required by the end user other than to specify server settings and manually launching server instances if segregation of apis is required.
If single machine mode is chosen, the handlers that normally run on a server will instead be run as internal functions, and use mspc channels in place of `tokio_rustls::TlsStream`.

To achieve this versatility we use `SynchronousCommunicator` enum for synchronous rest style messaging where we send a request and expect an immediate response and for async streams we use `SecondaryDataReceiver` and `SecondaryDataSender` enums which are just split streams or mspc senders and receivers.

On the server side we use a `Broadcaster` and `StreamManager` to forward data to subscribers (strategies and UI's) using a subscriber pattern.

Since all data is transferred using 0 cost deserialization with the `rkyv crate`, we are able to handle both remote and internal communication as bytes with almost 0 overhead, casting directly to the associated messages.

The enum variants used for communication can represent mspc channels or Tls connections depending on the `PlatformMode` enum.

Using this design we can cater to both novice traders who run their entire setup locally and advanced traders who use collocation services etc.

An api instance for a `DataVendor` is just an object that implements `VendorApiResponse` trait
You can implement your api however you like, as long as you return all the required functions implemented by the specific trait.

Brokerages utilize the `BrokerApiResponse` trait. 

On the client side the function calls are automatically forwarded to the correct api and so no changes to `ClientSideDataVendor` or `ClientSideBrokerage` need to be made

**Note: In the future I may implement different trait 'levels' so we can implement `Level 1 broker` `Level 2 Broker` to allow diversity in broker api integrations which can vary vastly.

All requests made by the engine will use the DataVendor or Brokerage variant in some way.
We are simply sending a request to the server using the DataVendor or Brokerage enum variant, and the server is returning a response with the data we require based on that variant.
```rust
/// in a strategy we could request the symbols for a market like this
let vendor = DataVendor::Test;

/// we will return a result from the fn, it will either be a Vec<Symbol> or a FundForgeError, this error type will contain a string to tell us if the error was on the client side, or if the server has some problem fetching data.
let symbols = vendor.symbols(MarketType::Forex).await.unwrap();
```

What the above function actually does is call a function to get the connection to the data server instance associated with that enum variant, then it sends a request for the symbols which also contains the enum variant to the data server, the server requests the api and retrieves the symbols, returning them in fund forge format as `Vec<Symbol>`. 

** Note at a later stage I will consider adding a hybrid mode, where some apis can be run locally and some can be run remotely, since some brokers like OKX allow multiple api instances.
The current design is simply to maximize utility of more client limiting brokerage apis. 

`DataVendor::Test` and `Brokerage::Test` do not actually use any live Api endpoints and are instead designed to simulate end points so that we can get a reliable response from the server using pre serialized or hardcoded data to test platform functionality during development and remain broker agnostic.

# Setup 
You will need to change these hard coded directories in `ff_standard_lib::helpers` `mod.rs`
```rust
/// This is the path to the folder we use to store all base data from vendors
pub fn get_data_folder() -> PathBuf {
    PathBuf::from("{PATH_TO_FOLDER}/fund-forge/ff_data_server/data")
}

// The path to the resources folder
pub fn get_resources() -> PathBuf {  
    PathBuf::from("{PATH_TO_FOLDER}/fund-forge/resources")  
}
```

On first run a `server_settings.toml` file will be created in `fund-forge/resources` it will contain the default settings based on your `get_toml_file_path()`
If you try to launch before changing the path, just delete the `server_settings.toml` and it will be recreated.
```rust
impl Default for ConnectionSettings {
        fn default() -> Self {
            ConnectionSettings {
                ssl_auth_folder: get_toml_file_path(),
                server_name: String::from("fundforge"),
                address:  SocketAddr::from_str("127.0.0.1:8080").unwrap(), //Note that the default toml launch option will only allow local host
                address_synchronous: SocketAddr::from_str("127.0.0.1:8081").unwrap() //Note that the default toml launch option will only allow local host
            }
        }
    }
```

There is also the option to override the `toml` launch, by passing in the following commands (not yet implemented)
```rust
#[derive(Debug, StructOpt)]
struct ServerLaunchOptions {
    /// Sets the data folder
    #[structopt(short = "f", long = "data_folder", parse(from_os_str), default_value = "./data")]
    pub data_folder: PathBuf,

    #[structopt(short = "f", long = "ssl_folder", parse(from_os_str), default_value = "{PATH_TO_YOUR_`fund-forge`_FOLDER}/resources/keys")]
    pub ssl_auth_folder: PathBuf,

    #[structopt(short = "s", long = "synchronous_address", default_value = "0.0.0.0:8080")]
    pub listener_address: String,

    #[structopt(short = "p", long = "async_address", default_value = "0.0.0.0:8081")]
    pub async_listener_address: String,
}
```


# Parsing Data and Time handling
All data should be saved as 1 file per month and all times for data should be Utc time, use the time parsing functions in `ff_standard_lib::helpers::converters` to parse time from your time zone, these functions use `chrono-tz` and will automatically handle historical time zone conversions such as day light savings times. All Base data time properties are serialized as Strings, these strings are auto parsed into `DateTime<Utc>` using `base_data.time_utc()` or. `DateTime<FixedOffset>` using `base_data.time_local(Tz)` when running strategies, the reason for parsing to string is simply for easier `ser/de` using `rkyv`.

There are a number of helpers built into `BaseDataEnum impl` which help with parsing and serializing `BaseDataEnum`. for example `BaseDataEnum::format_and_save()` can take a large collection of base data and format it into separate files, 1 file per month. There are also functions for checking the earliest or latest base data for a particular subscription which can be useful for updating historical data at regular intervals.

Please see the base_data_enum.rs file for more info when building DataVendor implementations.

When handling historical data it is assumed that the `time` property of `Candle` and `QuoteBar` object is the opening time, so in the historical data requests we add the `base_data_enum.resolution.as_seconds()` to get the `base_data_enum.time_created()` which represents the closing time of the bar. To properly align the historical candles and quotebars with other historical data types such as ticks, which represent a single instance in time and therefore do not need to be adjusted. To avoid look ahead bias on our bars during backtesting, a tick that occurred at say 16:00 will be correctly aligned with the bar that closed at 16:00 instead of the bar that opened. This also allows us to reliably combine historical data feeds of different resolutions.

# Decimal Accuracy
I have opted to just build decimal calculators to avoid rounding errors, these calculators use the rust_decimal crate and help us to avoid rounding errors without having the additional overhead in our base data when it is not actually needed.
# Strategy Registry
The strategy registry service is a server where running strategies register and forward events to the Ui, in `SinglMachine` mode this will just be done over mspc channels, in `MultiMachine` mode we will need to manually launch a server instance.

This service will allow the Ui to find any strategy that is online via Tls/Tcp and communicate by sending commands and receiving updates in an async manner. It will also keep a record of all strategy events that are forwarded by strategies to allow us to replay strategies and see what has happened.

There will be an option to store data received by strategies, so we can see exactly what data the strategy had access to when live trading rather than depending on historical data which may contain data points the strategy never actually received.

The registry will also forward commands like pause, play and stop or allow us to add drawing objects to strategies which they will then be able to interact with depending on the `StrategyInteractionMode`.

# Work in progress
We will use a broadcaster object to allow many incoming stream requests to share a single instance of an api data stream event.



# Fund Forge
fund-forge is an algorithmic trading engine written in rust.
It is designed to allow maximum utility of retail trading api's by limiting the need to have duplicate api instances.
All strategies share can share a single api instance for each brokerage or data vendor vby connecting via Tls to your fund forge data server instance/s.
All streaming data feeds etc can be shared and only 1 stream per symbol is maintained, any data of higher resolution than the primary data stream will be automatically consolidated on the strategy side by the strategies `SubscriptionHandler`.
This means if we have a DataVendor with a tick stream, we can subscribe to 15 minute Candles and the  engine will create those candles in real time.

## Licence 
I have started building a GUI in iced for a full rust implementation, I have also experimented with a Tauri gui, which would use a rust backend and javascript front end so that we could use the lightweight charts api made by Trading View,
the current problem with the TradingView option is that lightweight charts free api does not support Renko or Heikin Ashi charts, so I temporarily halted development on this option in favour of completing a [rust charting](https://youtu.be/BU9TU3e1-UY) api using iced.rs,
I am considering making a cheap paid cross-platform GUI in the future so that I can use the Trading View's `Platform edition api` which requires to have an established company, for this reason I will likely include only a single prohibition in the licence, to restrict commercial use in regard to re-selling any part of the fund-forge repository.
I would like the platform to be 100% open source but realistically to get the best maintainable development of a GUI I might need to look into this option should the repo gain popularity.
This would allow me to pay front end developers to help with the GUI development and also pay any licensing fees associated with TradingView.
I won't know how realistic this option is until I know how many people are using the engine.
Since the engine is open source you will need to undergo the rithmic verification independently, you can see Rithmic section below.

## Strategies
Once you have followed the setup instructions below, you can play with a test strategy by reviewing https://github.com/BurnOutTrader/fund-forge/blob/main/ff_strategies/README.md

## Gui
Basic charting functionality was tested months ago. The code base has since been refactored and charting now supports live streams. A local gui is in production using rust iced.
Old video of testing charting algorithm https://youtu.be/BU9TU3e1-UY.
I will complete a charting API in the future but since I am not experienced with GUI development functionality will be limited to charting only in the short term.
Unfortunatley the learning curve for GUI development in iced is rather steep and it is the only appropriate rust option for the type of GUI I am trying to build.

## Architecture
I have tried to maintain a reasonable separation of concerns throughout the code base to allow any backend implementations to be upgraded without effecting existing strategies.
Some of the current implementations are a little but crude, as a solo developer with limited rust experience I decided to just keep pushing forward and worry about optimization and perfection of various functions once I have a product capable of live testing.

All strategy functionality is accessed by calling the `FundForgeStrategy` object's associated functions, there is essentially a complete decoupling of strategy instance from the backend so that
upgrades can be implemented from the backend engine and handlers without causing breaking changes to strategies.

I am willing to accept improvements and pull requests that do not include any kind of binary file (all vendor data is serialized as binaries).
All pull requests should include only human-readable code and files.

The platform is designed to be as fast as possible, using `rkyv` for serialization and deserialization and network messaging, and `tokio` for async communication.
The full potential of using rkyv will be unlocked in future versions, currently it is only the most basic implementation of serializing and deserializing from bytes.
see: https://github.com/rkyv/rkyv

I have opted for hard code using `impl` over `dyn` or dynamic dispatch, using enums instead of inheritance when possible for better run time performance at the cost of slightly more hardcoding for new implementations.

'ff_strategy_registry': decouples the strategy instance to allow front end Gui implementations in any programming language. 

'ff_data_servers': decouples api instances from any specific server and allows a microservices approach to maintaining api connections.
If no settings are provided in the `resources/server_settings.toml` then that brokerage or vendor will use the default server addresses, this way we have the option for maximum and minimum segregation of api instances.

When running a Strategy or Ui each vendor or brokerage instance will automatically generate an Asynchronous connection per individual `Brokerage` or `DataVendor` specification, if no unique specs are input in the `server_settings.toml` then the default connection will be assumed to host the api instance.
We can have a unique `ff_data_server` instance for each brokerage and data vendor, or we can holst all api instances on a single instance of the data server.

All data transferred between strategies and the data server or strategy registry is transferred over raw Tls (not websocket) using 0 cost deserialization with the `rkyv crate`
The Tls handshake requires both server and client authentication certificates.

An api instance for a `DataVendor` is just an object that implements `VendorApiResponse` trait
All 'DataVendor' and `Brokerage` Responses are `DataServerResponse` enum variants. 
You can implement your api however you like, as long as you return all the required functions implemented by the specific trait, if your DataVendor or Brokerage doesn't have the ability to return the required data, simple return `DataServerResponse::Error{error: String}`.

Brokerages utilize the `BrokerApiResponse` trait. 

On the client side the function calls are automatically forwarded to the correct api and so no changes to `ClientSideDataVendor` or `ClientSideBrokerage` need to be made

**Note: In the future I may implement different trait 'levels' so we can implement `Level 1 broker` `Level 2 Broker` to allow diversity in broker api integrations which can vary vastly.

All requests made by the engine will use the DataVendor or Brokerage variant in some way.
We are simply sending a request to the server using the DataVendor or Brokerage enum variant, and the server is returning a response with the data we require based on that variant.
```rust
fn example() {
    /// in a strategy we could request the symbols for a market like this
    let vendor = DataVendor::Test;

    /// we will return a result from the fn, it will either be a Vec<Symbol> or a FundForgeError, this error type will contain a variant and a message string to tell us if the error was on the client side, or if the server has some problem fetching data.
    let symbols = vendor.symbols(MarketType::Forex).await.unwrap();
}
```
What the above function actually does is call a function to get the connection to the data server instance associated with that enum variant, then it sends a request for the symbols which also contains the enum variant to the data server, the server requests the correct api using a matching statement for each variant and retrieves the symbols from the correct api implementation, returning them in fund forge format as `Vec<Symbol>`.
`DataVendor::Test` and `Brokerage::Test` do not actually use any live Api endpoints and are instead designed to simulate end points so that we can get a reliable response from the server using pre serialized or hardcoded data to test platform functionality during development and remain broker and data vendor agnostic.

## Setup 
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
                server_name: String::from("fundforge"), //if using my default (insecure) certificates you will need to keep the currecnt server name.
                address:  SocketAddr::from_str("127.0.0.1:8080").unwrap(), //Note that the default toml launch option will only allow local host, if using a remote instance please make your own certificates
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

### Rithmic Credentials and Setup
To use rithmic API's you will need to request a dev kit for RProtocol (Proto Buffer) from rithmic, you will then need to complete the app conformance procedure.
You will need to have a unique app name in your rithmic_credentials.toml files.

## Parsing Data and Time handling
This is likely to change in the future by making the code more sophisticated but currently all data
should be saved as 1 file per month and all times for data should be Utc time, use the time parsing functions in `ff_standard_lib::helpers::converters` to parse time from your time zone, 
these functions use `chrono-tz` and will automatically handle historical time zone conversions such as daylight savings times. All Base data time properties are serialized as Strings, these strings are auto parsed into `DateTime<Utc>` using `base_data.time_utc()` or. `DateTime<FixedOffset>` using `base_data.time_local(Tz)` the reason for parsing to string is simply for easier `ser/de` using `rkyv` until DateTime is better supported.

There are a number of helpers built into `BaseDataEnum impl` which help with parsing and serializing `BaseDataEnum`. for example `BaseDataEnum::format_and_save()` 
can take a large collection of base data and format it into separate files, 1 file per month. There are also functions for checking the earliest or latest base data for a particular subscription which can be useful for updating historical data at regular intervals.

Please see the base_data_enum.rs file for more info when building DataVendor implementations.

When handling historical data it is assumed that the `time` property of `Candle` and `QuoteBar` object is the opening time, so in the historical data requests we add the `base_data_enum.resolution.as_seconds()` or `base_data_enum.resolution.as_duration()`  to get the `base_data_enum.time_created()` which represents the closing time of the bar. 
To properly align the historical candles and quotebars with other historical data types such as ticks, which represent a single instance in time and therefore do not need to be adjusted. To avoid look ahead bias on our bars during backtesting, a tick that occurred at say 16:00 will be correctly aligned with the bar that closed at 16:00 instead of the bar that opened. This also allows us to reliably combine historical data feeds of different resolutions.

I will be keeping DateTime<Utc> as the standard for the application, this will never change, all future timezone confusion can be avoided by parsing data to UTC as soon as it is received from the DataVendor.

## Decimal Accuracy
Using a new type pattern for Price and Volume, both are rust decimals. This adds some additional work when working with price or volume, but has the advantage of accuracy for crypto and fx products.
```rust 
pub type Volume = rust_decimal::decimal::Decimal;
pub type Price = rust_decimal::decimal::Decimal;
```

## Strategy Registry
The strategy registry service is a server where running strategies register and forward `StrategyEvents` to the Gui.

This service will allow the Ui to find any strategy that is online via Tls/Tcp and communicate by sending commands and receiving updates in an async manner. It will also keep a record of all strategy events that are forwarded by strategies to allow us to replay strategies and see what has happened.

There will be an option to store data received by strategies, so we can see exactly what data the strategy had access to when live trading rather than depending on historical data which may contain data points the strategy never actually received.

The registry will also forward commands like pause, play and stop or allow us to add drawing objects to strategies which they will then be able to interact with depending on the `StrategyInteractionMode`.

Currently, the strategy registry is moving from a broken to working state intermittently as I decided to focus on the engine functionality for the short term.

## Work in progress
I am making it very easy to implement new `Brokerage` and `DataVendor` apis, so that we can have a wide range of options for users to choose from.
Simply create an api object which implements the `VendorApiResponse` or `BrokerApiResponse` trait then you can use it in your strategy or UI.

The client side is already handled, but if you want to add new request/response types you can just add them to the `VendorApiResponse` or `BrokerApiResponse` trait and implement them in your api object and 
`ClientSideDataVendor` or `ClientSideBrokerage` object. 
Since I am avoiding dynamic dispatch in favour of using an enum variant for each api object this will require you to complete a matching statement for all existing api implementations.

It is easy to add a new `DataServerResponse` and `DataServerRequest` variants to handle new api requirements.




# Fund Forge
fund-forge is an algorithmic trading engine written in rust. \

Take a quick look at [strategy features here](https://github.com/BurnOutTrader/fund-forge/blob/main/ff_strategies/README.md)
You will need to complete the setup outlined below to run the test strategy.

It is currently not able to trade live and is using only a faux `Test` api implementation to help build the standardised models which will aid future api intergrations.
It is designed to allow maximum utility of retail trading api's by limiting the need to have duplicate api instances. \

All strategies share a single api instance for each brokerage or data vendor by connecting via Tls/TCP to your `ff_data_server` instance/s. \
This will allow us to use collocation services for running strategies on cloud hardware and will also allows a microservices structure for managing api instances.

I have tested running the data server remotely, it only adds a few seconds to backtests even at low data resolutions, this means we will be able to have our data server running on a server and keep a permanent copy of historical data in the cloud, while still back testing locally. \
All streaming data feeds etc can be shared and only 1 stream per symbol is maintained regardless of the number of running strategies, any data of higher resolution than the primary data stream will be automatically consolidated on the strategy side by the strategies `SubscriptionHandler`. \
This means if we have a DataVendor with a tick stream, we can subscribe to 15 minute Candles and the  engine will create those candles in real time. \
The Current state of the engine is implementing a  `Brokerage::Test` and `DataVendor::Test` variant as a means to develop standardised api requirements.
Strategies are to be run as individual rust programs, directly on your machine, in docker, or on cloud services like [linode](https://www.linode.com/lp/refer/?r=861446255d0586038773b79b486fea8fef9e9c70).

You can contact me by creating a git-hub issue or at my project email: BurnOutTrader@outlook.com this is not my main email, but I will try to keep an eye on it, please just create an issue if you have any questions.
The current repo is likely to receive a lot of changes and updates, some things will break or be completely overhauled, I recently conducted a major refactor to go from synchronous communication with the data server to using a callback system so much of the functionality, like charting etc is temporarily broken.
I am prone to doing a major overhaul to add a new feature or improve the design, since we are not ready for live trading this shouldn't be an issue.

If you want to work on Rithmic Api, you will need to apply for your own dev kit and credentials from rithmic, you will also need to complete the rithmic conformance procedure, 
since fund forge is not a company each user must do this and create their own unique app name to pass conformance. please contact [rithmic](https://yyy3.rithmic.com/?page_id=17).
The skeleton of my initial rithmic api is [here](https://github.com/BurnOutTrader/ff_rithmic_api) inside the ff_standard_lib there is another rithmic api object which uses the aforementioned project as a dependency (already included in cargo.toml as a git link)

## Warning 
Please do not launch your data server on a public address, despite using Tls it is currently suitable for private local host only.
I am not a professional software developer.

If you manage to get this live trading before me, then you will need to test properly, there will be bugs.

## Incomplete: current state
- Daily, Weekly or Monthly resolution subscriptions will have custom consolidators based on market hours, this is because data vendors have an inconsistent definition of daily bars.
  I will build custom consolidators for these types of resolutions in the future.
- Currently building a Rithmic API as the first live trading and back testing api. I did build an Oanda implementation, but they closed my live account without any reason (Not salty but I am not sure what the misunderstanding was) so I have scraped that for now as I needed to proceed with development, I will come back to oanda api later.
- Backtest ledgers and statistics very crude and incomplete/inaccurate.
- Only TEST variants API is working, which is just a hard coded .
- Currently working on simulating data streams and order updates for 'TEST' variants, while securing a rithmic account to continue rithmic API development
- Docker builds have not been tested recently and probably will not work.

## Current Objectives
- Complete all simulated functionality for the TEST api variants.
- Complete full Rithmic functionality after conformance is approved. 
- I will provide affiliate links to allow people to support the development as an alternative to direct donations. Each firm has its own advantages and disadvantages, I have never promoted either firm in any other place.
I will be doing this for all brokerages and prop firms which i use personally, I don't care if you use my affiliate links or not, my only concern is making money trading.
I personally don't normally do this sort of thing but since I am going to build both api variants for the following 2 trading firms for my own purposes I might as well offer the option.
I think these will be a good way to test the engine once I have Rithmic approval, I will build some live brokerages into the engine after I am happy with live trading development.
  - Complete Rithmic Api for [Apex Trader Funding](https://apextraderfunding.com/member/aff/go/burnouttrader) Affiliate coupon LISUNZQY (I am awaiting approval and information on connection to their server)
  - Complete Rithmic Api for [TopStep](https://www.topstep.com/) currently no affiliate. (I am awaiting approval and information on connection to their server)
- Complete at least 1 crypto, 1 forex/cfd and 1 equities api.
- Improve event driven functions for live data/trading scenarios by testing completed apis.
- Complete overhaul for the ledger and market handlers.
- Complete the back testing functionality by running test strategies on local paper ledger in parallel with live paper trading, to compare results and create a new ledger model.
- Conduct live testing
- Lock down handler and strategy Architecture to avoid breaking changes in future versions.
- Slowly improve performance by updating individual components as I learn and experiment more.
- Add more indicators including support for multi symbol indicators
- finish charting and gui api development.
- Add support for building strategies in other languages while using the rust engine and backend. This will be done via a mix of json and a c-types interface to convert from rust data types to a general purpose interface for other languages.

## Licence and Disclaimer
The project was a way for me to learn rust and build a portfolio of useful projects, my desire is to try and keep the engine itself open source, where I might get help with development from more seasoned developers. \
I haven't had any formal coding experience and I have learned by building this project after learning basic coding making strategies with pro real time, ninjatrader and quantconnect. \
After reading the above statement it should go without saying that you should not expect to use this project for live trading for the forseable future without conducting thorough paper testing. \
I have started building a GUI in iced for a full rust implementation, I have also experimented with a Tauri gui, which would use a rust backend and javascript front end so that we could use the lightweight charts api made by Trading View, \
the current problem with the TradingView option is that lightweight charts free api does not support Renko or Heikin Ashi charts, so I temporarily halted development on this option in favour of completing a [rust charting](https://youtu.be/BU9TU3e1-UY) api using iced.rs, \
I am considering making a cheap paid cross-platform GUI in the future so that I can use the Trading View's `Platform edition api` which requires to have an established company, for this reason I will likely include only a single prohibition in the licence, to restrict commercial use in regard to re-selling any part of the fund-forge repository. \
I would like the platform to be 100% open source but realistically to get the best maintainable development of a GUI I might need to look into this option should the repo gain popularity. \
This would allow me to pay front end developers to help with the GUI development and also pay any licensing fees associated with TradingView. \
I won't know how realistic this option is until I know how many people are using the engine, I will continue development of the rust charting api until I have a better sense of the overall direction and popularity (since this is a fairly niche repo). \
Since the engine is open source you will need to undergo the rithmic verification independently, you can see Rithmic section below.
[Current Licence](https://github.com/BurnOutTrader/fund-forge/blob/main/LICENCE.md)

## Fund Forge /src 
Fund forge /src is just for testing random functionality during development, it will not be used in the future, please use the test_strategy folder for testing and developing.

## Demonstration Testing Data
you can download data I have already parsed [here](https://1drv.ms/f/s!AllvRPz1aHoThKEZD9BHCDbvCbHRmg?e=fiBcr3)
Password "fundforge"

The parsed data includes Quote data for AUD-CAD and EUR-CAD from start of 06/2024 to end of 08/2024.
From this data your strategy will consolidate and Candles or Quotebars of any desired resolution, with open bar values being accurate to the latest quoted bid ask.

### For more testing and development data
You can download some free testing data [here](https://www.histdata.com/download-free-forex-data/?/ascii/tick-data-quotes)
Tick data from histdata.com will actually be parsed into the engine as `BaseDataEnum::Quotes(Quote)`
Since the tick data is actually best bid and ask data.
There is a crude test data parser [here](https://github.com/BurnOutTrader/fund-forge/tree/main/test_data_parser) 
You will need to manually download the files, then put all the .csv files into 1 folder and change the variables such as input/output folders and Symbol of the data.
change the following to suit the symbol and your directory.
```rust
const YOUR_FOLDER_PATH: String = "".to_string();
const SYMBOL_NAME: String = "".to_string();
```

After running the parsing program copy-paste the generated 'TEST' folder into ff_data_server/data

## Setup
I will simplify this setup in the future.
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
                address:  SocketAddr::from_str("127.0.0.1:8080").unwrap(), 
                address_synchronous: SocketAddr::from_str("127.0.0.1:8081").unwrap() //all communication is now async only using a callback system
            }
        }
    }
```

### Rithmic Credentials and Setup
To use rithmic API's you will need to request a dev kit for RProtocol (Proto Buffer) from rithmic, you will then need to complete the app conformance procedure.
You will need to have a unique app name in your rithmic_credentials.toml files.

## Strategies
Once you have followed the setup instructions below, you can play with a test strategy by reviewing [Strategies Guide](https://github.com/BurnOutTrader/fund-forge/blob/main/ff_strategies/README.md)

## Gui
Decoupled.
Basic charting functionality was tested months ago. The code base has since been refactored and charting now supports live streams. A local gui is in production using rust iced.
Old video of testing charting algorithm [Initial Charting Api](https://youtu.be/BU9TU3e1-UY)
I will complete a charting API in the future but since I am not experienced with GUI development functionality will be limited to charting only in the short term.
Unfortunatley the learning curve for GUI development in iced is rather steep and it is the only appropriate rust option for the type of GUI I am trying to build.
All Gui development is total decoupled from the engine by using the ff_strategy_registry as an intermediate server for forwarding messages between strategies and gui's
after the last refactor the strategy registry is not in a working state, but is easily fixed in the future.

## Architecture
I have tried to maintain a reasonable separation of concerns throughout the code base to allow any backend implementations to be upgraded without effecting existing strategies.
Some of the current implementations are a little but crude, as a solo developer with limited rust experience I decided to just keep pushing forward and worry about optimization and perfection of various functions once I have a product capable of live testing.
The final architecture of the engine and associated handlers has not been locked down, I am experimenting with different object-oriented and event driven designs.

All strategy functionality is accessed by calling the `FundForgeStrategy` object's associated functions, there is essentially a complete decoupling of strategy instance from the backend so that
upgrades can be implemented from the backend engine and handlers without causing breaking changes to strategies.

I am willing to accept improvements and pull requests that do not include any kind of binary file (all vendor data is serialized as binaries).
All pull requests should include only human-readable code and files.

The platform is designed to be as fast as possible, using `rkyv` for serialization and deserialization and network messaging, and `tokio` for async communication.
The full potential of using rkyv will be unlocked in future versions, currently it is only the most basic implementation of serializing and deserializing to and from archived bytes.
see: [rkyv](https://github.com/rkyv/rkyv)
see tests [here](https://github.com/BurnOutTrader/fund-forge/blob/main/ff_standard_lib/README.md)

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
Currently the market handler is very crude, backtesting results are innacurate, it is for development purposes only.

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






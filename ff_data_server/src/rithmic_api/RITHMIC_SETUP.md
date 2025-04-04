## Rithmic
### Rithmic Notes
When backtesting with rithmic, all historical data is currently saved as DataVendor::Rithmic data.

Rithmic Historical data is capped at around 40 Gb per week.

### Rithmic Setup
To use the rithmic api, you have to apply for a dev kit from rithmic and pass conformance, this is just a matter of creating a unique app name to pass into the RithmicClient;

Then you just follow the information you will get from rithmic, which is essentially just:
1. Contact rithmic and get the [dev kit](https://www.rithmic.com/apis)
1. prepend a message to your app name. (do not use fund forge as app name, it is already used)
2. login with the api and stay connected to the rithmic test end point's while rithmic's engineers do some work approving your application name.
3. receive back information required to complete the rithmic toml files in fund forge.

Rithmic conformance is easy to pass just put the test details given to you by rithmic into a new .toml file at `ff_data_server/data/rithmic_credentials/active/test.toml`
You can find the template file in `ff_data_server/data/rithmic_credentials/inactive`, just fill it out and move it to the active folder.

Only credentials files in the `active` directories will be used by the server.

If you are using a real brokerage you will need the correct fcm and IB id's, these will be returned as a print line when you attempt to login to rithmic (when you launch the data server).
If your ff_data_server prints this message "1088", "user has no permission to this account", Then you probably have the wrong FCM or IB id.
```
Show Orders Response (Template ID: 321) from Server: ResponseShowOrders { template_id: 321, user_msg: [], rp_code: ["1088", "user has no permission to this account"] }
Subscribe For Order Updates Response (Template ID: 309) from Server: ResponseSubscribeForOrderUpdates { template_id: 309, user_msg: [], rp_code: ["1088", "user has no permission to this account"] }
```

To correct this just look at the print line before this message, It will contain the correct FCM and IB id's, just change these in the credentials file and restart the server.
```
ResponseLogin { template_id: 11, template_version: Some("5.29"), user_msg: [], rp_code: ["0"], fcm_id: Some("AMPClearing"), ib_id: Some("AMP"), country_code: Some("AU"), state_code: None, unique_user_id: Some("xxxxxxxx"), heartbeat_interval: Some(60.0) }:PnlPlant
```

`rp_code: ["0"]` In any message from rithmic indicates success.

Since Fund Forge is not a company, each user must do this, you can find more information at [Rithmic](https://www.rithmic.com/apis).

## Rithmic Systems For Testing
I have had trouble logging in with rithmic paper accounts which are given by brokers, this is possibly because you are required to pay for the rithmic api with real brokerages.
You can use an online prop firm accounts without any problems.
Some prop firms do not want algo traders. 
Of all that I have spoken to TopStep seems to not have a problem with algo traders.
Be aware prop firms do not offer support for this platform, you should use RTrader to flatten and monitor accounts.

I am using:
- [Apex](https://apextraderfunding.com/member/aff/go/burnouttrader) I have an affiliate coupon: `LISUNZQY`
- [TopstepTrader](https://www.topsteptrader.com/) I have no affiliate yet.
- [TakeProfitTrader](https://takeprofittrader.com/) I have no affiliate yet.

## Using multiple rithmic systems
If using multiple rithmic systems only 1 system will be used for the data connection.
The Rithmic4Colo system is priority,
Rithmic01 is the next priority,
If none of these systems are active, then the server will use the first rithmic system in the active files list as the data connection.
If you are paying for data upgrades with your prop firm, you should determine which system is being used for data.

If you are using Rithmic4Colo or Rithmic01 then you only need data upgrades on 1 system, the other system will use the same data feed.

Multiple order and pnl connection will be maintained for each individual rithmic system, allowing you to trade multiple accounts with different brokers.

## File Structure
The active folder will contain the credentials for the rithmic systems you intend to use.

You can store login details for other systems in the inactive folder, the server will not login to these connections.

You only need data active for 1 rithmic connection, unless you want multiple rithmic data feeds to use for `DataVendor` subscriptions.

![folder_structure.png](misc/folder_structure.png)

For each rithmic RithmicSystem you intend to use, you will need to create a rithmic .toml file for the credentials.

You can set the variable `subscribe_data = false` if you only want to use the brokerage and not the data feeds for that system.

You will need to use the following folder/file structure

This is what you should call the files for each rithmic system.
```rust
pub enum RithmicSystem {
    Rithmic01,
    Rithmic04Colo,
    RithmicPaperTrading,
    TopstepTrader,
    SpeedUp,
    TradeFundrr,
    UProfitTrader,
    Apex,
    MESCapital,
    TheTradingPit,
    FundedFuturesNetwork,
    Bulenox,
    PropShopTrader,
    FourPropTrader,
    FastTrackTrading,
    Test
}

pub fn from_file_string(file_name: &str) -> Option<Self> {
    match file_name {
        "rithmic_01.toml" => Some(RithmicSystem::Rithmic01),
        "rithmic_04_colo.toml" => Some(RithmicSystem::Rithmic04Colo),
        "rithmic_paper_trading.toml" => Some(RithmicSystem::RithmicPaperTrading),
        "topstep_trader.toml" => Some(RithmicSystem::TopstepTrader),
        "speedup.toml" => Some(RithmicSystem::SpeedUp),
        "tradefundrr.toml" => Some(RithmicSystem::TradeFundrr),
        "uprofit_trader.toml" => Some(RithmicSystem::UProfitTrader),
        "apex.toml" => Some(RithmicSystem::Apex),
        "mes_capital.toml" => Some(RithmicSystem::MESCapital),
        "the_trading_pit.toml" => Some(RithmicSystem::TheTradingPit),
        "funded_futures_network.toml" => Some(RithmicSystem::FundedFuturesNetwork),
        "bulenox.toml" => Some(RithmicSystem::Bulenox),
        "propshop_trader.toml" => Some(RithmicSystem::PropShopTrader),
        "4prop_trader.toml" => Some(RithmicSystem::FourPropTrader),
        "fasttrack_trading.toml" => Some(RithmicSystem::FastTrackTrading),
        "test.toml" => Some(RithmicSystem::Test),
        _ => None,
    }
}
```

***After passing conformance: If the servers.toml is not already in your repo.***
You will need to populate the servers.toml file at ff_data_server/data/rithmic_credentials and fill in the server domains given to you by rithmic.
this is to generate a BTreeMap for Servers where Key is RithmicServer (eg: RithmicServer::Chicago) and value is the domain (eg: wss://{DETAILS_FROM_RITHMIC})
`ff_data_server/data/rithmic_credentials/server_domains/servers.toml`
```toml
[rithmic_servers]
Chicago = "You need to contact rithmic for this"
Sydney = "You need to contact rithmic for this"
SaoPaolo = "You need to contact rithmic for this"
Colo75 = "You need to contact rithmic for this"
Frankfurt = "You need to contact rithmic for this"
HongKong = "You need to contact rithmic for this"
Ireland = "You need to contact rithmic for this"
Mumbai = "You need to contact rithmic for this"
Seoul = "You need to contact rithmic for this"
CapeTown = "You need to contact rithmic for this"
Tokyo = "You need to contact rithmic for this"
Singapore = "You need to contact rithmic for this"
Test = "You need to contact rithmic for this"
```

For each RithmicSystem system you intend to use you will need a RithmicCredentials file in `ff_data_server/data/rithmic_credentials/active`
The templates files can be found in `ff_data_server/data/rithmic_credentials/inactive`

## Historical Data
Rithmic Historical data is capped at around 20 Gb per week.
(You can circumvent this by using multiple rithmic systems, prop firm accounts are a cheap way to speed up the initial data acquisition, for each prop firm you can download an additional 20gb per week)

To download historical data you need to add the symbols to the download list for the specified brokerage.
The download list can be found in ff_data-server/data/credentials/{Brokerage}_credentials/download_list.toml (see folder structure above).

Keep in mind rithmic limits history to 40gb per week per user, if you go over this limit then your historical data for live warm up will not be up to date.
Limit downloads to symbols you trade, if only backtesting, remove symbols from the list once you have the data you need.

The symbols should fund forge format, in fund forge `-` is used to replace `/` or `_` or any other symbols that are in the symbol name.

We also specify the BaseDataType
Rithmic Historical BaseDataTypes:
Ticks,
Candles,
```toml
symbols = [
    { symbol_name = "MNQ", base_data_type = "Ticks"},

    # If you want to specify a start date for the historical data, to avoid getting all the data, the server will only update from this date forwards.
    # You can change this date at any time in the toml, and on the next server launch the server will start downloading from the new date, up to the start of any existing data.
    # The server will also do this at run time, during its update cycle if you don't want to stop the server.
    { symbol_name = "MNQ", base_data_type = "Ticks", start_date = "2024-06-01"} 
]
```

Rithmic data starts from 2012, the tick seems to start from 2/6/2019.
If we want earlier data we will probably need to buy data sets and parse into fund forge format, I will make tools for this in the future.
I will also work on support for other historical resolutions, like 1 min.

Any symbols we specify in the `download_list.toml` file will be downloaded to the data directory, the historical data will be updated every 30 minutes as long as the server is running,
or if we actively subscribe to data it will be updated each time a new subscription event occurs.

You don't need to stop the server to add new symbols to the download list, just add the symbols to the list and the server will start downloading the new symbols at the next download interval.
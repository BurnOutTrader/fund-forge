# Oanda Setup
## Live Trading Will be Available Soon.

Please be aware no live trading functionality has been tested, Oanda accounts are in alpha.

Accounts need to be non-hedging or you will have major issues.

Below is an example of a hedging and non hedging account.

Please do not use a hedging account.
![hedging.png](misc/hedging.png)

## Credentials
In the following directory you need to setup the credential file, there is a template file in the `oanda_credentials/inactive`.
You need to fill it out and copy it into the `oanda_credentials/active` directory.

Only credentials files in active directories will be used by the server.

![file_structure.png](misc/file_structure.png)

Depending on if you are using a live or practice account you input the mode as "Practice" or "Live" respectively
```toml
api_key = "your-api-key-here"
mode = "Practice"  # "practice" or "live"
```

## Historical Data 
To download historical data you need to add the symbols to the download list for the specified brokerage.
The download list can be found in ff_data-server/data/credentials/{Brokerage}_credentials/download_list.toml (see folder structure above).

The symbols should fund forge format, in fund forge `-` is used to replace `/` or `_` or any other symbols that are in the symbol name.

We also specify the BaseDataType
Oanda Historical BaseDataTypes:
QuoteBars,

```toml
symbols = [
    { symbol_name = "NAS100-USD", base_data_type = "QuoteBars" },

    # If you want to specify a start date for the historical data, to avoid getting all the data, the server will only update from this date forwards.
    # You can change this date at any time in the toml, and on the next server launch the server will start downloading from the new date, up to the start of any existing data.
    # The server will also do this at run time, during its update cycle if you don't want to stop the server.
    { symbol_name = "NAS100-USD", base_data_type = "QuoteBars", start_date = "2024-06-01"}
]
```

Since we are downloading the lowest resolution data, the full Oanda data set would be about 80Gb from 2005 to current using 5 second quote bars.

Any symbols we specify in the `download_list.toml` file will be downloaded to the data directory, the historical data will be updated every 30 minutes as long as the server is running, 
or if we actively subscribe to data it will be updated each time a new subscription event occurs.

You don't need to stop the server to add new symbols to the download list, just add the symbols to the list and the server will start downloading the new symbols at the next download interval.

## Live Oanda Strategies
Since Oanda Uses quote data, but there is no historical quote data, live warm up does not work with Oanda.
Indicators and DataSubscriptions can still have instant history, if you subscribe after the strategy is warmed up.
Use the Warm-up complete strategy event for this: 
```rust
 StrategyEvent::WarmUpComplete => {
    let msg = String::from("Strategy: Warmup Complete");
    println!("{}", msg.as_str().bright_magenta());
    warmup_complete = true;

    let sub = DataSubscription::new(
        SymbolName::from("NAS100-USD"),
        DataVendor::Oanda,
        Resolution::Seconds(5),
        BaseDataType::QuoteBars,
        MarketType::CFD
    );
    strategy.subscribe(sub.clone(), 100, false).await;
    let data = strategy.bar_index(&sub, 0);
    println!("Strategy: Bar Index: {:?}", data);
}
```

OR you can subscribe directly to the resolution if you know you have historical data for the subscription.
```rust
let sub = DataSubscription::new(
    SymbolName::from("NAS100-USD"),
    DataVendor::Oanda,
    Resolution::Seconds(5),
    BaseDataType::QuoteBars,
    MarketType::CFD
);
strategy.subscribe_override(sub.clone(), 100, false).await;
```


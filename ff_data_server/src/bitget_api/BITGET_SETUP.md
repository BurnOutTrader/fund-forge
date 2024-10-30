# BitGet Api
You will need to create a bitget_credentials.toml here

![img.png](misc/img.png)


```toml
api_key = ""
secret_key = ""
passphrase = ""
```

## Historical Data
Not yet implemented for bitget, coming asap.

To download historical data you need to add the symbols to your the download list for the specified brokerage.
The download list can be found in ff_data-server/data/credentials/{Brokerage}_credentials/download_list.toml (see folder structure above).

The symbols should fund forge format, in fund forge `-` is used to replace `/` or `_` or any other symbols that are in the symbol name.
```toml
symbols = [
    "BTC-USDT",
]
```
Since we are downloading the lowest resolution data, the full Oanda data set would be about 80Gb from 2005 to current using 5 second quote bars.

Any symbols we specify in the `download_list.toml` file will be downloaded to the data directory, the historical data will be updated every 30 minutes as long as the server is running,
or if we actively subscribe to data it will be updated each time a new subscription event occurs.

You don't need to stop the server to add new symbols to the download list, just add the symbols to the list and the server will start downloading the new symbols at the next download interval.

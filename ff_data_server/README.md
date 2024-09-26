
## Rithmic
If you would like to work on the Rithmic API, you will need to apply for your own dev kit and credentials from Rithmic. Additionally, you will need to complete the Rithmic conformance procedure.

Since Fund Forge is not a company, each user must do this and create their own unique app name to pass conformance. You can find more information at [Rithmic](https://yyy3.rithmic.com/?page_id=17).

The skeleton of my initial Rithmic API is available [here](https://github.com/BurnOutTrader/ff_rithmic_api). Inside the `ff_standard_lib`, there is another Rithmic API object that uses the aforementioned project as a dependency (already included in `Cargo.toml` as a git link).

You will need to create a servers.toml file at ff_data_server/rithmic_credentials and fill in the server domains given to you by rithmic.
this is to generate a BTreeMap for Servers where Key is RithmicServer (eg: RithmicServer::Chicago) and value is the domain (eg: wss://{DETAILS_FROM_RITHMIC})
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

For each rithmic RithmicSystem you intend to use, you will need to create a rithmic .toml file for the credentials.

for each RithmicSystem system you intend to use you will need  RithmicCredentials file in ff_data_server/rithmic_credentials.
The file name is created using `credentials.system_name.file_string()`. This allows the credentials to be found by the data server.

save the toml file as `ff_data_server/rithmic_credentials/credentials.system_name.file_string();`
```rust
pub struct RithmicCredentials {
    pub user: String,
    pub server_name: RithmicServer,
    pub system_name: RithmicSystem,
    pub password: String,
}

pub fn example() {
    let credentials = RithmicCredentials {
        user: "Example trader".to_string(),
        server_name: RithmicServer::Chicago,
        system_name: RithmicSystem::TopStep,
        password: "password".to_string()
    };
    
    // Note that we use credentials.system_name.file_string() for the file name, so that the server knows where to find credentials.
    let save_path: String = format!("ff_data_server/rithmic_credentials/{}", credentials.system_name.file_string());
    credentials.save_credentials_to_file(&save_path);
}
```


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





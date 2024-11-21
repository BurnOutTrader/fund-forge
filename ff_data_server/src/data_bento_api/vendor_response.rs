use std::num::NonZeroU64;
use async_trait::async_trait;
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use crate::server_features::server_side_datavendor::VendorApiResponse;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::enums::{FuturesExchange, MarketType, PrimarySubscription, StrategyMode};
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, Symbol, SymbolName};
use ff_standard_lib::StreamName;
use chrono::{DateTime, Utc};
use databento::dbn::{Schema, TradeMsg};
use databento::historical::symbology::{SymbologyClient};
use databento::historical::timeseries::GetRangeParams;
use indicatif::{ProgressBar};
use time::macros::{datetime};
use ff_standard_lib::product_maps::rithmic::maps::{get_futures_exchange, get_futures_symbol_info};
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use crate::data_bento_api::api_client::DataBentoClient;
use crate::rithmic_api::products::get_futures_symbols;

#[async_trait]
impl VendorApiResponse for DataBentoClient {
    #[allow(unused)]
    async fn symbols_response(&self,  _mode: StrategyMode, _stream_name: StreamName, market_type: MarketType, time: Option<DateTime<Utc>>, callback_id: u64) -> DataServerResponse{
        match market_type {
            MarketType::Futures(exchange) => {
                let symbols = get_futures_symbols();
                let mut symbols_objects = vec![];
                for symbol_name in symbols {
                    let exchange = match get_futures_exchange(&symbol_name) {
                        Ok(exchange) => exchange,
                        Err(_) => continue
                    };
                    let s = Symbol::new(symbol_name.clone(), DataVendor::DataBento, MarketType::Futures(exchange.clone()));
                    symbols_objects.push(s);
                }
                DataServerResponse::Symbols{callback_id, symbols: symbols_objects, market_type}
            },
            _ => return DataServerResponse::Error{callback_id, error: FundForgeError::ServerErrorDebug("Unsupported market type".to_string())}
        };

        let mut client = self.historical_client.lock().await;
        let mut symbology_client: SymbologyClient = client.symbology();

        /*if let Some(time) = time {
            let year = time.year();
            let month = time.month();
            let day = time.day();
            let month: time::Month = match month {
                1 => time::Month::January,
                2 => time::Month::February,
                3 => time::Month::March,
                4 => time::Month::April,
                5 => time::Month::May,
                6 => time::Month::June,
                7 => time::Month::July,
                8 => time::Month::August,
                9 => time::Month::September,
                10 => time::Month::October,
                11 => time::Month::November,
                12 => time::Month::December,
                _ => return DataServerResponse::Error{callback_id, error: FundForgeError::ServerErrorDebug("Invalid month".to_string())}
            };
            let time = match time::Date::from_calendar_date(year, month, day as u8) {
                Ok(date) => date,
                Err(e) => return DataServerResponse::Error{callback_id, error: FundForgeError::ServerErrorDebug(format!("Failed to create date: {}", e))}
            };

            let date_range = DateRange::from((time.clone(), time));



            let params: ResolveParams = ResolveParams::builder()
                .dataset(dataset)
                .symbols(Symbols::All)
                .stype_in(SType::RawSymbol)
                .stype_out(SType::RawSymbol).date_range(date_range).build();

            let symbols = symbology_client.resolve(
                &params
            ).await.map_err(|e| FundForgeError::ServerErrorDebug(format!("Failed to resolve symbols: {}", e)));


            println!("Symbols: {:?}", symbols);
        }*/
        DataServerResponse::Error{callback_id, error: FundForgeError::ServerErrorDebug("No time provided".to_string())}
    }
    #[allow(unused)]
    async fn resolutions_response(&self, _mode: StrategyMode, _stream_name: StreamName, market_type: MarketType, callback_id: u64) -> DataServerResponse {
        DataServerResponse::Resolutions {
            callback_id,
            subscription_resolutions_types: vec![PrimarySubscription::new(Resolution::Ticks(1), BaseDataType::Ticks), PrimarySubscription::new(Resolution::Instant, BaseDataType::Quotes), PrimarySubscription::new(Resolution::Seconds(1), BaseDataType::Candles), PrimarySubscription::new(Resolution::Minutes(1), BaseDataType::Candles)],
            market_type
        }
    }
    async fn markets_response(&self, _mode: StrategyMode, _stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        DataServerResponse::Markets {
            callback_id,
            markets: vec![
                MarketType::Futures(FuturesExchange::CME),
                MarketType::Futures(FuturesExchange::CBOT),
                MarketType::Futures(FuturesExchange::COMEX),
                MarketType::Futures(FuturesExchange::NYBOT),
                MarketType::Futures(FuturesExchange::NYMEX),
                MarketType::Futures(FuturesExchange::MGEX)
            ],
        }
    }

    async fn decimal_accuracy_response(&self, _mode: StrategyMode, _stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        let info = match get_futures_symbol_info(&symbol_name) {
            Ok(info) => {
                info
            }
            Err(_e) => {
                return DataServerResponse::Error {callback_id, error: FundForgeError::ClientSideErrorDebug(format!("{} Accuracy Info not found with: {}", symbol_name, DataVendor::DataBento))}
            }
        };
        DataServerResponse::DecimalAccuracy {
            callback_id,
            accuracy: info.decimal_accuracy,
        }
    }

    async fn tick_size_response(&self, _mode: StrategyMode, _stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        let info = match get_futures_symbol_info(&symbol_name) {
            Ok(info) => {
                info
            }
            Err(_e) => {
                return DataServerResponse::Error {callback_id, error: FundForgeError::ClientSideErrorDebug(format!("{} Tick Size Info not found with: {}", symbol_name, DataVendor::DataBento))}
            }
        };
        DataServerResponse::TickSize {
            callback_id,
            tick_size: info.tick_size,
        }
    }

    #[allow(unused)]
    async fn data_feed_subscribe(&self, stream_name: StreamName, subscription: DataSubscription) -> DataServerResponse {
       todo!()
    }
    #[allow(unused)]
    async fn data_feed_unsubscribe(&self, stream_name: StreamName, subscription: DataSubscription) -> DataServerResponse {
        todo!()
    }
    #[allow(unused)]
    async fn base_data_types_response(&self,  _mode: StrategyMode, _stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        DataServerResponse::BaseDataTypes {
            callback_id,
            base_data_types: vec![BaseDataType::Ticks, BaseDataType::Quotes, BaseDataType::Candles],
        }
    }

    #[allow(unused)]
    async fn logout_command_vendors(&self, _stream_name: StreamName) {
        todo!()
    }

    #[allow(unused)]
    async fn session_market_hours_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, date_time: DateTime<Utc>, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    /*
        Overview
    Our historical API has the following structure:

    Metadata provides information about the datasets themselves.
    Time series provides all types of time series data. This includes subsampled data (second, minute, hour, daily aggregates), trades, top-of-book, order book deltas, order book snapshots, summary statistics, static data and macro indicators. We also provide properties of products such as expirations, tick sizes and symbols as time series data.
    Symbology provides methods that help find and resolve symbols across different symbology systems.
    Batch provides a means of submitting and querying for details of batch download requests.
    */
    #[allow(unused)]
    async fn update_historical_data(&self, symbol: Symbol, base_data_type: BaseDataType, resolution: Resolution, from: DateTime<Utc>, to: DateTime<Utc>, from_back: bool ,progress_bar: ProgressBar, is_bulk_download: bool) -> Result<(), FundForgeError> {

        let schema = match base_data_type {
            BaseDataType::Quotes => {}
            _ => return Err(FundForgeError::ClientSideErrorDebug(format!("{} Invalid Data Type for: {}", base_data_type, DataVendor::DataBento)))
        };

        let ranges = GetRangeParams::builder()
            .dataset("GLBX.MDP3")
            .date_time_range((
                datetime!(2023-06-06 00:00 UTC),
                datetime!(2023-06-10 12:10 UTC),
            ))
            .symbols(symbol.name)
            .schema(Schema::Trades)
            .limit(NonZeroU64::new(1))
            .build();

        let mut client = self.historical_client.lock().await;

        let mut decoder = client
            .timeseries()
            .get_range(&ranges).await.unwrap();

        match decoder.decode_record::<TradeMsg>().await {
            Ok(trade) => println!("{trade:#?}"),
            Err(e) => eprintln!("Failed to decode: {}", e)
        };

        Ok(())
    }
}
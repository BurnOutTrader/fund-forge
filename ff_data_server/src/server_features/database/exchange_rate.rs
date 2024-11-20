use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use ff_standard_lib::messages::data_server_messaging::FundForgeError;
use ff_standard_lib::product_maps::oanda::maps::OANDA_SYMBOL_INFO;
use ff_standard_lib::standardized_types::accounts::Currency;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::enums::{MarketType, OrderSide};
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::Symbol;
use crate::server_features::database::hybrid_storage::HybridStorage;

impl HybridStorage {
    #[allow(unused)]
    pub async fn get_exchange_rate(&self, from_currency: Currency, to_currency: Currency, date_time: DateTime<Utc>, data_vendor: DataVendor, side: OrderSide) -> Result<Decimal, FundForgeError> {
        let (resolutions, market_type, base_data_type) = match data_vendor {
            DataVendor::Bitget => (vec![Resolution::Minutes(1), Resolution::Hours(1)], MarketType::Crypto, BaseDataType::Candles),
            DataVendor::Oanda => (vec![Resolution::Seconds(5), Resolution::Minutes(1), Resolution::Hours(1)], MarketType::Forex, BaseDataType::QuoteBars),
            _ => return Err(FundForgeError::ServerErrorDebug(format!("Data Vendor not supported for currency conversion: {}", data_vendor)))
        };

        //eprintln!("Getting exchange rate for {}-{} at {}", from_currency.to_string(), to_currency.to_string(), date_time);

        let symbol_name = format!("{}-{}", from_currency.to_string(), to_currency.to_string());
        let has_symbol: bool = match data_vendor {
            DataVendor::Bitget => {
                todo!()
            }
            DataVendor::Oanda => {
                OANDA_SYMBOL_INFO.contains_key(&symbol_name)
            }
            _ => return Err(FundForgeError::ServerErrorDebug(format!("Data Vendor not supported for currency conversion: {}", data_vendor)))
        };
        if has_symbol {
            // Try direct currency pair
            for resolution in &resolutions {
                match self.get_data_point_asof(&Symbol::new(symbol_name.clone(), data_vendor, market_type), resolution, &base_data_type, date_time).await {
                    Ok(Some(data)) => {
                        match data {
                            BaseDataEnum::Candle(candle) => {
                                return Ok(candle.close);
                            },
                            BaseDataEnum::QuoteBar(quote_bar) => {
                                //eprintln!("Quote Bar: {:?}", quote_bar);
                                return match side {
                                    OrderSide::Buy => Ok(quote_bar.ask_close),
                                    OrderSide::Sell => Ok(quote_bar.bid_close),
                                }

                            },
                            _ => return Err(FundForgeError::ServerErrorDebug(format!("Unexpected data type for currency conversion: {:?}", data)))
                        }
                    },
                    Ok(None) => continue,
                    Err(e) => return Err(FundForgeError::ServerErrorDebug(format!("Error getting exchange rate: {}", e))),
                }
            }
        } else {

            // Try inverse currency pair
            let symbol_name = format!("{}-{}", to_currency.to_string(), from_currency.to_string());
            let has_symbol: bool = match data_vendor {
                DataVendor::Bitget => {
                    todo!()
                }
                DataVendor::Oanda => {
                    OANDA_SYMBOL_INFO.contains_key(&symbol_name)
                }
                _ => return Err(FundForgeError::ServerErrorDebug(format!("Data Vendor not supported for currency conversion: {}", data_vendor)))
            };

            if has_symbol {
                for resolution in &resolutions {
                    match self.get_data_point_asof(&Symbol::new(symbol_name.clone(), data_vendor, market_type), resolution, &base_data_type, date_time).await {
                        Ok(Some(data)) => {
                            match data {
                                BaseDataEnum::Candle(candle) => {
                                    return Ok(dec!(1) / candle.close);  // Take reciprocal
                                },
                                BaseDataEnum::QuoteBar(quote_bar) => {
                                    //eprintln!("Quote Bar: {:?}", quote_bar);
                                    return match side {
                                        OrderSide::Buy => Ok(dec!(1) / quote_bar.ask_close),  // Take reciprocal
                                        OrderSide::Sell => Ok(dec!(1) / quote_bar.bid_close),  // Take reciprocal
                                    }
                                },
                                _ => return Err(FundForgeError::ServerErrorDebug(format!("Unexpected data type for currency conversion: {:?}", data)))
                            }
                        },
                        Ok(None) => continue,
                        Err(e) => return Err(FundForgeError::ServerErrorDebug(format!("Error getting exchange rate: {}", e))),
                    }
                }
            }
        }

        // If we get_requests here, we couldn't find either direct or inverse rates
        Err(FundForgeError::ServerErrorDebug(format!(
            "Could not find exchange rate for {}-{} or {}-{} at {}",
            from_currency.to_string(),
            to_currency.to_string(),
            to_currency.to_string(),
            from_currency.to_string(),
            date_time
        )))
    }
}
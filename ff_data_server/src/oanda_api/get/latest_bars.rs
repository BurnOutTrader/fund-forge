use ff_standard_lib::messages::data_server_messaging::FundForgeError;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::Symbol;
use crate::oanda_api::api_client::OandaClient;
use crate::oanda_api::base_data_converters::{candle_from_candle, oanda_quotebar_from_candle};
use crate::oanda_api::support_and_conversions::{oanda_clean_instrument, resolution_to_oanda_interval};

impl OandaClient {
    pub async fn get_latest_bars(
        &self,
        symbol: &Symbol,
        base_data_type: BaseDataType,
        resolution: Resolution,
        account_id: &str,
        units: i32,
    ) -> Result<Vec<BaseDataEnum>, FundForgeError> {
        let interval = resolution_to_oanda_interval(&resolution)
            .ok_or_else(|| FundForgeError::ClientSideErrorDebug("Invalid resolution".to_string()))?;

        let instrument = oanda_clean_instrument(&symbol.name).await;

        // For bid/ask we use "BA" instead of separate "B" and "A" specifications
        let candle_spec = match base_data_type {
            BaseDataType::QuoteBars => format!("{}:{}:BA", instrument, interval),
            _ => return Err(FundForgeError::ClientSideErrorDebug("Unsupported data type".to_string())),
        };

        // Use UTC alignment
        let url = format!(
            "/accounts/{}/candles/latest?candleSpecifications={}&smooth=false&alignmentTimezone=UTC&units={}",
            account_id,
            candle_spec,
            units
        );

        let response = match self.send_rest_request(&url).await {
            Ok(response) => response,
            Err(e) => {
                return Err(FundForgeError::ClientSideErrorDebug(format!("Failed to get_requests latest bars: {}", e)));
            }
        };

        if !response.status().is_success() {
            return Err(FundForgeError::ClientSideErrorDebug(format!(
                "Failed to get_requests latest bars: HTTP {}",
                response.status()
            )));
        }

        let content = response.text().await.map_err(|e| {
            FundForgeError::ClientSideErrorDebug(format!("Failed to get_requests response text: {}", e))
        })?;

        let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            FundForgeError::ClientSideErrorDebug(format!("Failed to parse JSON: {}", e))
        })?;

        let latest_candles = json["latestCandles"].as_array().ok_or_else(|| {
            FundForgeError::ClientSideErrorDebug("No latestCandles array in response".to_string())
        })?;

        let mut bars = Vec::new();

        for candle_response in latest_candles {
            let candles = candle_response["candles"].as_array().ok_or_else(|| {
                FundForgeError::ClientSideErrorDebug("No candles array in response".to_string())
            })?;

            for price_data in candles {
                // Only process complete candles
                if !price_data["complete"].as_bool().unwrap_or(false) {
                    continue;
                }

                let bar: BaseDataEnum = match base_data_type {
                    BaseDataType::QuoteBars => {
                        match oanda_quotebar_from_candle(price_data, symbol.clone(), resolution.clone()) {
                            Ok(quotebar) => BaseDataEnum::QuoteBar(quotebar),
                            Err(e) => {
                                eprintln!("Failed to create quote bar: {}", e);
                                continue;
                            }
                        }
                    },
                    BaseDataType::Candles => {
                        match candle_from_candle(price_data, symbol.clone(), resolution.clone()) {
                            Ok(candle) => BaseDataEnum::Candle(candle),
                            Err(e) => {
                                eprintln!("Failed to create candle: {}", e);
                                continue;
                            }
                        }
                    },
                    _ => continue,
                };

                bars.push(bar);
            }
        }

        bars.sort_by_key(|bar| bar.time_utc());
        Ok(bars)
    }
}
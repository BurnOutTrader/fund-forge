use async_trait::async_trait;
use ff_standard_lib::messages::data_server_messaging::DataServerResponse;
use ff_standard_lib::server_features::server_side_brokerage::BrokerApiResponse;
use ff_standard_lib::standardized_types::enums::StrategyMode;
use ff_standard_lib::standardized_types::new_types::Volume;
use ff_standard_lib::standardized_types::subscriptions::SymbolName;
use ff_standard_lib::strategies::ledgers::AccountId;
use ff_standard_lib::StreamName;
use crate::bitget_api::api_client::BitgetClient;

#[async_trait]
impl BrokerApiResponse for BitgetClient {
    #[allow(unused)]
    async fn symbol_names_response(&self, mode: StrategyMode, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    #[allow(unused)]
    async fn account_info_response(&self, mode: StrategyMode, stream_name: StreamName, account_id: AccountId, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    #[allow(unused)]
    async fn symbol_info_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    #[allow(unused)]
    async fn intraday_margin_required_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, quantity: Volume, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    #[allow(unused)]
    async fn overnight_margin_required_response(&self, _mode: StrategyMode, _stream_name: StreamName, symbol_name: SymbolName, quantity: Volume, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    #[allow(unused)]
    async fn accounts_response(&self, mode: StrategyMode, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    #[allow(unused)]
    async fn logout_command(&self, stream_name: StreamName) {
        todo!()
    }
}
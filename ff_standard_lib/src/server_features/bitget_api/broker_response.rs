use async_trait::async_trait;
use crate::messages::data_server_messaging::DataServerResponse;
use crate::server_features::bitget_api::api_client::BitgetClient;
use crate::server_features::server_side_brokerage::BrokerApiResponse;
use crate::server_features::StreamName;
use crate::standardized_types::enums::StrategyMode;
use crate::standardized_types::new_types::Volume;
use crate::standardized_types::subscriptions::SymbolName;
use crate::strategies::ledgers::AccountId;

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
    async fn margin_required_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, quantity: Volume, callback_id: u64) -> DataServerResponse {
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
use ff_standard_lib::messages::data_server_messaging::FundForgeError;
use ff_standard_lib::standardized_types::accounts::Account;
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use crate::oanda_api::api_client::OandaClient;

/// Functions to manage the Oanda Account
/// Returns the list of accounts available for the Oanda brokerage
pub(crate) async fn get_oanda_accounts_list(oanda_client: &OandaClient) -> Result<Vec<Account>, FundForgeError> {
    let request_uri = "/accounts".to_string();

    let response = match oanda_client.send_rest_request(&request_uri).await {
        Ok(response) => response,
        Err(e) => {
            return Err(FundForgeError::ServerErrorDebug(format!("Failed to get_requests the account list from the server: {:?}", e)));
        }
    };

    let content = response.text().await.unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    let downloaded_accounts = json["accounts"].as_array().unwrap();

    if downloaded_accounts.len() == 0 {
        return Err(FundForgeError::ServerErrorDebug("No accounts found for oanda".to_string()));
    }

    let mut accounts : Vec<Account> = Vec::new();
    for account in downloaded_accounts {
        let id = account["id"].as_str().unwrap().to_string();
        let account = Account::new(Brokerage::Oanda, id.clone());
        accounts.push(account);
    }
    Ok(accounts)
}
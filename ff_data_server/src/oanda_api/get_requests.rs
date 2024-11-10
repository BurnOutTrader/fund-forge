use ff_standard_lib::messages::data_server_messaging::FundForgeError;
use ff_standard_lib::standardized_types::accounts::Account;
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::subscriptions::{SymbolName};
use crate::oanda_api::api_client::OandaClient;
use crate::oanda_api::instruments::OandaInstrument;
use crate::oanda_api::models::account::account::OandaAccount;
use crate::oanda_api::models::account::account_summary::AccountSummary;

pub(crate) async fn oanda_instruments_download(oanda_client: &OandaClient, account: &str) -> Option<Vec<OandaInstrument>>{
    let url = format!("/accounts/{}/instruments", account);

    let response = oanda_client.send_rest_request(&url).await.unwrap();
    if !response.status().is_success() {
        println!("Error getting instruments: {:?}", response)
    }

    let content = match response.text().await {
        Ok(content) => content,
        Err(e) => {
            println!("Error getting instruments: {:?}", e);
            return None;
        }
    };
    
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    let instruments_json = match json["instruments"].as_array() {
        Some(instruments) => instruments,
        None => {
            println!("Error getting oanda instruments: {:?}", json);
            return None;
        }
    };

    if instruments_json.len() == 0 {
        println!("No instruments found for account: {}", account);
        return None;
    }

    let mut instruments: Vec<OandaInstrument> = Vec::new();
    for instrument in instruments_json {
        let instrument_enum = OandaInstrument::from_json(instrument).unwrap();
        instruments.push(instrument_enum);
    }

    Some(instruments)
}

pub(crate) async fn oanda_clean_instrument(symbol_name: &SymbolName) -> SymbolName {
    symbol_name
        .replace("/", "_")
        .replace(":", "_")
        .replace("?", "_")
        .replace("-", "_")
        .replace(" ", "_")
        .to_uppercase()
}

/// Functions to manage the Oanda Account
/// Returns the list of accounts available for the Oanda brokerage
pub(crate) async fn oanda_accounts_list(oanda_client: &OandaClient) -> Result<Vec<Account>, FundForgeError> {
    let request_uri = "/accounts".to_string();

    let response = match oanda_client.send_rest_request(&request_uri).await {
        Ok(response) => response,
        Err(e) => {
            return Err(FundForgeError::ServerErrorDebug(format!("Failed to get the account list from the server: {:?}", e)));
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

#[allow(dead_code)]
pub(crate) async fn oanda_account_summary(oanda_client: &OandaClient, account_id: &str) -> Result<AccountSummary, FundForgeError> {
    let request_uri = format!("/accounts/{}/summary", account_id);

    let response = match oanda_client.send_rest_request(&request_uri).await {
        Ok(response) => response,
        Err(e) => {
            return Err(FundForgeError::ServerErrorDebug(
                format!("Failed to get account summary from server: {:?}", e)
            ));
        }
    };

    let content = match response.text().await {
        Ok(content) => content,
        Err(e) => {
            return Err(FundForgeError::ServerErrorDebug(
                format!("Failed to read response content: {:?}", e)
            ));
        }
    };

    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(json) => json,
        Err(e) => {
            return Err(FundForgeError::ServerErrorDebug(
                format!("Failed to parse JSON response: {:?}", e)
            ));
        }
    };

    // Assuming AccountSummary implements Deserialize
    match serde_json::from_value(json["account"].clone()) {
        Ok(summary) => Ok(summary),
        Err(e) => Err(FundForgeError::ServerErrorDebug(
            format!("Failed to deserialize account summary: {:?}", e)
        ))
    }
}
pub(crate) async fn oanda_account_details(oanda_client: &OandaClient, account_id: &str) -> Result<OandaAccount, FundForgeError> {
    let request_uri = format!("/accounts/{}", account_id);

    let response = match oanda_client.send_rest_request(&request_uri).await {
        Ok(response) => response,
        Err(e) => {
            return Err(FundForgeError::ServerErrorDebug(
                format!("Failed to get account summary from server: {:?}", e)
            ));
        }
    };

    if !response.status().is_success() {
        return Err(FundForgeError::ServerErrorDebug(
            format!("Server returned error status: {}", response.status())
        ));
    }

    let content = match response.text().await {
        Ok(content) => content,
        Err(e) => {
            return Err(FundForgeError::ServerErrorDebug(
                format!("Failed to read response content: {:?}", e)
            ));
        }
    };

    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(json) => json,
        Err(e) => {
            return Err(FundForgeError::ServerErrorDebug(
                format!("Failed to parse JSON response: {:?}", e)
            ));
        }
    };

    eprintln!("Account JSON: {}", json["account"]);

    match serde_json::from_value(json["account"].clone()) {
        Ok(summary) => Ok(summary),
        Err(e) => Err(FundForgeError::ServerErrorDebug(
            format!("Failed to deserialize account summary: {:?}", e)
        ))
    }
}

#[allow(dead_code)]
pub(crate) async fn get_oanda_account_details(oanda_client: &OandaClient, account_id: &str) -> Result<OandaAccount, FundForgeError> {
    let request_uri = format!("/accounts/{}", account_id);

    let response = match oanda_client.send_rest_request(&request_uri).await {
        Ok(response) => response,
        Err(e) => {
            return Err(FundForgeError::ServerErrorDebug(
                format!("Failed to get account details from server: {:?}", e)
            ));
        }
    };

    let content = match response.text().await {
        Ok(content) => content,
        Err(e) => {
            return Err(FundForgeError::ServerErrorDebug(
                format!("Failed to read response content: {:?}", e)
            ));
        }
    };

    //println!("Raw response: {}", content); // Add this for debugging

    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(json) => json,
        Err(e) => {
            return Err(FundForgeError::ServerErrorDebug(
                format!("Failed to parse JSON response: {:?}", e)
            ));
        }
    };

    match serde_json::from_value(json["account"].clone()) {
        Ok(account) => Ok(account),
        Err(e) => {
            // Print the actual JSON structure for debugging
            println!("Account JSON: {}", json["account"]);
            Err(FundForgeError::ServerErrorDebug(
                format!("Failed to deserialize account details: {:?}", e)
            ))
        }
    }
}
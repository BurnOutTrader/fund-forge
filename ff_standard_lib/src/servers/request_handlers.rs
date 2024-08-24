use std::path::PathBuf;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use crate::apis::brokerage::server_responses::BrokerApiResponse;
use crate::apis::vendor::server_responses::VendorApiResponse;
use crate::helpers::converters::load_as_bytes;
use crate::helpers::get_data_folder;
use crate::servers::communications_sync::SynchronousCommunicator;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::data_server_messaging::{BaseDataPayload, FundForgeError, SynchronousRequestType, SynchronousResponseType};
use crate::standardized_types::subscriptions::DataSubscription;

/// Manages sequential requests received through a secondary data receiver and sends responses via a secondary data sender.
///
/// This asynchronous function continuously listens for incoming data requests on the `secondary_data_receiver`.
/// Upon receiving a request, it attempts to parse the request into a known `RequestType`. If parsing fails,
/// an error is logged, and the function continues to listen for the next request. For successfully parsed requests,
/// it delegates to specific handler functions based on the request type (e.g., fetching historical base data,
/// symbols, or resolutions). Each handler function is awaited to process the request and generate a response.
/// If a handler function returns an error, it is converted into a `ResponseType::Error` before sending.
/// Finally, the response is serialized into bytes and sent through the `secondary_sender`. Errors encountered
/// while sending the response are logged.
///
/// # Parameters
/// - `secondary_data_receiver`: A `Receiver<Vec<u8>>` that receives data requests and sends responses.
///
/// # Returns
/// This function does not return a value. It runs indefinitely until the program is terminated or an unrecoverable
/// error occurs on the channels it listens to or sends data on.
pub async fn manage_sequential_requests(communicator: Arc<SynchronousCommunicator>) {
    tokio::task::spawn(async move {
        while let Some(data) = communicator.receive(true).await {
            let request = match SynchronousRequestType::from_bytes(&data) {
                Ok(request) => request,
                Err(e) => {
                    println!("Failed to parse request: {:?}", e);
                    continue;
                }
            };

            let response = match request {
                SynchronousRequestType::HistoricalBaseData { subscriptions, time } => base_data_response(subscriptions, time).await,
                SynchronousRequestType::SymbolsVendor(vendor, market) => vendor.basedata_symbols_response(market).await,
                SynchronousRequestType::SymbolsBroker(broker, market) => broker.symbols_response(market).await,
                SynchronousRequestType::Resolutions(vendor, market_type) => vendor.resolutions_response(market_type).await,
                SynchronousRequestType::AccountCurrency(broker, account_id) => broker.account_currency_reponse(account_id).await,
                SynchronousRequestType::AccountInfo(broker, account_id) => broker.account_info_response(account_id).await,
                SynchronousRequestType::Markets(vendor) => vendor.markets_response().await,
                SynchronousRequestType::DecimalAccuracy(vendor, symbol) => vendor.decimal_accuracy_response(symbol).await,
                SynchronousRequestType::TickSize(vendor, symbol) => vendor.tick_size_response(symbol).await,
            };

            let response = response.unwrap_or_else(|ff_error| SynchronousResponseType::Error(ff_error.into()));
            let bytes = SynchronousResponseType::to_bytes(&response);

            match communicator.send_no_response(bytes, true).await {
                Ok(_) => {}, //println!("Successfully sent data to secondary sender"),
                Err(e) => println!("Error sending data to secondary sender: {:?}", e),
            }
        };
    });
}

/// Retrieves the base data from the file system or the vendor and sends it back to the client via a NetworkMessage using the response function
pub(crate) async fn base_data_response(subscription: DataSubscription, time: String) -> Result<SynchronousResponseType, FundForgeError> {
    let data_folder = PathBuf::from(get_data_folder());
    let time: DateTime<Utc> = time.parse().unwrap();

    // now we can load the data from the file system
    let mut payloads: Vec<BaseDataPayload> = Vec::new();
    let file = BaseDataEnum::file_path(&data_folder,&subscription, &time).unwrap();
    //println!("file path: {:?}", file);
    if file.exists() {
        let data = load_as_bytes(file).unwrap();
        payloads.push(BaseDataPayload {
            bytes: data,
            subscription: subscription.clone()
        });
    }

    // return the ResponseType::HistoricalBaseData to the server fn that called this fn
    Ok(SynchronousResponseType::HistoricalBaseData(payloads))
}

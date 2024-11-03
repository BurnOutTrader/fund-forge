use urlencoding::encode;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::{Symbol};
use crate::oanda_api::get_requests::oanda_clean_instrument;
use crate::oanda_api::support_and_conversions::{add_time_to_date, resolution_to_oanda_interval, Interval};

///generates urls used for downloading intraday candles, always starts from the last time in the existing data else 2005-01-01 (oanda's earliest date.
#[allow(dead_code)]
pub(crate) async fn generate_urls(symbol: Symbol, resolution: Resolution, base_data_type: BaseDataType, start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> Vec<String> {
    let mut urls: Vec<String> = Vec::new();
    let interval = resolution_to_oanda_interval(&resolution);

    let instrument  = oanda_clean_instrument(&symbol.name).await;

    let mut start = start_time.naive_utc().clone();  // Start of the day

    let mut up_to_date = false;
    loop {
            let mut end = start + add_time_to_date(&interval);
            if &end > &end_time.naive_utc() {
                end = (end_time + Duration::minutes(30)).naive_utc().clone();
                up_to_date = true;
            }
            let url = generate_url(&start, &end, &instrument, &interval, &base_data_type);
            urls.push(url);
            start = end;

            //once we have a url up to current date and time, we can break the loop
            if up_to_date {
                break;
        }
    }
    urls
}

///generates each individual url used for downloading candles between a specific date range.
pub(crate) fn generate_url(start: &NaiveDateTime, end_time: &NaiveDateTime, instrument: &str, interval: &Interval, price_data_type: &BaseDataType) -> String{
    let start_string = format!("{}", start.format("%Y-%m-%dT%H:%M:%S%.9fZ"));
    let from_param = encode( & start_string);
    let end_string = format!("{}", end_time.format("%Y-%m-%dT%H:%M:%S%.9fZ"));
    let to_param = encode( & end_string);
    let price = match price_data_type {
        BaseDataType::QuoteBars => "BA",
        BaseDataType::Candles => "M",
        _ => panic!("price_data_type: History not supported for broker")
    };

    format!("/instruments/{}/candles?price={}&from={}&to={}&alignmentTimezone=UTC&granularity={}",
            instrument.to_string(),
            price,
            from_param,
            to_param,
            interval.to_string())
}










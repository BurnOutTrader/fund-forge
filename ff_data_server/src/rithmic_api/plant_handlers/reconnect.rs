use std::sync::Arc;
use ff_rithmic_api::rithmic_proto_objects::rti::request_login::SysInfraType;
use futures::stream::SplitStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tokio::net::TcpStream;
use std::time::Duration;
use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc, Weekday};
use tokio::time::sleep;
use crate::rithmic_api::api_client::RithmicClient;

pub(crate) async fn attempt_reconnect(
    client: &Arc<RithmicClient>,
    plant: SysInfraType,
) -> Option<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>> {
    let mut delay = Duration::from_secs(5); // Initial retry delay
    let max_delay = Duration::from_secs(300); // Maximum delay of 5 mins

    loop {
        let now = Utc::now();
        let chicago_time = now.with_timezone(&chrono_tz::America::Chicago);

        // If it's weekend or after hours, wait until 5 mins before the next market open.
        if is_weekend_or_off_hours(&chicago_time) {
            let next_market_open = next_chicago_market_open(&chicago_time);
            let reconnect_time = next_market_open - chrono::Duration::minutes(5);
            let wait_duration = reconnect_time - now;

            eprintln!(
                "Outside of market hours. Waiting until 5 mins before market open: {}",
                reconnect_time
            );

            sleep(Duration::from_secs(wait_duration.num_seconds() as u64)).await;
        } else {
            match client.connect_plant(plant).await {
                Ok(new_connection) => {
                    println!("Reconnected successfully");
                    return Some(new_connection);
                }
                Err(e) => {
                    eprintln!(
                        "Failed to reconnect: {:?}: {}. Retrying in {:?}",
                        plant, e, delay
                    );
                    sleep(delay).await;
                    delay = (delay * 2).min(max_delay); // Exponential backoff with max limit
                }
            }
        }
    }
}

fn is_weekend_or_off_hours(chicago_time: &DateTime<chrono_tz::Tz>) -> bool {
    let weekday = chicago_time.weekday();
    let hour = chicago_time.hour();

    // Market closes Friday at 5 PM and reopens Sunday at 5 PM (Chicago time).
    let after_friday_close = weekday == Weekday::Fri && hour >= 17;
    let before_sunday_open = weekday == Weekday::Sun && hour < 16;
    let is_weekend = weekday == Weekday::Sat;

    after_friday_close || before_sunday_open || is_weekend
}

fn next_chicago_market_open(chicago_time: &DateTime<chrono_tz::Tz>) -> DateTime<Utc> {
    let mut next_open = chicago_time.date_naive().and_hms_opt(17, 0, 0).unwrap(); // 5 PM

    // If it's Friday or Saturday, the next open is Sunday 5 PM.
    match chicago_time.weekday() {
        Weekday::Fri | Weekday::Sat => {
            next_open += chrono::Duration::days((7 - chicago_time.weekday().num_days_from_monday()) as i64 + 1);
        }
        _ => {
            // Otherwise, market opens the next day at 5 PM.
            next_open += chrono::Duration::days(1);
        }
    }

    let tz = chrono_tz::America::Chicago;
    let next_open = tz.from_local_datetime(&next_open).unwrap();
    next_open.to_utc() // Convert to UTC for consistent time comparison
}
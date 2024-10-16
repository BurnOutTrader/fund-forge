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
    let mut delay = Duration::from_secs(5);
    let max_delay = Duration::from_secs(300);

    loop {
        let now = Utc::now();
        let chicago_time = now.with_timezone(&chrono_tz::America::Chicago);
        let auckland_time = now.with_timezone(&chrono_tz::Pacific::Auckland);

        if is_weekend_or_off_hours(&chicago_time, &auckland_time) {
            let next_market_open = next_auckland_market_open(&auckland_time);
            let wait_duration = next_market_open - now;

            eprintln!(
                "Outside of market hours. Waiting until Auckland market open: {}",
                next_market_open.with_timezone(&chrono_tz::Pacific::Auckland)
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
                    delay = (delay * 2).min(max_delay);
                }
            }
        }
    }
}

fn is_weekend_or_off_hours(chicago_time: &DateTime<chrono_tz::Tz>, auckland_time: &DateTime<chrono_tz::Tz>) -> bool {
    let nyc_friday_5pm = chicago_time.weekday() == Weekday::Fri && chicago_time.hour() >= 17;
    let auckland_monday_before_955am = auckland_time.weekday() == Weekday::Mon &&
        (auckland_time.hour() < 9 ||
            (auckland_time.hour() == 9 && auckland_time.minute() < 55));

    nyc_friday_5pm || auckland_monday_before_955am ||
        (chicago_time.weekday() == Weekday::Sat || chicago_time.weekday() == Weekday::Sun)
}

fn next_auckland_market_open(auckland_time: &DateTime<chrono_tz::Tz>) -> DateTime<Utc> {
    let time_zone =chrono_tz::Pacific::Auckland;
    let mut next_open = auckland_time.date_naive().and_hms_opt(9, 55, 0).unwrap();
    if auckland_time.weekday() == Weekday::Sat {
        next_open = next_open + chrono::Duration::days(2);
    } else if auckland_time.weekday() == Weekday::Sun {
        next_open = next_open + chrono::Duration::days(1);
    } else if auckland_time.weekday() == Weekday::Mon && auckland_time.hour() < 9 {
        // Do nothing, next_open is already correct
    } else {
        next_open = next_open + chrono::Duration::days(1);
    }
    let next_open= time_zone.from_local_datetime(&next_open).unwrap();
    next_open.to_utc()
}
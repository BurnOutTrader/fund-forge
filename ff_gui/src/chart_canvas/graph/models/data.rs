use std::collections::BTreeMap;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use ff_standard_lib::helpers::converters::time_convert_utc_to_local;
use iced::{Rectangle};
use iced::widget::canvas:: Frame;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::candle::{Candle};
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::time_slices::TimeSlice;
use rust_decimal::Decimal;
use ff_standard_lib::standardized_types::base_data::history::get_compressed_historical_data;
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;
use crate::chart_canvas::graph::state::ChartState;
use crate::chart_canvas::graph::traits::TimeSeriesGraphElements;


#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
compare(PartialEq),
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub enum SeriesData {
    CandleStick(Candle),
    // Add other variants here...
}

impl SeriesData {

    /// Gets history for the chart from the ff_data_server in the correct format based on the subscription.
    pub async fn get_history(subscription: DataSubscription, to_date: DateTime<Utc>) -> BTreeMap<i64, Vec<SeriesData>> {
        let from_time = to_date - (subscription.resolution.as_duration() * 200);
        let time_slices = get_compressed_historical_data(vec![subscription], from_time.clone(), to_date).await.unwrap_or_else(|_e| {
            BTreeMap::new()
        });
        println!("Time Slices: {:?}", time_slices.len());
        SeriesData::from_time_slices(&time_slices)
    }

    /// Converts our BaseDataEnums into SeriesData enum variants.
    pub fn from_time_slices(time_slices:  &BTreeMap<i64, TimeSlice>) -> BTreeMap<i64, Vec<SeriesData>> {
        let mut data: BTreeMap<i64, Vec<SeriesData>> = BTreeMap::new();
        let mut prior_candle: Option<Candle> = None;

        for (time, slice) in time_slices.iter() {
            for base_data in slice.iter() {
                let series_data: SeriesData = match base_data {
                    BaseDataEnum::QuoteBar(quotebar)  => {
                        let candle = Candle::from_quotebar(quotebar.clone(), true);
                        prior_candle = Some(candle.clone());
                        SeriesData::CandleStick(candle)
                    },
                    BaseDataEnum::Candle(candle) => {
                        let candle = candle.clone();
                        prior_candle = Some(candle.clone());
                        SeriesData::CandleStick(candle.clone())
                    },
                    _ => panic!("Unimplemented data type"),
                };
                data.entry(time.clone() / 1000000).or_insert_with(Vec::new).push(series_data);
            }
        }
        data
    }

    pub fn draw_data(frame: &mut  Frame, view: &ChartState, bounds: &Rectangle, range_data: &BTreeMap<i64, Vec<SeriesData>>, logarithmic: bool, time_zone: &Tz)  {
        if range_data.is_empty() {
            return;
        }
        for (_, data_vec) in range_data {
            for data_object in data_vec {
                match data_object {
                    SeriesData::CandleStick(candle) => {
                        candle.draw_object(frame, view, bounds, logarithmic, &time_zone )
                    },
                    // Add other variants here...
                }
            }
        }
    }

    pub fn time_local(&self, time_zone: &Tz) -> i64 {
        match self {
            SeriesData::CandleStick(candle) => {
                candle.time_local(time_zone).timestamp()
            }
        }
    }

    pub fn time_utc(&self) -> i64 {
        match self {
            SeriesData::CandleStick(candle) => {
                candle.time_utc().timestamp()
            }
        }
    }

    pub fn lowest_value(&self) -> Decimal {
        match self {
            SeriesData::CandleStick(candle) => {
                candle.low
            }
        }
    }

    pub fn highest_value(&self) -> Decimal {
        match self {
            SeriesData::CandleStick(candle) => {
                candle.high
            }
        }
    }

    pub fn open_value(&self) -> Option<Decimal> {
        match self {
            SeriesData::CandleStick(candle) => {
                Some(candle.open)
            }
        }
    }

    pub fn close_value(&self) -> Option<Decimal> {
        match self {
            SeriesData::CandleStick(candle) => {
                Some(candle.close)
            }
        }
    }
}


use std::collections::BTreeMap;
use chrono::{DateTime, FixedOffset, Utc};
use chrono_tz::Tz;
use iced::{Rectangle};
use iced::widget::canvas:: Frame;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use ff_standard_lib::helpers::converters::time_convert_utc_datetime_to_fixed_offset;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::candle::{Candle, CandleCalculationType};
use ff_standard_lib::standardized_types::base_data::history::{history_many};
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;
use ff_standard_lib::standardized_types::time_slices::TimeSlice;
use crate::canvas::graph::state::ChartState;
use crate::canvas::graph::traits::TimeSeriesGraphElements;


#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
compare(PartialEq),
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub enum SeriesData {
    CandleStickData(Candle),
    // Add other variants here...
}

impl SeriesData {

    /// Gets history for the chart from the ff_data_server in the correct format based on the subscription.
    pub async fn get_history(subscription: DataSubscription, time_zone: &Tz, from_date: DateTime<FixedOffset>, to_date: DateTime<FixedOffset>) -> BTreeMap<i64, Vec<SeriesData>> {
        let subscriptions = vec![subscription.clone()];
        let history = history_many(subscriptions, from_date, to_date).await.unwrap();
        SeriesData::from_time_slices(&history, &time_zone)
    }

    /// Converts our BaseDataEnums into SeriesData enum variants.
    pub fn from_time_slices(time_slices:  &BTreeMap<DateTime<Utc>, TimeSlice>, time_zone: &Tz) -> BTreeMap<i64, Vec<SeriesData>> {
        let mut data: BTreeMap<i64, Vec<SeriesData>> = BTreeMap::new();
        let mut prior_candle: Option<Candle> = None;

        for (time, slice) in time_slices.iter() {
            for base_data in slice {
                let time = time_convert_utc_datetime_to_fixed_offset(&time_zone, time.clone()).timestamp();
                let series_data: SeriesData = match base_data {
                    BaseDataEnum::QuoteBar(quotebar)  => {
                        let mut candle = Candle::from_quotebar(quotebar.clone(), true);
                        prior_candle = Some(candle.clone());
                        SeriesData::CandleStickData(candle)
                    },
                    BaseDataEnum::Candle(candle) => {
                        let mut candle = candle.clone();
                        prior_candle = Some(candle.clone());
                        SeriesData::CandleStickData(candle.clone())
                    },
                    _ => panic!("Unimplemented data type"),
                };
                data.entry(time).or_insert_with(Vec::new).push(series_data);
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
                    SeriesData::CandleStickData(candle) => {
                        candle.draw_object(frame, view, bounds, logarithmic, &time_zone )
                    },
                    // Add other variants here...
                }
            }
        }
    }

    pub fn time_local(&self, time_zone: &Tz) -> i64 {
        match self {
            SeriesData::CandleStickData(candle) => {
                candle.time_local(time_zone).timestamp()
            }
        }
    }

    pub fn lowest_value(&self) -> f64 {
        match self {
            SeriesData::CandleStickData(candle) => {
                candle.low
            }
        }
    }

    pub fn highest_value(&self) -> f64 {
        match self {
            SeriesData::CandleStickData(candle) => {
                candle.high
            }
        }
    }

    pub fn open_value(&self) -> Option<f64> {
        match self {
            SeriesData::CandleStickData(candle) => {
                Some(candle.open)
            }
        }
    }

    pub fn close_value(&self) -> Option<f64> {
        match self {
            SeriesData::CandleStickData(candle) => {
                Some(candle.close)
            }
        }
    }
}


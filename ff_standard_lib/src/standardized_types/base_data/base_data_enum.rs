use std::collections::{BTreeMap};
use std::fmt::Error;
use std::fs::File;
use std::{fs};
use std::io::Write;
use std::path::PathBuf;
use chrono::{DateTime, FixedOffset, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use rkyv::{AlignedVec, Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use rkyv::ser::Serializer;
use rkyv::ser::serializers::AllocSerializer;
use crate::apis::vendor::DataVendor;
use crate::helpers::converters::{fund_forge_formatted_symbol_name, load_as_bytes, time_convert_utc_datetime_to_fixed_offset};
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::base_data::fundamental::Fundamental;
use crate::standardized_types::base_data::price::Price;
use crate::standardized_types::base_data::quote::Quote;
use crate::standardized_types::base_data::quotebar::QuoteBar;
use crate::standardized_types::base_data::tick::Tick;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::data_server_messaging::{FundForgeError};
use crate::standardized_types::enums::{MarketType, Resolution};
use crate::standardized_types::subscriptions::{DataSubscription, Symbol};
use crate::traits::bytes::Bytes;

/// Enum for the different types of base data
/// This is the main_window enum for all the different types of base data
/// It is used to store all the different types of base data in a single type
/// This is useful for when you want to store all the different types of base data in a single collection
/// # Variants
/// * `Price`       see [`BaseDataEnum::Price`](ff_data_vendors::base_data_types::base_data_enum::BaseDataEnum::Price)
/// * `Candle`      see [`BaseDataEnum::Candle`](ff_data_vendors::base_data_types::base_data_enum::BaseDataEnum::Candle)
/// * `QuoteBar`    see [`BaseDataEnum::QuoteBar`](ff_data_vendors::base_data_types::base_data_enum::BaseDataEnum::QuoteBar)
/// * `Tick`        see [`BaseDataEnum::Tick`](ff_data_vendors::base_data_types::base_data_enum::BaseDataEnum::Tick)
/// * `Quote`       see [`BaseDataEnum::Quote`](ff_data_vendors::base_data_types::base_data_enum::BaseDataEnum::Quote)
/// * `Fundamental` see [`BaseDataEnum::Fundamental`](ff_data_vendors::base_data_types::base_data_enum::BaseDataEnum::Fundamental)
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
compare(PartialEq),
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub enum BaseDataEnum {
    /// The `Price` struct is used to represent the price of an asset at a given time. see [`Price`](ff_data_vendors::base_data_types::price::Price)
    ///
    /// # Properties
    /// * `symbol` - The symbol of the asset.
    /// * `price` - The price of the asset.
    /// * `time` - The time the price was recorded.
    /// * `data_vendor` - The data vendor that provided the price.
    ///
    Price(Price),

    /// The `Candle` struct is used to represent the properties of an asset at a given Resolution. see [`Candle`](ff_data_vendors::base_data_types::candle::Candle)
    ///
    /// # Properties
    /// * `symbol` - The symbol of the asset.
    /// * `open` - The opening price of the asset.
    /// * `high` - The highest price of the asset.
    /// * `low` - The lowest price of the asset.
    /// * `close` - The closing price of the asset.
    /// * `volume` - The volume of the asset.
    /// * `time` - The time the price was recorded.
    /// * `is_closed` - A boolean value indicating if the candle is closed.
    /// * `data_vendor` - The data vendor that provided the price.
    /// * `resolution` - The resolution of the candle.
    Candle(Candle),
    /// The `QuoteBar` struct is used to represent the more detailed properties of an asset at a given Resolution. see [`QuoteBar`](ff_data_vendors::base_data_types::quotebar::QuoteBar)
    /// The QuoteBar is both a mid-price and bid-ask specific view.
    /// # Properties
    /// * `symbol`: The trading symbol of the asset.
    /// * `high`: The highest price.
    /// * `low`: The lowest price.
    /// * `open`: The opening price.
    /// * `close`: The closing price.
    /// * `bid_high`: The highest bid price.
    /// * `bid_low`: The lowest bid price.
    /// * `bid_open`: The opening bid price.
    /// * `bid_close`: The closing bid price.
    /// * `ask_high`: The highest ask price.
    /// * `ask_low`: The lowest ask price.
    /// * `ask_open`: The opening ask price.
    /// * `ask_close`: The closing ask price.
    /// * `volume`: The trading volume.
    /// * `range`: The difference between the high and low prices.
    /// * `time`: The opening time of the quote bar as a Unix timestamp.
    /// * `spread`: The difference between the highest ask price and the lowest bid price.
    /// * `is_closed`: Indicates whether the quote bar is closed.
    /// * `data_vendor`: The data vendor that provided the quote bar.
    /// * `resolution`: The resolution of the quote bar.
    QuoteBar(QuoteBar),

    /// The `Tick` struct is used to represent the price of an asset at a given time. see [`Tick`](ff_data_vendors::base_data_types::tick::Tick)
    ///
    /// # Properties
    /// * `symbol` - The symbol of the asset.
    /// * `price` - The price of the asset.
    /// *. `time` - The time the price was recorded.
    /// * `volume` - The volume of the trade or trades executed.
    /// * `side` - The side of the trade `Side` enum variant.
    /// * `data_vendor` - The data vendor of the trade.
    Tick(Tick),

    /// The `Quote` struct is used to represent the quoted prices of an asset at a given time, it represents best bid as 'bid' and best offer as 'ask'. see [`Quote`](ff_data_vendors::base_data_types::quote::Quote)
    ///
    /// # Properties
    /// * `symbol` - The symbol of the asset.
    /// * `ask` - The current best `ask` price.
    /// * `bid` - The current best `bid` price.
    /// * `ask_volume` - The volume on offer at the `ask` price.
    /// * `bid_volume` - The volume of `bids` at the bid price.
    /// * `time` - The time of the quote.
    /// * `data_vendor` - The data vendor of the quote.
    Quote(Quote),

    /// The `Fundamental` struct is used to represent any other data that does not fit into the other variants. see [`Fundamental`](ff_data_vendors::base_data_types::fundamental::Fundamental)
    /// It is a versatile data type that can be used to store any other data that you wish.
    /// # Properties
    /// * `symbol` - `String` The symbol of the asset.
    /// * `time` - `i64` The time stamp the price was recorded.
    /// * `value` - `Option<f64>` The value of the fundamental data.
    /// * `value_string` - `Option<String>` The value of the fundamental data as a string, this can be used to pass in json objects etc to the `fn on_data_updates`.
    /// * `value_bytes` - `Option<Vec<u8>>` The value of the fundamental data as a byte array.
    /// * `name` - `String` The name of the fundamental data: This can be used in the `ff_data_server` to specify how the server is to pull the data from the specified broker, this allows max versatility with minimum hard coding, or re-coding of the engine.
    /// * `bias` - `Bias` enum The bias of the fundamental data `Bias` enum variant.
    /// * `data_vendor` - `DataVendor` enum The data vendor of the fundamental data `DataVendor` enum variant.
    Fundamental(Fundamental),
}

impl BaseDataEnum {

    pub fn symbol(&self) -> &Symbol {
        match self {
            BaseDataEnum::Price(price) => &price.symbol,
            BaseDataEnum::Candle(candle) => &candle.symbol,
            BaseDataEnum::QuoteBar(quote_bar) => &quote_bar.symbol,
            BaseDataEnum::Tick(tick) => &tick.symbol,
            BaseDataEnum::Quote(quote) => &quote.symbol,
            BaseDataEnum::Fundamental(fundamental) => &fundamental.symbol,
        }
    }

    /// Returns the `is_closed` property of the `BaseDataEnum` variant for variants that implement it, else just returns true.
    pub fn is_closed(&self) -> bool {
        match self {
            BaseDataEnum::Candle(candle) => candle.is_closed,
            BaseDataEnum::QuoteBar(quote_bar) => quote_bar.is_closed,
            _ => true,
        }
    }
    
    pub fn subscription(&self) -> DataSubscription {
        match self {
            BaseDataEnum::Candle(candle) => DataSubscription::new(candle.symbol.name.clone(), candle.symbol.data_vendor.clone(), candle.resolution.clone(), self.base_data_type(), candle.symbol.market_type.clone()),
            BaseDataEnum::QuoteBar(bar) => DataSubscription::new(bar.symbol.name.clone(), bar.symbol.data_vendor.clone(), bar.resolution.clone(), self.base_data_type(), bar.symbol.market_type.clone()),
            _ => DataSubscription::new(self.symbol().name.clone(), self.symbol().data_vendor.clone(), Resolution::Instant, self.base_data_type(), self.symbol().market_type.clone()),
        }
    }

    /// Links `BaseDataEnum` to a `BaseDataType`
    pub fn base_data_type(&self) -> BaseDataType {
        match self {
            BaseDataEnum::Price(_) => BaseDataType::Prices,
            BaseDataEnum::Candle(_) => BaseDataType::Candles,
            BaseDataEnum::QuoteBar(_) => BaseDataType::QuoteBars,
            BaseDataEnum::Tick(_) => BaseDataType::Ticks,
            BaseDataEnum::Quote(_) => BaseDataType::Quotes,
            BaseDataEnum::Fundamental(_) => BaseDataType::Fundamentals,
        }
    }
    
    pub fn set_is_closed(&mut self, is_closed: bool) {
        match self {
            BaseDataEnum::Candle(candle) => candle.is_closed = is_closed,
            BaseDataEnum::QuoteBar(bar) => bar.is_closed = is_closed,
            _ => {}
        }
    }

    /// Deserializes from `Vec<u8>` to `Vec<BaseDataEnum>`
    pub fn from_array_bytes(data: &Vec<u8>) -> Result<Vec<BaseDataEnum>, Error> {
        let archived_quotebars = match rkyv::check_archived_root::<Vec<BaseDataEnum>>(&data[..]){
            Ok(data) => data,
            Err(e) => {
                format!("Failed to deserialize data: {}", e);
                return Err(Error);
            },
        };

        // Assuming you want to work with the archived data directly, or you can deserialize it further
        Ok(archived_quotebars.deserialize(&mut rkyv::Infallible).unwrap())
    }



    /// Returns the formatted file name for a `PriceData` file based on the `Subscription` and file date complete with .rkyv extension.
    pub fn file_name(subscription: &DataSubscription, date_time: &DateTime<Utc>) -> String {
        let cleaned_symbol = fund_forge_formatted_symbol_name(&subscription.symbol.name);
        return format!("{}_{}_{}.rkyv", cleaned_symbol, subscription.resolution.to_string(), date_time.format("%Y-%m").to_string());
    }

    /// Returns the path to the folder for all `Vec<Candle>` files based on the `Subscription`.
    pub fn folder_path(data_folder: &PathBuf, subscription: &DataSubscription) -> Result<PathBuf, std::io::Error> {
        let cleaned_symbol = fund_forge_formatted_symbol_name(&subscription.symbol.name);

        let folder_structure = format!("{}/{}/{}/{}/{}", &subscription.symbol.data_vendor.to_string(), &subscription.market_type.to_string(), &subscription.base_data_type.to_string(), &subscription.resolution.to_string(), &cleaned_symbol);
        let folder = data_folder.join(folder_structure);
        if !folder.exists() {
            match std::fs::create_dir_all(&folder) {
                Ok(_) => (),
                Err(e) => {
                    return Err(e);
                }
            }
        }
        Ok(folder)
    }

    ///Gets the file path for the `BaseData` file at the specified `DateTime<Utc>` based on the `Subscription` properties.
    pub fn file_path(data_folder: &PathBuf, subscription: &DataSubscription, date_time: &DateTime<Utc>) -> Result<PathBuf, std::io::Error> {
        let file_name = BaseDataEnum::file_name(subscription, date_time);
        let folder = BaseDataEnum::folder_path(data_folder, subscription)?;
        Ok(folder.join(file_name))
    }

    /// Saves the `Vec<BaseDataEnum>` to a file in the fund forge file system path format.
    pub fn save_as_rkyv(data_folder: &PathBuf, price_data_enum_vec: Vec<BaseDataEnum>, subscription: &DataSubscription) -> std::io::Result<()> {
        let folder = Self::folder_path(data_folder, &subscription)?;
        let time = price_data_enum_vec[0].time_utc().to_utc();

        let file_name = Self::file_name(&subscription, &time);
        let file_path = folder.join(file_name);

        // Ensure the parent directory exists
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(&parent)?;
            }
        }

        let bytes = Self::vec_to_aligned(price_data_enum_vec);
        let mut file = match File::create(file_path) {
            Ok(file) => file,
            Err(e) => {
                return Err(e);
            }
        };

        // Write the serialized bytes to the file
        match file.write_all(&bytes) {
            Ok(_) => (),
            Err(e) => {
                return Err(e);
            }
        }

        Ok(())
    }

    /// Helper method used to separate and price data to files in the fund forge file system path format.
    /// Makes it easier to download all available bars from a broker and save them to files.
    pub fn format_and_save(base_data_path: &PathBuf, price_data_enum_vec: BTreeMap<DateTime<Utc>, BaseDataEnum>, subscription: &DataSubscription) -> Result<(), Box<dyn std::error::Error>> {
        let mut data_dict: BTreeMap<String, Vec<BaseDataEnum>> = BTreeMap::new();
        println!("Formatting and saving {} data for: {}",price_data_enum_vec.len(), subscription.symbol.name);
        // separate data by types and dates
        for (time, data) in &price_data_enum_vec {
            let utc_time = time;
            let file_name = Self::file_name(&subscription, &utc_time);

            let key = file_name.clone();
            if !data_dict.contains_key(&key) {
                data_dict.insert(key.clone(), Vec::new());
            }
            data_dict.get_mut(&key).unwrap().push(data.clone());
        }

        // save files by date
        for (_key, data_vec) in data_dict {
            let time = data_vec[0].time_utc().to_utc();
            let _file_name = Self::file_name(&subscription, &time);

            match Self::save_as_rkyv(base_data_path, data_vec, &subscription) {
                Ok(_) => (),
                Err(e) => {
                    return Err(Box::new(e));
                }
            }
        }
        Ok(())
    }

    /// Serializes a `Vec<PriceDataEnum>` into `AlignedVec`
    pub fn vec_to_aligned(price_data: Vec<BaseDataEnum>) -> AlignedVec {
        // Create a new serializer
        let mut serializer = AllocSerializer::<20971520>::default();

        // Serialize the Vec<QuoteBar>
        serializer.serialize_value(&price_data).unwrap();

        // Get the serialized bytes
        let vec = serializer.into_serializer().into_inner();
        vec
    }

    /// Serializes a `Vec<PriceDataEnum>` into `Vec<u8>`
    /// This is the method used for quote bar request_response
    pub fn vec_to_bytes(price_data: Vec<BaseDataEnum>) -> Vec<u8> {
        // Get the serialized bytes
        let vec = BaseDataEnum::vec_to_aligned(price_data);
        vec.to_vec()
    }


    /// gets the yyyy-mm-01 date from the file name
    pub fn file_name_parse_date(string: &str) -> DateTime<Utc> {
        // Split the filename into parts using the underscore as delimiter
        let parts: Vec<&str> = string.split("_").collect();

        // The date part is expected to be the third part of the filename, hence index 2
        let datetime_part = parts.get(2).expect("Unexpected filename format: missing datetime part");

        // Splitting the datetime_part to separate the date from the extension, assuming .rkyv extension
        let date_part = datetime_part.split(".").next().expect("Unexpected datetime format: missing date part");

        // Splitting the date_part to separate year and month
        let date_parts: Vec<&str> = date_part.split("-").collect();

        // Parsing year and month, and setting day to 1 as per original function
        let year = date_parts[0].parse::<i32>().expect("Invalid year");
        let month = date_parts[1].parse::<u32>().expect("Invalid month");
        let day = 1;  // Assuming the day is always the first of the month

        // Constructing the DateTime<Utc> object
        let naive_date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
        let naive_time = naive_date.and_hms_opt(0, 0, 0).unwrap();  // Assuming time is 00:00:00

        Utc.from_utc_datetime(&naive_time)
    }

    fn get_all_entries(data_folder: &PathBuf, subscription: &DataSubscription) -> Vec<PathBuf> {
        let symbol = fund_forge_formatted_symbol_name(&subscription.symbol.name);
        let data_folder = Self::folder_path(data_folder, subscription).unwrap();
        let entries: Vec<_> = fs::read_dir(data_folder).unwrap()
            .filter_map(|res| {
                let path = res.ok()?.path();
                let file_name = path.file_name()?.to_str()?;
                if file_name.starts_with(&symbol) { // Make sure to use &symbol to get a &str from String
                    Some(path)
                } else {
                    None
                }
            })
            .collect();

        entries
    }

    /// get the earliest data time available for a subscription
    pub fn earliest_time_available(data_folder: &PathBuf, subscription: &DataSubscription) -> Option<DateTime<Utc>> {
        let mut entries: Vec<_> = Self::get_all_entries(data_folder, &subscription);

        if entries.len() == 0 {
            return None;
        }

        // Sort entries based on the date parsed from the file name
        entries.sort_by_key(|entry| Self::file_name_parse_date(&entry.file_name().expect("Failed to parse date").to_string_lossy().into_owned()));

        let file_path = entries.first().unwrap();
        let file = load_as_bytes(file_path.clone()).unwrap();

        let data = BaseDataEnum::from_array_bytes(&file).unwrap();

        let time = data.first().expect("Failed to load first").time_utc().to_utc();

        Some(time)
    }

    /// get the latest data time available for a subscription
    pub fn latest_time_available(data_folder: &PathBuf, subscription: &DataSubscription) -> Option<DateTime<Utc>> {
        let mut entries: Vec<_> = Self::get_all_entries(data_folder, &subscription);

        if entries.len() == 0 {
            return None;
        }

        // Sort entries based on the date parsed from the file name
        entries.sort_by_key(|entry| Self::file_name_parse_date(&entry.file_name().expect("Failed to parse data").to_string_lossy().into_owned()));

        let file_path = entries.last().unwrap();
        let file_name = file_path.file_name().expect("Failed to parse file name").to_string_lossy().into_owned();

        let file = load_as_bytes(file_path.clone()).unwrap();

        // if the data is empty return the date from the file name starting from the first day of the month at h m s 0,0,0
        let load_data = match BaseDataEnum::from_array_bytes(&file) {
            Ok(data) => data,
            Err(_e) => {
                return Some(Self::file_name_parse_date(&file_name))
            }
        };

        // if the data is empty return the date from the file name starting from the first day of the month at h m s 0,0,0
        if load_data.len() == 0 {
            return Some(Self::file_name_parse_date(&file_name))
        }

        //if file has no data inside we should return the date as 1st day of the month h m s 0,0,0, this is incase we have a file with no data we will be able to start from earliest date
        let file = load_as_bytes(file_path.clone()).unwrap();

        let data = BaseDataEnum::from_array_bytes(&file).unwrap();
        let time = data.last().expect("Failed to load first").time_utc().to_utc();

        return Some(time);
    }

    /// get the earliest data available for a subscription as a `Vec<BaseDataEnum>`
    pub fn earliest_data_available(data_folder: &PathBuf, subscription: &DataSubscription) -> Option<Vec<BaseDataEnum>> {
        let mut entries: Vec<_> = Self::get_all_entries(data_folder, &subscription);

        if entries.len() == 0 {
            return None;
        }

        // Sort entries based on the date parsed from the file name
        entries.sort_by_key(|entry| Self::file_name_parse_date(&entry.file_name().expect("Failed to parse data and sort").to_string_lossy().into_owned()));

        let file_path = entries.first().unwrap();
        let file = load_as_bytes(file_path.clone()).unwrap();

        let data = BaseDataEnum::from_array_bytes(&file).unwrap();

        Some(data)
    }

    /// get the latest data available for a subscription as a `Vec<BaseDataEnum>`
    pub fn latest_available_data(data_folder: &PathBuf, subscription: &DataSubscription) -> Option<Vec<BaseDataEnum>> {
        let mut entries: Vec<_> = Self::get_all_entries(data_folder, &subscription);
        if entries.len() == 0 {
            return None;
        }

        // Sort entries based on the date parsed from the file name
        entries.sort_by_key(|entry| Self::file_name_parse_date(&entry.file_name().expect("Failed to parse date and sort").to_string_lossy().into_owned()));

        let file_path = entries.last()?;
        let file = load_as_bytes(file_path.clone()).unwrap();

        match BaseDataEnum::from_array_bytes(&file) {
            Ok(data) => Some(data),
            Err(_e) => {
                None
            }
        }
    }
}

impl Bytes<Self> for BaseDataEnum {
    fn to_bytes(&self) -> Vec<u8> {
        let vec = rkyv::to_bytes::<_, 256>(self).unwrap();
        vec.into()
    }

    fn from_bytes(archived: &[u8]) -> Result<BaseDataEnum, FundForgeError> {
        // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<BaseDataEnum>(archived) { //Ignore this warning: Trait `Deserialize<ResponseType, SharedDeserializeMap>` is not implemented for `ArchivedRequestType` [E0277]
            Ok(response) => Ok(response),
            Err(e) => {
                Err(FundForgeError::ClientSideErrorDebug(e.to_string()))
            }
        }
    }
}

impl BaseData for BaseDataEnum {
    /// Returns the symbol of the `BaseDataEnum` variant
    fn symbol_name(&self) -> Symbol {
        match self {
            BaseDataEnum::Price(price) => price.symbol.clone(),
            BaseDataEnum::Candle(candle) => candle.symbol.clone(),
            BaseDataEnum::QuoteBar(quote_bar) => quote_bar.symbol.clone(),
            BaseDataEnum::Tick(tick) => tick.symbol.clone(),
            BaseDataEnum::Quote(quote) => quote.symbol.clone(),
            BaseDataEnum::Fundamental(fundamental) => fundamental.symbol.clone(),
        }
    }

    /// Returns the data time at the specified time zone offset as a `DateTime<FixedOffset>`
    fn time_local(&self, time_zone: &Tz) -> DateTime<FixedOffset> {
        time_convert_utc_datetime_to_fixed_offset(time_zone, self.time_utc())
    }

    /// This is the closing time of the `BaseDataEnum` variant, so for 1 hour quotebar, if the bar opens at 5pm, the closing time will be 6pm and the time_utc will be 6pm. time_object_unchanged(&self) will return the unaltered value.
    fn time_utc(&self) -> DateTime<Utc> {
        match self {
            BaseDataEnum::Price(price) => price.time_utc(),
            BaseDataEnum::Candle(candle) => candle.time_utc(),
            BaseDataEnum::QuoteBar(quote_bar) => quote_bar.time_utc(),
            BaseDataEnum::Tick(tick) => tick.time_utc(),
            BaseDataEnum::Quote(quote) => quote.time_utc(),
            BaseDataEnum::Fundamental(fundamental) => fundamental.time_utc(),
        }
    }

    fn time_created_utc(&self) -> DateTime<Utc> {
        match self {
            BaseDataEnum::Price(price) => price.time_utc(),
            BaseDataEnum::Candle(candle) => candle.time_utc() + candle.resolution.as_duration(),
            BaseDataEnum::QuoteBar(quote_bar) => quote_bar.time_utc() + quote_bar.resolution.as_duration(),
            BaseDataEnum::Tick(tick) => tick.time_utc(),
            BaseDataEnum::Quote(quote) => quote.time_utc(),
            BaseDataEnum::Fundamental(fundamental) => fundamental.time_utc(),
        }
    }

    fn time_created_local(&self, time_zone: &Tz) -> DateTime<FixedOffset> {
        time_convert_utc_datetime_to_fixed_offset(time_zone, self.time_created_utc())
    }

    /// Returns the `DataVendor` enum variant of the `BaseDataEnum` variant object
    fn data_vendor(&self) -> DataVendor {
        match self {
            BaseDataEnum::Price(price) => price.symbol.data_vendor.clone(),
            BaseDataEnum::Candle(candle) => candle.symbol.data_vendor.clone(),
            BaseDataEnum::QuoteBar(quote_bar) => quote_bar.symbol.data_vendor.clone(),
            BaseDataEnum::Tick(tick) => tick.symbol.data_vendor.clone(),
            BaseDataEnum::Quote(quote) => quote.symbol.data_vendor.clone(),
            BaseDataEnum::Fundamental(fundamental) => fundamental.symbol.data_vendor.clone(),
        }
    }

    fn market_type(&self) -> MarketType {
        match self {
            BaseDataEnum::Price(price) => price.symbol.market_type.clone(),
            BaseDataEnum::Candle(candle) => candle.symbol.market_type.clone(),
            BaseDataEnum::QuoteBar(quote_bar) => quote_bar.symbol.market_type.clone(),
            BaseDataEnum::Tick(tick) => tick.symbol.market_type.clone(),
            BaseDataEnum::Quote(quote) => quote.symbol.market_type.clone(),
            BaseDataEnum::Fundamental(fundamental) => fundamental.symbol.market_type.clone(),
        }
    }
}





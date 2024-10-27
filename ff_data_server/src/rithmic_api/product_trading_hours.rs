use ahash::AHashMap;
use chrono::NaiveTime;
use lazy_static::lazy_static;
use ff_standard_lib::standardized_types::trading_hours::{DaySession, TradingHours};

lazy_static! {
    pub static ref TRADING_HOURS: AHashMap<String, &'static TradingHours> = {
        let mut m = AHashMap::new();
        // Micro E-mini Equity Index Futures
        m.insert("MNQ".to_string(), &CME_HOURS); // Micro Nasdaq
        m.insert("MES".to_string(), &CME_HOURS); // Micro S&P 500
        m.insert("M2K".to_string(), &CME_HOURS); // Micro Russell 2000
        m.insert("MYM".to_string(), &CME_HOURS); // Micro Dow

        // E-mini Equity Index Futures
        m.insert("NQ".to_string(), &CME_HOURS);  // E-mini Nasdaq
        m.insert("ES".to_string(), &CME_HOURS);  // E-mini S&P 500
        m.insert("RTY".to_string(), &CME_HOURS); // E-mini Russell 2000
        m.insert("YM".to_string(), &CME_HOURS);  // E-mini Dow

        // Standard Equity Index Futures
        m.insert("SP".to_string(), &CME_HOURS);  // Full-size S&P 500
        m.insert("DJ".to_string(), &CME_HOURS);  // Full-size Dow

        // Sector Futures
        m.insert("GD".to_string(), &CME_HOURS);  // E-mini Financial Sector
        m.insert("GI".to_string(), &CME_HOURS);  // E-mini Technology Sector
        m.insert("GK".to_string(), &CME_HOURS);  // E-mini Energy Sector
        m.insert("GV".to_string(), &CME_HOURS);  // E-mini Health Care Sector
        m.insert("GX".to_string(), &CME_HOURS);  // E-mini Consumer Staples
        m.insert("GZ".to_string(), &CME_HOURS);  // E-mini Materials Sector

        // Interest Rate Futures
        m.insert("ZN".to_string(), &CME_HOURS);  // 10-Year T-Note
        m.insert("ZB".to_string(), &CME_HOURS);  // 30-Year T-Bond
        m.insert("ZF".to_string(), &CME_HOURS);  // 5-Year T-Note
        m.insert("ZT".to_string(), &CME_HOURS);  // 2-Year T-Note
        m.insert("UB".to_string(), &CME_HOURS);  // Ultra T-Bond
        m.insert("GE".to_string(), &CME_HOURS);  // Eurodollar
        m.insert("SR3".to_string(), &CME_HOURS); // 3-Month SOFR

        // FX Futures
        m.insert("6E".to_string(), &CME_HOURS);  // Euro FX
        m.insert("6B".to_string(), &CME_HOURS);  // British Pound
        m.insert("6J".to_string(), &CME_HOURS);  // Japanese Yen
        m.insert("6C".to_string(), &CME_HOURS);  // Canadian Dollar
        m.insert("6A".to_string(), &CME_HOURS);  // Australian Dollar
        m.insert("6N".to_string(), &CME_HOURS);  // New Zealand Dollar
        m.insert("6S".to_string(), &CME_HOURS);  // Swiss Franc

        // Micro FX Futures
        m.insert("M6E".to_string(), &CME_HOURS); // Micro Euro FX
        m.insert("M6A".to_string(), &CME_HOURS); // Micro AUD/USD
        m.insert("M6B".to_string(), &CME_HOURS); // Micro GBP/USD

        // Commodities
        m.insert("GC".to_string(), &CME_HOURS);  // Gold
        m.insert("SI".to_string(), &CME_HOURS);  // Silver
        m.insert("HG".to_string(), &CME_HOURS);  // Copper
        m.insert("PL".to_string(), &CME_HOURS);  // Platinum
        m.insert("PA".to_string(), &CME_HOURS);  // Palladium

        // Micro Metals
        m.insert("MGC".to_string(), &CME_HOURS); // Micro Gold
        m.insert("SIL".to_string(), &CME_HOURS); // Micro Silver

        // Metals and Commodities
        m.insert("GC".to_string(), &CME_HOURS);   // Gold Futures
        m.insert("SI".to_string(), &CME_HOURS);   // Silver Futures
        m.insert("HG".to_string(), &CME_HOURS);   // Copper Futures
        m.insert("PL".to_string(), &CME_HOURS);   // Platinum Futures
        m.insert("PA".to_string(), &CME_HOURS);   // Palladium Futures
        m.insert("ALI".to_string(), &CME_HOURS);  // Aluminum Futures
        m.insert("QC".to_string(), &CME_HOURS);   // E-mini Copper Futures

        // Micro Metals
        m.insert("MGC".to_string(), &CME_HOURS);  // Micro Gold Futures
        m.insert("SIL".to_string(), &CME_HOURS);  // Micro Silver Futures
        m.insert("MHG".to_string(), &CME_HOURS);  // Micro Copper Futures
        m.insert("M2K".to_string(), &CME_HOURS);  // Micro Platinum
        m.insert("MPA".to_string(), &CME_HOURS);  // Micro Palladium

        // E-mini Metals
        m.insert("QO".to_string(), &CME_HOURS);   // E-mini Gold Futures
        m.insert("QI".to_string(), &CME_HOURS);   // E-mini Silver Futures

        // Options on Futures (same hours as underlying)
        m.insert("OG".to_string(), &CME_HOURS);   // Options on Gold Futures
        m.insert("SO".to_string(), &CME_HOURS);   // Options on Silver Futures
        m.insert("HX".to_string(), &CME_HOURS);   // Options on Copper Futures
        m.insert("PO".to_string(), &CME_HOURS);   // Options on Platinum Futures
        m.insert("PAO".to_string(), &CME_HOURS);  // Options on Palladium Futures

        // Metal Spreads (trade same hours)
        m.insert("GS".to_string(), &CME_HOURS);   // Gold/Silver Spread
        m.insert("GSP".to_string(), &CME_HOURS);  // Gold/Platinum Spread
        m.insert("GPS".to_string(), &CME_HOURS);  // Gold/Palladium Spread
        m.insert("SPS".to_string(), &CME_HOURS);  // Silver/Platinum Spread
        m.insert("SPA".to_string(), &CME_HOURS);  // Silver/Palladium Spread

        m.insert("ZC".to_string(), &CBOT_GRAINS_HOURS);  // Corn
        m.insert("ZS".to_string(), &CBOT_GRAINS_HOURS);  // Soybeans
        m.insert("ZW".to_string(), &CBOT_GRAINS_HOURS);  // Wheat
        m.insert("ZL".to_string(), &CBOT_GRAINS_HOURS);  // Soybean Oil
        m.insert("ZM".to_string(), &CBOT_GRAINS_HOURS);  // Soybean Meal
        m.insert("ZO".to_string(), &CBOT_GRAINS_HOURS);  // Oats
        m.insert("KE".to_string(), &CBOT_GRAINS_HOURS);  // KC Wheat
        m.insert("ZR".to_string(), &CBOT_GRAINS_HOURS);  // Rough Rice
        m
    };
}


pub fn get_trading_hours(symbol: &str) -> Option<&'static TradingHours> {
    TRADING_HOURS.get(symbol).copied()
}

const fn const_time(hour: u32, min: u32, sec: u32) -> NaiveTime {
    match NaiveTime::from_hms_opt(hour, min, sec) {
        Some(t) => t,
        None => panic!("Invalid time"),
    }
}

const CME_HOURS: TradingHours = TradingHours {
    timezone: chrono_tz::America::Chicago,
    sunday: DaySession {
        open: Some(const_time(17, 0, 0)),
        close: None,
    },
    monday: DaySession {
        open: None,
        close: Some(const_time(16, 0, 0)),
    },
    tuesday: DaySession {
        open: Some(const_time(17, 0, 0)),
        close: Some(const_time(16, 0, 0)),
    },
    wednesday: DaySession {
        open: Some(const_time(17, 0, 0)),
        close: Some(const_time(16, 0, 0)),
    },
    thursday: DaySession {
        open: Some(const_time(17, 0, 0)),
        close: Some(const_time(16, 0, 0)),
    },
    friday: DaySession {
        open: Some(const_time(17, 0, 0)),
        close: Some(const_time(16, 0, 0)),
    },
};

// CBOT Grains Schedule
const CBOT_GRAINS_HOURS: TradingHours = TradingHours {
    timezone: chrono_tz::America::Chicago,
    sunday: DaySession {
        open: Some(const_time(19, 0, 0)),  // 7:00 PM CT
        close: None,                        // Continues into Monday
    },
    monday: DaySession {
        open: None,                         // Continues from Sunday
        close: Some(const_time(13, 20, 0)), // 1:20 PM CT
    },
    tuesday: DaySession {
        open: Some(const_time(19, 0, 0)),  // 7:00 PM CT
        close: Some(const_time(13, 20, 0)), // 1:20 PM CT
    },
    wednesday: DaySession {
        open: Some(const_time(19, 0, 0)),  // 7:00 PM CT
        close: Some(const_time(13, 20, 0)), // 1:20 PM CT
    },
    thursday: DaySession {
        open: Some(const_time(19, 0, 0)),  // 7:00 PM CT
        close: Some(const_time(13, 20, 0)), // 1:20 PM CT
    },
    friday: DaySession {
        open: Some(const_time(19, 0, 0)),  // 7:00 PM CT
        close: Some(const_time(13, 20, 0)), // 1:20 PM CT
    },
};

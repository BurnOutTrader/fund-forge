use ahash::AHashMap;
use chrono::NaiveTime;
use lazy_static::lazy_static;
use crate::standardized_types::market_maps::{DaySession, TradingHours};

lazy_static! {
    pub static ref TRADING_HOURS: AHashMap<&'static str, &'static TradingHours> = {
        let mut m = AHashMap::new();
        // Micro E-mini Equity Index Futures
        m.insert("MNQ", &CME_HOURS); // Micro Nasdaq
        m.insert("MES", &CME_HOURS); // Micro S&P 500
        m.insert("M2K", &CME_HOURS); // Micro Russell 2000
        m.insert("MYM", &CME_HOURS); // Micro Dow

        // E-mini Equity Index Futures
        m.insert("NQ", &CME_HOURS);  // E-mini Nasdaq
        m.insert("ES", &CME_HOURS);  // E-mini S&P 500
        m.insert("RTY", &CME_HOURS); // E-mini Russell 2000
        m.insert("YM", &CME_HOURS);  // E-mini Dow

        // Standard Equity Index Futures
        m.insert("SP", &CME_HOURS);  // Full-size S&P 500
        m.insert("DJ", &CME_HOURS);  // Full-size Dow

        // Sector Futures
        m.insert("GD", &CME_HOURS);  // E-mini Financial Sector
        m.insert("GI", &CME_HOURS);  // E-mini Technology Sector
        m.insert("GK", &CME_HOURS);  // E-mini Energy Sector
        m.insert("GV", &CME_HOURS);  // E-mini Health Care Sector
        m.insert("GX", &CME_HOURS);  // E-mini Consumer Staples
        m.insert("GZ", &CME_HOURS);  // E-mini Materials Sector

        // Interest Rate Futures
        m.insert("ZN", &CME_HOURS);  // 10-Year T-Note
        m.insert("ZB", &CME_HOURS);  // 30-Year T-Bond
        m.insert("ZF", &CME_HOURS);  // 5-Year T-Note
        m.insert("ZT", &CME_HOURS);  // 2-Year T-Note
        m.insert("UB", &CME_HOURS);  // Ultra T-Bond
        m.insert("GE", &CME_HOURS);  // Eurodollar
        m.insert("SR3", &CME_HOURS); // 3-Month SOFR

        // FX Futures
        m.insert("6E", &CME_HOURS);  // Euro FX
        m.insert("6B", &CME_HOURS);  // British Pound
        m.insert("6J", &CME_HOURS);  // Japanese Yen
        m.insert("6C", &CME_HOURS);  // Canadian Dollar
        m.insert("6A", &CME_HOURS);  // Australian Dollar
        m.insert("6N", &CME_HOURS);  // New Zealand Dollar
        m.insert("6S", &CME_HOURS);  // Swiss Franc
        m.insert("E7", &CME_HOURS);  // E-mini Euro FX

        // Micro FX Futures
        m.insert("M6E", &CME_HOURS); // Micro Euro FX
        m.insert("M6A", &CME_HOURS); // Micro AUD/USD
        m.insert("M6B", &CME_HOURS); // Micro GBP/USD

        // Metals and Commodities
        m.insert("GC", &CME_HOURS);   // Gold Futures
        m.insert("SI", &CME_HOURS);   // Silver Futures
        m.insert("HG", &CME_HOURS);   // Copper Futures
        m.insert("PL", &CME_HOURS);   // Platinum Futures
        m.insert("PA", &CME_HOURS);   // Palladium Futures
        m.insert("ALI", &CME_HOURS);  // Aluminum Futures
        m.insert("QC", &CME_HOURS);   // E-mini Copper Futures

        // Micro Metals
        m.insert("MGC", &CME_HOURS);  // Micro Gold Futures
        m.insert("SIL", &CME_HOURS);  // Micro Silver Futures
        m.insert("MHG", &CME_HOURS);  // Micro Copper Futures
        m.insert("M2K", &CME_HOURS);  // Micro Platinum
        m.insert("MPA", &CME_HOURS);  // Micro Palladium

        // E-mini Metals
        m.insert("QO", &CME_HOURS);   // E-mini Gold Futures
        m.insert("QI", &CME_HOURS);   // E-mini Silver Futures

        // Options on Futures (same hours as underlying)
        m.insert("OG", &CME_HOURS);   // Options on Gold Futures
        m.insert("SO", &CME_HOURS);   // Options on Silver Futures
        m.insert("HX", &CME_HOURS);   // Options on Copper Futures
        m.insert("PO", &CME_HOURS);   // Options on Platinum Futures
        m.insert("PAO", &CME_HOURS);  // Options on Palladium Futures

        // Metal Spreads (trade same hours)
        m.insert("GS", &CME_HOURS);   // Gold/Silver Spread
        m.insert("GSP", &CME_HOURS);  // Gold/Platinum Spread
        m.insert("GPS", &CME_HOURS);  // Gold/Palladium Spread
        m.insert("SPS", &CME_HOURS);  // Silver/Platinum Spread
        m.insert("SPA", &CME_HOURS);  // Silver/Palladium Spread

        // CBOT Equities
        m.insert("YM", &CME_HOURS);   // E-mini Dow ($5)
        m.insert("MYM", &CME_HOURS);  // Micro E-mini Dow ($0.50)
        m.insert("DJI", &CME_HOURS);  // DJIA Futures (Big Dow)
        m.insert("DOW", &CME_HOURS);  // Dow Jones Industrial Average Futures

        // Agricultural products use same CBOT_GRAINS_HOURS schedule
        m.insert("ZC", &CBOT_GRAINS_HOURS);  // Corn
        m.insert("ZS", &CBOT_GRAINS_HOURS);  // Soybeans
        m.insert("ZW", &CBOT_GRAINS_HOURS);  // Wheat
        m.insert("ZL", &CBOT_GRAINS_HOURS);  // Soybean Oil
        m.insert("ZM", &CBOT_GRAINS_HOURS);  // Soybean Meal
        m.insert("ZO", &CBOT_GRAINS_HOURS);  // Oats
        m.insert("KE", &CBOT_GRAINS_HOURS);  // KC Wheat
        m.insert("ZR", &CBOT_GRAINS_HOURS);  // Rough Rice

        // Mini Agricultural products use same CBOT_GRAINS_HOURS schedule
        m.insert("YC", &CBOT_GRAINS_HOURS);  // Mini-sized Corn
        m.insert("YK", &CBOT_GRAINS_HOURS);  // Mini-sized Soybeans
        m.insert("XW", &CBOT_GRAINS_HOURS);  // Mini-sized Wheat
        m.insert("XC", &CBOT_GRAINS_HOURS);  // E-mini Corn
        m.insert("XK", &CBOT_GRAINS_HOURS);  // E-mini Soybeans
        m.insert("KE", &CBOT_GRAINS_HOURS);  // KC Wheat

        // Add any product spreads that follow same schedule
        m.insert("ZS-ZM", &CBOT_GRAINS_HOURS);  // Soybean-Soybean Meal Spread
        m.insert("ZS-ZL", &CBOT_GRAINS_HOURS);  // Soybean-Soybean Oil Spread
        m.insert("ZM-ZL", &CBOT_GRAINS_HOURS);  // Soybean Meal-Soybean Oil Spread

        // German Index Products
        m.insert("FDAX", &EUREX_HOURS);   // DAX Futures
        m.insert("FDXM", &EUREX_HOURS);   // Mini-DAX Futures
        m.insert("OGBL", &EUREX_HOURS);   // Euro-Bund Futures
        m.insert("OGBM", &EUREX_HOURS);   // Mid-Term Euro-Bund Futures
        m.insert("OGBS", &EUREX_HOURS);   // Short-Term Euro-Bund Futures

        // European Index Products
        m.insert("FESX", &EUREX_HOURS);   // EURO STOXX 50 Index Futures
        m.insert("FESB", &EUREX_HOURS);   // EURO STOXX Banks Futures
        m.insert("FESE", &EUREX_HOURS);   // EURO STOXX Select Dividend 30 Futures

        // Volatility Products
        m.insert("V2TX", &EUREX_HOURS);   // VSTOXX Futures
        m.insert("EVIX", &EUREX_HOURS);   // Mini VSTOXX Futures

        m
    };
}

pub fn get_futures_trading_hours(symbol: &str) -> Option<&'static TradingHours> {
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

const EUREX_HOURS: TradingHours = TradingHours {
    timezone: chrono_tz::Europe::Berlin,
    sunday: DaySession {
        open: Some(const_time(8, 0, 0)),   // 08:00 CE(S)T Monday
        close: None,
    },
    monday: DaySession {
        open: None,
        close: Some(const_time(22, 0, 0)),  // 22:00 CE(S)T
    },
    tuesday: DaySession {
        open: Some(const_time(8, 0, 0)),    // 08:00 CE(S)T
        close: Some(const_time(22, 0, 0)),  // 22:00 CE(S)T
    },
    wednesday: DaySession {
        open: Some(const_time(8, 0, 0)),    // 08:00 CE(S)T
        close: Some(const_time(22, 0, 0)),  // 22:00 CE(S)T
    },
    thursday: DaySession {
        open: Some(const_time(8, 0, 0)),    // 08:00 CE(S)T
        close: Some(const_time(22, 0, 0)),  // 22:00 CE(S)T
    },
    friday: DaySession {
        open: Some(const_time(8, 0, 0)),    // 08:00 CE(S)T
        close: Some(const_time(22, 0, 0)),  // 22:00 CE(S)T
    },
};

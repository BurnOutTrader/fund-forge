use ahash::AHashMap;
use crate::standardized_types::subscriptions::SymbolName;


lazy_static! {
    static ref AVAILABLE_SYMBOL_NAMES: Vec<String> = vec![
        // CBOT Futures
        "XC", "XK", "XW", "YM", "ZB", "ZC", "ZF", "ZL", "ZM", "ZN", "ZO", "ZR", "ZS", "ZT", "ZW",

        // CME Futures
        "6A", "6B", "6C", "6E", "6J", "6M", "6N", "6S", "E7", "EMD", "ES", "GE", "GF", "HE", "J7",
        "LE", "NQ", "RF", "SP",

        // COMEX Futures
        "GC", "HG", "QI", "SI",

        // NYMEX Futures
        "CL", "HO", "NG", "PA", "PL", "QM", "RB",

        // Micro Futures
        "MES", "MNQ", "M2K", "MYM", "MGC", "SIL", "MCL", "MBT", "M6A", "M6B", "M6E", "MJY"
    ]
    .into_iter()
    .map(String::from)
    .collect();
}

lazy_static! {
    static ref FUTURES_CODE_TO_NAME: AHashMap<&'static str, &'static str> = {
        let mut futures = AHashMap::new();

        // CBOT Futures
        futures.insert("XC", "CBOT Mini-sized Corn Futures");
        futures.insert("XK", "CBOT Mini-sized Soybean Futures");
        futures.insert("XW", "CBOT Mini-sized Wheat Futures");
        futures.insert("YM", "Mini-sized Dow Futures ($5)");
        futures.insert("ZB", "30 Year US Treasury Bond Futures");
        futures.insert("ZC", "Corn Futures");
        futures.insert("ZF", "5 Year US Treasury Note Futures");
        futures.insert("ZL", "Soybean Oil Futures");
        futures.insert("ZM", "Soybean Meal Futures");
        futures.insert("ZN", "10 Year US Treasury Note Futures");
        futures.insert("ZO", "Oat Futures");
        futures.insert("ZR", "Rough Rice Futures");
        futures.insert("ZS", "Soybean Futures");
        futures.insert("ZT", "2 Year US Treasury Note Futures");
        futures.insert("ZW", "Wheat Futures");

        // CME Futures
        futures.insert("6A", "Australian Dollar");
        futures.insert("6B", "British Pound");
        futures.insert("6C", "Canadian Dollar");
        futures.insert("6E", "Euro Fx");
        futures.insert("6J", "Japanese Yen");
        futures.insert("6M", "Mexican Peso");
        futures.insert("6N", "New Zealand Dollar");
        futures.insert("6S", "Swiss Franc");
        futures.insert("E7", "E-Mini Euro Fx");
        futures.insert("EMD", "E-Mini S&P Midcap 400");
        futures.insert("ES", "E-Mini S&P 500");
        futures.insert("GE", "Eurodollar");
        futures.insert("GF", "E-Livestock Feeder Cattle");
        futures.insert("HE", "Lean Hog");
        futures.insert("J7", "E-Mini Japanese Yen");
        futures.insert("LE", "E-Livestock Live Cattle");
        futures.insert("NQ", "E-Mini Nasdaq-100");
        futures.insert("RF", "Euro Fx/Swiss Franc");
        futures.insert("SP", "S&P 500");

        // COMEX Futures
        futures.insert("GC", "COMEX Gold Futures");
        futures.insert("HG", "COMEX Copper Futures");
        futures.insert("QG", "COMEX miNY Silver Futures");
        futures.insert("QI", "COMEX miNY Silver Futures");
        futures.insert("QO", "COMEX miNY Gold Futures");
        futures.insert("SI", "COMEX Silver Futures");

        // NYMEX Futures
        futures.insert("CL", "Light Sweet Crude Oil");
        futures.insert("HO", "Heating Oil");
        futures.insert("NG", "Natural Gas");
        futures.insert("PA", "NYMEX Palladium");
        futures.insert("PL", "NYMEX Platinum");
        futures.insert("QG", "NYMEX miNY Natural Gas");
        futures.insert("QM", "NYMEX miNY Crude Oil");
        futures.insert("RB", "New York Harbor RBOB Gasoline");

        // Micro Futures
        futures.insert("MES", "Micro E-mini S&P 500");
        futures.insert("MNQ", "Micro E-mini Nasdaq-100");
        futures.insert("M2K", "Micro E-mini Russell 2000");
        futures.insert("MYM", "Micro E-mini Dow");
        futures.insert("MGC", "Micro Gold");
        futures.insert("SIL", "Micro Silver");
        futures.insert("MCL", "Micro Crude Oil");
        futures.insert("MBT", "Micro Bitcoin");
        futures.insert("M6A", "Micro AUD/USD");
        futures.insert("M6B", "Micro GBP/USD");
        futures.insert("M6E", "Micro EUR/USD");
        futures.insert("MJY", "Micro JPY/USD");

        futures
    };
}

use std::collections::HashMap;
use lazy_static::lazy_static;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use chrono::{NaiveTime};
use crate::messages::data_server_messaging::FundForgeError;
use crate::standardized_types::enums::FuturesExchange;
use crate::standardized_types::symbol_info::{CommissionInfo, SymbolInfo};
use crate::standardized_types::accounts::Currency;
use crate::standardized_types::market_hours::{DaySession, TradingHours};

lazy_static! {
    static ref CODE_TO_EXCHANGE_MAP: HashMap<&'static str, FuturesExchange> = {
        let mut map = HashMap::new();

        // CBOT contracts
        for code in ["XC", "XK", "XW", "YM", "ZB", "ZC", "ZF", "ZL", "ZM", "ZN", "ZO", "ZR", "ZS", "ZT", "ZW"] {
            map.insert(code, FuturesExchange::CBOT);
        }

        // CME contracts
        for code in ["6A", "6B", "6C", "6E", "6J", "6M", "6N", "6S", "E7", "EMD", "ES", "GE", "GF", "HE", "J7", "LE", "NQ", "RF", "SP"] {
            map.insert(code, FuturesExchange::CME);
        }

        // COMEX contracts
        for code in ["GC", "HG", "QI", "QQ", "SI", "MGC", "SIL"] {
            map.insert(code, FuturesExchange::COMEX);
        }

        // NYMEX contracts
        for code in ["CL", "HO", "NG", "PA", "PL", "QG", "QM", "RB", "MCL", "MBT"] {
            map.insert(code, FuturesExchange::NYMEX);
        }

        // Micro Futures
        for code in ["MES", "MNQ", "M2K", "MYM", "M6A", "M6B", "M6E", "MJY"] {
            map.insert(code, FuturesExchange::CME); // All micro contracts are typically on CME
        }

        map
    };
}

pub fn get_futures_exchange(code: &str) -> Result<FuturesExchange, FundForgeError> {
    match CODE_TO_EXCHANGE_MAP.get(code).copied() {
        Some(exchange) => Ok(exchange),
        None => Err(FundForgeError::ClientSideErrorDebug(format!("Unknown futures code: {}, please add mapping", code))),
    }
}

lazy_static! {
    static ref SYMBOL_INFO_MAP: HashMap<&'static str, SymbolInfo> = {
        let mut map = HashMap::new();

        macro_rules! add_symbol {
            ($symbol:expr, $currency:expr, $value_per_tick:expr, $tick_size:expr, $accuracy:expr) => {
                map.insert($symbol, SymbolInfo {
                    symbol_name: $symbol.to_string(),
                    pnl_currency: $currency,
                    value_per_tick: dec!($value_per_tick),
                    tick_size: dec!($tick_size),
                    decimal_accuracy: $accuracy,
                    base_currency: None,
                });
            };
        }

        // Grains
        add_symbol!("XC", Currency::USD, 1.25, 0.25, 2);  // 5.0/4
        add_symbol!("XK", Currency::USD, 1.25, 0.25, 2);  // 5.0/4
        add_symbol!("XW", Currency::USD, 1.25, 0.25, 2);  // 5.0/4
        add_symbol!("YM", Currency::USD, 5.0, 1.0, 0);    // Already correct as 1 tick = 1 point
        add_symbol!("ZB", Currency::USD, 1.953125, 0.0625, 4);  // 31.25/16
        add_symbol!("ZC", Currency::USD, 12.5, 0.25, 2);  // 50.0/4
        add_symbol!("ZF", Currency::USD, 0.244140625, 0.0078125, 6);  // 31.25/128
        add_symbol!("ZL", Currency::USD, 6.0, 0.01, 4);   // 600.0/100
        add_symbol!("ZM", Currency::USD, 10.0, 0.1, 1);   // 100.0/10
        add_symbol!("ZN", Currency::USD, 0.48828125, 0.015625, 5);  // 31.25/64
        add_symbol!("ZO", Currency::USD, 12.5, 0.25, 2);  // 50.0/4
        add_symbol!("ZR", Currency::USD, 0.25, 0.005, 4); // 50.0/200
        add_symbol!("ZS", Currency::USD, 12.5, 0.25, 2);  // 50.0/4
        add_symbol!("ZT", Currency::USD, 0.244140625, 0.0078125, 6);  // 31.25/128
        add_symbol!("ZW", Currency::USD, 12.5, 0.25, 2);  // 50.0/4

        // CME Futures
        add_symbol!("6A", Currency::USD, 1.0, 0.0001, 4);  // 10.0/10
        add_symbol!("6B", Currency::USD, 0.625, 0.0001, 4);  // 6.25/10
        add_symbol!("6C", Currency::USD, 1.0, 0.0001, 4);  // 10.0/10
        add_symbol!("6E", Currency::USD, 1.25, 0.0001, 4);  // 12.5/10
        add_symbol!("6J", Currency::USD, 0.0125, 0.000001, 6);  // 12.5/1000
        add_symbol!("6M", Currency::USD, 0.1, 0.00001, 5);  // 10.0/100
        add_symbol!("6N", Currency::USD, 1.0, 0.0001, 4);  // 10.0/10
        add_symbol!("6S", Currency::USD, 1.25, 0.0001, 4);  // 12.5/10
        add_symbol!("E7", Currency::USD, 0.625, 0.0001, 4);  // 6.25/10
        add_symbol!("EMD", Currency::USD, 2.5, 0.05, 2);  // 50.0/20
        add_symbol!("ES", Currency::USD, 12.5, 0.25, 2);  // 50.0/4
        add_symbol!("GE", Currency::USD, 0.0625, 0.0025, 4);  // 25.0/400
        add_symbol!("GF", Currency::USD, 1.25, 0.025, 3);  // 50.0/40
        add_symbol!("HE", Currency::USD, 0.1, 0.0025, 4);  // 40.0/400
        add_symbol!("J7", Currency::USD, 0.00625, 0.000001, 6);  // 6.25/1000
        add_symbol!("LE", Currency::USD, 1.0, 0.025, 3);  // 40.0/40
        add_symbol!("NQ", Currency::USD, 5.0, 0.25, 2);  // 20.0/4
        add_symbol!("RF", Currency::USD, 1.25, 0.0001, 4);  // 12.5/10
        add_symbol!("SP", Currency::USD, 25.0, 0.1, 2);  // 250.0/10

        // COMEX Futures
        add_symbol!("GC", Currency::USD, 10.0, 0.1, 2);  // 100.0/10
        add_symbol!("HG", Currency::USD, 0.0125, 0.0005, 4);  // 25.0/2000
        add_symbol!("QI", Currency::USD, 0.03125, 0.0025, 4);  // 12.5/400
        add_symbol!("SI", Currency::USD, 0.125, 0.005, 3);  // 25.0/200

        // NYMEX Futures
        add_symbol!("CL", Currency::USD, 10.0, 0.01, 2);  // 1000.0/100
        add_symbol!("HO", Currency::USD, 4.2, 0.0001, 4);  // 42000.0/10000
        add_symbol!("NG", Currency::USD, 10.0, 0.001, 3);  // 10000.0/1000
        add_symbol!("PA", Currency::USD, 5.0, 0.05, 2);  // 100.0/20
        add_symbol!("PL", Currency::USD, 5.0, 0.1, 2);  // 50.0/10
        add_symbol!("QM", Currency::USD, 5.0, 0.01, 2);  // 500.0/100
        add_symbol!("RB", Currency::USD, 4.2, 0.0001, 4);  // 42000.0/10000

        // Micro Futures
        add_symbol!("MES", Currency::USD, 1.25, 0.25, 2);  // 5.0/4
        add_symbol!("MNQ", Currency::USD, 0.50, 0.25, 2);  // 2.0/4
        add_symbol!("M2K", Currency::USD, 0.50, 0.1, 2);  // 5.0/10
        add_symbol!("MYM", Currency::USD, 0.50, 1.0, 0);  // Already correct
        add_symbol!("MGC", Currency::USD, 1.0, 0.1, 2);  // 10.0/10
        add_symbol!("SIL", Currency::USD, 0.0125, 0.005, 3);  // 2.5/200
        add_symbol!("MCL", Currency::USD, 1.0, 0.01, 2);  // 100.0/100
        add_symbol!("MBT", Currency::USD, 1.25, 0.25, 2);  // 5.0/4
        add_symbol!("M6A", Currency::USD, 0.1, 0.0001, 4);  // 1.0/10
        add_symbol!("M6B", Currency::USD, 0.0625, 0.0001, 4);  // 0.625/10
        add_symbol!("M6E", Currency::USD, 0.125, 0.0001, 4);  // 1.25/10
        add_symbol!("MJY", Currency::USD, 0.00125, 0.000001, 6);  // 1.25/1000

        map
    };
}

pub fn get_futures_symbol_info(symbol: &str) -> Result<SymbolInfo, FundForgeError> {
    match SYMBOL_INFO_MAP.get(symbol) {
        Some(info) => Ok(info.clone()),
        None => Err(FundForgeError::ClientSideErrorDebug(format!("Unknown futures symbol: {}, please add mapping", symbol))),
    }
}

lazy_static! {
    static ref INTRADAY_MARGINS: HashMap<&'static str, Decimal> = {
        let mut map = HashMap::new();
        map.insert("MES", dec!(40.00));
        map.insert("MNQ", dec!(100.00));
        map.insert("MYM", dec!(50.00));
        map.insert("M2K", dec!(50.00));
        map.insert("ES", dec!(400.00));
        map.insert("NQ", dec!(1000.00));
        map.insert("YM", dec!(500.00));
        map.insert("RTY", dec!(500.00));
        map.insert("EMD", dec!(3775.00));
        map.insert("NKD", dec!(2250.00));
        map.insert("6A", dec!(362.50));
        map.insert("6B", dec!(475.00));
        map.insert("6C", dec!(250.00));
        map.insert("6E", dec!(525.00));
        map.insert("6J", dec!(700.00));
        map.insert("6N", dec!(350.00));
        map.insert("6S", dec!(925.00));
        map.insert("E7", dec!(262.50));
        map.insert("J7", dec!(350.00));
        map.insert("M6A", dec!(36.25));
        map.insert("M6B", dec!(47.50));
        map.insert("M6E", dec!(52.50));
        map.insert("MJY", dec!(70.00));
        map.insert("CL", dec!(1650.00));
        map.insert("QM", dec!(825.00));
        map.insert("MCL", dec!(165.00));
        map.insert("NG", dec!(5500.00));
        map.insert("QG", dec!(1460.00));
        map.insert("RB", dec!(7900.00));
        map.insert("HO", dec!(8600.00));
        map.insert("GC", dec!(2075.00));
        map.insert("QO", dec!(1037.50));
        map.insert("MGC", dec!(207.50));
        map.insert("HG", dec!(1525.00));
        map.insert("QC", dec!(762.50));
        map.insert("SI", dec!(11000.00));
        map.insert("QI", dec!(5500.00));
        map.insert("SIL", dec!(2200.00));
        map.insert("PL", dec!(2800.00));
        map.insert("ZB", dec!(925.00));
        map.insert("ZF", dec!(350.00));
        map.insert("ZN", dec!(500.00));
        map.insert("ZT", dec!(262.50));
        map.insert("ZC", dec!(1300.00));
        map.insert("ZW", dec!(2000.00));
        map.insert("ZS", dec!(2400.00));
        map.insert("ZL", dec!(3150.00));
        map.insert("ZM", dec!(3100.00));
        map.insert("ZO", dec!(1400.00));
        map.insert("ZR", dec!(1575.00));
        map.insert("XC", dec!(260.00));
        map.insert("XW", dec!(400.00));
        map.insert("XK", dec!(480.00));
        map
    };

    static ref OVERNIGHT_MARGINS: HashMap<&'static str, Decimal> = {
        let mut map = HashMap::new();
        map.insert("MES", dec!(1460.00));
        map.insert("MNQ", dec!(2220.00));
        map.insert("MYM", dec!(1040.00));
        map.insert("M2K", dec!(760.00));
        map.insert("ES", dec!(14600.00));
        map.insert("NQ", dec!(22200.00));
        map.insert("YM", dec!(10400.00));
        map.insert("RTY", dec!(7600.00));
        map.insert("EMD", dec!(15100.00));
        map.insert("NKD", dec!(12000.00));
        map.insert("6A", dec!(1450.00));
        map.insert("6B", dec!(1900.00));
        map.insert("6C", dec!(1000.00));
        map.insert("6E", dec!(2100.00));
        map.insert("6J", dec!(2800.00));
        map.insert("6N", dec!(1450.00));
        map.insert("6S", dec!(3700.00));
        map.insert("E7", dec!(1050.00));
        map.insert("J7", dec!(1400.00));
        map.insert("M6A", dec!(145.00));
        map.insert("M6B", dec!(190.00));
        map.insert("M6E", dec!(210.00));
        map.insert("MJY", dec!(280.00));
        map.insert("CL", dec!(6600.00));
        map.insert("QM", dec!(3300.00));
        map.insert("MCL", dec!(660.00));
        map.insert("NG", dec!(5500.00));
        map.insert("QG", dec!(1460.00));
        map.insert("RB", dec!(7900.00));
        map.insert("HO", dec!(8600.00));
        map.insert("GC", dec!(10000.00));
        map.insert("QO", dec!(5000.00));
        map.insert("MGC", dec!(1000.00));
        map.insert("HG", dec!(6100.00));
        map.insert("QC", dec!(3050.00));
        map.insert("SI", dec!(11000.00));
        map.insert("QI", dec!(5500.00));
        map.insert("SIL", dec!(2200.00));
        map.insert("PL", dec!(2800.00));
        map.insert("ZB", dec!(3700.00));
        map.insert("ZF", dec!(1400.00));
        map.insert("ZN", dec!(2000.00));
        map.insert("ZT", dec!(1050.00));
        map.insert("ZC", dec!(1300.00));
        map.insert("ZW", dec!(2000.00));
        map.insert("ZS", dec!(2400.00));
        map.insert("ZL", dec!(3150.00));
        map.insert("ZM", dec!(3100.00));
        map.insert("ZO", dec!(1400.00));
        map.insert("ZR", dec!(1575.00));
        map.insert("XC", dec!(260.00));
        map.insert("XW", dec!(400.00));
        map.insert("XK", dec!(480.00));
        map
    };
}

lazy_static! {
    static ref COMMISSION_PER_CONTRACT: HashMap<&'static str, CommissionInfo> = {
        let mut map = HashMap::new();

        // Stock Index Futures
        map.insert("YM", CommissionInfo { per_side: dec!(1.90), currency: Currency::USD });
        map.insert("ZDJ", CommissionInfo { per_side: dec!(1.54), currency: Currency::USD });
        map.insert("M2K", CommissionInfo { per_side: dec!(0.50), currency: Currency::USD });
        map.insert("MES", CommissionInfo { per_side: dec!(0.50), currency: Currency::USD });
        map.insert("FDXS", CommissionInfo { per_side: dec!(0.27), currency: Currency::EUR });
        map.insert("MYM", CommissionInfo { per_side: dec!(0.50), currency: Currency::USD });
        map.insert("FSXE", CommissionInfo { per_side: dec!(0.23), currency: Currency::EUR });
        map.insert("ES", CommissionInfo { per_side: dec!(1.90), currency: Currency::USD });
        map.insert("MJNK", CommissionInfo { per_side: dec!(39.90), currency: Currency::JPY });
        map.insert("MNQ", CommissionInfo { per_side: dec!(0.50), currency: Currency::USD });
        map.insert("NQ", CommissionInfo { per_side: dec!(1.90), currency: Currency::USD });
        map.insert("EMD", CommissionInfo { per_side: dec!(1.85), currency: Currency::USD });
        map.insert("NKD", CommissionInfo { per_side: dec!(2.88), currency: Currency::USD });
        map.insert("SP", CommissionInfo { per_side: dec!(2.88), currency: Currency::USD });
        map.insert("ZND", CommissionInfo { per_side: dec!(2.88), currency: Currency::USD });
        map.insert("FXXP", CommissionInfo { per_side: dec!(0.90), currency: Currency::EUR });
        map.insert("FDAX", CommissionInfo { per_side: dec!(1.77), currency: Currency::EUR });
        map.insert("FESB", CommissionInfo { per_side: dec!(0.80), currency: Currency::EUR });
        map.insert("FESX", CommissionInfo { per_side: dec!(0.90), currency: Currency::EUR });
        map.insert("FDXM", CommissionInfo { per_side: dec!(0.76), currency: Currency::EUR });
        map.insert("RTY", CommissionInfo { per_side: dec!(1.90), currency: Currency::USD });
        map.insert("VX", CommissionInfo { per_side: dec!(2.27), currency: Currency::USD });
        map.insert("FVS", CommissionInfo { per_side: dec!(0.72), currency: Currency::EUR });
        map.insert("VXM", CommissionInfo { per_side: dec!(0.35), currency: Currency::USD });

        // Currency Futures
        map.insert("6Z", CommissionInfo { per_side: dec!(2.13), currency: Currency::USD });
        map.insert("RMB", CommissionInfo { per_side: dec!(2.13), currency: Currency::USD });
        map.insert("6M", CommissionInfo { per_side: dec!(2.13), currency: Currency::USD });
        map.insert("TRE", CommissionInfo { per_side: dec!(2.13), currency: Currency::USD });
        map.insert("6L", CommissionInfo { per_side: dec!(2.13), currency: Currency::USD });
        map.insert("6N", CommissionInfo { per_side: dec!(2.13), currency: Currency::USD });
        map.insert("PLN", CommissionInfo { per_side: dec!(2.13), currency: Currency::USD });
        map.insert("SEK", CommissionInfo { per_side: dec!(2.13), currency: Currency::USD });
        map.insert("TRY", CommissionInfo { per_side: dec!(2.13), currency: Currency::USD });
        map.insert("6A", CommissionInfo { per_side: dec!(2.12), currency: Currency::USD });
        map.insert("6B", CommissionInfo { per_side: dec!(2.13), currency: Currency::USD });
        map.insert("6C", CommissionInfo { per_side: dec!(2.13), currency: Currency::USD });
        map.insert("6E", CommissionInfo { per_side: dec!(2.13), currency: Currency::USD });
        map.insert("6J", CommissionInfo { per_side: dec!(2.13), currency: Currency::USD });
        map.insert("6S", CommissionInfo { per_side: dec!(2.13), currency: Currency::USD });
        map.insert("E7", CommissionInfo { per_side: dec!(1.38), currency: Currency::USD });
        map.insert("J7", CommissionInfo { per_side: dec!(1.38), currency: Currency::USD });
        map.insert("M6A", CommissionInfo { per_side: dec!(0.39), currency: Currency::USD });
        map.insert("M6B", CommissionInfo { per_side: dec!(0.39), currency: Currency::USD });
        map.insert("MCD", CommissionInfo { per_side: dec!(0.39), currency: Currency::USD });
        map.insert("M6E", CommissionInfo { per_side: dec!(0.39), currency: Currency::USD });
        map.insert("MJY", CommissionInfo { per_side: dec!(0.39), currency: Currency::USD });
        map.insert("MSF", CommissionInfo { per_side: dec!(0.39), currency: Currency::USD });
        map.insert("DX", CommissionInfo { per_side: dec!(1.88), currency: Currency::USD });

        // Energy Futures
        map.insert("CL", CommissionInfo { per_side: dec!(2.13), currency: Currency::USD });
        map.insert("MCL", CommissionInfo { per_side: dec!(0.65), currency: Currency::USD });
        map.insert("MNG", CommissionInfo { per_side: dec!(0.75), currency: Currency::USD });
        map.insert("HO", CommissionInfo { per_side: dec!(2.13), currency: Currency::USD });
        map.insert("NG", CommissionInfo { per_side: dec!(2.13), currency: Currency::USD });
        map.insert("QG", CommissionInfo { per_side: dec!(1.03), currency: Currency::USD });
        map.insert("QH", CommissionInfo { per_side: dec!(1.73), currency: Currency::USD });
        map.insert("QM", CommissionInfo { per_side: dec!(1.73), currency: Currency::USD });
        map.insert("QU", CommissionInfo { per_side: dec!(1.73), currency: Currency::USD });
        map.insert("RB", CommissionInfo { per_side: dec!(2.13), currency: Currency::USD });

        // Metals Futures
        map.insert("GC", CommissionInfo { per_side: dec!(2.12), currency: Currency::USD });
        map.insert("HG", CommissionInfo { per_side: dec!(2.12), currency: Currency::USD });
        map.insert("MHG", CommissionInfo { per_side: dec!(0.75), currency: Currency::USD });
        map.insert("MGC", CommissionInfo { per_side: dec!(0.65), currency: Currency::USD });
        map.insert("QC", CommissionInfo { per_side: dec!(1.54), currency: Currency::USD });
        map.insert("QI", CommissionInfo { per_side: dec!(1.54), currency: Currency::USD });
        map.insert("QO", CommissionInfo { per_side: dec!(1.54), currency: Currency::USD });
        map.insert("SI", CommissionInfo { per_side: dec!(2.12), currency: Currency::USD });
        map.insert("SIL", CommissionInfo { per_side: dec!(1.15), currency: Currency::USD });
        map.insert("PA", CommissionInfo { per_side: dec!(2.07), currency: Currency::USD });
        map.insert("PL", CommissionInfo { per_side: dec!(2.12), currency: Currency::USD });

        // Financial Futures
        map.insert("SR", CommissionInfo { per_side: dec!(1.13), currency: Currency::USD });
        map.insert("10YY", CommissionInfo { per_side: dec!(0.45), currency: Currency::USD });
        map.insert("30YY", CommissionInfo { per_side: dec!(0.45), currency: Currency::USD });
        map.insert("2YY", CommissionInfo { per_side: dec!(0.45), currency: Currency::USD });
        map.insert("5YY", CommissionInfo { per_side: dec!(0.45), currency: Currency::USD });
        map.insert("UB", CommissionInfo { per_side: dec!(1.47), currency: Currency::USD });
        map.insert("MWN", CommissionInfo { per_side: dec!(0.45), currency: Currency::USD });
        map.insert("JGB", CommissionInfo { per_side: dec!(499.90), currency: Currency::JPY });
        map.insert("MTN", CommissionInfo { per_side: dec!(0.45), currency: Currency::USD });
        map.insert("Z3N", CommissionInfo { per_side: dec!(1.17), currency: Currency::USD });
        map.insert("ZB", CommissionInfo { per_side: dec!(1.39), currency: Currency::USD });
        map.insert("ZF", CommissionInfo { per_side: dec!(1.17), currency: Currency::USD });
        map.insert("ZN", CommissionInfo { per_side: dec!(1.32), currency: Currency::USD });
        map.insert("TN", CommissionInfo { per_side: dec!(1.32), currency: Currency::USD });
        map.insert("ZQ", CommissionInfo { per_side: dec!(1.49), currency: Currency::USD });
        map.insert("ZT", CommissionInfo { per_side: dec!(1.17), currency: Currency::USD });
        map.insert("GE", CommissionInfo { per_side: dec!(1.72), currency: Currency::USD });
        map.insert("GLB", CommissionInfo { per_side: dec!(1.72), currency: Currency::USD });
        map.insert("FGBL", CommissionInfo { per_side: dec!(0.77), currency: Currency::EUR });
        map.insert("FGBM", CommissionInfo { per_side: dec!(0.77), currency: Currency::EUR });
        map.insert("FGBS", CommissionInfo { per_side: dec!(0.77), currency: Currency::EUR });
        map.insert("FBTP", CommissionInfo { per_side: dec!(0.74), currency: Currency::EUR });
        map.insert("FOAT", CommissionInfo { per_side: dec!(0.77), currency: Currency::EUR });
        map.insert("FGBX", CommissionInfo { per_side: dec!(0.77), currency: Currency::EUR });
        map.insert("FBTS", CommissionInfo { per_side: dec!(0.77), currency: Currency::EUR });

        // Grains Futures
        map.insert("XC", CommissionInfo { per_side: dec!(1.55), currency: Currency::USD });
        map.insert("XK", CommissionInfo { per_side: dec!(1.56), currency: Currency::USD });
        map.insert("XW", CommissionInfo { per_side: dec!(1.56), currency: Currency::USD });
        map.insert("ZC", CommissionInfo { per_side: dec!(2.62), currency: Currency::USD });
        map.insert("ZE", CommissionInfo { per_side: dec!(2.62), currency: Currency::USD });
        map.insert("ZL", CommissionInfo { per_side: dec!(2.62), currency: Currency::USD });
        map.insert("ZM", CommissionInfo { per_side: dec!(2.62), currency: Currency::USD });
        map.insert("ZO", CommissionInfo { per_side: dec!(2.62), currency: Currency::USD });
        map.insert("ZR", CommissionInfo { per_side: dec!(2.62), currency: Currency::USD });
        map.insert("ZS", CommissionInfo { per_side: dec!(2.62), currency: Currency::USD });
        map.insert("ZW", CommissionInfo { per_side: dec!(2.62), currency: Currency::USD });

        // Softs Futures
        map.insert("DA", CommissionInfo { per_side: dec!(2.42), currency: Currency::USD });
        map.insert("LBS", CommissionInfo { per_side: dec!(2.42), currency: Currency::USD });
        map.insert("CC", CommissionInfo { per_side: dec!(2.63), currency: Currency::USD });
        map.insert("CT", CommissionInfo { per_side: dec!(2.63), currency: Currency::USD });
        map.insert("KC", CommissionInfo { per_side: dec!(2.62), currency: Currency::USD });
        map.insert("OJ", CommissionInfo { per_side: dec!(2.63), currency: Currency::USD });
        map.insert("SB", CommissionInfo { per_side: dec!(2.63), currency: Currency::USD });

        // Meats Futures
        map.insert("GF", CommissionInfo { per_side: dec!(2.62), currency: Currency::USD });
        map.insert("HE", CommissionInfo { per_side: dec!(2.62), currency: Currency::USD });
        map.insert("LE", CommissionInfo { per_side: dec!(2.62), currency: Currency::USD });

        map
    };
}

pub fn find_base_symbol(symbol: &SymbolName) -> Option<String> {
    // Check if the full symbol is in the list
    if AVAILABLE_SYMBOL_NAMES.contains(symbol) {
        return Some(symbol.to_string());
    }

    // Check if the first three characters of the symbol are in the list
    if symbol.len() >= 3 {
        let first_three = &symbol[..3];
        if AVAILABLE_SYMBOL_NAMES.iter().any(|s| s.as_str() == first_three) {
            return Some(first_three.to_string());
        }
    }

    // Check if the symbol without the last two characters is in the list
    if symbol.len() > 2 {
        let without_last_two = &symbol[..symbol.len() - 2];
        if AVAILABLE_SYMBOL_NAMES.iter().any(|s| s.as_str() == without_last_two) {
            return Some(without_last_two.to_string());
        }
    }

    None
}

pub fn get_available_rithmic_symbol_names() -> &'static Vec<String> {
    &AVAILABLE_SYMBOL_NAMES
}

#[allow(dead_code)]
pub fn futures_code_to_name() -> &'static AHashMap<&'static str, &'static str> {
    &FUTURES_CODE_TO_NAME
}

#[allow(dead_code)]
pub fn get_exchange_by_symbol_name(code: &str) -> Option<FuturesExchange> {
    match CODE_TO_EXCHANGE_MAP.get(code) {
        Some(exchange) => Some(*exchange),
        None => None,
    }
}

pub fn get_rithmic_symbol_info(symbol: &str) -> Result<SymbolInfo, String> {
    SYMBOL_INFO_MAP.get(symbol)
        .cloned()
        .ok_or_else(|| format!("{} not found", symbol))
}

pub fn get_rithmic_intraday_margin_in_usd(symbol: &str) -> Option<Decimal> {
    INTRADAY_MARGINS.get(symbol).cloned()
}

pub fn get_overnight_margin(symbol: &str) -> Option<Decimal> {
    OVERNIGHT_MARGINS.get(symbol).cloned()
}

pub fn get_futures_commissions_info(symbol_name: &SymbolName) -> Result<CommissionInfo, String> {
    if let Some(commission_info) = COMMISSION_PER_CONTRACT.get(symbol_name.as_str()) {
        return Ok(commission_info.clone())
    }
    Err(format!("No Symbol Found: {}", symbol_name))
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

pub const CME_HOURS: TradingHours = TradingHours {
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
    saturday: DaySession {
        open: None,
        close: None,
    },
};
// CBOT Grains Schedule
pub const CBOT_GRAINS_HOURS: TradingHours = TradingHours {
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
    saturday: DaySession {
        open: None,
        close: None,
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
    saturday: DaySession {
        open: None,
        close: None,
    },
};


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
        m.insert("CL", &CME_HOURS);   // Crude Oil Futures
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

lazy_static! {
    static ref MONTHLY_ROLLOVER_DAYS: AHashMap<&'static str, u32> = {
        let mut map = AHashMap::new();

        // NYMEX Contracts
        map.insert("CL", 18);
        map.insert("HO", 25);
        map.insert("NG", 28);
        map.insert("RB", 25);
        map.insert("PA", 25);
        map.insert("PL", 25);
        map.insert("QG", 28);
        map.insert("QM", 18);
        map.insert("MCL", 18);

        // COMEX Contracts
        map.insert("GC", 28);
        map.insert("HG", 28);
        map.insert("QI", 25);
        map.insert("QQ", 25);
        map.insert("SI", 25);

        // Micro Futures
        map.insert("MGC", 28);
        map.insert("SIL", 25);

        map
    };
}

pub fn extract_symbol_from_contract(contract: &str) -> String {
    // Ensure the contract is long enough to contain a symbol and month-year code
    if contract.len() < 4 {
        return contract.to_string();
    }

    // Extract the symbol by removing the last three characters (month and year)
    let symbol = &contract[..contract.len() - 3];

    symbol.to_string()
}


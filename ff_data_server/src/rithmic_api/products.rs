use ahash::AHashMap;
use rust_decimal_macros::dec;

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

pub fn get_available_symbol_names() -> &'static Vec<String> {
    &AVAILABLE_SYMBOL_NAMES
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

#[allow(dead_code)]
pub fn futures_code_to_name() -> &'static AHashMap<&'static str, &'static str> {
    &FUTURES_CODE_TO_NAME
}

use std::collections::HashMap;
use lazy_static::lazy_static;
use ff_standard_lib::standardized_types::enums::FuturesExchange;
use ff_standard_lib::standardized_types::symbol_info::SymbolInfo;
use ff_standard_lib::strategies::ledgers::Currency;
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
        for code in ["GC", "HG", "QI", "QQ", "SI"] {
            map.insert(code, FuturesExchange::COMEX);
        }

        // NYMEX contracts
        for code in ["CL", "HO", "NG", "PA", "PL", "QG", "QM", "RB"] {
            map.insert(code, FuturesExchange::NYMEX);
        }

        // Micro Futures
        for code in ["MES", "MNQ", "M2K", "MYM", "MGC", "SIL", "MCL", "MBT", "M6A", "M6B", "M6E", "MJY"] {
            map.insert(code, FuturesExchange::CME); // All micro contracts are typically on CME
        }

        map
    };
}

#[allow(dead_code)]
pub fn get_exchange_by_code(code: &str) -> Option<FuturesExchange> {
    CODE_TO_EXCHANGE_MAP.get(code).cloned()
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
                });
            };
        }

    add_symbol!("XC", Currency::USD, 5.0, 0.25, 2);
    add_symbol!("XK", Currency::USD, 5.0, 0.25, 2);
    add_symbol!("XW", Currency::USD, 5.0, 0.25, 2);
    add_symbol!("YM", Currency::USD, 5.0, 1.0, 0);
    add_symbol!("ZB", Currency::USD, 31.25, 0.0625, 4);
    add_symbol!("ZC", Currency::USD, 50.0, 0.25, 2);
    add_symbol!("ZF", Currency::USD, 31.25, 0.0078125, 6);
    add_symbol!("ZL", Currency::USD, 600.0, 0.01, 4);
    add_symbol!("ZM", Currency::USD, 100.0, 0.1, 1);
    add_symbol!("ZN", Currency::USD, 31.25, 0.015625, 5);
    add_symbol!("ZO", Currency::USD, 50.0, 0.25, 2);
    add_symbol!("ZR", Currency::USD, 50.0, 0.005, 4);
    add_symbol!("ZS", Currency::USD, 50.0, 0.25, 2);
    add_symbol!("ZT", Currency::USD, 31.25, 0.0078125, 6);
    add_symbol!("ZW", Currency::USD, 50.0, 0.25, 2);

    // CME Futures
    add_symbol!("6A", Currency::USD, 10.0, 0.0001, 4);
    add_symbol!("6B", Currency::USD, 6.25, 0.0001, 4);
    add_symbol!("6C", Currency::USD, 10.0, 0.0001, 4);
    add_symbol!("6E", Currency::USD, 12.5, 0.0001, 4);
    add_symbol!("6J", Currency::USD, 12.5, 0.000001, 6);
    add_symbol!("6M", Currency::USD, 10.0, 0.00001, 5);
    add_symbol!("6N", Currency::USD, 10.0, 0.0001, 4);
    add_symbol!("6S", Currency::USD, 12.5, 0.0001, 4);
    add_symbol!("E7", Currency::USD, 6.25, 0.0001, 4);
    add_symbol!("EMD", Currency::USD, 50.0, 0.05, 2);
    add_symbol!("ES", Currency::USD, 50.0, 0.25, 2);
    add_symbol!("GE", Currency::USD, 25.0, 0.0025, 4);
    add_symbol!("GF", Currency::USD, 50.0, 0.025, 3);
    add_symbol!("HE", Currency::USD, 40.0, 0.0025, 4);
    add_symbol!("J7", Currency::USD, 6.25, 0.000001, 6);
    add_symbol!("LE", Currency::USD, 40.0, 0.025, 3);
    add_symbol!("NQ", Currency::USD, 20.0, 0.25, 2);
    add_symbol!("RF", Currency::USD, 12.5, 0.0001, 4);
    add_symbol!("SP", Currency::USD, 250.0, 0.1, 2);

    // COMEX Futures
    add_symbol!("GC", Currency::USD, 100.0, 0.1, 2);
    add_symbol!("HG", Currency::USD, 25.0, 0.0005, 4);
    add_symbol!("QI", Currency::USD, 12.5, 0.0025, 4);
    add_symbol!("SI", Currency::USD, 25.0, 0.005, 3);

    // NYMEX Futures
    add_symbol!("CL", Currency::USD, 1000.0, 0.01, 2);
    add_symbol!("HO", Currency::USD, 42000.0, 0.0001, 4);
    add_symbol!("NG", Currency::USD, 10000.0, 0.001, 3);
    add_symbol!("PA", Currency::USD, 100.0, 0.05, 2);
    add_symbol!("PL", Currency::USD, 50.0, 0.1, 2);
    add_symbol!("QM", Currency::USD, 500.0, 0.01, 2);
    add_symbol!("RB", Currency::USD, 42000.0, 0.0001, 4);

    // Micro Futures
    add_symbol!("MES", Currency::USD, 5.0, 0.25, 2);
    add_symbol!("MNQ", Currency::USD, 2.0, 0.25, 2);
    add_symbol!("M2K", Currency::USD, 5.0, 0.1, 2);
    add_symbol!("MYM", Currency::USD, 0.5, 1.0, 0);
    add_symbol!("MGC", Currency::USD, 10.0, 0.1, 2);
    add_symbol!("SIL", Currency::USD, 2.5, 0.005, 3);
    add_symbol!("MCL", Currency::USD, 100.0, 0.01, 2);
    add_symbol!("MBT", Currency::USD, 5.0, 0.25, 2);
    add_symbol!("M6A", Currency::USD, 1.0, 0.0001, 4);
    add_symbol!("M6B", Currency::USD, 0.625, 0.0001, 4);
    add_symbol!("M6E", Currency::USD, 1.25, 0.0001, 4);
    add_symbol!("MJY", Currency::USD, 1.25, 0.000001, 6);

        map
    };
}

pub fn get_symbol_info(symbol: &str) -> Result<SymbolInfo, String> {
    SYMBOL_INFO_MAP.get(symbol)
        .cloned()
        .ok_or_else(|| format!("{} not found", symbol))
}



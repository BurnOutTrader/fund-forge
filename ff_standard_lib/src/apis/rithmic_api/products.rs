use ahash::AHashMap;

pub fn futures_code_to_name() -> AHashMap<&'static str, &'static str> {
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

    futures
}

use std::collections::HashMap;
use crate::standardized_types::enums::Exchange;

// Function to map contract code to an exchange using the Exchange enum
pub fn get_code_to_exchange_map() -> HashMap<&'static str, Exchange> {
    let mut code_to_exchange_map = HashMap::new();
    // CBOT contracts
    code_to_exchange_map.insert("XC", Exchange::CBOT);
    code_to_exchange_map.insert("XK", Exchange::CBOT);
    code_to_exchange_map.insert("XW", Exchange::CBOT);
    code_to_exchange_map.insert("YM", Exchange::CBOT);
    code_to_exchange_map.insert("ZB", Exchange::CBOT);
    code_to_exchange_map.insert("ZC", Exchange::CBOT);
    code_to_exchange_map.insert("ZF", Exchange::CBOT);
    code_to_exchange_map.insert("ZL", Exchange::CBOT);
    code_to_exchange_map.insert("ZM", Exchange::CBOT);
    code_to_exchange_map.insert("ZN", Exchange::CBOT);
    code_to_exchange_map.insert("ZO", Exchange::CBOT);
    code_to_exchange_map.insert("ZR", Exchange::CBOT);
    code_to_exchange_map.insert("ZS", Exchange::CBOT);
    code_to_exchange_map.insert("ZT", Exchange::CBOT);
    code_to_exchange_map.insert("ZW", Exchange::CBOT);

    // CME contracts
    code_to_exchange_map.insert("6A", Exchange::CME);
    code_to_exchange_map.insert("6B", Exchange::CME);
    code_to_exchange_map.insert("6C", Exchange::CME);
    code_to_exchange_map.insert("6E", Exchange::CME);
    code_to_exchange_map.insert("6J", Exchange::CME);
    code_to_exchange_map.insert("6M", Exchange::CME);
    code_to_exchange_map.insert("6N", Exchange::CME);
    code_to_exchange_map.insert("6S", Exchange::CME);
    code_to_exchange_map.insert("E7", Exchange::CME);
    code_to_exchange_map.insert("EMD", Exchange::CME);
    code_to_exchange_map.insert("ES", Exchange::CME);
    code_to_exchange_map.insert("GE", Exchange::CME);
    code_to_exchange_map.insert("GF", Exchange::CME);
    code_to_exchange_map.insert("HE", Exchange::CME);
    code_to_exchange_map.insert("J7", Exchange::CME);
    code_to_exchange_map.insert("LE", Exchange::CME);
    code_to_exchange_map.insert("NQ", Exchange::CME);
    code_to_exchange_map.insert("RF", Exchange::CME);
    code_to_exchange_map.insert("SP", Exchange::CME);

    // COMEX contracts
    code_to_exchange_map.insert("GC", Exchange::COMEX);
    code_to_exchange_map.insert("HG", Exchange::COMEX);
    code_to_exchange_map.insert("QI", Exchange::COMEX);
    code_to_exchange_map.insert("QQ", Exchange::COMEX);
    code_to_exchange_map.insert("SI", Exchange::COMEX);

    // NYMEX contracts
    code_to_exchange_map.insert("CL", Exchange::NYMEX);
    code_to_exchange_map.insert("HO", Exchange::NYMEX);
    code_to_exchange_map.insert("NG", Exchange::NYMEX);
    code_to_exchange_map.insert("PA", Exchange::NYMEX);
    code_to_exchange_map.insert("PL", Exchange::NYMEX);
    code_to_exchange_map.insert("QG", Exchange::NYMEX);
    code_to_exchange_map.insert("QM", Exchange::NYMEX);
    code_to_exchange_map.insert("RB", Exchange::NYMEX);

    // Add more mappings as necessary...

    code_to_exchange_map
}

// Function to input contract code and get the exchange
pub fn get_exchange_by_code(code: &str) -> Option<Exchange> {
    let code_to_exchange_map = get_code_to_exchange_map();
    code_to_exchange_map.get(code).cloned()
}



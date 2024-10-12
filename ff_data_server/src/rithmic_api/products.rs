use ahash::AHashMap;

#[allow(dead_code)]
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
use ff_standard_lib::standardized_types::enums::FuturesExchange;

// Function to map contract code to an exchange using the Exchange enum
pub fn get_code_to_exchange_map() -> HashMap<&'static str, FuturesExchange> {
    let mut code_to_exchange_map = HashMap::new();
    // CBOT contracts
    code_to_exchange_map.insert("XC", FuturesExchange::CBOT);
    code_to_exchange_map.insert("XK", FuturesExchange::CBOT);
    code_to_exchange_map.insert("XW", FuturesExchange::CBOT);
    code_to_exchange_map.insert("YM", FuturesExchange::CBOT);
    code_to_exchange_map.insert("ZB", FuturesExchange::CBOT);
    code_to_exchange_map.insert("ZC", FuturesExchange::CBOT);
    code_to_exchange_map.insert("ZF", FuturesExchange::CBOT);
    code_to_exchange_map.insert("ZL", FuturesExchange::CBOT);
    code_to_exchange_map.insert("ZM", FuturesExchange::CBOT);
    code_to_exchange_map.insert("ZN", FuturesExchange::CBOT);
    code_to_exchange_map.insert("ZO", FuturesExchange::CBOT);
    code_to_exchange_map.insert("ZR", FuturesExchange::CBOT);
    code_to_exchange_map.insert("ZS", FuturesExchange::CBOT);
    code_to_exchange_map.insert("ZT", FuturesExchange::CBOT);
    code_to_exchange_map.insert("ZW", FuturesExchange::CBOT);

    // CME contracts
    code_to_exchange_map.insert("6A", FuturesExchange::CME);
    code_to_exchange_map.insert("6B", FuturesExchange::CME);
    code_to_exchange_map.insert("6C", FuturesExchange::CME);
    code_to_exchange_map.insert("6E", FuturesExchange::CME);
    code_to_exchange_map.insert("6J", FuturesExchange::CME);
    code_to_exchange_map.insert("6M", FuturesExchange::CME);
    code_to_exchange_map.insert("6N", FuturesExchange::CME);
    code_to_exchange_map.insert("6S", FuturesExchange::CME);
    code_to_exchange_map.insert("E7", FuturesExchange::CME);
    code_to_exchange_map.insert("EMD", FuturesExchange::CME);
    code_to_exchange_map.insert("ES", FuturesExchange::CME);
    code_to_exchange_map.insert("GE", FuturesExchange::CME);
    code_to_exchange_map.insert("GF", FuturesExchange::CME);
    code_to_exchange_map.insert("HE", FuturesExchange::CME);
    code_to_exchange_map.insert("J7", FuturesExchange::CME);
    code_to_exchange_map.insert("LE", FuturesExchange::CME);
    code_to_exchange_map.insert("NQ", FuturesExchange::CME);
    code_to_exchange_map.insert("RF", FuturesExchange::CME);
    code_to_exchange_map.insert("SP", FuturesExchange::CME);

    // COMEX contracts
    code_to_exchange_map.insert("GC", FuturesExchange::COMEX);
    code_to_exchange_map.insert("HG", FuturesExchange::COMEX);
    code_to_exchange_map.insert("QI", FuturesExchange::COMEX);
    code_to_exchange_map.insert("QQ", FuturesExchange::COMEX);
    code_to_exchange_map.insert("SI", FuturesExchange::COMEX);

    // NYMEX contracts
    code_to_exchange_map.insert("CL", FuturesExchange::NYMEX);
    code_to_exchange_map.insert("HO", FuturesExchange::NYMEX);
    code_to_exchange_map.insert("NG", FuturesExchange::NYMEX);
    code_to_exchange_map.insert("PA", FuturesExchange::NYMEX);
    code_to_exchange_map.insert("PL", FuturesExchange::NYMEX);
    code_to_exchange_map.insert("QG", FuturesExchange::NYMEX);
    code_to_exchange_map.insert("QM", FuturesExchange::NYMEX);
    code_to_exchange_map.insert("RB", FuturesExchange::NYMEX);

    // Add more mappings as necessary...

    code_to_exchange_map
}

#[allow(dead_code)]
// Function to input contract code and get the exchange
pub fn get_exchange_by_code(code: &str) -> Option<FuturesExchange> {
    let code_to_exchange_map = get_code_to_exchange_map();
    code_to_exchange_map.get(code).cloned()
}



use ahash::AHashMap;
use chrono::{DateTime, Datelike, Utc};
use lazy_static::lazy_static;


lazy_static! {
    static ref ROLLOVER_DAYS: AHashMap<&'static str, u32> = {
        let mut map = AHashMap::new();

        // CME Contracts
        map.insert("MES", 12);
        map.insert("MNQ", 12);
        map.insert("MYM", 12);
        map.insert("M2K", 12);
        map.insert("ES", 12);
        map.insert("NQ", 12);
        map.insert("YM", 12);
        map.insert("RTY", 12);
        map.insert("EMD", 12);
        map.insert("6A", 12);
        map.insert("6B", 12);
        map.insert("6C", 12);
        map.insert("6E", 12);
        map.insert("6J", 12);
        map.insert("6M", 12);
        map.insert("6N", 12);
        map.insert("6S", 12);
        map.insert("E7", 12);
        map.insert("J7", 12);
        map.insert("MJY", 12);

        // CBOT Contracts
        map.insert("YM", 12);
        map.insert("ZB", 12);
        map.insert("ZC", 12);
        map.insert("ZF", 12);
        map.insert("ZL", 12);
        map.insert("ZM", 12);
        map.insert("ZN", 12);
        map.insert("ZO", 12);
        map.insert("ZR", 12);
        map.insert("ZS", 12);
        map.insert("ZT", 12);
        map.insert("ZW", 12);
        map.insert("XC", 12);
        map.insert("XW", 12);
        map.insert("XK", 12);

        // COMEX Contracts
        map.insert("GC", 28);
        map.insert("HG", 28);
        map.insert("QI", 25);
        map.insert("QQ", 25);
        map.insert("SI", 25);

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

pub fn get_front_month(symbol: &str, utc_time: DateTime<Utc>, is_quarterly: bool) -> String {
    let rollover_day = *ROLLOVER_DAYS.get(symbol).unwrap_or(&15); // Default rollover day
    let current_day = utc_time.day();

    let month_code = if is_quarterly {
        // Quarterly contract logic
        match utc_time.month() {
            1 | 2 => 'H',  // March
            3 | 4 | 5 => 'M',  // June
            6 | 7 | 8 => 'U',  // September
            9 | 10 | 11 => 'Z', // December
            12 => 'H', // Roll over to March next year
            _ => return symbol.to_string(),
        }
    } else {
        // Monthly contract logic
        match utc_time.month() {
            1 => 'F',  // January
            2 => 'G',  // February
            3 => 'H',  // March
            4 => 'J',  // April
            5 => 'K',  // May
            6 => 'M',  // June
            7 => 'N',  // July
            8 => 'Q',  // August
            9 => 'U',  // September
            10 => 'V', // October
            11 => 'X', // November
            12 => 'Z', // December
            _ => return symbol.to_string(),
        }
    };

    let (month_code, year) = if current_day >= rollover_day {
        if is_quarterly {
            // Move to the next quarterly contract
            let next_month_code = match month_code {
                'H' => 'M',  // March -> June
                'M' => 'U',  // June -> September
                'U' => 'Z',  // September -> December
                'Z' => 'H',  // December -> March next year
                _ => month_code,
            };
            let next_year = if next_month_code == 'H' && month_code == 'Z' {
                utc_time.year() % 100 + 1
            } else {
                utc_time.year() % 100
            };
            (next_month_code, next_year)
        } else {
            // Move to the next monthly contract
            let next_month_code = match utc_time.month() + 1 {
                1 => 'F',
                2 => 'G',
                3 => 'H',
                4 => 'J',
                5 => 'K',
                6 => 'M',
                7 => 'N',
                8 => 'Q',
                9 => 'U',
                10 => 'V',
                11 => 'X',
                12 => 'Z',
                _ => month_code,
            };
            let next_year = if next_month_code == 'F' {
                utc_time.year() % 100 + 1
            } else {
                utc_time.year() % 100
            };
            (next_month_code, next_year)
        }
    } else {
        (month_code, utc_time.year() % 100) // Stay in current month and year
    };

    format!("{}{}{}", symbol, month_code, year)
}

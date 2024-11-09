use std::collections::HashMap;
use lazy_static::lazy_static;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal_macros::dec;
use crate::standardized_types::accounts::Currency;
use crate::standardized_types::symbol_info::SymbolInfo;

lazy_static! {
    pub static ref OANDA_SYMBOL_INFO: HashMap<String, SymbolInfo> = {
        let mut m = HashMap::new();

        m.insert("AUD-USD".to_string(), SymbolInfo {
            symbol_name: "AUD-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(0.00001),   // USD 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("EUR-USD".to_string(), SymbolInfo {
            symbol_name: "EUR-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(0.00001),   // USD 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("GBP-USD".to_string(), SymbolInfo {
            symbol_name: "GBP-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(0.00001),   // USD 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("NZD-USD".to_string(), SymbolInfo {
            symbol_name: "NZD-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(0.00001),   // USD 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("USD-CAD".to_string(), SymbolInfo {
            symbol_name: "USD-CAD".to_string(),
            pnl_currency: Currency::CAD,
            value_per_tick: dec!(0.00001),   // CAD 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("USD-CHF".to_string(), SymbolInfo {
            symbol_name: "USD-CHF".to_string(),
            pnl_currency: Currency::CHF,
            value_per_tick: dec!(0.00001),   // CHF 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("USD-JPY".to_string(), SymbolInfo {
            symbol_name: "USD-JPY".to_string(),
            pnl_currency: Currency::JPY,
            value_per_tick: dec!(0.01),      // JPY 0.01 per 0.01 tick size for 1 unit
            tick_size: dec!(0.01),
            decimal_accuracy: 2,
        });

                m.insert("EUR-GBP".to_string(), SymbolInfo {
            symbol_name: "EUR-GBP".to_string(),
            pnl_currency: Currency::GBP,
            value_per_tick: dec!(0.00001),   // GBP 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("EUR-JPY".to_string(), SymbolInfo {
            symbol_name: "EUR-JPY".to_string(),
            pnl_currency: Currency::JPY,
            value_per_tick: dec!(0.01),      // JPY 0.01 per 0.01 tick size for 1 unit
            tick_size: dec!(0.01),
            decimal_accuracy: 2,
        });

        m.insert("EUR-CHF".to_string(), SymbolInfo {
            symbol_name: "EUR-CHF".to_string(),
            pnl_currency: Currency::CHF,
            value_per_tick: dec!(0.00001),   // CHF 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("AUD-CAD".to_string(), SymbolInfo {
            symbol_name: "AUD-CAD".to_string(),
            pnl_currency: Currency::CAD,
            value_per_tick: dec!(0.00001),   // CAD 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("AUD-CHF".to_string(), SymbolInfo {
            symbol_name: "AUD-CHF".to_string(),
            pnl_currency: Currency::CHF,
            value_per_tick: dec!(0.00001),   // CHF 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("AUD-JPY".to_string(), SymbolInfo {
            symbol_name: "AUD-JPY".to_string(),
            pnl_currency: Currency::JPY,
            value_per_tick: dec!(0.01),      // JPY 0.01 per 0.01 tick size for 1 unit
            tick_size: dec!(0.01),
            decimal_accuracy: 2,
        });

        m.insert("AUD-NZD".to_string(), SymbolInfo {
            symbol_name: "AUD-NZD".to_string(),
            pnl_currency: Currency::NZD,
            value_per_tick: dec!(0.00001),   // NZD 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("CAD-CHF".to_string(), SymbolInfo {
            symbol_name: "CAD-CHF".to_string(),
            pnl_currency: Currency::CHF,
            value_per_tick: dec!(0.00001),   // CHF 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("CAD-JPY".to_string(), SymbolInfo {
            symbol_name: "CAD-JPY".to_string(),
            pnl_currency: Currency::JPY,
            value_per_tick: dec!(0.01),      // JPY 0.01 per 0.01 tick size for 1 unit
            tick_size: dec!(0.01),
            decimal_accuracy: 2,
        });

        m.insert("CHF-JPY".to_string(), SymbolInfo {
            symbol_name: "CHF-JPY".to_string(),
            pnl_currency: Currency::JPY,
            value_per_tick: dec!(0.01),      // JPY 0.01 per 0.01 tick size for 1 unit
            tick_size: dec!(0.01),
            decimal_accuracy: 2,
        });

        m.insert("EUR-AUD".to_string(), SymbolInfo {
            symbol_name: "EUR-AUD".to_string(),
            pnl_currency: Currency::AUD,
            value_per_tick: dec!(0.00001),   // AUD 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("EUR-CAD".to_string(), SymbolInfo {
            symbol_name: "EUR-CAD".to_string(),
            pnl_currency: Currency::CAD,
            value_per_tick: dec!(0.00001),   // CAD 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("EUR-NOK".to_string(), SymbolInfo {
            symbol_name: "EUR-NOK".to_string(),
            pnl_currency: Currency::NOK,
            value_per_tick: dec!(0.00001),   // NOK 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("EUR-NZD".to_string(), SymbolInfo {
            symbol_name: "EUR-NZD".to_string(),
            pnl_currency: Currency::NZD,
            value_per_tick: dec!(0.00001),   // NZD 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("EUR-SEK".to_string(), SymbolInfo {
            symbol_name: "EUR-SEK".to_string(),
            pnl_currency: Currency::SEK,
            value_per_tick: dec!(0.00001),   // SEK 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("GBP-AUD".to_string(), SymbolInfo {
            symbol_name: "GBP-AUD".to_string(),
            pnl_currency: Currency::AUD,
            value_per_tick: dec!(0.00001),   // AUD 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("GBP-CAD".to_string(), SymbolInfo {
            symbol_name: "GBP-CAD".to_string(),
            pnl_currency: Currency::CAD,
            value_per_tick: dec!(0.00001),   // CAD 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("GBP-CHF".to_string(), SymbolInfo {
            symbol_name: "GBP-CHF".to_string(),
            pnl_currency: Currency::CHF,
            value_per_tick: dec!(0.00001),   // CHF 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

               m.insert("GBP-JPY".to_string(), SymbolInfo {
            symbol_name: "GBP-JPY".to_string(),
            pnl_currency: Currency::JPY,
            value_per_tick: dec!(0.01),     // JPY 0.01 per 0.01 tick size for 1 unit
            tick_size: dec!(0.01),
            decimal_accuracy: 2,
        });

        m.insert("GBP-NZD".to_string(), SymbolInfo {
            symbol_name: "GBP-NZD".to_string(),
            pnl_currency: Currency::NZD,
            value_per_tick: dec!(0.00001),  // NZD 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("NZD-CAD".to_string(), SymbolInfo {
            symbol_name: "NZD-CAD".to_string(),
            pnl_currency: Currency::CAD,
            value_per_tick: dec!(0.00001),  // CAD 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("NZD-CHF".to_string(), SymbolInfo {
            symbol_name: "NZD-CHF".to_string(),
            pnl_currency: Currency::CHF,
            value_per_tick: dec!(0.00001),  // CHF 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("NZD-JPY".to_string(), SymbolInfo {
            symbol_name: "NZD-JPY".to_string(),
            pnl_currency: Currency::JPY,
            value_per_tick: dec!(0.01),     // JPY 0.01 per 0.01 tick size for 1 unit
            tick_size: dec!(0.01),
            decimal_accuracy: 2,
        });

        m.insert("USD-NOK".to_string(), SymbolInfo {
            symbol_name: "USD-NOK".to_string(),
            pnl_currency: Currency::NOK,
            value_per_tick: dec!(0.00001),  // NOK 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("USD-SEK".to_string(), SymbolInfo {
            symbol_name: "USD-SEK".to_string(),
            pnl_currency: Currency::SEK,
            value_per_tick: dec!(0.00001),  // SEK 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("USD-CNH".to_string(), SymbolInfo {
            symbol_name: "USD-CNH".to_string(),
            pnl_currency: Currency::CNH,
            value_per_tick: dec!(0.00001),  // CNH 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("USD-MXN".to_string(), SymbolInfo {
            symbol_name: "USD-MXN".to_string(),
            pnl_currency: Currency::MXN,
            value_per_tick: dec!(0.00001),  // MXN 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("USD-ZAR".to_string(), SymbolInfo {
            symbol_name: "USD-ZAR".to_string(),
            pnl_currency: Currency::ZAR,
            value_per_tick: dec!(0.00001),  // ZAR 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("SGD-JPY".to_string(), SymbolInfo {
            symbol_name: "SGD-JPY".to_string(),
            pnl_currency: Currency::JPY,
            value_per_tick: dec!(0.01),     // JPY 0.01 per 0.01 tick size for 1 unit
            tick_size: dec!(0.01),
            decimal_accuracy: 2,
        });

        m.insert("USD-HKD".to_string(), SymbolInfo {
            symbol_name: "USD-HKD".to_string(),
            pnl_currency: Currency::HKD,
            value_per_tick: dec!(0.00001),  // HKD 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("USD-SGD".to_string(), SymbolInfo {
            symbol_name: "USD-SGD".to_string(),
            pnl_currency: Currency::SGD,
            value_per_tick: dec!(0.00001),  // SGD 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("EUR-CZK".to_string(), SymbolInfo {
            symbol_name: "EUR-CZK".to_string(),
            pnl_currency: Currency::CZK,
            value_per_tick: dec!(0.00001),  // CZK 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("EUR-HUF".to_string(), SymbolInfo {
            symbol_name: "EUR-HUF".to_string(),
            pnl_currency: Currency::HUF,
            value_per_tick: dec!(0.00001),  // HUF 0.00001 per 0.001 tick size for 1 unit
            tick_size: dec!(0.001),
            decimal_accuracy: 3,
        });

        m.insert("EUR-PLN".to_string(), SymbolInfo {
            symbol_name: "EUR-PLN".to_string(),
            pnl_currency: Currency::PLN,
            value_per_tick: dec!(0.00001),  // PLN 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("USD-CZK".to_string(), SymbolInfo {
            symbol_name: "USD-CZK".to_string(),
            pnl_currency: Currency::CZK,
            value_per_tick: dec!(0.00001),  // CZK 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("USD-HUF".to_string(), SymbolInfo {
            symbol_name: "USD-HUF".to_string(),
            pnl_currency: Currency::HUF,
            value_per_tick: dec!(0.00001),  // HUF 0.00001 per 0.001 tick size for 1 unit
            tick_size: dec!(0.001),
            decimal_accuracy: 3,
        });

        m.insert("USD-PLN".to_string(), SymbolInfo {
            symbol_name: "USD-PLN".to_string(),
            pnl_currency: Currency::PLN,
            value_per_tick: dec!(0.00001),  // PLN 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("ZAR-JPY".to_string(), SymbolInfo {
            symbol_name: "ZAR-JPY".to_string(),
            pnl_currency: Currency::JPY,
            value_per_tick: dec!(0.01),     // JPY 0.01 per 0.01 tick size for 1 unit
            tick_size: dec!(0.01),
            decimal_accuracy: 2,
        });

        m.insert("USD-TRY".to_string(), SymbolInfo {
            symbol_name: "USD-TRY".to_string(),
            pnl_currency: Currency::TRY,
            value_per_tick: dec!(0.00001),  // TRY 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("EUR-TRY".to_string(), SymbolInfo {
            symbol_name: "EUR-TRY".to_string(),
            pnl_currency: Currency::TRY,
            value_per_tick: dec!(0.00001),  // TRY 0.00001 per 0.00001 tick size for 1 unit
            tick_size: dec!(0.00001),
            decimal_accuracy: 5,
        });

        m.insert("TRY-JPY".to_string(), SymbolInfo {
            symbol_name: "TRY-JPY".to_string(),
            pnl_currency: Currency::JPY,
            value_per_tick: dec!(0.01),     // JPY 0.01 per 0.01 tick size for 1 unit
            tick_size: dec!(0.01),
            decimal_accuracy: 2,
        });

                m.insert("BTC-USD".to_string(), SymbolInfo {
            symbol_name: "BTC-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(1.0),     // USD 1 per 1.0 tick size for lot
            tick_size: dec!(1.0),
            decimal_accuracy: 2,
        });

        m.insert("BCH-USD".to_string(), SymbolInfo {
            symbol_name: "BCH-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(1.0),     // USD 1 per 1.0 tick size for lot
            tick_size: dec!(1.0),
            decimal_accuracy: 2,
        });

        m.insert("ETH-USD".to_string(), SymbolInfo {
            symbol_name: "ETH-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(1.0),     // USD 1 per 1.0 tick size for lot
            tick_size: dec!(1.0),
            decimal_accuracy: 2,
        });

        m.insert("LTC-USD".to_string(), SymbolInfo {
            symbol_name: "LTC-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(1.0),     // USD 1 per 1.0 tick size for lot
            tick_size: dec!(1.0),
            decimal_accuracy: 3,
        });

        m.insert("AUS200-USD".to_string(), SymbolInfo {
            symbol_name: "AUS200-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(1.0),     // USD 1 per 1.0 tick size for lot
            tick_size: dec!(1.0),
            decimal_accuracy: 2,
        });

        m.insert("CHINA50-USD".to_string(), SymbolInfo {
            symbol_name: "CHINA50-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(1.0),     // USD 1 per 1.0 tick size for lot
            tick_size: dec!(1.0),
            decimal_accuracy: 2,
        });

        m.insert("EU50-USD".to_string(), SymbolInfo {
            symbol_name: "EU50-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(1.0),     // USD 1 per 1.0 tick size for lot
            tick_size: dec!(1.0),
            decimal_accuracy: 2,
        });

        m.insert("GER30-USD".to_string(), SymbolInfo {
            symbol_name: "GER30-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(1.0),     // USD 1 per 1.0 tick size for lot
            tick_size: dec!(1.0),
            decimal_accuracy: 2,
        });

        m.insert("HK50-USD".to_string(), SymbolInfo {
            symbol_name: "HK50-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(1.0),     // USD 1 per 1.0 tick size for lot
            tick_size: dec!(1.0),
            decimal_accuracy: 2,
        });

        m.insert("US100-USD".to_string(), SymbolInfo {
            symbol_name: "US100-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(1.0),     // USD 1 per 1.0 tick size for lot
            tick_size: dec!(1.0),
            decimal_accuracy: 2,
        });

        m.insert("NAS100-USD".to_string(), SymbolInfo {
            symbol_name: "NAS100-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(1.0),     // USD 1 per 1.0 tick size for lot
            tick_size: dec!(1.0),
            decimal_accuracy: 2,
        });

        m.insert("US30-USD".to_string(), SymbolInfo {
            symbol_name: "US30-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(1.0),     // USD 1 per 1.0 tick size for lot
            tick_size: dec!(1.0),
            decimal_accuracy: 2,
        });

        m.insert("US500-USD".to_string(), SymbolInfo {
            symbol_name: "US500-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(1.0),     // USD 1 per 1.0 tick size for lot
            tick_size: dec!(1.0),
            decimal_accuracy: 2,
        });

        m.insert("US2000-USD".to_string(), SymbolInfo {
            symbol_name: "US2000-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(1.0),     // USD 1 per 1.0 tick size for lot
            tick_size: dec!(0.001),
            decimal_accuracy: 3,
        });

        m.insert("FRA40-USD".to_string(), SymbolInfo {
            symbol_name: "FRA40-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(1.0),     // USD 1 per 1.0 tick size for lot
            tick_size: dec!(1.0),
            decimal_accuracy: 2,
        });

        m.insert("UK100-USD".to_string(), SymbolInfo {
            symbol_name: "UK100-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(1.0),     // USD 1 per 1.0 tick size for lot
            tick_size: dec!(1.0),
            decimal_accuracy: 2,
        });

        m.insert("INDIA50-USD".to_string(), SymbolInfo {
            symbol_name: "INDIA50-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(1.0),     // USD 1 per 1.0 tick size for lot
            tick_size: dec!(1.0),
            decimal_accuracy: 2,
        });

        m.insert("JP225-USD".to_string(), SymbolInfo {
            symbol_name: "JP225-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(1.0),     // USD 1 per 1.0 tick size for lot
            tick_size: dec!(1.0),
            decimal_accuracy: 2,
        });

        m.insert("TWIX-USD".to_string(), SymbolInfo {
            symbol_name: "TWIX-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(1.0),     // USD 1 per 1.0 tick size for lot
            tick_size: dec!(1.0),
            decimal_accuracy: 2,
        });

        m.insert("NL25-USD".to_string(), SymbolInfo {
            symbol_name: "NL25-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(1.0),     // USD 1 per 1.0 tick size for lot
            tick_size: dec!(0.001),
            decimal_accuracy: 3,
        });

        m.insert("SING30-USD".to_string(), SymbolInfo {
            symbol_name: "SING30-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(1.0),     // USD 1 per 1.0 tick size for lot
            tick_size: dec!(0.01),
            decimal_accuracy: 2,
        });

        m.insert("CH20-USD".to_string(), SymbolInfo {
            symbol_name: "CH20-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(1.0),     // USD 1 per 1.0 tick size for lot
            tick_size: dec!(1.0),
            decimal_accuracy: 2,
        });

        m.insert("ES35-USD".to_string(), SymbolInfo {
            symbol_name: "ES35-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(1.0),     // USD 1 per 1.0 tick size for lot
            tick_size: dec!(1.0),
            decimal_accuracy: 2,
        });

        m.insert("UKOIL-USD".to_string(), SymbolInfo {
            symbol_name: "UKOIL-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(10.0),    // USD 10 per 0.01 tick size for lot
            tick_size: dec!(0.01),
            decimal_accuracy: 3,
        });

        m.insert("USOIL-USD".to_string(), SymbolInfo {
            symbol_name: "USOIL-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(10.0),    // USD 10 per 0.01 tick size for lot
            tick_size: dec!(0.01),
            decimal_accuracy: 3,
        });

        m.insert("NATGAS-USD".to_string(), SymbolInfo {
            symbol_name: "NATGAS-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(10.0),    // USD 10 per 0.001 tick size for lot
            tick_size: dec!(0.001),
            decimal_accuracy: 3,
        });

        m.insert("COPPER-USD".to_string(), SymbolInfo {
            symbol_name: "COPPER-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(2.5),     // USD 2.5 per 0.0001 tick size for lot
            tick_size: dec!(0.0001),
            decimal_accuracy: 4,
        });

        m.insert("WHEAT-USD".to_string(), SymbolInfo {
            symbol_name: "WHEAT-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(5.0),     // USD 5 per 0.001 tick size for lot
            tick_size: dec!(0.001),
            decimal_accuracy: 3,
        });

        m.insert("CORN-USD".to_string(), SymbolInfo {
            symbol_name: "CORN-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(5.0),     // USD 5 per 0.001 tick size for lot
            tick_size: dec!(0.001),
            decimal_accuracy: 3,
        });

        m.insert("SOYBEANS-USD".to_string(), SymbolInfo {
            symbol_name: "SOYBEANS-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(6.0),     // USD 6 per 0.01 tick size for lot
            tick_size: dec!(0.01),
            decimal_accuracy: 2,
        });

        m.insert("SUGAR-USD".to_string(), SymbolInfo {
            symbol_name: "SUGAR-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(4.0),     // USD 4 per 0.0001 tick size for lot
            tick_size: dec!(0.0001),
            decimal_accuracy: 4,
        });

        m.insert("XAG-USD".to_string(), SymbolInfo {
            symbol_name: "XAG-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(5.0),     // USD 5 per 0.001 tick size for lot
            tick_size: dec!(0.001),
            decimal_accuracy: 3,
        });

        m.insert("XAU-USD".to_string(), SymbolInfo {
            symbol_name: "XAU-USD".to_string(),
            pnl_currency: Currency::USD,
            value_per_tick: dec!(100.0),   // USD 100 per 1.0 tick size for lot
            tick_size: dec!(1.0),
            decimal_accuracy: 2,
        });

        m
    };
}

#[derive(Debug)]
struct MarginTier {
    max_position: f64, // in millions
    margin_percent: Decimal, // as a percentage
}

impl MarginTier {
    fn calculate_margin(&self, position: Decimal, contract_value: Decimal) -> Decimal {
        position * contract_value * (self.margin_percent / dec!(100))
    }
}

lazy_static! {
    static ref MARGIN_TIERS: HashMap<&'static str, Vec<MarginTier>> = {
        let mut m = HashMap::new();

        m.insert("AUD-USD", vec![
            MarginTier { max_position: 2.0, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 50.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("EUR-USD", vec![
            MarginTier { max_position: 2.0, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 50.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("GBP-USD", vec![
            MarginTier { max_position: 2.0, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 50.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

         m.insert("NZD-USD", vec![
            MarginTier { max_position: 2.0, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 50.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("USD-CAD", vec![
            MarginTier { max_position: 2.0, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 50.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("USD-CHF", vec![
            MarginTier { max_position: 2.0, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(4.00) },
            MarginTier { max_position: 50.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("USD-JPY", vec![
            MarginTier { max_position: 2.0, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 50.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("EUR-GBP", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.33) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("EUR-JPY", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.33) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("EUR-CHF", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(4.00) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("AUD-CAD", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.33) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("AUD-CHF", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(4.00) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("AUD-JPY", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.33) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("AUD-NZD", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.33) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("CAD-CHF", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(4.00) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("CAD-JPY", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.33) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("CHF-JPY", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(4.00) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("EUR-AUD", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.33) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("EUR-CAD", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.33) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("EUR-NOK", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.33) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("EUR-NZD", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.33) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("EUR-SEK", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.33) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("GBP-AUD", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.33) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("GBP-CAD", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.33) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("GBP-CHF", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(4.00) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("GBP-JPY", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.33) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("GBP-NZD", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.33) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("NZD-CAD", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.33) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("NZD-CHF", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(4.00) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("NZD-JPY", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.33) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("USD-NOK", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.33) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("USD-SEK", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.33) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("USD-CNH", vec![
            MarginTier { max_position: 0.5, margin_percent: dec!(1.33) },
            MarginTier { max_position: 2.0, margin_percent: dec!(2.00) },
            MarginTier { max_position: 10.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("USD-MXN", vec![
            MarginTier { max_position: 0.5, margin_percent: dec!(1.33) },
            MarginTier { max_position: 2.0, margin_percent: dec!(2.00) },
            MarginTier { max_position: 10.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("USD-ZAR", vec![
            MarginTier { max_position: 0.5, margin_percent: dec!(1.33) },
            MarginTier { max_position: 2.0, margin_percent: dec!(2.00) },
            MarginTier { max_position: 10.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("SGD-JPY", vec![
            MarginTier { max_position: 0.5, margin_percent: dec!(4.00) },
            MarginTier { max_position: 1.0, margin_percent: dec!(6.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(10.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("USD-HKD", vec![
            MarginTier { max_position: 0.5, margin_percent: dec!(4.00) },
            MarginTier { max_position: 1.0, margin_percent: dec!(10.00) },
            MarginTier { max_position: 5.0, margin_percent: dec!(10.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("USD-SGD", vec![
            MarginTier { max_position: 0.5, margin_percent: dec!(4.00) },
            MarginTier { max_position: 1.0, margin_percent: dec!(6.67) },
            MarginTier { max_position: 5.0, margin_percent: dec!(10.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("EUR-CZK", vec![
            MarginTier { max_position: 0.5, margin_percent: dec!(1.33) },
            MarginTier { max_position: 2.0, margin_percent: dec!(2.00) },
            MarginTier { max_position: 10.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("EUR-HUF", vec![
            MarginTier { max_position: 0.5, margin_percent: dec!(1.33) },
            MarginTier { max_position: 2.0, margin_percent: dec!(2.00) },
            MarginTier { max_position: 10.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("EUR-PLN", vec![
            MarginTier { max_position: 0.5, margin_percent: dec!(1.33) },
            MarginTier { max_position: 2.0, margin_percent: dec!(2.00) },
            MarginTier { max_position: 10.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("USD-CZK", vec![
            MarginTier { max_position: 0.5, margin_percent: dec!(1.33) },
            MarginTier { max_position: 2.0, margin_percent: dec!(2.00) },
            MarginTier { max_position: 10.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("USD-HUF", vec![
            MarginTier { max_position: 0.5, margin_percent: dec!(1.33) },
            MarginTier { max_position: 2.0, margin_percent: dec!(2.00) },
            MarginTier { max_position: 10.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("USD-PLN", vec![
            MarginTier { max_position: 0.5, margin_percent: dec!(1.33) },
            MarginTier { max_position: 2.0, margin_percent: dec!(2.00) },
            MarginTier { max_position: 10.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("ZAR-JPY", vec![
            MarginTier { max_position: 0.5, margin_percent: dec!(1.33) },
            MarginTier { max_position: 2.0, margin_percent: dec!(2.00) },
            MarginTier { max_position: 10.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("USD-TRY", vec![
            MarginTier { max_position: 2.0, margin_percent: dec!(10.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(10.00) },
        ]);

        m.insert("EUR-TRY", vec![
            MarginTier { max_position: 2.0, margin_percent: dec!(10.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(10.00) },
        ]);

        m.insert("TRY-JPY", vec![
            MarginTier { max_position: 2.0, margin_percent: dec!(10.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(10.00) },
        ]);

        m.insert("BTC-USD", vec![
            MarginTier { max_position: 0.1, margin_percent: dec!(10.00) },
            MarginTier { max_position: 0.5, margin_percent: dec!(20.00) },
            MarginTier { max_position: 0.5, margin_percent: dec!(30.00) },
        ]);

        m.insert("BCH-USD", vec![
            MarginTier { max_position: 0.05, margin_percent: dec!(20.00) },
            MarginTier { max_position: 0.2, margin_percent: dec!(30.00) },
            MarginTier { max_position: 0.5, margin_percent: dec!(50.00) },
        ]);

        m.insert("ETH-USD", vec![
            MarginTier { max_position: 0.05, margin_percent: dec!(15.00) },
            MarginTier { max_position: 0.2, margin_percent: dec!(25.00) },
            MarginTier { max_position: 0.5, margin_percent: dec!(50.00) },
        ]);

        m.insert("LTC-USD", vec![
            MarginTier { max_position: 0.05, margin_percent: dec!(15.00) },
            MarginTier { max_position: 0.2, margin_percent: dec!(25.00) },
            MarginTier { max_position: 0.5, margin_percent: dec!(50.00) },
        ]);

        m.insert("AUS200-USD", vec![
            MarginTier { max_position: 1.5, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("CHINA50-USD", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 2.0, margin_percent: dec!(1.33) },
            MarginTier { max_position: 10.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("EU50-USD", vec![
            MarginTier { max_position: 1.5, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("GER30-USD", vec![
            MarginTier { max_position: 1.5, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("HK50-USD", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 2.0, margin_percent: dec!(1.33) },
            MarginTier { max_position: 10.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("US100-USD", vec![
            MarginTier { max_position: 1.5, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("NAS100-USD", vec![
            MarginTier { max_position: 1.5, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("US30-USD", vec![
            MarginTier { max_position: 1.5, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("US500-USD", vec![
            MarginTier { max_position: 1.5, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("US2000-USD", vec![
            MarginTier { max_position: 1.5, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("FRA40-USD", vec![
            MarginTier { max_position: 1.5, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("UK100-USD", vec![
            MarginTier { max_position: 1.5, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("INDIA50-USD", vec![
            MarginTier { max_position: 1.0, margin_percent: dec!(0.67) },
            MarginTier { max_position: 2.0, margin_percent: dec!(1.33) },
            MarginTier { max_position: 10.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("JP225-USD", vec![
            MarginTier { max_position: 1.5, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("TWIX-USD", vec![
            MarginTier { max_position: 1.5, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("NL25-USD", vec![
            MarginTier { max_position: 1.5, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("SING30-USD", vec![
            MarginTier { max_position: 1.5, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("CH20-USD", vec![
            MarginTier { max_position: 1.5, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("ES35-USD", vec![
            MarginTier { max_position: 1.5, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        // Commodities
        m.insert("UKOIL-USD", vec![
            MarginTier { max_position: 100_000.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 2_000_000.0, margin_percent: dec!(2.00) },
            MarginTier { max_position: 5_000_000.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("USOIL-USD", vec![
            MarginTier { max_position: 100_000.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 2_000_000.0, margin_percent: dec!(2.00) },
            MarginTier { max_position: 5_000_000.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("NATGAS-USD", vec![
            MarginTier { max_position: 100_000.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 2_000_000.0, margin_percent: dec!(3.50) },
            MarginTier { max_position: 5_000_000.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("COPPER-USD", vec![
            MarginTier { max_position: 2_000_000.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 5_000_000.0, margin_percent: dec!(2.00) },
            MarginTier { max_position: 20_000_000.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("WHEAT-USD", vec![
            MarginTier { max_position: 100_000.0, margin_percent: dec!(4.00) },
            MarginTier { max_position: 500_000.0, margin_percent: dec!(6.00) },
            MarginTier { max_position: 1_500_000.0, margin_percent: dec!(10.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("CORN-USD", vec![
            MarginTier { max_position: 100_000.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: 500_000.0, margin_percent: dec!(8.00) },
            MarginTier { max_position: 1_500_000.0, margin_percent: dec!(10.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("SOYBEANS-USD", vec![
            MarginTier { max_position: 100_000.0, margin_percent: dec!(3.00) },
            MarginTier { max_position: 500_000.0, margin_percent: dec!(7.00) },
            MarginTier { max_position: 1_500_000.0, margin_percent: dec!(10.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("SUGAR-USD", vec![
            MarginTier { max_position: 500_000.0, margin_percent: dec!(4.00) },
            MarginTier { max_position: 700_000.0, margin_percent: dec!(6.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(10.00) },
        ]);

        m.insert("XAG-USD", vec![
            MarginTier { max_position: 2_000_000.0, margin_percent: dec!(2.50) },
            MarginTier { max_position: 5_000_000.0, margin_percent: dec!(4.00) },
            MarginTier { max_position: 20_000_000.0, margin_percent: dec!(10.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m.insert("XAU-USD", vec![
            MarginTier { max_position: 0.1, margin_percent: dec!(0.50) },
            MarginTier { max_position: 5.0, margin_percent: dec!(1.00) },
            MarginTier { max_position: 20.0, margin_percent: dec!(5.00) },
            MarginTier { max_position: f64::INFINITY, margin_percent: dec!(20.00) },
        ]);

        m
    };
}
fn find_margin_tier(position_size: Decimal, margin_tiers: &[MarginTier]) -> &MarginTier {
    for tier in margin_tiers {
        if tier.max_position.is_infinite() {
            return tier;
        }

        // Only try to convert non-infinite values
        let max_pos = Decimal::from_f64(tier.max_position).unwrap();
        if position_size <= max_pos {
            return tier;
        }
    }
    // If we haven't found a tier (shouldn't happen due to infinity tier), return last tier
    margin_tiers.last().unwrap()
}

pub fn calculate_oanda_margin(symbol_name: &str, quantity: Decimal, contract_value: Decimal) -> Option<Decimal> {
    let margin_tiers = MARGIN_TIERS.get(symbol_name)?;

    // Convert quantity to position size for tier comparison
    let position_size = quantity * contract_value;

    // Find the appropriate tier
    let tier = find_margin_tier(position_size, margin_tiers);

    // Calculate margin using the found tier
    Some(tier.calculate_margin(quantity, contract_value))
}
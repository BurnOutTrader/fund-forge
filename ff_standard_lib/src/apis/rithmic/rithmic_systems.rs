use serde::{Deserialize, Serialize};
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use strum_macros::Display;

#[derive(Serialize, Deserialize, Clone, Eq, Serialize_rkyv, Deserialize_rkyv,
    Archive, PartialEq, Debug, Hash, PartialOrd, Ord, Display, Copy)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum RithmicSystem {
    Rithmic04Colo,
    Rithmic01,
    RithmicPaperTrading,
    TopstepTrader,
    SpeedUp,
    TradeFundrr,
    UProfitTrader,
    Apex,
    MESCapital,
    TheTradingPit,
    FundedFuturesNetwork,
    Bulenox,
    PropShopTrader,
    FourPropTrader,
    FastTrackTrading,
    Test
}

impl RithmicSystem {
    /// Converts the enum variant to its corresponding string representation.
    pub fn to_string(&self) -> String {
        match self {
            RithmicSystem::Rithmic01 => "Rithmic 01".to_string(),
            RithmicSystem::Rithmic04Colo => "Rithmic 04 Colo".to_string(),
            RithmicSystem::RithmicPaperTrading => "Rithmic Paper Trading".to_string(),
            RithmicSystem::TopstepTrader => "TopstepTrader".to_string(),
            RithmicSystem::SpeedUp => "SpeedUp".to_string(),
            RithmicSystem::TradeFundrr => "TradeFundrr".to_string(),
            RithmicSystem::UProfitTrader => "UProfitTrader".to_string(),
            RithmicSystem::Apex => "Apex".to_string(),
            RithmicSystem::MESCapital => "MES Capital".to_string(),
            RithmicSystem::TheTradingPit => "The Trading Pit".to_string(),
            RithmicSystem::FundedFuturesNetwork => "Funded Futures Network".to_string(),
            RithmicSystem::Bulenox => "Bulenox".to_string(),
            RithmicSystem::PropShopTrader => "PropShopTrader".to_string(),
            RithmicSystem::FourPropTrader => "4PropTrader".to_string(),
            RithmicSystem::FastTrackTrading => "FastTrackTrading".to_string(),
            RithmicSystem::Test => "Test".to_string()
        }
    }

    /// Converts a string into the corresponding enum variant.
    /// Returns None if the string doesn't match any variant.
    pub fn from_string(s: &str) -> Option<RithmicSystem> {
        match s {
            "Rithmic 01" => Some(RithmicSystem::Rithmic01),
            "Rithmic 04 Colo" => Some(RithmicSystem::Rithmic04Colo),
            "Rithmic Paper Trading" => Some(RithmicSystem::RithmicPaperTrading),
            "TopstepTrader" => Some(RithmicSystem::TopstepTrader),
            "SpeedUp" => Some(RithmicSystem::SpeedUp),
            "TradeFundrr" => Some(RithmicSystem::TradeFundrr),
            "UProfitTrader" => Some(RithmicSystem::UProfitTrader),
            "Apex" => Some(RithmicSystem::Apex),
            "MES Capital" => Some(RithmicSystem::MESCapital),
            "The Trading Pit" => Some(RithmicSystem::TheTradingPit),
            "Funded Futures Network" => Some(RithmicSystem::FundedFuturesNetwork),
            "Bulenox" => Some(RithmicSystem::Bulenox),
            "PropShopTrader" => Some(RithmicSystem::PropShopTrader),
            "4PropTrader" => Some(RithmicSystem::FourPropTrader),
            "FastTrackTrading" => Some(RithmicSystem::FastTrackTrading),
            "Test" => Some(RithmicSystem::Test),
            _ => None,
        }
    }

    pub fn file_string(&self) -> String {
        match self {
            RithmicSystem::Rithmic01 => "rithmic_01.toml".to_string(),
            RithmicSystem::Rithmic04Colo => "rithmic_04_colo.toml".to_string(),
            RithmicSystem::RithmicPaperTrading => "rithmic_paper_trading.toml".to_string(),
            RithmicSystem::TopstepTrader => "topstep_trader.toml".to_string(),
            RithmicSystem::SpeedUp => "speedup.toml".to_string(),
            RithmicSystem::TradeFundrr => "tradefundrr.toml".to_string(),
            RithmicSystem::UProfitTrader => "uprofit_trader.toml".to_string(),
            RithmicSystem::Apex => "apex.toml".to_string(),
            RithmicSystem::MESCapital => "mes_capital.toml".to_string(),
            RithmicSystem::TheTradingPit => "the_trading_pit.toml".to_string(),
            RithmicSystem::FundedFuturesNetwork => "funded_futures_network.toml".to_string(),
            RithmicSystem::Bulenox => "bulenox.toml".to_string(),
            RithmicSystem::PropShopTrader => "propshop_trader.toml".to_string(),
            RithmicSystem::FourPropTrader => "4prop_trader.toml".to_string(),
            RithmicSystem::FastTrackTrading => "fasttrack_trading.toml".to_string(),
            RithmicSystem::Test => "test.toml".to_string(),
        }
    }

    pub fn from_file_string(file_name: &str) -> Option<Self> {
        match file_name {
            "rithmic_01.toml" => Some(RithmicSystem::Rithmic01),
            "rithmic_04_colo.toml" => Some(RithmicSystem::Rithmic04Colo),
            "rithmic_paper_trading.toml" => Some(RithmicSystem::RithmicPaperTrading),
            "topstep_trader.toml" => Some(RithmicSystem::TopstepTrader),
            "speedup.toml" => Some(RithmicSystem::SpeedUp),
            "tradefundrr.toml" => Some(RithmicSystem::TradeFundrr),
            "uprofit_trader.toml" => Some(RithmicSystem::UProfitTrader),
            "apex.toml" => Some(RithmicSystem::Apex),
            "mes_capital.toml" => Some(RithmicSystem::MESCapital),
            "the_trading_pit.toml" => Some(RithmicSystem::TheTradingPit),
            "funded_futures_network.toml" => Some(RithmicSystem::FundedFuturesNetwork),
            "bulenox.toml" => Some(RithmicSystem::Bulenox),
            "propshop_trader.toml" => Some(RithmicSystem::PropShopTrader),
            "4prop_trader.toml" => Some(RithmicSystem::FourPropTrader),
            "fasttrack_trading.toml" => Some(RithmicSystem::FastTrackTrading),
            "test.toml" => Some(RithmicSystem::Test),
            _ => None,
        }
    }
}
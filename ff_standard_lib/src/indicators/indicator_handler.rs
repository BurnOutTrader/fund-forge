use std::sync::Arc;
use ahash::AHashMap;
use futures::future::join_all;
use tokio::sync::RwLock;
use crate::indicators::indicator_trait::{IndicatorEnum, IndicatorName, Indicators};
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::subscriptions::{DataSubscription, Symbol};

/*
pub struct IndicatorSymbolHandler {
    symbol: Symbol,
    indicators: AHashMap<DataSubscription, AHashMap<IndicatorName, IndicatorEnum>>
}

impl IndicatorSymbolHandler {
    pub fn new(symbol: Symbol) -> Self {
        Self {
            symbol,
            indicators: Default::default(),
        }
    }
    
    pub fn add_indicator(&mut self, indicator: IndicatorEnum) {
        let subscription = indicator.subscription().clone();
        let name = indicator.name().clone();
        
        if !self.indicators.contains_key(&subscription) {
            self.indicators.insert(subscription.clone(), AHashMap::new());
        }
        
        self.indicators.get_mut(&subscription).unwrap().insert(name, indicator);
    }
}

pub struct IndicatorHandler {
    indicators: Arc<RwLock<AHashMap<Symbol, IndicatorSymbolHandler>>>,
}

impl IndicatorHandler {
    pub fn new() -> Self {
        Self {
            indicators: Arc::new(RwLock::new(Default::default())),
        }
    }

    pub(crate) async fn add_indicator(&mut self, indicator: IndicatorEnum) {
        let subscription = indicator.subscription().clone();

        let mut indicators = self.indicators.write().await;

        if !indicators.contains_key(&subscription.symbol) {
            indicators.insert(subscription.symbol.clone(), IndicatorSymbolHandler::new(subscription.symbol.clone()));
        }
        
        let subscription = indicator.subscription();
        let mut indicators_for_symbol
        
    }

    pub(crate) async fn remove_indicator(&mut self, indicator_name: &str) {
        let mut indicators = self.indicators.write().await;

        for subscription_indicators in indicators.values_mut() {
            if subscription_indicators.remove(indicator_name).is_some() {
                break;
            }
        }
    }

    pub(crate) async fn update_indicators(&self, base_data: &BaseDataEnum) -> Option<IndicatorValues> {
        let mut tasks = vec![];

        let indicators = self.indicators.read().await;
        if let Some(indicator_map) = indicators.get(&base_data.symbol()) {
            
        }
        
        None
    }

    pub(crate) async fn indicator_index(&self, indicator_name: &str, index: usize) -> Option<IndicatorValues> {
        for indicator in &self.indicators {
            if indicator.name() == indicator_name {
                return indicator.index(index);
            }
        }
        None
    }
}*/
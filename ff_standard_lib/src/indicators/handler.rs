/*use ahash::AHashMap;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::subscriptions::DataSubscription;

pub struct IndicatorHandler {
    indicators: AHashMap<DataSubscription, IndicatorEnum>
}

impl IndicatorHandler {

    pub(crate) fn add_indicator(&mut self, indicator: IndicatorEnum) {
        self.indicators.push(indicator);
    }

    pub(crate) fn remove_indicator(&mut self, indicator_name: &str) {
        self.indicators.retain(|indicator| indicator.name() != indicator_name);
    }

    pub(crate) fn update_indicators(&mut self, base_data: &BaseDataEnum) -> Option<IndicatorResults> {
        let mut results = IndicatorResults::new();
        for indicator in &mut self.indicators {
            if let Some(result) = indicator.update(base_data.clone()) {
                results.extend(result);
            }
        }
        match results.is_empty() {
            true => None,
            false => Some(results),
        }
    }

    pub(crate) fn indicator_index(&self, indicator_name: &str, index: usize) -> Option<IndicatorResults> {
        for indicator in &self.indicators {
            if indicator.name() == indicator_name {
                return indicator.index(index);
            }
        }
        None
    }
}*/
use std::collections::{VecDeque};
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;


#[derive(Clone)]
pub struct RollingWindow {
    last: VecDeque<BaseDataEnum>,
    pub(crate) number: usize,
}

impl RollingWindow {
    pub fn new(number: usize) -> Self {
        RollingWindow {
            last: VecDeque::with_capacity(number),
            number,
        }
    }

    pub fn add(&mut self, data: BaseDataEnum) {
        if self.last.len() == self.number {
            self.last.pop_back(); // Remove the oldest data
        }
        self.last.push_front(data); // Add the latest data at the front
    }

    pub fn last(&self) -> Option<&BaseDataEnum> {
        self.last.front()
    }

    pub fn get(&self, index: usize) -> Option<&BaseDataEnum> {
        self.last.get(index)
    }

    pub fn len(&self) -> usize {
        self.last.len()
    }

    pub fn is_full(&self) -> bool {
        self.last.len() == self.number
    }

    pub fn clear(&mut self) {
        self.last.clear();
    }
}

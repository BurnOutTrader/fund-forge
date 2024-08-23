use std::collections::{VecDeque};
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;


#[derive(Clone)]
pub struct RollingWindow<T> {
    pub(crate) history: VecDeque<T>,
    pub(crate) number: usize,
}

impl<T> RollingWindow<T> {
    pub fn new(number: usize) -> Self {
        RollingWindow {
            history: VecDeque::with_capacity(number),
            number,
        }
    }

    pub fn add(&mut self, data: T) {
        if self.history.len() == self.number {
            self.history.pop_back(); // Remove the oldest data
        }
        self.history.push_front(data); // Add the latest data at the front
    }

    pub fn last(&self) -> Option<&T> {
        self.history.front()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.history.get(index)
    }

    pub fn len(&self) -> usize {
        self.history.len()
    }

    pub fn is_full(&self) -> bool {
        self.history.len() == self.number
    }

    pub fn clear(&mut self) {
        self.history.clear();
    }
    
    pub fn history(&self) -> &VecDeque<T> {
        &self.history
    }
}

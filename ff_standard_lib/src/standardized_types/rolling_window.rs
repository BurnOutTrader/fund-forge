use std::collections::{VecDeque};

/// Keeps track of the last N data points.
/// When the window is full, adding a new data point will remove the oldest data point.
/// The data points are stored in a VecDeque.
/// The data points can be accessed by index. where 0 is the latest data point.
#[derive(Clone, Debug)]
pub struct RollingWindow<T> {
    pub(crate) history: VecDeque<T>,
    pub(crate) number: u64,
}

impl<T> RollingWindow<T> {
    pub fn new(number: u64) -> Self {
        RollingWindow {
            history: VecDeque::with_capacity(number as usize),
            number,
        }
    }

    pub fn add(&mut self, data: T) {
        if self.history.len() as u64 == self.number {
            self.history.pop_back(); // Remove the oldest data
        }
        self.history.push_front(data); // Add the latest data at the front
    }

    pub fn last(&self) -> Option<&T> {
        self.history.front()
    }

    pub fn get(&self, index: u64) -> Option<&T> {
        self.history.get(index as usize)
    }

    pub fn len(&self) -> usize {
        self.history.len()
    }

    pub fn is_full(&self) -> bool {
        self.history.len() as u64 == self.number
    }
    
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    pub fn clear(&mut self) {
        self.history.clear();
    }
    
    pub fn history(&self) -> &VecDeque<T> {
        &self.history
    }
}

use std::collections::{VecDeque};

/// Keeps track of the last N data points.
/// When the window is full, adding a new data point will remove the oldest data point.
/// The data points are stored in a VecDeque.
/// The data points can be accessed by index. where 0 is the latest data point.
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

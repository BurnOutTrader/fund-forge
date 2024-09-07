use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub struct RollingWindow<T> {
    pub(crate) history: Vec<T>,
    pub(crate) number: usize,
}

impl<T: std::clone::Clone> RollingWindow<T> {
    pub fn new(number: u64) -> Self {
        RollingWindow {
            history: Vec::with_capacity(number as usize),
            number: number as usize,
        }
    }

    pub fn clear(&mut self) {
        self.history.clear()
    }

    pub fn is_empty(&self) -> bool   {
        self.history.is_empty()
    }

    pub fn add(&mut self, data: T) {
        // Add the latest data at the front
        self.history.insert(0, data);

        // Remove the oldest data if we exceed the desired number
        if self.history.len() > self.number {
            self.history.pop(); // Remove the last element
        }
    }

    pub fn last(&self) -> Option<&T> {
        self.history.first() // The latest data is at index 0
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

    pub fn history(&self) -> Vec<T> {
        self.history.clone()
    }
}

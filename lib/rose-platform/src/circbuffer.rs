use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct CircBuffer<T> {
    storage: VecDeque<T>,
}

impl<T> CircBuffer<T> {
    pub fn new(max_length: usize) -> Self {
        let storage = VecDeque::with_capacity(max_length);
        Self { storage }
    }

    pub fn add(&mut self, value: T) -> Option<T> {
        if self.storage.len() < self.storage.capacity() {
            self.storage.push_back(value);
            None
        } else {
            let old_value = self.storage.pop_front().unwrap();
            self.storage.push_back(value);
            Some(old_value)
        }
    }

    pub fn len(&self) -> usize {
        self.storage.len()
    }

    pub fn is_empty(&self) -> bool {
        self.storage.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.storage.capacity()
    }

    pub fn iter(&self) -> impl Iterator<Item=&T> {
        self.storage.iter()
    }
}
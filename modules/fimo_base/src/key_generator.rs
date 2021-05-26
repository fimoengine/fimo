use std::fmt::{Debug, Formatter};

/// Generator of unique keys.
#[derive(Default, Ord, PartialOrd, Eq, PartialEq)]
pub struct KeyGenerator<T, Gen: Fn(&T) -> T> {
    next_key: T,
    freed_keys: Vec<T>,
    generator: Gen,
}

impl<T: Debug, Gen: Fn(&T) -> T> Debug for KeyGenerator<T, Gen> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyGenerator")
            .field("next_key", &self.next_key)
            .field("freed_keys", &self.freed_keys)
            .finish()
    }
}

impl<T, Gen: Fn(&T) -> T> KeyGenerator<T, Gen> {
    /// Constructs an instance
    pub fn new(key: T, generator: Gen) -> Self {
        Self {
            next_key: key,
            freed_keys: vec![],
            generator,
        }
    }

    /// Fetches the next key.
    pub fn next_key(&mut self) -> T {
        if self.freed_keys.is_empty() {
            let mut next = (self.generator)(&self.next_key);
            std::mem::swap(&mut self.next_key, &mut next);
            next
        } else {
            self.freed_keys.pop().unwrap()
        }
    }

    /// Frees a key.
    pub fn free_key(&mut self, key: T) {
        self.freed_keys.push(key)
    }
}

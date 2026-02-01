use std::collections::VecDeque;

/// A fixed-capacity ring buffer backed by VecDeque.
/// When full, pushing a new element automatically drops the oldest one.
#[derive(Clone, Debug)]
pub struct RingBuffer<T> {
    buf: VecDeque<T>,
    capacity: usize,
}

impl<T> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buf: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Push an element. If at capacity, the oldest element is dropped.
    pub fn push(&mut self, item: T) {
        if self.buf.len() >= self.capacity {
            self.buf.pop_front();
        }
        self.buf.push_back(item);
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.buf.iter()
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.buf.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ringbuffer_capacity() {
        let mut rb = RingBuffer::new(3);
        rb.push(1);
        rb.push(2);
        rb.push(3);
        assert_eq!(rb.len(), 3);
        rb.push(4);
        assert_eq!(rb.len(), 3);
        let items: Vec<_> = rb.iter().copied().collect();
        assert_eq!(items, vec![2, 3, 4]);
    }

    #[test]
    fn test_ringbuffer_empty() {
        let rb: RingBuffer<i32> = RingBuffer::new(5);
        assert_eq!(rb.len(), 0);
        assert_eq!(rb.iter().count(), 0);
    }
}

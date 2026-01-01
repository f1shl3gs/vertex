mod cache;

use std::cell::UnsafeCell;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

use cache::CachePadded;

#[repr(align(2))]
struct Node<T> {
    next: AtomicPtr<Node<T>>,
    value: Option<T>,
}

impl<T> Node<T> {
    fn new(value: Option<T>) -> *mut Self {
        Box::into_raw(Box::new(Self {
            next: AtomicPtr::new(ptr::null_mut()),
            value,
        }))
    }
}

/// the queue is in an inconsistent state. Popping data should succeed, but some pushers
/// have yet to make enough progress in order allow a pop to succeed. It is recommended
/// that a `pop()` occur "in the near future" in order to see if the sender has made
/// progress or not
#[derive(Debug, PartialEq)]
pub struct InconsistentError;

/// The multi-producer single-consumer structure. This is not cloneable, but
/// it may be safely shared so long as it is guaranteed that there is only
/// one popper at a time, many pushers are allowed. This queue problem is not
/// the best implementation, but it is the simplest one.
///
/// See: <http://www.1024cores.net/home/lock-free-algorithms/queues/non-intrusive-mpsc-node-based-queue>
pub struct Queue<T> {
    head: CachePadded<AtomicPtr<Node<T>>>,
    tail: CachePadded<UnsafeCell<*mut Node<T>>>,
}

unsafe impl<T: Send> Send for Queue<T> {}
unsafe impl<T: Sync> Sync for Queue<T> {}

impl<T> Drop for Queue<T> {
    fn drop(&mut self) {
        unsafe {
            let mut current = *self.tail.get();

            while !current.is_null() {
                let next = (*current).next.load(Ordering::Relaxed);
                drop(Box::from_raw(current));
                current = next;
            }
        }
    }
}

impl<T> Default for Queue<T> {
    /// Creates a new queue that is safe to share among multiple producers
    /// and one consumer
    fn default() -> Self {
        let stub = Node::new(None);

        Self {
            head: CachePadded::new(AtomicPtr::new(stub)),
            tail: CachePadded::new(UnsafeCell::new(stub)),
        }
    }
}

impl<T> Queue<T> {
    /// Pushes a new value into this queue.
    pub fn push(&self, item: T) {
        unsafe {
            let node = Node::new(Some(item));
            let prev = self.head.swap(node, Ordering::AcqRel);

            (*prev).next.store(node, Ordering::Release);
        }
    }

    /// Pops an item from this queue.
    ///
    /// Note that the current implementation means that this function cannot
    /// return `Option<T>`. It is possible for this queue to be in an inconsistent
    /// state where many pushes have succeeded and completely finished, but
    /// pops cannot return `Some(T)`. This inconsistent state happens when a
    /// pusher is preempted at an inopportune moment.
    ///
    /// This inconsistent state means that this queue does indeed have data,
    /// but it does not currently have access to it at this time.
    ///
    /// This function is unsafe because only one thread can call it at a time.
    pub fn pop(&self) -> Result<Option<T>, InconsistentError> {
        unsafe {
            let tail = *self.tail.get();
            let next = (*tail).next.load(Ordering::Acquire);

            if !next.is_null() {
                *self.tail.get() = next;

                let item = (*next).value.take().unwrap();
                drop(Box::from_raw(tail));

                return Ok(Some(item));
            }

            if self.head.load(Ordering::Acquire) == tail {
                // empty
                Ok(None)
            } else {
                Err(InconsistentError)
            }
        }
    }

    /// Attempts to pop from the front, if the item satisfies the given predication
    ///
    /// Returns `None` if the queue is observed to be empty, or the head does not
    /// satisfy the given condition
    pub fn pop_if(&self, pred: impl Fn(&T) -> bool) -> Result<Option<T>, InconsistentError> {
        unsafe {
            let tail = *self.tail.get();
            let next = (*tail).next.load(Ordering::Acquire);

            if !next.is_null() {
                let value = (*next).value.as_ref().unwrap();
                if !pred(value) {
                    return Ok(None);
                }

                *self.tail.get() = next;

                let item = (*next).value.take().unwrap();
                drop(Box::from_raw(tail));

                return Ok(Some(item));
            }

            if self.head.load(Ordering::Acquire) == tail {
                // empty
                Ok(None)
            } else {
                Err(InconsistentError)
            }
        }
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.head.load(Ordering::Acquire) == unsafe { *self.tail.get() }
    }
}

// extra methods
impl<T> Queue<T> {
    pub fn head(&self) -> Option<&T> {
        unsafe {
            let tail = *self.tail.get();
            let next = (*tail).next.load(Ordering::Acquire);

            if next.is_null() {
                return None;
            }

            (*next).value.as_ref()
        }
    }

    pub fn tail(&self) -> Option<&T> {
        let head = self.head.load(Ordering::Relaxed);
        if head.is_null() {
            None
        } else {
            unsafe { (*head).value.as_ref() }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::print_stdout)]

    use std::sync::Arc;

    use tokio::task::JoinSet;

    use super::*;

    #[test]
    fn head_and_tail() {
        let q = Queue::default();
        assert_eq!(q.head(), None);
        assert_eq!(q.tail(), None);

        q.push(1);
        assert_eq!(q.head(), Some(&1));
        assert_eq!(q.tail(), Some(&1));

        q.push(2);
        assert_eq!(q.head(), Some(&1));
        assert_eq!(q.tail(), Some(&2));
    }

    #[test]
    fn push_and_pop() {
        let queue = Queue::<i32>::default();
        assert_eq!(queue.pop(), Ok(None));
        assert!(queue.is_empty());

        queue.push(1);
        assert!(!queue.is_empty());
        queue.push(2);
        assert!(!queue.is_empty());
        queue.push(3);
        assert!(!queue.is_empty());

        unsafe {
            let mut current = *queue.tail.get();

            while !current.is_null() {
                println!("{:?}, {:?}", current, (*current).value);

                let next = (*current).next.load(Ordering::Relaxed);
                current = next;
            }
        }

        assert_eq!(queue.pop(), Ok(Some(1)));
        assert_eq!(queue.pop(), Ok(Some(2)));
        assert_eq!(queue.pop(), Ok(Some(3)));

        assert!(queue.is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 32)]
    async fn mpsc() {
        const SENDERS: usize = 32;
        const MESSAGES: usize = SENDERS * 100000;

        let mut tasks = JoinSet::new();
        let queue = Arc::new(Queue::<usize>::default());
        for _ in 0..SENDERS {
            let q = Arc::clone(&queue);
            tasks.spawn(async move {
                for i in 0..MESSAGES / SENDERS {
                    q.push(i);
                }
            });
        }

        let mut received = 0;
        for _ in 0..MESSAGES {
            loop {
                match queue.pop() {
                    Ok(Some(_)) => {
                        received += 1;
                        break;
                    }
                    Ok(None) => {
                        // threads we spawned, might be scheduled at any time, somehow
                        // producers might stop for a while and receiver won't, so pop
                        // return None.
                    }
                    Err(_) => {}
                }

                tokio::task::yield_now().await;
            }
        }

        tasks.join_all().await;

        assert_eq!(received, MESSAGES)
    }

    #[test]
    fn pop_if() {
        let queue = Queue::default();
        assert_eq!(queue.pop_if(|_value| true).unwrap(), None);

        queue.push(1);
        assert_eq!(queue.pop_if(|value| *value == 2).unwrap(), None);
        assert_eq!(queue.pop_if(|value| *value == 1).unwrap(), Some(1));
        assert_eq!(queue.pop_if(|_value| true).unwrap(), None);
    }
}

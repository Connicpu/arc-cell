extern crate spin;

use spin::Mutex;
use std::sync::{Arc, Weak};
use std::fmt;

/// A Cell for containing a strong reference
pub struct ArcCell<T> {
    inner: Mutex<Arc<T>>,
}

impl<T> ArcCell<T> {
    /// Constructs an ArcCell which initially points to `value`
    pub fn new(value: Arc<T>) -> ArcCell<T> {
        ArcCell {
            inner: Mutex::new(value)
        }
    }

    /// Get the pointer contained in this cell as it exists at this moment
    pub fn get(&self) -> Arc<T> {
        let inner = self.inner.lock();
        (*inner).clone()
    }

    /// Set the pointer for the next observer
    pub fn set(&self, value: Arc<T>) {
        let mut inner = self.inner.lock();
        *inner = value;
    }
}

impl<T: fmt::Debug> fmt::Debug for ArcCell<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        self.get().fmt(fmt)
    }
}

/// A Cell for containing a weak reference
pub struct WeakCell<T> {
    inner: Mutex<Weak<T>>,
}

impl<T> WeakCell<T> {
    /// Construct the Cell with an empty Weak pointer. Upgrading this
    /// value will always return None.
    pub fn empty() -> WeakCell<T> {
        WeakCell {
            inner: Mutex::new(Weak::new())
        }
    }

    /// Get the Weak pointer as it is at this moment
    pub fn get(&self) -> Weak<T> {
        let inner = self.inner.lock();
        (*inner).clone()
    }

    /// Try to upgrade the Weak pointer as it is now into a Strong pointer
    pub fn upgrade(&self) -> Option<Arc<T>> {
        let inner = self.inner.lock();
        inner.upgrade()
    }

    /// Set a Weak pointer you currently have as the pointer in this cell
    pub fn set(&self, value: Weak<T>) {
        let mut inner = self.inner.lock();
        *inner = value;
    }

    /// Downgrade a Strong pointer and store it in the cell
    pub fn store(&self, value: &Arc<T>) {
        self.set(Arc::downgrade(&value));
    }
}

impl<T: fmt::Debug> fmt::Debug for WeakCell<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        self.upgrade().fmt(fmt)
    }
}

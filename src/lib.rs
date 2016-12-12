use std::sync::{Arc, Weak};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::fmt;
use std::mem;
use std::marker::PhantomData;

/// A Cell for containing a strong reference
pub struct ArcCell<T> {
    inner: AtomicUsize,
    _marker: PhantomData<T>,
}

impl<T> ArcCell<T> {
    /// Constructs an ArcCell which initially points to `value`
    pub fn new(value: Arc<T>) -> ArcCell<T> {
        ArcCell {
            inner: AtomicUsize::new(unsafe { mem::transmute(value) }),
            _marker: PhantomData,
        }
    }

    fn take(&self) -> Arc<T> {
        loop {
            let ptr = self.inner.swap(0, Ordering::Acquire);
            if ptr != 0 {
                return unsafe { mem::transmute(ptr) };
            }
        }
    }

    fn put(&self, ptr: Arc<T>) {
        self.inner.store(unsafe { mem::transmute(ptr) }, Ordering::Release);
    }

    /// Get the pointer contained in this cell as it exists at this moment
    pub fn get(&self) -> Arc<T> {
        let ptr = self.take();
        let res = ptr.clone();
        self.put(ptr);
        res
    }

    /// Set the pointer for the next observer
    pub fn set(&self, value: Arc<T>) -> Arc<T> {
        let old = self.take();
        self.put(value);
        old
    }
}

impl<T: fmt::Debug> fmt::Debug for ArcCell<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        self.get().fmt(fmt)
    }
}

/// A Cell for containing a weak reference
pub struct WeakCell<T> {
    inner: AtomicUsize,
    _marker: PhantomData<T>,
}

impl<T> WeakCell<T> {
    /// Constructs the Cell with a value already inside
    pub fn new(value: Weak<T>) -> WeakCell<T> {
        WeakCell {
            inner: AtomicUsize::new(unsafe { mem::transmute(value) }),
            _marker: PhantomData,
        }
    }

    /// Construct the Cell with an empty Weak pointer. Upgrading this
    /// value will always return None.
    pub fn empty() -> WeakCell<T> {
        WeakCell::new(Weak::new())
    }

    fn take(&self) -> Weak<T> {
        loop {
            let ptr = self.inner.swap(0, Ordering::Acquire);
            if ptr != 0 {
                return unsafe { mem::transmute(ptr) };
            }
        }
    }

    fn put(&self, ptr: Weak<T>) {
        self.inner.store(unsafe { mem::transmute(ptr) }, Ordering::Release);
    }

    /// Get the Weak pointer as it is at this moment
    pub fn get(&self) -> Weak<T> {
        let ptr = self.take();
        let res = ptr.clone();
        self.put(ptr);
        res
    }

    /// Try to upgrade the Weak pointer as it is now into a Strong pointer
    pub fn upgrade(&self) -> Option<Arc<T>> {
        self.get().upgrade()
    }

    /// Set a Weak pointer you currently have as the pointer in this cell
    pub fn set(&self, value: Weak<T>) -> Weak<T> {
        let old = self.take();
        self.put(value);
        old
    }

    /// Downgrade a Strong pointer and store it in the cell
    pub fn store(&self, value: &Arc<T>) {
        self.set(Arc::downgrade(&value));
    }

    /// Resets the stored value to be empty
    pub fn reset(&self) -> Weak<T> {
        self.set(Weak::new())
    }
}

impl<T: fmt::Debug> fmt::Debug for WeakCell<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        self.upgrade().fmt(fmt)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use {ArcCell, WeakCell};

    #[test]
    fn arc_cell() {
        let data1 = Arc::new(5);
        let data2 = Arc::new(6);

        let cell = ArcCell::new(data1);
        assert_eq!(*cell.get(), 5);
        cell.set(data2);
        assert_eq!(*cell.get(), 6);
    }

    #[test]
    fn weak_cell() {
        let data = Arc::new(5);

        let cell = WeakCell::empty();
        cell.store(&data);
        assert_eq!(cell.upgrade(), Some(data.clone()));
        drop(data);
        assert_eq!(cell.upgrade(), None);
    }
}

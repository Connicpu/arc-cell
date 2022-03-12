use std::fmt;
use std::marker::PhantomData;
use std::mem;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Weak};

const EMPTY: usize = 0;
// It's impossible for an Arc to have this address because the inside is at least 2 usizes big
const TAKEN: usize = usize::MAX;

/// A Cell for containing a strong reference
pub struct ArcCell<T> {
    inner: AtomicUsize,
    _marker: PhantomData<Arc<T>>,
}

impl<T> ArcCell<T> {
    /// Constructs an ArcCell which initially points to `value`
    pub fn new(value: Option<Arc<T>>) -> ArcCell<T> {
        ArcCell {
            inner: AtomicUsize::new(arc_to_val(value)),
            _marker: PhantomData,
        }
    }

    fn inner_take(&self) -> Option<Arc<T>> {
        unsafe { val_to_arc(take_val(&self.inner)) }
    }

    fn put(&self, ptr: Option<Arc<T>>) {
        put_val(&self.inner, arc_to_val(ptr));
    }

    /// Get the pointer contained in this cell as it exists at this moment
    pub fn get(&self) -> Option<Arc<T>> {
        let ptr = self.inner_take();
        let res = ptr.clone();
        self.put(ptr);
        res
    }

    /// Set the pointer for the next observer
    pub fn set(&self, value: Option<Arc<T>>) -> Option<Arc<T>> {
        let old = self.inner_take();
        self.put(value);
        old
    }

    /// Take the inner value, replacing it with None
    pub fn take(&self) -> Option<Arc<T>> {
        self.set(None)
    }
}

impl<T> Default for ArcCell<T> {
    fn default() -> Self {
        ArcCell::new(None)
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
    _marker: PhantomData<Weak<T>>,
}

impl<T> WeakCell<T> {
    /// Constructs the weak cell with the given value.
    pub fn new(value: Option<Weak<T>>) -> WeakCell<T> {
        WeakCell {
            inner: AtomicUsize::new(unsafe { mem::transmute(value) }),
            _marker: PhantomData,
        }
    }

    fn inner_take(&self) -> Option<Weak<T>> {
        unsafe { val_to_weak(take_val(&self.inner)) }
    }

    fn put(&self, ptr: Option<Weak<T>>) {
        put_val(&self.inner, weak_to_val(ptr));
    }

    /// Get the Weak pointer as it is at this moment
    pub fn get(&self) -> Option<Weak<T>> {
        let ptr = self.inner_take();
        let res = ptr.clone();
        self.put(ptr);
        res
    }

    /// Set a Weak pointer you currently have as the pointer in this cell
    pub fn set(&self, value: Option<Weak<T>>) -> Option<Weak<T>> {
        let old = self.inner_take();
        self.put(value);
        old
    }

    /// Resets the stored value to be empty
    pub fn take(&self) -> Option<Weak<T>> {
        self.set(None)
    }

    /// Try to upgrade the Weak pointer as it is now into a Strong pointer
    pub fn upgrade(&self) -> Option<Arc<T>> {
        self.get().and_then(|weak| weak.upgrade())
    }

    /// Downgrade a Strong pointer and store it in the cell
    pub fn store(&self, value: &Arc<T>) {
        self.set(Some(Arc::downgrade(&value)));
    }
}

impl<T: fmt::Debug> fmt::Debug for WeakCell<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        self.upgrade().fmt(fmt)
    }
}

#[cfg(test)]
mod tests {
    use crate::{ArcCell, WeakCell};
    use std::sync::Arc;

    #[test]
    fn arc_cell() {
        let data1 = Arc::new(5);
        let data2 = Arc::new(6);

        let cell = ArcCell::new(Some(data1));
        assert_eq!(cell.get().as_deref(), Some(&5));
        cell.set(Some(data2));
        assert_eq!(cell.get().as_deref(), Some(&6));
    }

    #[test]
    fn weak_cell() {
        let data = Arc::new(5);

        let cell = WeakCell::new(None);
        cell.store(&data);
        assert_eq!(cell.upgrade(), Some(data.clone()));
        drop(data);
        assert_eq!(cell.upgrade(), None);
    }
}

fn take_val(inner: &AtomicUsize) -> usize {
    let mut ptr = inner.load(Ordering::SeqCst);
    loop {
        // Try to take it ourselves
        match inner.compare_exchange_weak(ptr, TAKEN, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(TAKEN) => ptr = TAKEN, // Someone else wass working on it, retry
            Ok(ptr) => break ptr,
            Err(new_ptr) => ptr = new_ptr, // Someone got to it first, retry
        }
    }
}

fn put_val(inner: &AtomicUsize, val: usize) {
    inner.store(val, Ordering::SeqCst);
}

fn arc_to_val<T>(ptr: Option<Arc<T>>) -> usize {
    match ptr {
        Some(ptr) => Arc::into_raw(ptr) as usize,
        None => EMPTY,
    }
}

unsafe fn val_to_arc<T>(val: usize) -> Option<Arc<T>> {
    match val {
        TAKEN => panic!("Something terrible has happened"),
        EMPTY => None,
        ptr => Some(Arc::from_raw(ptr as *const T)),
    }
}

fn weak_to_val<T>(ptr: Option<Weak<T>>) -> usize {
    match ptr {
        Some(ptr) => Weak::into_raw(ptr) as usize,
        None => EMPTY,
    }
}

unsafe fn val_to_weak<T>(val: usize) -> Option<Weak<T>> {
    match val {
        TAKEN => panic!("Something terrible has happened"),
        EMPTY => None,
        ptr => Some(Weak::from_raw(ptr as *const T)),
    }
}

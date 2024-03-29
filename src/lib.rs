#![doc = include_str!("../README.md")]
#![cfg_attr(feature = "const-new", feature(const_fn_trait_bound))]

use std::{
    fmt::{Debug, Formatter},
    marker::PhantomData,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Weak,
    },
};

/// Atomically swappable/clonable Arc pointer value.
pub type ArcCell<T> = AtomicCell<Arc<T>>;
/// Atomically swappable/clonable Weak Arc pointer value.
pub type WeakCell<T> = AtomicCell<Weak<T>>;

/// Atomically swappable/clonable/optional Arc pointer value.
pub type OptionalArcCell<T> = AtomicCell<Option<Arc<T>>>;
/// Atomically swappable/clonable/optional Weak Arc pointer value.
pub type OptionalWeakCell<T> = AtomicCell<Option<Weak<T>>>;

/// An atomic-based cell designed for holding Arc-style pointers.
pub struct AtomicCell<T: AtomicCellStorable> {
    value: AtomicUsize,
    _marker: PhantomData<T>,
}

impl<T: AtomicCellStorable> AtomicCell<T> {
    /// Create a new AtomicCell with the given initial value.
    pub fn new(value: T) -> Self {
        AtomicCell {
            value: AtomicUsize::new(value.into_value()),
            _marker: PhantomData,
        }
    }

    /// Replace the value in the cell, returning the old value.
    pub fn set(&self, value: T) -> T {
        let old = self.internal_take();
        self.internal_put(value);
        old
    }

    fn internal_take(&self) -> T {
        unsafe {
            let mut current = self.value.load(Ordering::SeqCst);
            T::from_value(loop {
                // Try to take it ourselves
                match self.value.compare_exchange_weak(
                    current,
                    T::TAKEN_VALUE,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    Ok(val) if val != T::TAKEN_VALUE => break val,
                    Ok(_) => current = T::TAKEN_VALUE, // Someone else was working on it, retry
                    Err(new_val) => current = new_val, // Someone got to it first, retry
                }

                // Hint to the CPU we're in a spin loop to reduce power consumption and allow
                // another hyperthread to possibly start.
                core::hint::spin_loop();
            })
        }
    }

    fn internal_put(&self, value: T) {
        let _old = self.value.swap(value.into_value(), Ordering::SeqCst);
        debug_assert_eq!(_old, T::TAKEN_VALUE);
    }
}

impl<T: AtomicCellStorable> Drop for AtomicCell<T> {
    fn drop(&mut self) {
        unsafe {
            let _ = T::from_value(self.value.load(Ordering::SeqCst));
        }
    }
}

impl<T: AtomicCellStorable + Clone> AtomicCell<T> {
    /// Returns a clone of the stored value.
    pub fn get(&self) -> T {
        let value = self.internal_take();
        let copy = value.clone();
        self.internal_put(value);
        copy
    }
}

impl<T: AtomicCellStorable + Clone> Clone for AtomicCell<T> {
    fn clone(&self) -> AtomicCell<T> {
        AtomicCell::new(self.get())
    }
}

impl<T: AtomicCellStorable + Default> AtomicCell<T> {
    /// Take the value stored in the cell, replacing it with the default value.
    pub fn take(&self) -> T {
        // We must construct the new value first in case it panics.
        let new_value = T::default();

        let value = self.internal_take();
        self.internal_put(new_value);

        value
    }
}

impl<T: AtomicCellStorable + Default> Default for AtomicCell<T> {
    fn default() -> Self {
        AtomicCell::new(T::default())
    }
}

#[cfg(feature = "const-new")]
impl<T: AtomicCellStorable + AtomicCellConstInit> AtomicCell<T> {
    pub const fn const_new() -> Self {
        AtomicCell {
            value: AtomicUsize::new(T::DEFAULT_VALUE),
            _marker: PhantomData,
        }
    }
}

impl<T> AtomicCell<Weak<T>> {
    /// Create a new AtomicCell with an empty Weak<T> stored inside.
    pub fn empty() -> Self {
        AtomicCell::new(Weak::new())
    }

    /// Attempt to upgrade the Weak pointer to a strong Arc pointer.
    pub fn upgrade(&self) -> Option<Arc<T>> {
        self.get().upgrade()
    }

    /// Downgrade the Arc value and store it in the cell.
    pub fn store(&self, arc: &Arc<T>) {
        self.set(Arc::downgrade(arc));
    }
}

impl<T> AtomicCell<Option<Weak<T>>> {
    /// Attempt to upgrade the Weak pointer to a strong Arc pointer (if it is not None).
    pub fn upgrade(&self) -> Option<Arc<T>> {
        self.get().and_then(|weak| weak.upgrade())
    }

    /// Downgrade the Arc value and store it in the cell.
    pub fn store(&self, arc: &Arc<T>) {
        self.set(Some(Arc::downgrade(arc)));
    }
}

impl<T: AtomicCellStorable + Clone + Debug> Debug for AtomicCell<T> {
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        fmt.debug_tuple("AtomicCell").field(&self.get()).finish()
    }
}

/// It is up to the implementer to ensure this is safe to implement.
///
/// `from_value` and `into_value` should never panic nor return TAKEN_VALUE.
/// It is also up to the implementer to ensure that if T implements Clone,
/// its implementation of clone() will never panic.
pub unsafe trait AtomicCellStorable {
    /// A sentinel value that a valid instance should never occupy.
    const TAKEN_VALUE: usize;
    /// Convert an instance into a raw value, transferring ownership.
    fn into_value(self) -> usize;
    /// Convert a raw value back into an instance.
    unsafe fn from_value(value: usize) -> Self;
}

unsafe impl<T> AtomicCellStorable for Arc<T> {
    const TAKEN_VALUE: usize = usize::MAX;

    fn into_value(self) -> usize {
        Arc::into_raw(self) as usize
    }

    unsafe fn from_value(value: usize) -> Self {
        Arc::from_raw(value as *const T)
    }
}

unsafe impl<T> AtomicCellStorable for Weak<T> {
    // This must be MAX-1 because MAX is the sentinel value Weak uses for the empty state.
    const TAKEN_VALUE: usize = usize::MAX - 1;

    fn into_value(self) -> usize {
        Weak::into_raw(self) as usize
    }

    unsafe fn from_value(value: usize) -> Self {
        Weak::from_raw(value as *const T)
    }
}

const EMPTY_OPTION: usize = 0;

unsafe impl<T> AtomicCellStorable for Option<Arc<T>> {
    const TAKEN_VALUE: usize = <Arc<T> as AtomicCellStorable>::TAKEN_VALUE;

    fn into_value(self) -> usize {
        match self {
            None => EMPTY_OPTION,
            Some(arc) => Arc::into_raw(arc) as usize,
        }
    }

    unsafe fn from_value(value: usize) -> Self {
        match value {
            EMPTY_OPTION => None,
            value => Some(Arc::from_raw(value as *const T)),
        }
    }
}

unsafe impl<T> AtomicCellStorable for Option<Weak<T>> {
    const TAKEN_VALUE: usize = <Weak<T> as AtomicCellStorable>::TAKEN_VALUE;

    fn into_value(self) -> usize {
        match self {
            None => EMPTY_OPTION,
            Some(arc) => Weak::into_raw(arc) as usize,
        }
    }

    unsafe fn from_value(value: usize) -> Self {
        match value {
            EMPTY_OPTION => None,
            value => Some(Weak::from_raw(value as *const T)),
        }
    }
}

pub unsafe trait AtomicCellConstInit {
    const DEFAULT_VALUE: usize;
}

unsafe impl<T> AtomicCellConstInit for Option<Arc<T>> {
    const DEFAULT_VALUE: usize = EMPTY_OPTION;
}

unsafe impl<T> AtomicCellConstInit for Option<Weak<T>> {
    const DEFAULT_VALUE: usize = EMPTY_OPTION;
}

#[cfg(test)]
mod tests {
    use crate::{ArcCell, WeakCell};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

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

    #[test]
    fn cell_drops() {
        static DROPS: AtomicUsize = AtomicUsize::new(0);
        struct DropCount;
        impl std::ops::Drop for DropCount {
            fn drop(&mut self) {
                DROPS.fetch_add(1, Ordering::SeqCst);
            }
        }
        {
            let _cell = ArcCell::new(Arc::new(DropCount));
        }
        assert_eq!(DROPS.load(Ordering::SeqCst), 1);
    }
}

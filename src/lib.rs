use std::sync::{Arc, RwLock, Weak};

pub struct ArcCell<T> {
    inner: RwLock<Arc<T>>,
}

impl<T> ArcCell<T> {
    /// Constructs an ArcCell which initially points to `value`
    pub fn new(value: Arc<T>) -> ArcCell<T> {
        ArcCell {
            inner: RwLock::new(value)
        }
    }

    /// 
    pub fn get(&self) -> Arc<T> {
        let inner = self.inner.read().expect("It should have been impossible for this to get poisoned");
        (*inner).clone()
    }

    pub fn set(&self, value: Arc<T>) {
        let mut inner = self.inner.write().expect("It should have been impossible for this to get poisoned");
        *inner = value;
    }
}

pub struct WeakCell<T> {
    inner: RwLock<Weak<T>>,
}

impl<T> WeakCell<T> {
    pub fn empty() -> WeakCell<T> {
        WeakCell {
            inner: RwLock::new(Weak::new())
        }
    }

    pub fn get(&self) -> Weak<T> {
        let inner = self.inner.read().expect("It should have been impossible for this to get poisoned");
        (*inner).clone()
    }

    pub fn upgrade(&self) -> Option<Arc<T>> {
        let inner = self.inner.read().expect("It should have been impossible for this to get poisoned");
        inner.upgrade()
    }

    pub fn set(&self, value: Weak<T>) {
        let mut inner = self.inner.write().expect("It should have been impossible for this to get poisoned");
        *inner = value;
    }

    pub fn store(&self, value: &Arc<T>) {
        self.set(Arc::downgrade(&value));
    }
}

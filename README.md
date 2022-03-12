# arc-cell

[Documentation](https://docs.rs/arc-cell/)

A simple library for a concurrent Cell-like object containing an Arc/Weak reference.

```toml
[dependencies]
arc-cell = "0.2"
```

# Usage

### Lightweight swappable Arc member

```rust
use std::sync::Arc;
use arc_cell::ArcCell;

pub struct Thing {
    data: ArcCell<Vec<u8>>,
}

impl Thing {
    pub fn update(&self, data: Arc<Vec<u8>>) {
        self.data.set(Some(data));
    }
}
```

### Self-referencial structure

```rust
use std::sync::Arc;
use arc_cell::WeakCell;

pub struct Thing {
    self_ref: WeakCell<Thing>,
    // ...
}

impl Thing {
    pub fn new() -> Arc<Thing> {
        let thing = Arc::new(Thing {
            self_ref: WeakCell::new(None),
        });
        
        thing.self_ref.store(&thing);
        thing
    }
    
    pub fn clone_ref(&self) -> Arc<Thing> {
        self.self_ref.upgrade().expect("This should be valid if we have a valid self")
    }
}
```

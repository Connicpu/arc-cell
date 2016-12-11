# arc-cell

[Documentation](https://docs.rs/arc-cell/)

A simple library for a concurrent Cell-like object containing an Arc/Weak reference.

```toml
[dependencies]
arc-cell = "0.1"
```

# usage

### self-referencial structure

```rust
use arc_cell::WeakCell;

struct Thing {
    self_ref: WeakCell<Thing>,
    // ...
}

impl Thing {
    pub fn new() -> Arc<Thing> {
        let thing = Arc::new(Thing {
            self_ref: WeakCell::empty(),
        });
        
        thing.self_ref.store(&thing);
        thing
    }
    
    pub fn clone_ref(&self) -> Arc<Thing> {
        self.self_ref.upgrade().expect("This should be valid if we have a valid self")
    }
}
```

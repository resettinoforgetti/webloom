//! Source-grounded TeX82 foundations. See docs/web-map.md for traceability.
//! Algorithms derived from Donald E. Knuth's TeX 3.141592653.

pub mod arithmetic;
pub mod memory;

use arithmetic::Arithmetic;
use memory::{Memory, MemoryConfig, MemoryError};

/// Only the state required by the implemented slice; no global mutable state.
pub struct Engine {
    pub arithmetic: Arithmetic,
    pub memory: Memory,
}

impl Engine {
    pub fn new(config: MemoryConfig) -> Result<Self, MemoryError> {
        Ok(Self {
            arithmetic: Arithmetic::default(),
            memory: Memory::new(config)?,
        })
    }
}

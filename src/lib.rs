//! Source-grounded TeX82 foundations. See docs/web-map.md for traceability.
//! Algorithms derived from Donald E. Knuth's TeX 3.141592653.

pub mod arithmetic;
pub mod io;
pub mod memory;

use arithmetic::Arithmetic;
use io::{HostFixture, IoConfig, IoState};
use memory::{Memory, MemoryConfig, MemoryError};

/// Only the state required by the implemented slice; no global mutable state.
pub struct Engine {
    pub arithmetic: Arithmetic,
    pub memory: Memory,
    pub io: IoState,
}

impl Engine {
    pub fn new(config: MemoryConfig) -> Result<Self, MemoryError> {
        let io_config = IoConfig {
            mem_bot: usize::from(config.mem_bot),
            mem_top: usize::from(config.mem_top),
            ..IoConfig::default()
        };
        let io = IoState::new(io_config, HostFixture::with_pinned_pool(&[]))
            .map_err(|_| MemoryError::InvalidConfiguration)?;
        Ok(Self {
            arithmetic: Arithmetic::default(),
            memory: Memory::new(config)?,
            io,
        })
    }

    pub fn with_io(
        memory_config: MemoryConfig,
        io_config: IoConfig,
        host: HostFixture,
    ) -> Result<Self, MemoryError> {
        let io = IoState::new(io_config, host).map_err(|_| MemoryError::InvalidConfiguration)?;
        Ok(Self {
            arithmetic: Arithmetic::default(),
            memory: Memory::new(memory_config)?,
            io,
        })
    }
}

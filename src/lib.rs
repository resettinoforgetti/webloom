//! Source-grounded TeX82 foundations. See docs/web-map.md for traceability.
//! Algorithms derived from Donald E. Knuth's TeX 3.141592653.

pub mod arithmetic;
pub mod io;
pub mod memory;
pub mod tables;

use arithmetic::Arithmetic;
use io::{HostFixture, IoConfig, IoState, NoopErrorHooks, Termination};
use memory::{Memory, MemoryConfig, MemoryError};
use tables::{Eqtb, HashTable, SaveStack, SemanticNest};

/// Only the state required by the implemented slice; no global mutable state.
pub struct Engine {
    pub arithmetic: Arithmetic,
    pub memory: Memory,
    pub io: IoState,
    pub nest: SemanticNest,
    pub eqtb: Eqtb,
    pub hash: HashTable,
    pub save_stack: SaveStack,
}

impl Engine {
    pub fn new(config: MemoryConfig) -> Result<Self, MemoryError> {
        let io_config = IoConfig {
            mem_bot: usize::from(config.mem_bot),
            mem_top: usize::from(config.mem_top),
            ..IoConfig::default()
        };
        Self::with_io(config, io_config, HostFixture::with_pinned_pool(&[]))
    }

    pub fn with_io(
        memory_config: MemoryConfig,
        io_config: IoConfig,
        host: HostFixture,
    ) -> Result<Self, MemoryError> {
        let mut io =
            IoState::new(io_config, host).map_err(|_| MemoryError::InvalidConfiguration)?;
        let mut memory = Memory::new(memory_config)?;
        let nest = SemanticNest::new(io.config.nest_size, memory.contrib_head());
        let eqtb = Eqtb::new(io.config.hash_size, &mut memory, &mut io);
        let hash = HashTable::new(&eqtb.layout);
        let save_stack = SaveStack::new(io.config.save_size);
        Ok(Self {
            arithmetic: Arithmetic::default(),
            memory,
            io,
            nest,
            eqtb,
            hash,
            save_stack,
        })
    }

    /// §§47, 1336: Load `tex.pool` strings (if not already loaded) and populate `init_prim`.
    pub fn init_prim(&mut self) -> Result<(), Termination> {
        if self.io.str_ptr <= 256 && !self.io.get_strings_started()? {
            return Err(Termination::FinalEnd);
        }
        let mut hooks = NoopErrorHooks;
        self.hash
            .init_prim(&mut self.io, &mut self.eqtb, &self.memory, &mut hooks)
    }
}

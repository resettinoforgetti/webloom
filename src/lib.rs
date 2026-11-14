//! Source-grounded TeX82 foundations. See docs/web-map.md for traceability.
//! Algorithms derived from Donald E. Knuth's TeX 3.141592653.

pub mod arithmetic;
mod expansion;
pub mod io;
pub mod memory;
pub mod scanner;
pub mod tables;

use arithmetic::Arithmetic;
use io::{HostFixture, IoConfig, IoState, Termination};
use memory::{Memory, MemoryConfig, MemoryError};
use scanner::{Scanner, ScannerErrorHooks};
use tables::{Eqtb, HashTable, SaveStack, SemanticNest, Token};

/// Only the state required by the implemented slice; no global mutable state.
pub struct Engine {
    pub arithmetic: Arithmetic,
    pub memory: Memory,
    pub io: IoState,
    pub nest: SemanticNest,
    pub eqtb: Eqtb,
    pub hash: HashTable,
    pub save_stack: SaveStack,
    pub scanner: Scanner,
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
        let scanner = Scanner::new(io.config.param_size, &mut io);
        Ok(Self {
            arithmetic: Arithmetic::default(),
            memory,
            io,
            nest,
            eqtb,
            hash,
            save_stack,
            scanner,
        })
    }

    /// §§47, 1336: Load `tex.pool` strings (if not already loaded) and populate `init_prim`.
    pub fn init_prim(&mut self) -> Result<(), Termination> {
        if self.io.str_ptr <= 256 && !self.io.get_strings_started()? {
            return Err(Termination::FinalEnd);
        }
        let mut hooks = io::NoopErrorHooks;
        self.hash
            .init_prim(&mut self.io, &mut self.eqtb, &self.memory, &mut hooks)
    }

    /// Load a single line of text into `io.buffer` at `start = 1` for scanning.
    pub fn set_line_bytes(&mut self, bytes: &[u8]) {
        self.scanner.set_line_bytes(&mut self.io, &self.eqtb, bytes);
    }

    /// §§341–362: `get_next`.
    pub fn get_next(&mut self) -> Result<(), Termination> {
        self.scanner.get_next(
            &mut self.io,
            &mut self.eqtb,
            &mut self.hash,
            &mut self.memory,
        )
    }

    /// §365: `get_token`.
    pub fn get_token(&mut self) -> Result<(), Termination> {
        self.scanner.get_token(
            &mut self.io,
            &mut self.eqtb,
            &mut self.hash,
            &mut self.memory,
        )
    }

    /// Convenience wrapper around `get_token` returning `Token(cur_tok)`.
    pub fn next_token(&mut self) -> Result<Token, Termination> {
        self.scanner.next_token(
            &mut self.io,
            &mut self.eqtb,
            &mut self.hash,
            &mut self.memory,
        )
    }

    /// Invoke `io.error_with_hooks` with live M4 `ScannerErrorHooks`.
    pub fn error(&mut self) -> Result<(), Termination> {
        let mut hooks = ScannerErrorHooks {
            scanner: &mut self.scanner,
            eqtb: &mut self.eqtb,
            hash: &mut self.hash,
            memory: &mut self.memory,
        };
        self.io.error_with_hooks(&mut hooks)
    }
}

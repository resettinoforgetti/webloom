//! Indexed single-word and variable-size memory, Knuth tex.web §§110–130,162–164.
//! Format preparation/loading are intentionally unavailable.

const GLUE_SPEC_SIZE: u16 = 4; // §150
const LOW_STATIC_WORDS: u16 = 5 * GLUE_SPEC_SIZE; // §162
const INITIAL_FREE_WORDS: u16 = 1000; // §164
const HIGH_STATIC_WORDS: u16 = 14; // §162

/// §125's merge-only request; its result is not a memory address.
pub const COALESCE_REQUEST: i32 = 1 << 30;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeAllocation {
    Allocated(MemPtr),
    Coalesced,
}

/// Validation only: the WEB free lists, not this table, choose addresses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Allocation {
    Free,
    Single,
    Block(u16),
    Interior,
}

/// A halfword address or null. This is not an eqtb index or an arbitrary flag.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct MemPtr(u16);

impl MemPtr {
    pub const NULL: Self = Self(0);

    pub const fn address(self) -> u16 {
        self.0
    }
}

/// A single backing value makes all implemented views coherent (§113).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Word(u32);

impl Word {
    fn info(self) -> u16 {
        // The shift leaves exactly 16 bits; narrowing cannot discard data.
        (self.0 >> 16) as u16
    }
    fn link(self) -> u16 {
        // The mask proves this narrowing is lossless.
        (self.0 & 0xffff) as u16
    }
    fn set_info(&mut self, value: u16) {
        self.0 = (self.0 & 0xffff) | (u32::from(value) << 16);
    }
    fn set_link(&mut self, value: u16) {
        self.0 = (self.0 & 0xffff0000) | u32::from(value);
    }
    #[cfg(test)]
    fn quarters(self) -> [u8; 4] {
        self.0.to_be_bytes()
    }
    fn set_quarters(&mut self, bytes: [u8; 4]) {
        self.0 = u32::from_be_bytes(bytes);
    }
    #[cfg(test)]
    fn integer(self) -> i32 {
        i32::from_be_bytes(self.0.to_be_bytes())
    }
    fn set_integer(&mut self, value: i32) {
        self.0 = u32::from_be_bytes(value.to_be_bytes());
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MemoryConfig {
    pub mem_bot: u16,
    pub mem_top: u16,
    pub mem_max: u16,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            mem_bot: 0,
            mem_top: 30000,
            mem_max: 30000,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryError {
    InvalidConfiguration,
    HostAllocation,
    InvalidNode,
    InvalidSize,
    WrongAllocationKind,
    SizeMismatch {
        expected: u16,
        actual: u16,
    },
    CyclicList,
    /// Terminal TeX capacity failure, not an arithmetic error.
    Exhausted {
        capacity: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryStats {
    pub avail: MemPtr,
    pub lo_mem_max: u16,
    pub hi_mem_min: u16,
    pub mem_end: u16,
    pub dyn_used: usize,
    pub rover: MemPtr,
    pub var_used: usize,
    pub exhausted: bool,
}

pub struct Memory {
    config: MemoryConfig,
    words: Vec<Word>,
    allocations: Vec<Allocation>,
    state: MemoryStats,
}

impl Memory {
    pub fn new(config: MemoryConfig) -> Result<Self, MemoryError> {
        let MemoryConfig {
            mem_bot,
            mem_top,
            mem_max,
        } = config;
        if mem_bot > 1
            || u32::from(mem_top) < u32::from(mem_bot) + 1100
            || mem_top > mem_max
            || mem_max == u16::MAX
        {
            return Err(MemoryError::InvalidConfiguration);
        }
        // Storage offset is mem_bot, not a host-sized semantic pointer.
        let len = usize::from(mem_max - mem_bot) + 1;
        let mut words = Vec::new();
        words
            .try_reserve_exact(len)
            .map_err(|_| MemoryError::HostAllocation)?;
        words.resize(len, Word::default());
        let mut allocations = Vec::new();
        allocations
            .try_reserve_exact(len)
            .map_err(|_| MemoryError::HostAllocation)?;
        allocations.resize(len, Allocation::Free);
        let mut memory = Self {
            config,
            words,
            allocations,
            state: MemoryStats {
                avail: MemPtr::NULL,
                lo_mem_max: mem_bot + LOW_STATIC_WORDS + INITIAL_FREE_WORDS,
                hi_mem_min: mem_top - HIGH_STATIC_WORDS + 1,
                mem_end: mem_top,
                dyn_used: usize::from(HIGH_STATIC_WORDS),
                rover: MemPtr(mem_bot + LOW_STATIC_WORDS),
                var_used: usize::from(LOW_STATIC_WORDS),
                exhausted: false,
            },
        };
        memory.initialize();
        Ok(memory)
    }

    /// §164, including dependencies in §§150,797,820,981.
    fn initialize(&mut self) {
        let b = self.config.mem_bot;
        for offset in [0, 4, 8, 12, 16] {
            self.word_mut(b + offset).set_link(1);
        }
        for (offset, order, stretch) in [
            (4, 1, 65536),
            (8, 2, 65536),
            (12, 1, 65536),
            (16, 1, -65536),
        ] {
            self.word_mut(b + offset).set_quarters([order, 0, 0, 1]);
            self.word_mut(b + offset + 2).set_integer(stretch);
        }
        self.word_mut(b + 12).set_quarters([1, 1, 0, 1]);
        self.word_mut(b + 15).set_integer(65536);
        let rover = b + LOW_STATIC_WORDS;
        self.word_mut(rover).set_info(INITIAL_FREE_WORDS);
        self.word_mut(rover).set_link(u16::MAX);
        self.word_mut(rover + 1).set_info(rover);
        self.word_mut(rover + 1).set_link(rover);
        let top = self.config.mem_top;
        self.word_mut(top - 9).set_link(256); // end_span, §797
        self.word_mut(top - 7).set_info(256); // type(active)=1, subtype=0, §820
        self.word_mut(top - 6).set_info(u16::MAX); // line_number(active)
        self.word_mut(top).set_info(511); // type=1, subtype=255, §981
        self.word_mut(top).set_link(top);
    }

    // These helpers are used only with validated dynamic nodes or bounded
    // initialization/frontier indices. Raw words never escape this module.
    fn offset(&self, address: u16) -> usize {
        usize::from(address - self.config.mem_bot)
    }
    fn word(&self, address: u16) -> Word {
        self.words[self.offset(address)]
    }
    fn word_mut(&mut self, address: u16) -> &mut Word {
        let i = self.offset(address);
        &mut self.words[i]
    }
    fn running(&self) -> Result<(), MemoryError> {
        if self.state.exhausted {
            Err(MemoryError::Exhausted {
                capacity: self.words.len(),
            })
        } else {
            Ok(())
        }
    }
    fn validate(&self, p: MemPtr) -> Result<(), MemoryError> {
        if p == MemPtr::NULL
            || p.0 < self.config.mem_bot
            || p.0 > self.config.mem_max
            || !matches!(
                self.allocations[self.offset(p.0)],
                Allocation::Single | Allocation::Block(_)
            )
        {
            Err(MemoryError::InvalidNode)
        } else {
            Ok(())
        }
    }

    fn validate_single(&self, p: MemPtr) -> Result<(), MemoryError> {
        self.validate(p)?;
        if self.allocations[self.offset(p.0)] == Allocation::Single {
            Ok(())
        } else {
            Err(MemoryError::WrongAllocationKind)
        }
    }

    fn left(&self, p: u16) -> u16 {
        self.word(p + 1).info()
    }

    fn right(&self, p: u16) -> u16 {
        self.word(p + 1).link()
    }

    fn unlink_free_block(&mut self, p: u16) {
        let (left, right) = (self.left(p), self.right(p));
        self.word_mut(right + 1).set_info(left);
        self.word_mut(left + 1).set_link(right);
    }

    fn insert_before_rover(&mut self, p: u16, size: u16) {
        let rover = self.state.rover.0;
        let left = self.left(rover);
        self.word_mut(p).set_info(size);
        self.word_mut(p).set_link(u16::MAX);
        self.word_mut(p + 1).set_info(left);
        self.word_mut(p + 1).set_link(rover);
        self.word_mut(left + 1).set_link(p);
        self.word_mut(rover + 1).set_info(p);
    }

    fn finish_block(&mut self, p: u16, size: u16) -> NodeAllocation {
        self.word_mut(p).set_link(0);
        let start = self.offset(p);
        self.allocations[start..start + usize::from(size)].fill(Allocation::Interior);
        self.allocations[start] = Allocation::Block(size);
        self.state.var_used += usize::from(size);
        NodeAllocation::Allocated(MemPtr(p))
    }

    /// §§125–129. Sizes below two are invalid. Other positive sizes have WEB's
    /// capacity semantics, including requests larger than the address space.
    /// `COALESCE_REQUEST` merges free blocks without allocating or growing.
    pub fn get_node(&mut self, size: i32) -> Result<NodeAllocation, MemoryError> {
        self.running()?;
        if size < 2 {
            return Err(MemoryError::InvalidSize);
        }
        loop {
            let mut p = self.state.rover.0;
            loop {
                let mut q = p + self.word(p).info();
                while self.word(q).link() == u16::MAX {
                    if q == self.state.rover.0 {
                        self.state.rover = MemPtr(self.right(q));
                    }
                    self.unlink_free_block(q);
                    q += self.word(q).info();
                }
                let r = i32::from(q) - size;
                if r > i32::from(p) + 1 {
                    // Here 0 < r < q <= lo_mem_max, so both narrowings fit.
                    let r = r as u16;
                    self.word_mut(p).set_info(r - p);
                    self.state.rover = MemPtr(p);
                    return Ok(self.finish_block(r, size as u16));
                }
                if r == i32::from(p) && self.right(p) != p {
                    self.state.rover = MemPtr(self.right(p));
                    self.unlink_free_block(p);
                    return Ok(self.finish_block(p, size as u16));
                }
                self.word_mut(p).set_info(q - p);
                p = self.right(p);
                if p == self.state.rover.0 {
                    break;
                }
            }
            if size == COALESCE_REQUEST {
                return Ok(NodeAllocation::Coalesced);
            }
            let low = u32::from(self.state.lo_mem_max);
            let high = u32::from(self.state.hi_mem_min);
            let limit = u32::from(self.config.mem_bot) + u32::from(u16::MAX);
            if low + 2 < high && low + 2 <= limit {
                // §126 divides the nonnegative difference, not a midpoint sum.
                let top = if high - low >= 1998 {
                    low + 1000
                } else {
                    low + 1 + (high - low) / 2
                }
                .min(limit) as u16;
                self.insert_before_rover(low as u16, top - low as u16);
                self.state.lo_mem_max = top;
                self.word_mut(top).set_link(0);
                self.word_mut(top).set_info(0);
                self.state.rover = MemPtr(low as u16);
            } else {
                self.state.exhausted = true;
                return Err(MemoryError::Exhausted {
                    capacity: self.words.len(),
                });
            }
        }
    }

    /// §130. Reinsert immediately before rover; coalescing is deferred until
    /// allocation. Invalid reclamation is rejected before any state changes.
    pub fn free_node(&mut self, p: MemPtr, size: u16) -> Result<(), MemoryError> {
        self.running()?;
        if size < 2 {
            return Err(MemoryError::InvalidSize);
        }
        self.validate(p)?;
        let start = self.offset(p.0);
        let Allocation::Block(expected) = self.allocations[start] else {
            return Err(MemoryError::WrongAllocationKind);
        };
        if expected != size {
            return Err(MemoryError::SizeMismatch {
                expected,
                actual: size,
            });
        }
        self.insert_before_rover(p.0, size);
        self.allocations[start..start + usize::from(size)].fill(Allocation::Free);
        self.state.var_used -= usize::from(size);
        Ok(())
    }

    pub fn stats(&self) -> MemoryStats {
        self.state
    }

    /// §120. Exhaustion is terminal, as in WEB's overflow stop.
    pub fn get_avail(&mut self) -> Result<MemPtr, MemoryError> {
        self.running()?;
        let p = if self.state.avail != MemPtr::NULL {
            let p = self.state.avail;
            self.state.avail = MemPtr(self.word(p.0).link());
            p
        } else if self.state.mem_end < self.config.mem_max {
            self.state.mem_end += 1;
            MemPtr(self.state.mem_end)
        } else {
            self.state.hi_mem_min -= 1;
            if self.state.hi_mem_min <= self.state.lo_mem_max {
                self.state.exhausted = true;
                return Err(MemoryError::Exhausted {
                    capacity: self.words.len(),
                });
            }
            MemPtr(self.state.hi_mem_min)
        };
        self.word_mut(p.0).set_link(0);
        let i = self.offset(p.0);
        self.allocations[i] = Allocation::Single;
        self.state.dyn_used += 1;
        Ok(p)
    }

    pub fn info(&self, p: MemPtr) -> Result<u16, MemoryError> {
        self.validate(p)?;
        Ok(self.word(p.0).info())
    }
    pub fn set_info(&mut self, p: MemPtr, value: u16) -> Result<(), MemoryError> {
        self.running()?;
        self.validate(p)?;
        self.word_mut(p.0).set_info(value);
        Ok(())
    }
    pub fn link(&self, p: MemPtr) -> Result<MemPtr, MemoryError> {
        self.validate(p)?;
        Ok(MemPtr(self.word(p.0).link()))
    }
    pub fn set_link(&mut self, p: MemPtr, next: MemPtr) -> Result<(), MemoryError> {
        self.running()?;
        self.validate(p)?;
        if next != MemPtr::NULL {
            self.validate(next)?;
        }
        self.word_mut(p.0).set_link(next.0);
        Ok(())
    }

    /// §123. Validation is atomic on invalid caller state, then the source's
    /// tail-splice algorithm preserves the order of the reclaimed list.
    pub fn flush_list(&mut self, p: MemPtr) -> Result<(), MemoryError> {
        self.running()?;
        if p == MemPtr::NULL {
            return Ok(());
        }
        let mut r = p;
        let mut count = 0;
        let tail = loop {
            self.validate_single(r)?;
            count += 1;
            if count > self.state.dyn_used - usize::from(HIGH_STATIC_WORDS) {
                return Err(MemoryError::CyclicList);
            }
            let next = MemPtr(self.word(r.0).link());
            if next == MemPtr::NULL {
                break r;
            }
            r = next;
        };
        r = p;
        while r != MemPtr::NULL {
            let i = self.offset(r.0);
            self.allocations[i] = Allocation::Free;
            r = MemPtr(self.word(r.0).link());
        }
        self.state.dyn_used -= count;
        let avail = self.state.avail;
        self.word_mut(tail.0).set_link(avail.0);
        self.state.avail = p;
        Ok(())
    }
}

#[cfg(test)]
#[path = "memory_model_tests.rs"]
mod model_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_views_have_one_backing_value() {
        let mut w = Word::default();
        w.set_quarters([0x12, 0x34, 0xab, 0xcd]);
        assert_eq!(
            (w.info(), w.link(), w.integer()),
            (0x1234, 0xabcd, 0x1234abcd)
        );
        w.set_info(0xfedc);
        assert_eq!(w.quarters(), [0xfe, 0xdc, 0xab, 0xcd]);
        w.set_link(0x7654);
        assert_eq!(w.quarters(), [0xfe, 0xdc, 0x76, 0x54]);
        w.set_integer(-65536);
        assert_eq!((w.info(), w.link()), (65535, 0));
        assert_eq!(w.integer(), -65536);
    }

    #[test]
    fn initialization_matches_static_layout() {
        for b in [0, 1] {
            let m = Memory::new(MemoryConfig {
                mem_bot: b,
                mem_top: b + 1100,
                mem_max: b + 1100,
            })
            .unwrap();
            for offset in [0, 4, 8, 12, 16] {
                assert_eq!(m.word(b + offset).link(), 1);
            }
            assert_eq!(m.word(b + 4).quarters(), [1, 0, 0, 1]);
            assert_eq!(m.word(b + 8).quarters(), [2, 0, 0, 1]);
            assert_eq!(m.word(b + 12).quarters(), [1, 1, 0, 1]);
            assert_eq!(m.word(b + 15).integer(), 65536);
            assert_eq!(m.word(b + 18).integer(), -65536);
            assert_eq!(
                (m.word(b + 20).info(), m.word(b + 20).link()),
                (1000, 65535)
            );
            assert_eq!(
                (m.word(b + 21).info(), m.word(b + 21).link()),
                (b + 20, b + 20)
            );
            assert_eq!(m.word(b + 1020), Word::default());
            assert_eq!(m.word(b + 1100 - 9).link(), 256);
            assert_eq!(m.word(b + 1100 - 7).info(), 256);
            assert_eq!(m.word(b + 1100 - 6).info(), 65535);
            assert_eq!(m.word(b + 1100).quarters()[..2], [1, 255]);
            assert_eq!(m.word(b + 1100).link(), b + 1100);
            // Null, the low region, unused space, static heads, and the
            // empty_flag are not dynamic nodes, even when in the word array.
            for raw in [0, b, b + 20, b + 1020, b + 1086, b + 1087, b + 1100, 65535] {
                assert_eq!(m.info(MemPtr(raw)), Err(MemoryError::InvalidNode));
            }
        }
    }
}

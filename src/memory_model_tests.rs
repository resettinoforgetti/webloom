//! Independent interval oracle: no packed words, sentinels, or linked headers.
use super::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::ops::Range;

struct Model {
    config: MemoryConfig,
    free: Vec<Range<u16>>,
    blocks: BTreeMap<u16, u16>,
    singles: BTreeSet<u16>,
    avail: VecDeque<u16>,
    rover: u16,
    low: u16,
    high: u16,
    end: u16,
    exhausted: bool,
}

impl Model {
    fn new(config: MemoryConfig) -> Self {
        let start = config.mem_bot + 20;
        Self {
            config,
            free: std::iter::once(start..start + 1000).collect(),
            blocks: BTreeMap::new(),
            singles: BTreeSet::new(),
            avail: VecDeque::new(),
            rover: start,
            low: start + 1000,
            high: config.mem_top - 13,
            end: config.mem_top,
            exhausted: false,
        }
    }

    fn capacity_error(&mut self) -> MemoryError {
        self.exhausted = true;
        MemoryError::Exhausted {
            capacity: usize::from(self.config.mem_max - self.config.mem_bot) + 1,
        }
    }

    fn index(&self, start: u16) -> usize {
        self.free
            .iter()
            .position(|range| range.start == start)
            .unwrap()
    }

    fn next(&self, start: u16) -> u16 {
        self.free[(self.index(start) + 1) % self.free.len()].start
    }

    fn insert(&mut self, range: Range<u16>) {
        self.free.insert(self.index(self.rover), range);
    }

    fn allocate(&mut self, size: i32) -> Result<NodeAllocation, MemoryError> {
        loop {
            let mut start = self.rover;
            loop {
                // Join adjacent free intervals regardless of their ring positions.
                loop {
                    let end = self.free[self.index(start)].end;
                    let Some(next) = self.free.iter().position(|range| range.start == end) else {
                        break;
                    };
                    if self.rover == end {
                        self.rover = self.next(end);
                    }
                    let merged = self.free.remove(next);
                    let current = self.index(start);
                    self.free[current].end = merged.end;
                }
                let index = self.index(start);
                let range = self.free[index].clone();
                let length = i32::from(range.end - range.start);
                let remainder = length - size;
                if remainder >= 2 || (remainder == 0 && self.free.len() > 1) {
                    let address = range.end - size as u16;
                    if remainder == 0 {
                        self.rover = self.next(start);
                        self.free.remove(index);
                    } else {
                        self.free[index].end = address;
                        self.rover = start;
                    }
                    assert!(self.blocks.insert(address, size as u16).is_none());
                    return Ok(NodeAllocation::Allocated(MemPtr(address)));
                }
                start = self.next(start);
                if start == self.rover {
                    break;
                }
            }
            if size == COALESCE_REQUEST {
                return Ok(NodeAllocation::Coalesced);
            }
            if self.high - self.low <= 2 {
                return Err(self.capacity_error());
            }
            let growth = (1 + (self.high - self.low) / 2).min(1000);
            let old = self.low;
            self.low += growth;
            self.insert(old..self.low);
            self.rover = old;
        }
    }

    fn single(&mut self) -> Result<MemPtr, MemoryError> {
        let address = if let Some(p) = self.avail.pop_front() {
            p
        } else if self.end < self.config.mem_max {
            self.end += 1;
            self.end
        } else {
            self.high -= 1;
            if self.high <= self.low {
                return Err(self.capacity_error());
            }
            self.high
        };
        assert!(self.singles.insert(address));
        Ok(MemPtr(address))
    }

    fn check(&self, memory: &Memory) {
        assert_eq!(
            memory.stats(),
            MemoryStats {
                avail: MemPtr(self.avail.front().copied().unwrap_or(0)),
                lo_mem_max: self.low,
                hi_mem_min: self.high,
                mem_end: self.end,
                dyn_used: 14 + self.singles.len(),
                rover: MemPtr(self.rover),
                var_used: 20
                    + self
                        .blocks
                        .values()
                        .map(|&size| usize::from(size))
                        .sum::<usize>(),
                exhausted: self.exhausted,
            }
        );
        let mut p = self.rover;
        let first = self.index(self.rover);
        for n in 0..self.free.len() {
            let interval = &self.free[(first + n) % self.free.len()];
            assert_eq!(p, interval.start);
            assert_eq!(memory.word(p).info(), interval.end - interval.start);
            assert_eq!(memory.word(p).link(), u16::MAX);
            let next = memory.right(p);
            assert_eq!(memory.left(next), p);
            p = next;
        }
        assert_eq!(p, self.rover);
        let mut expected = vec![Allocation::Free; memory.words.len()];
        for (&start, &size) in &self.blocks {
            let offset = memory.offset(start);
            expected[offset..offset + usize::from(size)].fill(Allocation::Interior);
            expected[offset] = Allocation::Block(size);
        }
        for &p in &self.singles {
            assert_eq!(expected[memory.offset(p)], Allocation::Free);
            expected[memory.offset(p)] = Allocation::Single;
        }
        assert_eq!(memory.allocations, expected);
        // Every low dynamic word belongs to exactly one live or free interval.
        let mut intervals: Vec<_> = self.free.clone();
        intervals.extend(self.blocks.iter().map(|(&p, &size)| p..p + size));
        intervals.sort_by_key(|r| r.start);
        let mut end = self.config.mem_bot + 20;
        for interval in intervals {
            assert_eq!(interval.start, end);
            end = interval.end;
        }
        assert_eq!(end, self.low);
    }
}

#[test]
fn mixed_sequences_match_an_independent_interval_and_queue_model() {
    for bot in [0, 1] {
        for top in [1100, 3000, 65530] {
            for initial_seed in 1_u64..=8 {
                let config = MemoryConfig {
                    mem_bot: bot,
                    mem_top: bot + top,
                    mem_max: bot + top + 3,
                };
                let mut memory = Memory::new(config).unwrap();
                let mut model = Model::new(config);
                let mut seed = initial_seed;
                for _ in 0..1200 {
                    seed = seed.wrapping_mul(6364136223846793005_u64).wrapping_add(1);
                    match seed % 10 {
                        0..=3 => {
                            let size = 2 + ((seed >> 32) % 100) as i32;
                            assert_eq!(memory.get_node(size), model.allocate(size));
                        }
                        4 | 5 if !model.blocks.is_empty() => {
                            let index = (seed >> 32) as usize % model.blocks.len();
                            let (&p, &size) = model.blocks.iter().nth(index).unwrap();
                            memory.free_node(MemPtr(p), size).unwrap();
                            model.blocks.remove(&p);
                            model.insert(p..p + size);
                        }
                        6 => assert_eq!(memory.get_avail(), model.single()),
                        7 if !model.singles.is_empty() => {
                            let nodes: Vec<_> = model.singles.iter().copied().take(3).collect();
                            for pair in nodes.windows(2) {
                                memory.set_link(MemPtr(pair[0]), MemPtr(pair[1])).unwrap();
                            }
                            memory
                                .set_link(MemPtr(*nodes.last().unwrap()), MemPtr::NULL)
                                .unwrap();
                            memory.flush_list(MemPtr(nodes[0])).unwrap();
                            for &p in nodes.iter().rev() {
                                model.singles.remove(&p);
                                model.avail.push_front(p);
                            }
                        }
                        _ => assert_eq!(
                            memory.get_node(COALESCE_REQUEST),
                            model.allocate(COALESCE_REQUEST)
                        ),
                    }
                    model.check(&memory);
                    if model.exhausted {
                        break;
                    }
                }
            }
        }
    }
}

#[test]
fn interior_and_wrong_size_reclamation_are_atomic_including_raw_memory() {
    let mut memory = Memory::new(MemoryConfig::default()).unwrap();
    let NodeAllocation::Allocated(p) = memory.get_node(6).unwrap() else {
        unreachable!()
    };
    let before = (
        memory.words.clone(),
        memory.allocations.clone(),
        memory.stats(),
    );
    assert_eq!(
        memory.free_node(MemPtr(p.0 + 1), 5),
        Err(MemoryError::InvalidNode)
    );
    assert_eq!(
        memory.free_node(p, 5),
        Err(MemoryError::SizeMismatch {
            expected: 6,
            actual: 5
        })
    );
    assert_eq!(memory.info(MemPtr(p.0 + 1)), Err(MemoryError::InvalidNode));
    assert_eq!(memory.flush_list(p), Err(MemoryError::WrongAllocationKind));
    assert_eq!(memory.get_node(-2), Err(MemoryError::InvalidSize));
    assert_eq!(
        (
            memory.words.clone(),
            memory.allocations.clone(),
            memory.stats()
        ),
        before
    );
}

#[test]
fn coalescing_can_remove_rover_and_preserves_the_remaining_ring() {
    let mut memory = Memory::new(MemoryConfig {
        mem_bot: 0,
        mem_top: 1100,
        mem_max: 1100,
    })
    .unwrap();
    let NodeAllocation::Allocated(a) = memory.get_node(998).unwrap() else {
        unreachable!()
    };
    memory.get_node(2).unwrap();
    memory.free_node(a, 998).unwrap();
    assert_eq!(memory.state.rover, MemPtr(1020));
    assert_eq!(
        memory.get_node(COALESCE_REQUEST),
        Ok(NodeAllocation::Coalesced)
    );
    assert_eq!(memory.state.rover, MemPtr(20));
    assert_eq!(memory.word(20).info(), 1032);
    assert_eq!((memory.left(20), memory.right(20)), (20, 20));
    assert_eq!(memory.state.var_used, 22);
    assert_eq!(memory.state.lo_mem_max, 1054);
}

#[test]
fn allocating_a_block_clears_only_its_head_link() {
    let mut memory = Memory::new(MemoryConfig::default()).unwrap();
    let NodeAllocation::Allocated(p) = memory.get_node(6).unwrap() else {
        unreachable!()
    };
    memory.word_mut(p.0 + 2).set_integer(-123456);
    memory.free_node(p, 6).unwrap();
    assert_eq!(memory.get_node(6), Ok(NodeAllocation::Allocated(p)));
    assert_eq!(memory.word(p.0).link(), 0);
    assert_eq!(memory.word(p.0 + 2).integer(), -123456);
}

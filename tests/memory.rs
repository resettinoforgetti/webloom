use std::collections::VecDeque;
use webloom::memory::{MemPtr, Memory, MemoryConfig, MemoryError};

fn small(extra: u16) -> Memory {
    Memory::new(MemoryConfig {
        mem_bot: 0,
        mem_top: 1100,
        mem_max: 1100 + extra,
    })
    .unwrap()
}

#[test]
fn allocation_order_reclamation_and_retained_info() {
    let mut m = small(2);
    let a = m.get_avail().unwrap();
    let b = m.get_avail().unwrap();
    let c = m.get_avail().unwrap();
    assert_eq!([a.address(), b.address(), c.address()], [1101, 1102, 1086]);
    m.set_info(b, 65535).unwrap();
    m.flush_list(c).unwrap();
    m.set_link(a, b).unwrap();
    m.flush_list(a).unwrap();
    assert_eq!(m.stats().dyn_used, 14);
    for expected in [a, b, c] {
        let p = m.get_avail().unwrap();
        assert_eq!(p, expected);
        assert_eq!(m.link(p).unwrap(), MemPtr::NULL);
    }
    assert_eq!(m.info(b).unwrap(), 65535);
    assert_eq!(m.get_avail().unwrap().address(), 1085);
}

#[test]
fn smallest_valid_memory_and_terminal_exhaustion() {
    for bot in [0, 1] {
        let mut m = Memory::new(MemoryConfig {
            mem_bot: bot,
            mem_top: bot + 1100,
            mem_max: bot + 1100,
        })
        .unwrap();
        for expected in (bot + 1021..=bot + 1086).rev() {
            assert_eq!(m.get_avail().unwrap().address(), expected);
        }
        assert_eq!(m.stats().dyn_used, 80);
        let error = MemoryError::Exhausted { capacity: 1101 };
        assert_eq!(m.get_avail(), Err(error));
        assert_eq!(m.stats().hi_mem_min, m.stats().lo_mem_max);
        let stopped = m.stats();
        assert_eq!(m.get_avail(), Err(error));
        assert_eq!(m.flush_list(MemPtr::NULL), Err(error));
        assert_eq!(m.stats(), stopped);
    }
}

#[test]
fn invalid_lists_are_rejected_without_partial_reclamation() {
    let mut m = small(0);
    let a = m.get_avail().unwrap();
    let b = m.get_avail().unwrap();
    m.set_link(a, b).unwrap();
    m.set_link(b, a).unwrap();
    let before = m.stats();
    assert_eq!(m.flush_list(a), Err(MemoryError::CyclicList));
    assert_eq!(m.stats(), before);
    m.set_link(b, MemPtr::NULL).unwrap();
    m.flush_list(a).unwrap();
    let before = m.stats();
    assert_eq!(m.flush_list(a), Err(MemoryError::InvalidNode));
    assert!(m.set_info(a, 1).is_err());
    assert!(m.link(a).is_err());
    m.flush_list(MemPtr::NULL).unwrap();
    assert_eq!(m.stats(), before);
    assert!(m.set_link(MemPtr::NULL, b).is_err());

    let a = m.get_avail().unwrap();
    let b = m.get_avail().unwrap();
    m.set_link(a, b).unwrap();
    m.flush_list(b).unwrap(); // creates a dangling link in a, an invalid caller state
    let before = m.stats();
    assert_eq!(m.flush_list(a), Err(MemoryError::InvalidNode));
    assert_eq!(m.stats(), before);
    assert!(m.set_link(a, b).is_err());
}

#[test]
fn invalid_configurations_and_maximum_halfword_boundary() {
    for c in [
        MemoryConfig {
            mem_bot: 2,
            mem_top: 3000,
            mem_max: 3000,
        },
        MemoryConfig {
            mem_bot: 0,
            mem_top: 1099,
            mem_max: 1100,
        },
        MemoryConfig {
            mem_bot: 1,
            mem_top: 1100,
            mem_max: 1100,
        },
        MemoryConfig {
            mem_bot: 0,
            mem_top: 1100,
            mem_max: 1099,
        },
        MemoryConfig {
            mem_bot: 0,
            mem_top: 1100,
            mem_max: 65535,
        },
    ] {
        assert!(matches!(
            Memory::new(c),
            Err(MemoryError::InvalidConfiguration)
        ));
    }
    let mut m = Memory::new(MemoryConfig {
        mem_bot: 0,
        mem_top: 65533,
        mem_max: 65534,
    })
    .unwrap();
    assert_eq!(m.get_avail().unwrap().address(), 65534);
    assert_eq!(m.get_avail().unwrap().address(), 65519);
}

#[test]
fn deterministic_sequences_match_an_independent_address_queue() {
    let mut m = small(3);
    let mut virgin: VecDeque<u16> = [1101, 1102, 1103]
        .into_iter()
        .chain((1021..=1086).rev())
        .collect();
    let mut free = VecDeque::new();
    let mut live = Vec::new();
    let mut seed = 82_u64;
    for step in 0..5000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        if live.is_empty() || (live.len() < 50 && seed % 3 != 0) {
            let expected = free.pop_front().or_else(|| virgin.pop_front()).unwrap();
            let p = m.get_avail().unwrap();
            assert_eq!(p.address(), expected);
            assert_eq!(m.link(p).unwrap(), MemPtr::NULL);
            m.set_info(p, (step % 65536) as u16).unwrap();
            live.push(p);
        } else {
            let count = 1 + (seed as usize % live.len().min(5));
            let nodes: Vec<_> = live.drain(..count).collect();
            for pair in nodes.windows(2) {
                m.set_link(pair[0], pair[1]).unwrap();
            }
            m.set_link(*nodes.last().unwrap(), MemPtr::NULL).unwrap();
            m.flush_list(nodes[0]).unwrap();
            for p in nodes.into_iter().rev() {
                free.push_front(p.address());
            }
        }
        assert_eq!(m.stats().dyn_used, 14 + live.len());
        let unique: std::collections::HashSet<_> = live.iter().copied().collect();
        assert_eq!(unique.len(), live.len());
    }
}

use webloom::memory::{
    COALESCE_REQUEST, MemPtr, Memory, MemoryConfig, MemoryError, NodeAllocation,
};

fn memory(bot: u16, top: u16, extra: u16) -> Memory {
    Memory::new(MemoryConfig {
        mem_bot: bot,
        mem_top: bot + top,
        mem_max: bot + top + extra,
    })
    .unwrap()
}

fn allocate(m: &mut Memory, size: i32) -> MemPtr {
    match m.get_node(size).unwrap() {
        NodeAllocation::Allocated(p) => p,
        NodeAllocation::Coalesced => panic!("unexpected merge-only result"),
    }
}

#[test]
fn splitting_reinsertion_and_coalescing_preserve_source_addresses() {
    for bot in [0, 1] {
        let mut m = memory(bot, 1100, 0);
        let a = allocate(&mut m, 10);
        let b = allocate(&mut m, 20);
        assert_eq!([a.address(), b.address()], [bot + 1010, bot + 990]);
        assert_eq!(m.stats().rover.address(), bot + 20);
        assert_eq!(m.stats().var_used, 50);
        assert_eq!(m.link(a).unwrap(), MemPtr::NULL);
        m.set_info(a, 1234).unwrap();
        m.set_link(a, b).unwrap();
        m.free_node(a, 10).unwrap();
        m.free_node(b, 20).unwrap();
        let before = m.stats();
        assert_eq!(m.get_node(COALESCE_REQUEST), Ok(NodeAllocation::Coalesced));
        assert_eq!(m.stats(), before);
        assert_eq!(allocate(&mut m, 30).address(), bot + 990);
        assert_eq!(m.stats().var_used, 50);
    }
}

#[test]
fn exact_fit_keeps_the_ring_nonempty_and_skips_a_one_word_remainder() {
    for (size, address) in [(998, 22), (999, 55), (1000, 54)] {
        let mut m = memory(0, 1100, 0);
        assert_eq!(allocate(&mut m, size).address(), address);
        assert_eq!(m.stats().lo_mem_max, if size == 998 { 1020 } else { 1054 });
    }
    let mut m = memory(0, 1100, 0);
    let a = allocate(&mut m, 998);
    assert_eq!(a.address(), 22);
    let b = allocate(&mut m, 2);
    assert_eq!(b.address(), 1052);
    assert_eq!(m.stats().rover.address(), 1020);
    let c = allocate(&mut m, 32);
    assert_eq!(c.address(), 1020); // Whole-block removal from a two-block ring.
    assert_eq!(m.stats().rover.address(), 20);
    assert_eq!(m.stats().var_used, 1052);
}

#[test]
fn growth_uses_the_source_threshold_and_integer_difference() {
    // hi_mem_min = top - 13; test both sides of §126's 1998 threshold.
    for (top, low) in [(3030, 2019), (3031, 2020), (3032, 2020)] {
        for bot in [0, 1] {
            let mut m = memory(bot, top, 0);
            allocate(&mut m, 998);
            let p = allocate(&mut m, 2);
            assert_eq!(m.stats().lo_mem_max, bot + low);
            assert_eq!(p.address(), bot + low - 2);
            assert!(m.stats().lo_mem_max < m.stats().hi_mem_min);
        }
    }
}

#[test]
fn invalid_requests_and_reclamation_leave_the_allocator_usable() {
    let mut m = memory(0, 1100, 0);
    let block = allocate(&mut m, 4);
    let single = m.get_avail().unwrap();
    let before = m.stats();
    for size in [i32::MIN, -1, 0, 1] {
        assert_eq!(m.get_node(size), Err(MemoryError::InvalidSize));
        assert_eq!(m.stats(), before);
    }
    assert_eq!(m.free_node(block, 1), Err(MemoryError::InvalidSize));
    assert_eq!(
        m.free_node(block, 3),
        Err(MemoryError::SizeMismatch {
            expected: 4,
            actual: 3
        })
    );
    assert_eq!(
        m.free_node(single, 4),
        Err(MemoryError::WrongAllocationKind)
    );
    assert_eq!(m.free_node(MemPtr::NULL, 4), Err(MemoryError::InvalidNode));
    assert_eq!(m.flush_list(block), Err(MemoryError::WrongAllocationKind));
    m.set_link(single, block).unwrap();
    assert_eq!(m.flush_list(single), Err(MemoryError::WrongAllocationKind));
    assert_eq!(m.stats(), before);
    m.set_link(single, MemPtr::NULL).unwrap();
    m.flush_list(single).unwrap();
    m.free_node(block, 4).unwrap();
    let before = m.stats();
    assert_eq!(m.free_node(block, 4), Err(MemoryError::InvalidNode));
    assert_eq!(m.set_info(block, 0), Err(MemoryError::InvalidNode));
    assert_eq!(m.stats(), before);
    assert_eq!(allocate(&mut m, 4), block);
}

#[test]
fn mixed_allocations_and_both_exhaustion_paths_share_the_frontier() {
    for bot in [0, 1] {
        let mut m = memory(bot, 1100, 2);
        let block = allocate(&mut m, 998);
        allocate(&mut m, 2); // Grows lo_mem_max to bot+1054.
        assert_eq!(m.get_avail().unwrap().address(), bot + 1101);
        assert_eq!(m.get_avail().unwrap().address(), bot + 1102);
        for address in (bot + 1055..=bot + 1086).rev() {
            assert_eq!(m.get_avail().unwrap().address(), address);
        }
        let error = MemoryError::Exhausted { capacity: 1103 };
        assert_eq!(m.get_avail(), Err(error));
        let stopped = m.stats();
        assert_eq!(m.get_node(2), Err(error));
        assert_eq!(m.get_node(COALESCE_REQUEST), Err(error));
        assert_eq!(m.free_node(block, 998), Err(error));
        assert_eq!(m.stats(), stopped);
    }
    for size in [65535, COALESCE_REQUEST + 1, i32::MAX] {
        let mut m = memory(0, 1100, 0);
        let hi = m.stats().hi_mem_min;
        let error = MemoryError::Exhausted { capacity: 1101 };
        assert_eq!(m.get_node(size), Err(error));
        assert_eq!(m.stats().hi_mem_min, hi);
        assert!(m.stats().lo_mem_max + 2 >= hi);
        let stopped = m.stats();
        assert_eq!(m.get_avail(), Err(error));
        assert_eq!(m.get_node(2), Err(error));
        assert_eq!(m.stats(), stopped);
    }
}

#[test]
fn merge_only_never_grows_or_exhausts_even_when_no_allocation_can_fit() {
    let mut m = memory(0, 1100, 0);
    allocate(&mut m, 998); // Leave a singleton two-word free block.
    for _ in 0..66 {
        m.get_avail().unwrap();
    }
    let before = m.stats();
    assert_eq!(m.get_node(COALESCE_REQUEST), Ok(NodeAllocation::Coalesced));
    assert_eq!(m.stats(), before);
    assert_eq!(
        m.get_node(2),
        Err(MemoryError::Exhausted { capacity: 1101 })
    );
}

#[test]
fn maximum_capacity_supports_large_blocks_without_halfword_wraparound() {
    for bot in [0, 1] {
        let mut m = Memory::new(MemoryConfig {
            mem_bot: bot,
            mem_top: 65533,
            mem_max: 65534,
        })
        .unwrap();
        let p = allocate(&mut m, 64000);
        assert!(u32::from(p.address()) + 64000 <= u32::from(m.stats().lo_mem_max));
        assert_eq!(m.stats().var_used, 64020);
        assert_eq!(m.get_avail().unwrap().address(), 65534);
        m.free_node(p, 64000).unwrap();
        assert_eq!(m.stats().var_used, 20);
        assert_eq!(m.get_node(COALESCE_REQUEST), Ok(NodeAllocation::Coalesced));
        assert_eq!(allocate(&mut m, 64000), p);
    }
}

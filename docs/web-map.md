# Verified WEB map

Numbers refer only to the pinned `vendor/knuth/tex.web`, not a change-file output.
Run `python3 scripts/verify_sources.py` to verify hashes and section anchors.
For unnumbered WEB prose sections the table uses their subject, not an invented
formal title. Starred part titles and routine identifiers are taken from source.

| Sections | Source title / subject | Implementation and verification | Status |
|---|---|---|---|
| 11–14 | Capacity parameters and constant checks | MemoryConfig; memory configuration tests | Memory subset |
| 99,101,104 | Arithmetic with scaled dimensions; scaled; shared arithmetic state | arithmetic.rs; Scaled/domain/state tests | Implemented slice |
| 105 | mult_and_add | Arithmetic::mult_and_add; boundary and wide-product tests | Implemented |
| 106 | x_over_n | Arithmetic::x_over_n; signed division, zero divisor, state tests | Implemented |
| 107 | xn_over_d | Arithmetic::xn_over_d; limb-overflow return, remainder, wide-product oracle | Implemented within documented domain |
| 108 | badness | arithmetic::badness; threshold vectors, all 1291 finite ratio inputs | Implemented |
| 109 | Glue ratios | Discussed in semantics.md | Unimplemented |
| 110–113 | Packed data; range checks; conversions; variant records | memory::Word; overlapping_views_have_one_backing_value | Integer/half/quarter subset; glue_ratio absent |
| 115–118 | Dynamic memory allocation; regions; statistics; avail/link/info | Memory, MemPtr, MemoryStats; sequence tests | Implemented single-word subset |
| 119 | Runaway diagnostic dependencies | Explicit terminal MemoryError | Diagnostic text deferred |
| 120 | get_avail | Memory::get_avail; frontier order/exhaustion tests | Implemented |
| 121–122 | free_avail / fast_get_avail | No dedicated API; flush_list handles singleton reclamation | Unimplemented macros, no stubs |
| 123 | flush_list | Memory::flush_list; order, null, cycles, double reclaim tests | Implemented with precondition validation |
| 124 | Variable-size free-list structure | Initial rover block only; initialization unit test | No variable-size operations |
| 133,150 | type/subtype; glue specification layout | Memory::initialize; static glue tests | Layout dependencies only |
| 162–164 | Memory layout; initialization variable; table entries | Memory::initialize; initialization_matches_static_layout | Memory subset, not full INITEX |
| 797,820,981 | end_span, active, page_ins_head initialization | Memory::initialize; reserved word assertions | Initialization only |

Sections 100,102–103 (half, decimal conversion and printing) are not claimed as
implemented. Nor are surrounding box, alignment, paragraph, or page algorithms.

Next coherent slice: variable-size allocation, §§124–130, using the already
initialized rover ring and shared high/low frontiers. §131–132 sort_avail is a
separate format-preparation dependency and can wait for format work.
Acceptance: exact rover movement, coalescing, splitting, whole-block removal,
get_node(2^30) merge-only behavior, free_node reinsertion order, low-frontier
growth, mixed single/variable allocations, and exhaustion at small capacities;
deterministic independent interval-model tests in debug and release.
Section 126 divides the nonnegative frontier difference by two; preserve that
expression rather than substituting an algebraic midpoint with different bounds.

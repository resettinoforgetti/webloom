# Implemented design

`Engine` owns `Arithmetic` and `Memory`; there are no globals or host services.
Arithmetic state includes the sticky error flag and division remainder.
`Scaled` is a signed integer in the source's magnitude-below-2^31 domain,
not a restriction to user-entered dimensions (which have the smaller 2^30 bound).

Memory is one fixed-size indexed vector, allocated once. `MemPtr` distinguishes
addresses from numeric fields; null is the source's halfword zero. Private raw
words use a single u32 backing value, with masks for overlapping fields.
Single-word allocation follows the WEB avail stack and frontier order. Variable
blocks use the source's doubly linked rover ring, deferred coalescing, splitting
from a block's high end, and shared high/low frontiers. One free block always
remains in the ring. A side table distinguishes free words, single-word nodes,
variable-block heads (with their size), and interiors. It validates access and
reclamation without selecting addresses or changing valid allocation order.
No other engine subsystems are scaffolded.

`get_node(i32)` returns `NodeAllocation::Allocated(MemPtr)` or, for the
`COALESCE_REQUEST` sentinel, `NodeAllocation::Coalesced`. The latter represents
WEB's max_halfword return without treating that flag as an address.
`free_node(MemPtr, u16)` requires the exact original size. `MemoryStats` includes
`rover` and `var_used` (initially 20 static low words); `dyn_used` remains the
independent high-word count. Existing head-field access supports both allocation
kinds; `flush_list` accepts only single-word nodes. Interior word/node-layout
access belongs to the subsequent node milestone.

Fallible APIs separate violated internal preconditions from source arithmetic
errors and fatal memory exhaustion. The CLI reports status and rejects all
typesetting requests with failure status. There is no host file-search layer,
format loader, diagnostic renderer, or fake DVI output.

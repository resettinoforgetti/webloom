# Implemented design

`Engine` owns `Arithmetic` and `Memory`; there are no globals or host services.
Arithmetic state includes the sticky error flag and division remainder.
`Scaled` is a signed integer in the source's magnitude-below-2^31 domain,
not a restriction to user-entered dimensions (which have the smaller 2^30 bound).

Memory is one fixed-size indexed vector, allocated once. `MemPtr` distinguishes
addresses from numeric fields; null is the source's halfword zero. Private raw
words use a single u32 backing value, with masks for overlapping fields.
Allocation follows the WEB avail stack and frontier order. A side table detects
invalid access and double reclamation without changing valid addresses/order.
The low region is initialized and reserved but variable-size allocation is
unavailable. No other engine subsystems are scaffolded.

Fallible APIs separate violated internal preconditions from source arithmetic
errors and fatal memory exhaustion. The CLI reports status and rejects all
typesetting requests with failure status. There is no host file-search layer,
format loader, diagnostic renderer, or fake DVI output.

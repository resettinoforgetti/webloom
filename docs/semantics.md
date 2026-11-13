# Compatibility profile and contracts

Categories used here: **S** specified by pinned WEB; **P** explicit parameter or
platform adaptation; **U** outside stated preconditions or unresolved.

## Integer arithmetic

**S** §§99–108: integers have magnitude below 2^31; scaled values have 16
fraction bits. `Scaled` excludes -2^31. Entered dimensions are limited to
2^30-1, but internal scaled values are not restricted to that narrower range.
The sticky `arith_error` is never cleared by these routines. Only division
routines change `remainder`. Resetting the flag is an explicit caller action.
**P** new engine state starts with false/zero; the full WEB's callers normally
reset the flag before computations. Invalid API inputs leave both fields intact.

| Routine | Accepted domain and exact behavior |
|---|---|
| mult_and_add (§105) | n excludes -2^31; max >= 0. When n=0 return y without a magnitude check. Otherwise require abs(y)<=max and max+abs(y)<=2^31-1, keeping both source division numerators nonnegative and representable. Negate x,n for negative n. Use both WEB inequalities; failure sets flag and returns zero. Remainder unchanged. |
| x_over_n (§106) | x,n exclude -2^31. Quotient truncates toward zero, remainder has x's sign even when n<0; x=q*n+r. n=0 sets flag, returns zero, stores x in remainder. |
| xn_over_d (§107) | 0<=n<=65536, 1<=d<=65536; x excludes -2^31. Require source intermediate t,u,v <=2^31-1. Compute base-32768 limbs. If u/d>=32768 set flag and return signed **uncombined u**; still store signed v%d. Otherwise combine quotient limbs. |
| badness (§108) | t>=0, s signed scaled. t=0 returns 0 even for s<=0; otherwise s<=0 returns 10000. Preserve thresholds 7230584,1663497,1290 and integer rounding (r^3+131072)/262144. No arithmetic state mutation. |

Bounds: mult_and_add's division numerators lie in [0,2^31-1]; successful
n*x is bounded by max+abs(y), so the final expression is representable.
x_over_n only divides absolute values <=2^31-1.
For xn_over_d, t<=32767*65536=2147418112; u may exceed the source
nonnegative_integer range even under the stated n,d bounds when x is large.
This API detects that **U** case rather than silently extending Pascal's range.
v<d*32768<=2^31; a successful combined quotient is <2^30.
All bounds are checked before shared state changes, using i64 temporaries.
For badness, 7230584*297=2147483448; s/297>=5601 in the large branch;
1290^3+131072=2146820072 <2^31. Division is nonnegative throughout the
source algorithm. Signed division truncates even at half fractions; it has no
round-to-nearest tie rule. Badness adds a half unit before truncation (exact
half-unit cube ties are unreachable, as explained in testing.md).

**U** arithmetic beyond these preconditions is not assigned universal Pascal
overflow behavior. Widening is for validation, not a new accepted TeX range.
Unrestricted dimension addition/subtraction (§104) is not implemented.
Glue ratios (§109) are real-valued with explicitly host-dependent rounding;
neither glue ratio storage nor its arithmetic is implemented. No whole-engine
floating-point determinism claim is made.

## Memory

**S/P** §§110–124,162–164: quarterword 0..255, halfword 0..65535, null=0,
empty_flag=65535. Default mem_min=mem_bot=0, mem_top=mem_max=30000.
Configurable mem_bot is 0 or 1 (the latter supports the eventual TRIP profile),
mem_top>=mem_bot+1100 and mem_top<=mem_max<65535. Production-style extension
above mem_top is supported; extension below mem_bot is deliberately unavailable.
These bounds keep all addresses representable and honor §§14,111.

Five static glue specifications occupy 20 low words. The circular free block
starts at mem_bot+20, size 1000; lo_mem_max=mem_bot+1020 is a zero sentinel.
Fourteen static high words occupy mem_top-13..mem_top. Their initialization
includes the special assignments from §§797,820,981. They are inaccessible as
dynamic nodes. This is memory-table initialization, **not complete INITEX**.
The smallest accepted configuration has 66 dynamic high words before exhaustion.

`get_avail` (§120) pops avail first, then grows mem_end, then decrements
hi_mem_min. It clears only link and preserves info. `flush_list` (§123) attaches
the old avail to the tail, preserving list order; null is a no-op. Lists must
be finite, null-terminated, and contain only currently allocated dynamic nodes.
Statistics follow the source's stat variant: dyn_used starts at 14.

**U/P** WEB does not initialize info in virgin dynamic words. Rust zero-fills
the backing array so every read is memory-safe; this does not establish that
source callers may rely on a virgin info value of zero. Callers must initialize
payloads before semantic use. Recycled info is preserved and tested. MemPtr is
an index, not an ownership capability: callers must use the originating Memory
and stop using a node after reclamation. The live bitmap catches currently free
addresses, but intentionally does not add generations or change address reuse.

**P** memory word bits: lh=bits31..16, rh=15..0, b0=31..24, b1=23..16,
b2=15..8, b3=7..0. Integer/scaled views share all bits. One backing integer
prevents inconsistent views; no dependence on native endian order or Pascal ABI.
Only tested views are exposed; glue ratios remain unavailable. The source makes
no relative-field-position assumption (§110); our mapping is not a format ABI.

**P** safe API adaptation: validate reclamation before mutation, and reject
cycles, nonlive addresses, reserved values and double frees. No valid source
caller observes this validation. Fatal exhaustion records the source's collided
hi_mem_min and permanently marks the allocator exhausted; returns a capacity
error instead of running missing runaway/overflow diagnostics (§119/§120).
Later mutation remains unavailable. Allocation indices and successful reuse
order are unchanged. Host allocation failure is distinct from TeX capacity.

## Host and future boundaries

**P** future byte input will use identity 8-bit ASCII-compatible translation
(§§17–24); no Unicode normalization. Text files will use LF and explicit byte
paths, with deterministic fixture-local search. This avoids locale-dependent
decoding and implicit distribution paths. These choices are declared, **not yet
implemented or tested**; §49's printable-string rules must remain intact.
No document files are opened by the current CLI; it uses host CLI argument bytes
only for exact ASCII option matching. No external command execution exists.

Only the memory table has capacity parameters now. Future table baselines are
the pristine §§11–12 constants (buf_size=500, stack_size=200, font_max=75,
font_mem_size=20000, max_strings=3000, pool_size=32000, save_size=600,
hash_size=2100, trie_size=8000); these tables do not exist in this milestone.
INITEX, format dumping/loading and complete diagnostics are unsupported.
Independent equivalent formats, not binary interchange, are the future contract.
No e-TeX/pdfTeX/XeTeX/LuaTeX features are included.

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
| half (§100) | x excludes -2^31 and 2^31-1 (where odd x+1 would exceed 2^31-1). Returns (x+1)/2 for odd x and x/2 for even x. |
| round_decimals (§102) | 0<=k<=17 decimal digits in 0..=9. Accumulates a=(a+dig[k]*131072)/10 from k-1 down to 0 and returns (a+1)/2 in 0..=65536. |
| print_scaled (§103) | s signed scaled. Prints '-' and negates when s<0, prints integer part and '.', then emits the shortest decimal fraction that round_decimals reconstructs to s. |
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

**S/P** §§110–130,162–164: quarterword 0..255, halfword 0..65535, null=0,
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

`get_node` (§§125–129) takes a signed source integer. Values below two are
invalid API inputs; all other positive requests retain source semantics,
including capacity exhaustion for unfulfillable large requests. The distinguished
2^30 request coalesces the ring, returns `Coalesced`, and never grows low memory,
allocates a node, or exhausts a running allocator. It can move rover while merging.
Ordinary allocation clears only the returned head's link; other payload words
are retained and must be initialized by their semantic caller.

Free blocks are at least two words. Allocation merges physically adjacent free
successors, splits from the high end only if at least two words remain, and
removes an exact-fit block only if another ring member exists. A one-word
remainder is unusable. §126 growth uses the nonnegative frontier difference:
1000 words when the difference is >=1998, otherwise 1 + difference/2. Growth
inserts the old low sentinel before rover and makes it rover; the new low
sentinel has zero info/link. The high frontier is unchanged by low exhaustion.

`free_node` (§130) inserts immediately before rover without eager coalescing.
`var_used` starts at 20 and changes by the allocated/reclaimed size. Single-word
and variable-size statistics remain independent even though they share capacity.
`sort_avail` (§§131–132) and format serialization remain unavailable.

**P** checked reclamation requires an allocated block head and its exact size.
Invalid sizes, interior/free/reserved addresses, wrong-kind frees, size mismatch,
and double reclamation are rejected before mutation. `flush_list` validates every
node as single-word, including when a valid head links to a variable block.
These errors do not stop the allocator. Capacity exhaustion is terminal for both
allocation kinds: even later merge-only requests and reclamation fail unchanged.

**U/P** WEB does not initialize info in virgin dynamic words. Rust zero-fills
the backing array so every read is memory-safe; this does not establish that
source callers may rely on a virgin info value of zero. Callers must initialize
payloads before semantic use. Recycled info is preserved and tested. MemPtr is
an index, not an ownership capability: callers must use the originating Memory
and stop using a node after reclamation. Allocation metadata catches currently
free/interior addresses and wrong allocation kinds, but intentionally does not
add generations or change address reuse.

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

## I/O, strings, printing, errors, and host boundary

**S/P** §§1–98,241–246,511–538: `IoConfig` defaults to pristine §§11–12
capacities (`buf_size=500`, `error_line=72`, `half_error_line=42`,
`max_print_line=79`, `max_in_open=6`, `max_strings=3000`,
`string_vacancies=8000`, `pool_size=32000`, `file_name_size=40`,
`dvi_buf_size=800`) and also provides the TRIP profile (`IoConfig::trip()`).
`check_constants` enforces §§14,111,522 (`bad` cases 1–7, 17, 18, 31).

`CharTranslator::pristine` (§§21,23–24) maps printable ASCII `32..=126` to
itself, blanks `xchr[0..31]` and `xchr[127..255]`, and computes `xord` in source
loop order so `' '` maps to 32 while non-printable external bytes map to
`invalid_code=127`. `CharTranslator::identity_eight_bit` provides the §23
permissive 8-bit adaptation while keeping §49's printable-character predicate
(`" "..="~"`) intact for `get_strings_started`.

`AlphaFileIn` models Pascal file variables (`f^`, `get`, `eoln`, `eof`),
including `/I` suppressed initial `get` on `term_in` (§33) and `eoln=true` when
`eof` occurs mid-line (§30). `input_ln` (§31) strips trailing spaces, updates
`max_buf_stack`, and reports buffer overflow (§35) via `FinalEnd` when
`format_ident=0` or `overflow("buffer size", buf_size)` when a format is active.
`get_strings_started` (§§47–53) builds strings `0..255` (`^^@`..`^^_`, printable
ASCII, `^^?`, `^^80`..`^^ff`) and reads `TeXformats:TEX.POOL` with
`string_vacancies` and `*504454778` checksum verification.

Printing (`§§54–71`) honors all selectors (`0..15`, `no_print=16`,
`term_only=17`, `log_only=18`, `term_and_log=19`, `pseudo=20`, `new_string=21`),
`max_print_line` wrapping, `new_line_char`, `tally`, `trick_buf`, `slow_print`,
and overflow-safe `print_int` (including `-2^31`). Error handling (`§§72–98`)
implements all four interaction modes, interactive `0..9`/`E`/`H`/`I`/`Q`/`R`/`S`/`X`/`?`
responses, the 100-error fatal threshold, `normalize_selector`, `fatal_error`,
`overflow`, `confusion`, and `pause_for_instructions`, returning explicit
`Result<_, Termination>` (`JumpOut` / `FinalEnd`) instead of non-local `goto`.
Syntactic/token callbacks (`show_context`, `get_token`, `give_err_help`) are
routed through `ErrorHooks`; `Engine::error` attaches live `ScannerErrorHooks`.

File-name routines (`§§511–538`) split `:`/`>` areas and `.` extensions, pack
fixed-width `name_of_file`, format the `§536` transcript banner using the host
`Clock` (default `1776-07-04 12:00`), and look up files in the user area and
`TeXinputs:` inside the deterministic `HostFixture`. No ambient OS filesystem
search, locale decoding, or external command execution is performed. The CLI
still rejects document typesetting, INITEX, and formats until M4–M8 are complete.

## Syntactic state and macro arguments (M4, partial)

`Eqtb` implements the six equivalent-table regions and level tracking;
`HashTable` performs the source hash/probe algorithm and registers primitive
names. Registration does not mean the primitive's execution is implemented.
`SaveStack` restores local equivalents, preserves global assignments, and returns
after-group tokens to its caller. Box reclamation and box/nest display still
depend on later node algorithms. `SemanticNest` stores and pushes/pops list state.

`Scanner::get_next` and `get_token` implement category-code lexical states,
control-sequence lookup, ^^ conversion, comments/spaces/end-of-line handling,
outer-token recovery, and token-list parameter substitution. Token inputs retain
reference-counted macro bodies and reclaim owned backed-up lists and parameters
when popped. Full alignment-template processing remains outside this slice.

`Engine::macro_call` (§§387–401) requires a valid compiled macro selected by
`get_token`: `cur_cs` identifies its equivalent and `cur_chr` its reference-count
head. Parameter text contains at most nine `match` tokens followed by
`end_match`; replacement text uses numbered `out_param` tokens. Definition
scanning and general `get_x_token`/`expand` dispatch remain unimplemented; the
API does not execute raw `\def` input.

Arguments are scanned without expansion, using full token equality (including
category codes). Undelimited arguments skip spaces and consume one token/group;
delimited arguments match only outside groups and retain overlapping delimiter
prefixes. Exactly one enclosing group is stripped when it forms the entire
argument. A delimiter ending in `#{` corrects `align_state` before the brace
is scanned again in the replacement text. Compulsory prefix mismatches consume
the mismatch and discard the call, as in §398.

Non-long paragraph recovery returns `\par` to the input, releases already
collected arguments, and corrects unmatched braces. A delimiter may itself
contain `\par`, including the partial-match exception described in §397.
Extra right braces use §395's inserted-paragraph recovery; forbidden outer
tokens use the scanner's existing recovery and avoid a duplicate runaway error.
The caller's scanner status and warning index are restored on return, including
terminal `Termination` results. Terminal overflow ends execution and does not
promise reusable partial argument lists.

Collected arguments remain off `param_stack` until exhausted inputs have been
popped. This preserves old parameters while scanning nested arguments and
reclaims old inputs for tail calls. The new body holds a reference until popped,
and parameter capacity is checked before writing the stack. Tracing prints the
definition and each argument via the same token diagnostics used by the scanner.

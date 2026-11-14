# Verified CLI engine roadmap

Target: classic Knuth TeX 3.141592653, compatibility first, Linux x86-64, stable
Rust >=1.85. The deliverable is an independently initialized command-line engine
with format loading, DVI output, and reproducible, documented TRIP results.

## Current implementation status

- M1: CI, full source coverage ledger, and the pinned reference build/TRIP
  qualification gate (`webloom-classic-reference-v1`) are implemented; see
  [reference qualification](reference.md).
- M2: variable-size allocation implemented and tested against an independent
  interval model; see [the verified WEB map](web-map.md).
- M3: host byte streams, file lookup, clocks, character translation, line
  buffers, string pool (`TEX.POOL`), printing, error reporting, interaction
  modes, and Part 29 file-name/transcript routines are implemented; token-driven
  `scan_file_name` (`§526`) and `show_context`/`get_token` callbacks connect in M4.
- M4–M9: not implemented. The CLI still rejects typesetting, INITEX, and formats.

No later integration or whole-engine compatibility gate is passed by M1–M3. A
milestone is complete only when its stated acceptance checks pass; a declared
owner in [coverage.md](coverage.md) is not implementation proof.

## Milestones and exit criteria

| ID | Work | Acceptance |
|---|---|---|
| M1 | CI on Rust 1.85.0/current stable; source checks; full WEB ledger; pinned complete reference build chain | Both compiler profiles pass; reference builds reproducibly and passes the official TRIP procedure with individually documented permitted differences |
| M2 | §§124–130 variable-size allocation, checked reclamation, allocation-kind metadata, rover/usage statistics | Exact splitting/coalescing/ring/frontier behavior, mixed allocation sequences, invalid frees, small-capacity exhaustion, debug/release interval-model checks |
| M3 | Host byte streams, file lookup, clocks, character translation, line buffers, string pool, printing, errors and interaction modes | Byte and line boundaries, printable-character rules, pool limits, wrapping, scripted errors/recovery, deterministic host fixtures |
| M4 | Commands, equivalents/hash/save tables, grouping, tokens/input stacks, scanners, macros, expansion, conditionals, assignments and primitive registration | Source-derived boundaries and malformed inputs; nested inputs/groups; local/global state; argument and expansion control; reference fixtures where interfaces match |
| M5 | Node layouts/accessors, constructors/copy/destruction/display, TFM/fonts, ligatures/kerns, packing, DVI shipping and semantic modes | Controlled boxes compare glyphs/rules/glue/kerns/nesting/movements/fonts; explicit validated f64 glue profile; DVI structure and controlled raw bytes |
| M6 | Paragraph breaking, discretionaries, hyphenation, vertical breaking, page building, insertions, marks, adjustments, output routines | Matching line/page decisions, hyphenation, insertions and output routines; plain-TeX smoke test once required primitives exist |
| M7 | Math structures/scanning/styles/spacing/fractions/scripts/delimiters/displays; alignments; remaining primitives, deferred writes, specials and termination | Focused math/alignment comparisons; every runtime WEB section accounted for; no required primitive remains a placeholder |
| M8 | Complete INITEX/hyphenation initialization, sort_avail, independently versioned format dump/load, CLI modes | Fresh/reloaded initialization agrees; malformed formats fail before state activation; each engine builds its own equivalent format |
| M9 | Isolated bounded comparison runner and official TRIP qualification | Reproducible results, retained raw artifacts/diffs, narrowly tested permitted normalizations, and all subsystem gates passed |

Implement each milestone as focused reviewable changes. Keep arithmetic and
allocation tests active while integrating higher-level code. Update the precise
WEB map, contracts, verification evidence, and coverage status with each slice.

## Interface rules and boundaries

`Engine` owns semantic state (`Arithmetic`, `Memory`, and `IoState`), with
`HostFixture` supplying deterministic byte streams, terminal interaction,
explicit file search, and clock values. Keep `MemPtr` as an index and raw memory
words private. Add checked typed node access in M5.

Future CLI operations include initialization, format selection, interaction
mode, input execution, and explicit input/output directories. Unsupported
operations must fail visibly. Formats use explicit field encodings, a versioned
compatibility header and validation before loading; distribution format ABI
compatibility is not required.

Keep the Rust engine dependency-free and unsafe code forbidden. Input remains
byte-oriented with the documented LF policy. No Unicode or extended-engine
features, PDF output, external command execution, or user distribution packaging
are in scope. Fonts/macros are explicit runtime/fixture inputs.

See [testing.md](testing.md) for artifact capture, bounds, clock controls, and the
official TRIP sequence. Never equate source checks, upstream filtered tests, or
the current allocator model with whole-engine compatibility.

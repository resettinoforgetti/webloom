# Webloom

A small Rust foundation targeting Knuth's TeX 3.141592653 (2021).
This is not yet a TeX implementation and cannot typeset documents.

Implemented: scaled arithmetic (`§§99–108`), indexed single-word/variable-size
memory allocation (`§§110–130, 162–164`), character translation, line buffers,
string pool (`TEX.POOL`), multi-selector printing, error/interaction handling,
and Part 29 file-name/transcript routines (`§§1–98, 241–246, 511–538`).
No dependencies; stable Rust 1.85 or later, edition 2024.

Run `cargo test`, `cargo test --release`, and `cargo run -- --version`.
See [sources](docs/sources.md), [reference qualification](docs/reference.md),
[semantics](docs/semantics.md), [architecture](docs/architecture.md),
[WEB map](docs/web-map.md), and [testing](docs/testing.md). Original source
notices remain in `vendor/`.

The [engine roadmap](docs/roadmap.md) tracks the remaining milestones. The
[coverage ledger](docs/coverage.md) assigns all 1,380 WEB sections to milestones;
it does not claim that unimplemented sections are supported. CI checks both
Rust 1.85.0 and current stable, including debug and release tests.

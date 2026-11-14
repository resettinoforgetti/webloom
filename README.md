# Webloom

A small Rust foundation targeting Knuth's TeX 3.141592653 (2021).
This is not yet a TeX implementation and cannot typeset documents.

Implemented: four arithmetic routines and indexed single-word/variable-size allocation.
No dependencies; stable Rust 1.85 or later, edition 2024.

Run `cargo test`, `cargo test --release`, and `cargo run -- --version`.
See [sources](docs/sources.md), [semantics](docs/semantics.md),
[architecture](docs/architecture.md), [WEB map](docs/web-map.md), and
[testing](docs/testing.md). Original source notices remain in `vendor/`.

The [engine roadmap](docs/roadmap.md) tracks the remaining milestones. The
[coverage ledger](docs/coverage.md) assigns all 1,380 WEB sections to milestones;
it does not claim that unimplemented sections are supported. CI checks both
Rust 1.85.0 and current stable, including debug and release tests.

# Verification

From the repository root, on any configured stable Rust >=1.85:

```
python3 scripts/verify_sources.py
python3 scripts/verify_coverage.py
python3 -B scripts/test_reference_trip.py
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --release
```

This workspace's toolchain lives outside the repository. Set:

```
export RUSTUP_HOME=/workspace/.rustup
export CARGO_HOME=/workspace/.cargo
export PATH=/workspace/.cargo/bin:$PATH
```

There are no crate dependencies or external test services. Tests are
source-derived, **not differential verification**. Arithmetic uses fixed
boundary vectors, state-transition checks, an i64 direct-product oracle within
proved bounds, and 20,000 deterministic cases. The oracle for §107 overflow
returns signed floor(abs(x*n)/32768), independently of the implementation's
limb decomposition. Badness tests cover both branch thresholds, saturation,
and every finite quantized ratio r=0..1290. There is no exact cube-rounding tie:
r^3 mod 2^18 = 2^17 would require 3*v2(r)=17. All neighboring rounding cases
are included; signed divisions explicitly exercise half fractions.

Memory tests cover overlapping views, reserved initialization, mem_bot=0 and 1,
both frontier directions, all 66 usable words of the smallest valid memory,
retained info, list-tail splicing, invalid/cyclic lists, fatal state, and 5,000
operations compared with an independent address queue. CLI tests require failure
for document and initialization requests.

Variable-size memory adds an independent interval/ring model with separate
single-word queues. Across 48 deterministic configurations/seeds it checks mixed
operations until exhaustion or 1,200 steps: returned addresses, rover/order,
reciprocal links, usage counts, allocation-kind metadata, and complete low-region
partitioning. Fixed cases cover exact fit, singleton retention, one-word
remainders, growth thresholds, maximum halfword bounds, merge-only behavior,
retained payloads, and atomic invalid reclamation.

GitHub Actions runs the checks above on Rust 1.85.0 and current stable, with both
debug and release Rust tests. Python scripts require Python >=3.11. Reference
builds are opt-in; routine CI tests the capture runner with small subprocesses
without downloading or compiling TeX Live.

## Reference and TRIP: reference profile qualified

No Webloom/reference differential runner is built yet: Webloom exposes
arithmetic/memory as Rust APIs, while the reference TeX executable exposes a
document interpreter. There is no common runnable interface at this milestone.
TANGLE generation and byte-identical source mirror comparison are provenance
checks only.

Build and qualify the pinned reference profile with:

```
python3 -B scripts/build_reference.py
python3 -B scripts/build_reference.py --build target/reference-build-v1-repeat
python3 -B scripts/reference_trip.py --repeat-build target/reference-build-v1-repeat
```

Single-build capture without `--repeat-build` returns 2 (`captured_requires_review`).
Passing `--repeat-build` runs the qualification gate across both builds, a
perturbed clock environment (`TZ=PST8PDT`, `SOURCE_DATE_EPOCH`,
`FORCE_SOURCE_DATE`), and the narrow Appendix A allowance validator while
retaining raw `.diff` files; see [reference qualification](reference.md).
Webloom must implement input, expansion, typesetting, diagnostics, INITEX, and
independent formats before its own TRIP run or document/DVI comparison is
possible.

The future two-engine runner must create separate temporary working directories with
identical copied fixture bytes. Bound wall time (initially 30 seconds/process)
and combined output/file bytes (initially 16 MiB/process); limit breaches fail
the test, retaining partial results. Capture stdout, stderr, logs, every generated
file, exit status/signal, exact command/environment, executable/input hashes,
capacities, script input and clock inputs. Retain directories on any failure.
Report the first differing artifact and byte/line offset, preserving raw diffs.
Never broadly remove numbers, whitespace, diagnostics, or DVI fields.

Control LC_ALL=C, TZ=UTC, input bytes, fixture-only font/macro/search paths,
working directory, time/date values, interaction mode and scripted responses.
Disable shell escape in the reference (`-no-shell-escape` for Web2C); the new
engine must not add an external command facility. Build equivalent formats
separately; do not feed a distribution .fmt file into Webloom.

## Exact TRIP procedure

Follow `vendor/knuth/tripman.tex`, Appendix A, in order. `reference_trip.py`
executes this sequence and validates the documented profile allowances.

1. In each isolated fixture directory, run `pltotf trip.pl trip.tfm`, then
   `tftopl trip.tfm tmp.pl`; compare tmp.pl with the supplied trip.pl, investigating
   differences. Record the utility identities. Preserve the supplied originals.
2. Build the special INITEX/stat reference and Rust profile with mem_min=mem_bot=1,
   mem_top=mem_max=3000, error_line=64, half_error_line=32, max_print_line=72;
   other parameters normally §§11–12 defaults, including dvi_buf_size=800.
   Preserve §49's unprintable-character defaults.
3. Invoke `/qualified/reference/tex -ini -no-shell-escape`
   with exact stdin bytes `\n\\input trip\n`. Save terminal capture as tripin.fot
   and generated trip.log as tripin.log. This creates trip.fmt. The future Rust
   executable must expose an equivalent initialization invocation before use.
4. Invoke the same reference command with exact stdin bytes ` &trip  trip \n`.
   Do not override interaction mode: the loaded format must retain the
   `nonstopmode` set by trip.tex. An errorstop override aborts at its first
   intentional error.
   Compare trip.log, trip.dvi, tripos.tex, empty 8terminal.tex and terminal
   capture trip.fot against the official outputs. Do the equivalent Rust run
   using its independently constructed format.
5. Compare logs under Appendix A step 5's allowances: banners/relative file
   prefixes; narrowly identified glue-set rounding on the eight documented
   `\vbox` lines; actually differing Web2C capacity floors and string
   statistics; and the reference-only Web2C duplicate-hyphenation adaptation
   (while keeping pristine §940 authoritative for Webloom). Dates match
   `1776-07-04 12:00` literally via `scripts/reference-clock.ch`.
6. Run `dvitype -output-level=2 '-page-start=*.*.*.*.*.*.*.*.*.*' -max-pages=1000000
   -dpi=72.27 -magnification=0 trip.dvi`. Compare with trip.typ per Appendix A
   step 6. Verify that raw DVI bytes match everywhere except the two
   glue-rounded 4-byte vertical movement operands (`down4` at 1121 and `y4` at
   2535), retaining the raw byte diff.

TRIP is a major gate, not proof of all behavior. Continue focused subsystem
tests independently.

## Run record

2026-11-13, rustc 1.100.0: initial `cargo check --all-targets` and both test
profiles passed (15 tests each). Initial formatting/lint attempts reported
missing rustfmt/Clippy; those components were subsequently installed. Final
check outcomes after formatting and final implementation review:

| Command | Outcome |
|---|---|
| `cargo fmt --check` | Passed |
| `cargo check --all-targets` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed, no warnings |
| `cargo test` | Passed, 15 tests, none ignored |
| `cargo test --release` | Passed, 15 tests, none ignored |
| `python3 scripts/verify_sources.py` | Passed, 23 files, 32 anchors, 1380 WEB sections |
| `git diff --check -- . ':!vendor'` | Passed; pristine vendor whitespace preserved |
| `cargo run -- --version` | Passed; reports foundation/unsupported typesetting |

Compiler host: x86_64-unknown-linux-gnu; LLVM 23.1.5;
Cargo 1.100.0 (3acc79c0c 2026-11-05). No unrelated existing checks failed
(the workspace had no pre-existing project). The declared MSRV was not separately
tested. TRIP, document/DVI differential comparisons and binary format interchange
were not run; the first two are blocked by the missing engine subsystems and
reference build qualification, and format interchange is outside the contract.

### 2026-11-14 implementation checkpoint

- Rust 1.85.0 and stable: 26 tests passed in each of debug and release;
  no ignored tests. Formatting and Clippy (`--all-targets -- -D warnings`) pass.
- Source hashes and 36 WEB anchors pass; the coverage ledger accounts for
  all 55 parts and 1,380 sections.
- Six Python capture and qualification tests pass: exact binary I/O/nonzero exit
  capture, timeout retention, generated-file size limits, raw diff offsets,
  candidate build verification, and narrow Appendix A allowance validation
  (including rejection of date drift, box-dimension drift, memory-usage drift,
  and DVI byte tampering).
- The pinned reference profile (`webloom-classic-reference-v1`) builds in two
  separate directories with identical hashes for six executables and six
  generated source artifacts, and passes the two-build TRIP qualification gate
  with clock-perturbation invariance verified.
- Webloom still cannot typeset, initialize formats, or run TRIP. No whole-engine
  compatibility is claimed.

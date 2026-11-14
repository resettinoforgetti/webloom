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

## Reference and TRIP: candidate captured, not qualified

No Webloom/reference differential runner is built: Webloom exposes arithmetic/memory
as Rust APIs, while the installed TeX executable exposes a document interpreter.
There is no common runnable interface at this milestone. TANGLE generation and
byte-identical source mirror comparison are provenance checks only.

A new pinned reference candidate can be built with
`python3 scripts/build_reference.py`; capture its official TRIP sequence with
`python3 -B scripts/reference_trip.py`. The latter returns 2 for a completed run
requiring review, 1 for failure, and never reports qualification automatically.
See [reference evidence](reference.md) for repeat-build results and unresolved
profile/clock differences. The older vendored Web2C files and Debian inventory
remain historical inputs, not the new build chain. Webloom must implement input,
expansion, typesetting, diagnostics, INITEX and independent formats before its own
TRIP run or document/DVI comparison is possible.

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
engine must not add an external command facility. SOURCE_DATE_EPOCH alone is
not evidence that a classic executable's time inputs are controlled: verify its
effect or use a documented clock adaptation. Build equivalent formats separately;
do not feed a distribution .fmt file into Webloom.

## Exact TRIP procedure

Follow `vendor/knuth/tripman.tex`, Appendix A, in order. The paths below name a
future *qualified* Web2C build. `reference_trip.py` exercises this sequence on
the candidate; it deliberately leaves qualification pending.

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
5. Compare logs under Appendix A step 5's allowances: dates/file names;
   narrowly identified glue-set and accent-kern rounding; actually differing
   capacities; help messages; string statistics; memory statistics. No allowance
   is automatically applied here. Each future normalization needs an explicit
   rationale and tests proving it cannot conceal a typesetting difference.
6. Run `dvitype -output-level=2 '-page-start=*.*.*.*.*.*.*.*.*.*' -max-pages=1000000
   -dpi=72.27 -magnification=0 trip.dvi` (verify flags against the pinned
   utility). Compare with trip.typ per Appendix A step 6. Compare raw DVI bytes
   when clocks/comments/font metadata and all other affecting inputs are
   controlled. Retain the raw byte diff even if the official manual permits
   semantically equivalent movement commands or narrow rounding differences.

TRIP is a major gate, not proof of all behavior. Continue focused subsystem
tests independently. The next allocator milestone's acceptance criteria are
listed in web-map.md.

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

- Rust 1.85.0 and stable 1.100.0: 26 tests passed in each of debug and release;
  no ignored tests. Formatting and Clippy (`--all-targets -- -D warnings`) pass.
- Source hashes and 36 WEB anchors pass; the coverage ledger accounts for
  all 55 parts and 1,380 sections.
- Four Python capture tests pass: exact binary I/O/nonzero exit capture,
  timeout retention, generated-file size limits, and raw diff offsets.
- The pinned reference candidate builds in two separate directories with
  identical hashes for six executables and six generated source artifacts.
- Reference-only TRIP capture produces 16 pages and a 2,920-byte DVI file.
  Font round trip and tripos.tex are identical; 8terminal.tex is empty. Raw
  log/terminal/DVI differences are retained and described in reference.md.
- Webloom still cannot typeset, initialize formats, or run TRIP. No whole-engine
  compatibility or fully qualified differential reference is claimed.

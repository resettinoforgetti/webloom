# Reference oracle qualification

Status: **qualified reference profile (`webloom-classic-reference-v1`)**. This is
reference-only qualification. Webloom's Rust engine cannot yet run document or
TRIP tests, and no whole-engine Webloom compatibility is claimed.

## Reproduce

Requirements: Linux, Python >=3.11, Git, C/C++ compilers, GNU Make, Perl, and the
usual C library development files. The Rust engine itself remains dependency-free.

```
python3 -B scripts/build_reference.py
python3 -B scripts/build_reference.py --build target/reference-build-v1-repeat
python3 -B scripts/reference_trip.py --repeat-build target/reference-build-v1-repeat
```

The build script fetches the locked source when absent. Existing source trees
must match the revision and be clean; mismatched trees are never overwritten.
Each build directory retains its input identity, logs, local change hashes, and
`webloom-build.json`. `verify_candidate` rejects stale profiles or modified
executables/generated sources before any capture or qualification run.

Each process is limited to 30 seconds and each generated file to 16 MiB. The
runner additionally checks the aggregate working-directory file size, including
inputs and previous steps, against 16 MiB; a breach fails the run and preserves
evidence. It retains exact stdin/stdout/stderr, exit codes/signals, commands,
environment, hashes, generated artifacts, and byte/line diff offsets. Process
groups are terminated on completion or breach. Single-build capture without
`--repeat-build` exits with status 2 (`captured_requires_review`). Passing
`--repeat-build` runs the two-build qualification gate and exits with status 0
only when both builds, the clock-perturbation probe, and all narrow Appendix A
rules pass while retaining raw diffs.

## Locked profile and deterministic clock adaptation

- Source: TeX Live commit `dce038dd1bd98fac739254da4a2066c8347cdf6c`.
- Normative `tex.web` matches the repository SHA-256
  `c62ab513ef167e93f71a23bd34f311e243210afd7c7a0f9b779614b71e398324`.
- Ordered upstream changes: `tex.ch`, `enctex1.ch`, `enctex-tex.ch`,
  `enctex2.ch`, `tex-binpool.ch`. SyncTeX and extended engines are disabled.
- Checked local clock change: `scripts/reference-clock.ch` (SHA-256
  `43884c36b17692d1ec18131342583569e28b039c112974a2597c7c91d0576d1a`), appended
  to `tex_ch_srcs` after `tex-binpool.ch` without modifying `vendor/knuth/tex.web`
  or the upstream checkout. Classic `onlyTeX` excludes the `SOURCE_DATE_EPOCH`
  and `FORCE_SOURCE_DATE` code paths in `texmfmp.c`; `reference-clock.ch` keeps
  Web2C's C `date_and_time` call (which installs the interrupt handler) and
  restores the deterministic timestamp of pristine `tex.web` §241
  (`sys_time=12*60`, `sys_day=4`, `sys_month=7`, `sys_year=1776`).

## Repeat build and clock-invariance verification

- Two separate build directories (`target/reference-build-v1` and
  `target/reference-build-v1-repeat`) produced identical SHA-256 digests for
  all six executables (`tex`, `tangle`, `tie`, `dvitype`, `pltotf`, `tftopl`)
  and all six generated source artifacts (`tex-final.ch`, `tex.p`, `tex.pool`,
  `tex0.c`, `texini.c`, `texd.h`).
- On Debian 12 (`gcc`/`g++` 12.2.0-14+deb12u1, GNU Make 4.3, GNU ld 2.40),
  the `tex` executable SHA-256 is
  `13d6c36270b9acf9e623f4882d35f32764078e48e401c71f646648281208bc3d` and
  `tex0.c` is `6e0bc5dc7b23306745de6f182c05c4f4c40e61824618c8b47585ed777145aa22`.
- Running the primary build under `TZ=UTC` and the repeat build under
  `TZ=PST8PDT`, `SOURCE_DATE_EPOCH=946684800`, `FORCE_SOURCE_DATE=1` produced
  identical SHA-256 digests across all 13 output artifacts (`trip.tfm`,
  `tmp.pl`, `tripin.log`, `tripin.stdout`, `tripin.stderr`, `trip.log`,
  `trip.stdout`, `trip.stderr`, `trip.dvi`, `tripos.tex`, `8terminal.tex`,
  `dvitype.stdout`, `dvitype.stderr`).
- In `trip.fmt` (135,298 bytes), all 135,294 initialized bytes match between
  the two runs; the 4 bytes at offset `132404..132408` immediately after
  `font_ptr = 3` are `font_check[null_font]`, which `tex.web` §552 leaves
  uninitialized before §1322 dumps `font_check[0..font_ptr]` and which Web2C
  allocates via `malloc`. Webloom's Rust engine and format dump will
  zero-initialize `font_check[null_font]`.

## Validated TRIP rules and oracle authority

`validate_allowances` in `scripts/reference_trip.py` requires raw `.diff` files
to be retained and checks that every difference against `vendor/knuth` belongs
to one of the narrow rules below:

1. **Clock literals (no wildcard)**: `4 JUL 1776 12:00`,
   `(preloaded format=trip 1776.7.4)`, and `' TeX output 1776.07.04:1200'` must
   match Knuth's master files literally. Any date drift is rejected.
2. **Banners, relative file prefixes, and non-TTY stdin echo (Appendix A 5a)**:
   `(TeX Live 2027/dev)` in the `TeX` and `DVItype` banners, `./trip.tex` and
   `./tripos.tex` path prefixes, and non-echoed pipe input lines at the start
   of `tripin.stdout` and `trip.stdout`.
3. **String pool statistics and compile-time capacity floors (Appendix A 5d, 5f)**:
   `1374 strings of total length 24704` and `9 ops out of 35111` in `tripin`;
   `44 strings out of 1626`, `261 string characters out of 7296`, and
   `372 multiletter control sequences out of 15000+0` in `trip.log`. Actual
   usage counts (`2825 words of memory out of 3000`, `471 memory locations
   dumped; current usage is 96&278`, `372 multiletter control sequences`,
   `3041 words of font info for 4 fonts`, `7i,7n,9p,113b,38s`) are not relaxed
   and must match exactly.
4. **Duplicate hyphenation exception (`1 hyphenation exception out of 659`)**:
   Web2C `tex.ch` §§939–940 replaces duplicate hyphenation exceptions rather
   than leaving duplicate entries as in pristine `tex.web` §940 (`2 hyphenation
   exceptions out of 307`). **Pristine `tex.web` §940 remains authoritative for
   Webloom**; this allowance applies only when qualifying the Web2C reference
   build.
5. **Eight `\vbox` `glue set` displays (Appendix A 5b)**: only lines `1405`,
   `2019`, `2534`, `3327`, `3865`, `4035`, `4421`, and `4743` of `trip.log` may
   differ, only in the floating-point ratio before `fil` (within `5e-4`), and
   their `\vbox(<height>+<depth>)x<width>, glue set ` prefix must match
   identically.
6. **Two glue-rounded vertical DVI movements (Appendix A 6)**: `trip.dvi`
   (2,920 bytes) must match `vendor/knuth/trip.dvi` at every byte except the
   4-byte signed operands of `down4` at byte `1121` (`639342177` versus
   `639342208`) and `y4` at byte `2535` (`203921756` versus `203921760`), and
   `dvitype.stdout` lines `419`, `1087`, and `1088` must report those exact
   decoded integers.

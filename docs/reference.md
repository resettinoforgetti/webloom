# Reference candidate evidence

Status: **built and exercised; not a qualified differential oracle**. This is
reference-only work. Webloom's Rust engine cannot yet run document or TRIP tests.

## Reproduce

Requirements: Linux, Python >=3.11, Git, C/C++ compilers, GNU Make, Perl, and the
usual C library development files. The Rust engine itself remains dependency-free.

```
python3 -B scripts/build_reference.py
python3 -B scripts/build_reference.py --build target/reference-build-repeat
python3 -B scripts/reference_trip.py
```

The build script fetches the locked source when absent. Existing source trees
must match the revision and be clean; mismatched trees are never overwritten.
Each build directory retains its input identity, logs and `webloom-build.json`.
The runner verifies executable hashes before invoking them. It creates a unique
directory below `target/reference-runs`, copies fixture bytes, restricts search
paths to that directory, and passes `-no-shell-escape` to both TeX invocations.

Each process is limited to 30 seconds and each generated file to 16 MiB. The
runner additionally checks the aggregate working-directory file size, including
inputs and previous steps, against 16 MiB; a breach fails the run and preserves
evidence. It retains exact stdin/stdout/stderr, exit codes/signals, commands,
environment, hashes, generated artifacts, and byte/line diff offsets. Process
groups are terminated on completion or breach. No normalization is applied.
Exit 2 means capture completed but still requires qualification review; exit 1
means the run failed. No exit status certifies compatibility.

## Observed build, 2026-11-14

- Source: TeX Live commit `dce038dd1bd98fac739254da4a2066c8347cdf6c`.
- Normative tex.web matches the repository SHA-256 exactly.
- Ordered changes: tex.ch, enctex1.ch, enctex-tex.ch, enctex2.ch,
  tex-binpool.ch. SyncTeX and extended engines are disabled. The locked change
  chain is an explicit platform adaptation, not pristine WEB.
- Tools: Debian GCC/G++ 14.2.0-19, GNU Make 4.4.1, GNU ld 2.44.
- Two separate build directories on this host produced identical SHA-256 values
  for tex, tangle, tie, dvitype, pltotf, tftopl and for tex-final.ch, tex.p,
  tex.pool, tex0.c, texini.c, texd.h. This is a same-toolchain repeat-build result,
  not a claim of reproducibility across arbitrary compiler versions/platforms.
- TeX executable SHA-256:
  `be1f9720ba8e1e5ee318c1fb7199902c0316956cd943995d53f156088f7b4ba1`.

## Observed TRIP capture

The completed run `trip-mowuo06f` produced 16 pages and 2,920 DVI bytes. PLtoTF
followed by TFtoPL reproduced trip.pl byte for byte. tripos.tex matches the
official output and 8terminal.tex exists and is empty. Both TeX invocations
returned 1 because TRIP deliberately triggers errors; the required artifacts
were produced. DVItype completed successfully.

Observed raw differences:

- Build banners, current dates, `./` file prefixes and terminal input echo.
- String counts/lengths, capacities and hyphenation statistics. In particular
  Web2C reports hash capacity 15000+0, hyphenation capacity 659 and nest capacity
  60; the requested pristine capacities are not all representable by this build.
- Eight box-display glue values differ by floating-point rounding. Box dimensions
  in those lines are unchanged. Examples include 16342.0fil versus
  16341.99998fil and 9737.58789fil versus 9737.58742fil.
- DVItype reports `down4 639342208` versus `639342177`, and `y4`/`y0 203921760`
  versus `203921756`, alongside the date comment and banner.
- The reference counts one hyphenation exception where pristine TRIP reports
  two. The pinned upstream triptrap/README explains its replacement of duplicate
  exceptions; that adaptation must not silently redefine Webloom's source semantics.

Appendix A permits specific date/name, glue rounding, capacity/string-statistic,
and DVI differences. These observations are **not** blanket allowances: any
future normalization needs narrow rules and tests that reject unrelated changes.
Raw artifacts and diffs remain under the run directory; `result.json` has hashes
and the first differing offsets for each compared artifact.

## Remaining qualification work

1. Specify how this platform profile may be used as an oracle, including its
   duplicate-hyphenation behavior and larger compile-time capacities. Keep
   pristine WEB authoritative where the profiles differ.
2. Add and test a documented clock adaptation. Classic `onlyTeX` excludes the
   SOURCE_DATE_EPOCH and FORCE_SOURCE_DATE paths in texmfmp.c; setting those
   variables alone does not control its clock.
3. Validate individually justified TRIP comparison rules, retaining raw diffs.
   Run the resulting qualification gate against both builds.

The capture also corrected two errors in the earlier procedure: the loaded
format must retain its dumped nonstop interaction mode, and this DVItype uses
`-page-start`/`-dpi`, not `-start-page`/`-resolution`. Forcing errorstop mode
aborts at TRIP's first intentional error and is not a valid test run.

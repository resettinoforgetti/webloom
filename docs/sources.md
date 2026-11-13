# Sources and target

Selected normative source: Knuth's TeX **3.141592653**, January 2021 lineage,
distributed February 5, 2021. This is not the original 1982 release.
`vendor/knuth/tex.web` is pristine, including commentary. Downloaded 2026-11-13
from https://tug.org/texlive/devsrc/Build/source/texk/web2c/tex.web and compared
byte for byte with https://ctan.math.illinois.edu/systems/knuth/dist/tex/tex.web.
`vendor/SHA256SUMS` pins every retained input; URLs alone are not pins.
The normative tex.web SHA-256 is
`c62ab513ef167e93f71a23bd34f311e243210afd7c7a0f9b779614b71e398324`.

The official TRIP materials in `vendor/knuth/` came from the same CTAN
`systems/knuth/dist/tex/` directory: tripman.tex, trip.tex, trip.pl, trip.tfm,
tripin.log, tripin.fot, trip.log, trip.fot, trip.dvi, trip.typ, tripos.tex.
The manual identifies Version 3, January 1990; its header says updated
December 4, 1989. Test inputs/outputs include the 2021 updates. Read Appendix A
alongside the actual fixtures, rather than assuming all files have one date.

Notices: tex.web permits unlimited redistribution **unchanged**, and requires
a different filename for modified WEB. It identifies TeX as an AMS trademark.
The TRIP manual requires successful validation before calling an implementation
TeX. Webloom uses its own package/executable name and makes no compatibility
certification claim. The Rust translation preserves attribution to Donald E.
Knuth; this repository does not relicense the vendored distribution.

## Reference candidate (not a qualified differential oracle)

`vendor/reference/inventory.txt` records executable hashes, versions, shared
libraries, Debian package versions and configuration lookup. `/usr/bin/tex`
reports TeX 3.141592653 (TeX Live 2025/dev/Debian), kpathsea 6.4.0/dev;
texlive-binaries is 2024.20240313.70630+ds-6; texlive-base is 2024.20250309-1.
This is the classic TeX executable, not pdfTeX or an extended engine.
Its exact build flags and full applied change-file chain have not been recovered.
No comparison run is represented as reference-qualified.

`vendor/reference/tex.ch` and `configure.ac` were downloaded from the corresponding
TeX Live devsrc directory on 2026-11-13. These are **platform adaptations**, not
canonical source, and are not established as the inputs to the installed binary.
They are retained for investigation only; none is applied to our normative WEB.
The retrieved tex.ch identifies SVN revision 79415 (2026-06-19), includes
MLTeX adaptations and shell-execution changes, and carries its own redistribution
notice (unchanged copies permitted). These extensions are not adopted.
The installed Debian copyright notice and texmf.cnf files are retained separately.
The reference's configured paths/capacities are not Webloom's profile.

Local plain.tex and cmr10.tfm were found via `kpsewhich` and copied under
`vendor/reference/`; their original paths appear in the inventory below.
No installed tex.pool or tex.fmt was found by `kpsewhich`. A format must be built
independently for each future engine; binary format interchange is not required.
TRIP uses its own supplied font and does not require plain.tex.

TANGLE 4.6 generated pristine Pascal and the string pool without a change file:

```
cd vendor/generated
tangle ../knuth/tex.web > tangle.log
```

The output, pool and successful transcript are retained and checksummed. Pascal
is an inspection aid, not a runnable port. Web2C C output and web2w were not
needed or obtained. No other Rust project was used as a semantic source.

## Environment

The initial `/workspace` contained no checkout or instructions; its `.git` was
an empty read-only mount. A new Git repository was initialized at
`/workspace/webloom`. No user source files were replaced.
No Rust toolchain was initially installed. A workspace-local stable toolchain
was installed using rustup: rustc 1.100.0 (0ef1bc386 2026-11-09).
Declared MSRV 1.85 is the edition-2024 baseline; only 1.100 has been tested.
Install command: `CARGO_HOME=/workspace/.cargo RUSTUP_HOME=/workspace/.rustup
sh /tmp/rustup-init.sh -y --no-modify-path --profile minimal --default-toolchain
stable`, using the installer from https://sh.rustup.rs; then
`rustup component add rustfmt clippy` with the same environment.
No compiler already present was upgraded.

Local supporting input origins:

- `/usr/share/texlive/texmf-dist/tex/plain/base/plain.tex`
- `/usr/share/texlive/texmf-dist/fonts/tfm/public/cm/cmr10.tfm`
- `/usr/share/texlive/texmf-dist/web2c/texmf.cnf`
- `/etc/texmf/web2c/texmf.cnf`
- `/usr/share/doc/texlive-binaries/copyright`

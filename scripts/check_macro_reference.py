#!/usr/bin/env python3
"""Check the shared M4 argument fixtures using the qualified reference build."""
import argparse
import csv
import json
import os
from pathlib import Path
import tempfile

from build_reference import verify_candidate
from reference_trip import ROOT, run, sha256


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build", type=Path, default=ROOT / "target/reference-build-v1")
    parser.add_argument("--output", type=Path, default=ROOT / "target/macro-reference")
    args = parser.parse_args()
    build = args.build.resolve()
    profile = verify_candidate(build)
    fixture = ROOT / "tests/fixtures/macro-arguments.tsv"
    cases = list(csv.reader((line for line in fixture.read_text().splitlines()
                            if line and not line.startswith("#")), delimiter="\t"))
    args.output.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix="macros-", dir=args.output.resolve()))
    # INITEX does not initially assign the usual brace and parameter catcodes.
    lines = [r"\catcode123=1 \catcode125=2 \catcode35=6 \nonstopmode"]
    expected = []
    for name, delimiter, argument, result in cases:
        lines.extend([
            r"\def\a#1" + delimiter + r"{\def\result{#1}}",
            r"\a " + argument,
            r"\immediate\write16{CASE:" + name + r":\meaning\result:END}",
        ])
        expected.append(f"CASE:{name}:macro:->{result}:END")
    lines.append(r"\end")
    (work / "macros.tex").write_text("\n".join(lines) + "\n")
    (work / "texmf.cnf").write_text("TEXINPUTS = .\nTEXFORMATS = .\nshell_escape = f\n")
    env = {"PATH": os.defpath, "LC_ALL": "C", "TZ": "UTC", "TEXMFCNF": str(work),
           "MKTEXFMT": "0", "MKTEXTFM": "0", "HOME": str(work)}
    record = run([str(build / "texk/web2c/tex"), "-ini", "-no-shell-escape", "macros.tex"],
                 work, env, "macros")
    actual = [line for line in (work / "macros.stdout").read_text().splitlines()
              if line.startswith("CASE:")]
    passed = record["exit_code"] == 0 and record["failure"] is None and actual == expected
    report = {"passed": passed, "fixture_sha256": sha256(fixture),
              "profile": profile["lock"], "run": record,
              "expected": expected, "actual": actual, "environment": env}
    (work / "comparison.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"{'PASS' if passed else 'FAIL'}: {len(cases)} macro argument fixtures; artifacts: {work}")
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())

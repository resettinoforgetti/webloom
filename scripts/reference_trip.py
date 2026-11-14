#!/usr/bin/env python3
"""Capture a bounded official TRIP run with raw diffs; never silently normalize."""
import argparse
import difflib
import hashlib
import json
import os
from pathlib import Path
import resource
import shutil
import signal
import subprocess
import tempfile
import time

from build_reference import verify_candidate

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "vendor/knuth"


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def run(command, cwd, env, name, stdin=b"", seconds=30, limit=16 * 1024 * 1024):
    """Retain stdout/stderr/input; bound each file and total working-directory bytes."""
    cwd = Path(cwd)
    (cwd / f"{name}.stdin").write_bytes(stdin)
    start = time.monotonic()
    reason = None

    def file_limit():
        resource.setrlimit(resource.RLIMIT_FSIZE, (limit, limit))

    with (cwd / f"{name}.stdin").open("rb") as source, (cwd / f"{name}.stdout").open("wb") as stdout, (cwd / f"{name}.stderr").open("wb") as stderr:
        process = subprocess.Popen(command, cwd=cwd, env=env, stdin=source,
                                   stdout=stdout, stderr=stderr, start_new_session=True,
                                   preexec_fn=file_limit)
        try:
            while True:
                total = sum(p.stat().st_size for p in cwd.rglob("*") if p.is_file())
                if total >= limit:
                    reason = "output_limit"
                    break
                if time.monotonic() - start >= seconds:
                    reason = "timeout"
                    break
                if process.poll() is not None:
                    break
                time.sleep(0.01)
        finally:
            # Terminate any children too, including after the parent exits.
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait()
    record = {"command": command, "exit_code": process.returncode, "failure": reason,
              "seconds": time.monotonic() - start, "stdin_sha256": sha256(cwd / f"{name}.stdin"),
              "executable_sha256": sha256(Path(command[0]))}
    (cwd / f"{name}.json").write_text(json.dumps(record, indent=2) + "\n")
    return record


def compare(expected, actual, diff_path):
    if not actual.exists():
        return {"status": "missing", "actual": str(actual)}
    left, right = expected.read_bytes(), actual.read_bytes()
    if left == right:
        return {"status": "identical", "sha256": sha256(actual)}
    offset = next((i for i, (a, b) in enumerate(zip(left, right)) if a != b), min(len(left), len(right)))
    # difflib.diff_bytes preserves raw non-UTF-8 bytes in diagnostic text files.
    diff_path.write_bytes(b"".join(difflib.diff_bytes(
        difflib.unified_diff, left.splitlines(keepends=True), right.splitlines(keepends=True),
        fromfile=os.fsencode(expected), tofile=os.fsencode(actual))))
    return {"status": "different", "first_byte_offset": offset,
            "expected_line": left[:offset].count(b"\n") + 1,
            "expected_sha256": sha256(expected), "actual_sha256": sha256(actual),
            "diff": str(diff_path)}


def capture(build, output_root, clock_environment=None):
    build = Path(build).resolve()
    report = verify_candidate(build)
    binaries = build / "texk/web2c"
    output_root = Path(output_root).resolve()
    output_root.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix="trip-", dir=output_root))
    print(f"Retaining all TRIP evidence in {work}", flush=True)
    for name in ("trip.tex", "trip.pl"):
        shutil.copyfile(FIXTURES / name, work / name)
    # Capacities from Appendix A and pristine WEB §§11–12; no distribution paths.
    (work / "texmf.cnf").write_text("""mem_bot = 1
main_memory = 3000
error_line = 64
half_error_line = 32
max_print_line = 72
max_strings = 3000
pool_size = 32000
string_vacancies = 8000
font_mem_size = 20000
font_max = 75
stack_size = 200
nest_size = 40
buf_size = 500
save_size = 600
dvi_buf_size = 800
trie_size = 8000
shell_escape = f
TEXINPUTS = .
TFMFONTS = .
TEXFORMATS = .
TEXFONTMAPS = .
TEXPOOL = .
MKTEXFMT = 0
MKTEXTFM = 0
MKTEXTEX = 0
""")
    # Do not inherit user's TeX configuration. The final WEB change fixes the
    # clock independently of these host settings; qualification probes that.
    env = {"PATH": os.defpath, "LC_ALL": "C", "TZ": "UTC",
           "TEXMFCNF": str(work), "shell_escape": "f", "MKTEXFMT": "0",
           "MKTEXTFM": "0", "MKTEXTEX": "0"}
    if clock_environment:
        if set(clock_environment) - {"TZ", "SOURCE_DATE_EPOCH", "FORCE_SOURCE_DATE"}:
            raise ValueError("Only clock environment overrides are supported")
        env.update(clock_environment)
    results = {"qualified": False, "environment": env, "build_report_sha256": sha256(build / "webloom-build.json"),
               "profile": report["lock"]["profile"], "clock_profile": report["lock"]["clock"],
               "inputs": {name: sha256(work / name) for name in ("trip.tex", "trip.pl", "texmf.cnf")},
               "steps": {}, "comparisons": {}}
    summary = work / "result.json"
    try:
        steps = [
            ("pltotf", [str(binaries / "pltotf"), "trip.pl", "trip.tfm"], b"", (0,)),
            ("tftopl", [str(binaries / "tftopl"), "trip.tfm", "tmp.pl"], b"", (0,)),
            ("tripin", [str(binaries / "tex"), "-ini", "-no-shell-escape"], b"\n\\input trip\n", (0, 1)),
            # Preserve the nonstop interaction mode dumped by trip.tex. A CLI
            # errorstop override aborts at the first intentional TRIP error.
            ("trip", [str(binaries / "tex"), "-ini", "-no-shell-escape"], b" &trip  trip \n", (0, 1)),
            ("dvitype", [str(binaries / "dvitype"), "-output-level=2", "-page-start=*.*.*.*.*.*.*.*.*.*", "-max-pages=1000000", "-dpi=72.27", "-magnification=0", "trip.dvi"], b"", (0,)),
        ]
        for name, command, stdin, allowed in steps:
            record = run(command, work, env, name, stdin)
            results["steps"][name] = record
            if record["failure"] or record["exit_code"] not in allowed:
                raise RuntimeError(f"{name} failed; see {work}")
            if name == "tftopl" and (work / "tmp.pl").read_bytes() != (work / "trip.pl").read_bytes():
                results["comparisons"]["trip.pl"] = compare(work / "trip.pl", work / "tmp.pl", work / "trip.pl.diff")
                raise RuntimeError("Font round trip differs; investigate before engine comparisons")
            if name == "tripin":
                if not (work / "trip.fmt").is_file():
                    raise RuntimeError("INITEX did not produce trip.fmt")
                (work / "trip.log").rename(work / "tripin.log")
            if name == "trip" and not (work / "trip.dvi").is_file():
                raise RuntimeError("TRIP did not produce trip.dvi; inspect trip.stdout and trip.log")
        actual_names = {"tripin.fot": "tripin.stdout", "trip.fot": "trip.stdout", "trip.typ": "dvitype.stdout"}
        for name in ("tripin.log", "tripin.fot", "trip.log", "trip.fot", "tripos.tex", "trip.dvi", "trip.typ"):
            results["comparisons"][name] = compare(FIXTURES / name, work / actual_names.get(name, name), work / f"{name}.diff")
        terminal = work / "8terminal.tex"
        results["comparisons"]["8terminal.tex"] = {"status": "identical" if terminal.is_file() and terminal.stat().st_size == 0 else "missing_or_nonempty"}
        results["status"] = "captured_requires_review"
    except (RuntimeError, OSError) as error:
        results.update(status="failed", error=str(error))
    finally:
        results["artifacts"] = {p.name: {"bytes": p.stat().st_size, "sha256": sha256(p)} for p in work.iterdir() if p.is_file() and p != summary}
        summary.write_text(json.dumps(results, indent=2) + "\n")
    print(f"{results['status']}: {summary}")
    return work, results


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build", type=Path, default=ROOT / "target/reference-build-v1")
    parser.add_argument("--output", type=Path, default=ROOT / "target/reference-runs")
    args = parser.parse_args()
    try:
        _, results = capture(args.build, args.output)
    except (ValueError, OSError, KeyError) as error:
        raise SystemExit(str(error)) from error
    # Qualification always needs review of the raw differences and clock/profile
    # adaptations, even if all selected artifacts happen to match byte for byte.
    return 1 if results["status"] == "failed" else 2


if __name__ == "__main__":
    raise SystemExit(main())

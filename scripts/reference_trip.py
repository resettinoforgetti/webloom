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


GLUE_SET_LINES = {1405, 2019, 2534, 3327, 3865, 4035, 4421, 4743}
BANNER = "This is TeX, Version 3.141592653 (TeX Live 2027/dev)"


def _require(condition, message):
    if not condition:
        raise ValueError(message)


def validate_allowances(work, results, fixtures=FIXTURES):
    """Verify every TRIP diff against narrow Appendix A rules while keeping raw diffs."""
    work, fixtures = Path(work), Path(fixtures)
    _require(results.get("status") == "captured_requires_review", "Capture did not complete cleanly")
    for step, record in results.get("steps", {}).items():
        _require(record.get("failure") is None, f"Step {step} had failure {record.get('failure')}")
    _require((work / "tmp.pl").read_bytes() == (fixtures / "trip.pl").read_bytes(), "trip.pl round-trip mismatch")
    _require((work / "tripos.tex").read_bytes() == (fixtures / "tripos.tex").read_bytes(), "tripos.tex mismatch")
    _require(results["comparisons"]["tripos.tex"]["status"] == "identical", "tripos.tex comparison status mismatch")
    _require((work / "8terminal.tex").is_file() and (work / "8terminal.tex").stat().st_size == 0, "8terminal.tex must be empty")
    _require(results["comparisons"]["8terminal.tex"]["status"] == "identical", "8terminal.tex comparison status mismatch")

    for name in ("tripin.log", "tripin.fot", "trip.log", "trip.fot", "trip.dvi", "trip.typ"):
        comp = results["comparisons"].get(name, {})
        diff_file = work / f"{name}.diff"
        _require(comp.get("status") == "different", f"{name} must record its raw profile difference")
        _require(comp.get("diff") == str(diff_file) and diff_file.is_file() and diff_file.stat().st_size > 0,
                 f"Missing retained raw diff for {name}")

    # 1. tripin.log: banner, ./trip.tex, string pool summary, trie op capacity.
    exp_in_log = (fixtures / "tripin.log").read_text().splitlines()
    act_in_log = (work / "tripin.log").read_text().splitlines()
    _require(len(act_in_log) == len(exp_in_log), "tripin.log line count changed")
    in_log_allowed = {
        1: f"{BANNER} (INITEX)  4 JUL 1776 12:00",
        3: "(./trip.tex",
        451: "1374 strings of total length 24704",
        461: "Hyphenation trie of length 273 has 9 ops out of 35111",
    }
    for idx, (exp, act) in enumerate(zip(exp_in_log, act_in_log), 1):
        if idx in in_log_allowed:
            _require(act == in_log_allowed[idx], f"tripin.log line {idx} unexpected: {act!r}")
        else:
            _require(act == exp, f"tripin.log line {idx} differs outside allowances: {act!r} != {exp!r}")

    # 2. tripin.fot (tripin.stdout): non-echoed pipe input + same string/trie summary.
    exp_in_fot = (fixtures / "tripin.fot").read_text().splitlines()
    act_in_fot = (work / "tripin.stdout").read_text().splitlines()
    _require(exp_in_fot[:5] == [
        "This is TeX, Version 3.141592653 (INITEX)",
        "**",
        "Please type the name of your input file.",
        "**\\input trip",
        "(trip.tex",
    ], "Unexpected master tripin.fot header")
    _require(act_in_fot[:3] == [
        f"{BANNER} (INITEX)",
        "**Please type the name of your input file.",
        "**(./trip.tex",
    ], f"Unexpected tripin.fot header: {act_in_fot[:3]!r}")
    _require(len(act_in_fot[3:]) == len(exp_in_fot[5:]), "tripin.fot body line count changed")
    for offset, (exp, act) in enumerate(zip(exp_in_fot[5:], act_in_fot[3:]), 4):
        if offset == 72:
            _require(act == "1374 strings of total length 24704", f"tripin.fot line {offset} unexpected: {act!r}")
        elif offset == 82:
            _require(act == "Hyphenation trie of length 273 has 9 ops out of 35111", f"tripin.fot line {offset} unexpected: {act!r}")
        else:
            _require(act == exp, f"tripin.fot line {offset} differs outside allowances: {act!r} != {exp!r}")

    # 3. trip.fot (trip.stdout): banner and non-echoed stdin only; all body lines identical.
    exp_fot = (fixtures / "trip.fot").read_text().splitlines()
    act_fot = (work / "trip.stdout").read_text().splitlines()
    _require(exp_fot[:3] == [
        "This is TeX, Version 3.141592653 (INITEX)",
        "** &trip  trip ",
        "(trip.tex ##",
    ], "Unexpected master trip.fot header")
    _require(act_fot[:2] == [
        f"{BANNER} (INITEX)",
        "**(./trip.tex ##",
    ], f"Unexpected trip.fot header: {act_fot[:2]!r}")
    _require(act_fot[2:] == exp_fot[3:], "trip.fot body differs outside header allowance")

    # 4. trip.log: banner, relative paths, 8 glue-set float ratios, and 4 summary lines.
    exp_log = (fixtures / "trip.log").read_text().splitlines()
    act_log = (work / "trip.log").read_text().splitlines()
    _require(len(act_log) == len(exp_log), "trip.log line count changed")
    log_allowed = {
        1: f"{BANNER} (preloaded format=trip 1776.7.4)  4 JUL 1776 12:00",
        3: "(./trip.tex ##",
        1955: "(./tripos.tex",
        6382: "(./tripos.tex",
        7298: " 44 strings out of 1626",
        7299: " 261 string characters out of 7296",
        7301: " 372 multiletter control sequences out of 15000+0",
        7303: " 1 hyphenation exception out of 659",
    }
    glue_deviations = {}
    for idx, (exp, act) in enumerate(zip(exp_log, act_log), 1):
        if idx in log_allowed:
            _require(act == log_allowed[idx], f"trip.log line {idx} unexpected: {act!r}")
        elif idx in GLUE_SET_LINES:
            head = ", glue set "
            _require(head in exp and head in act and exp.endswith("fil") and act.endswith("fil"),
                     f"trip.log line {idx} missing glue-set structure")
            exp_prefix, exp_ratio = exp[:-3].split(head, 1)
            act_prefix, act_ratio = act[:-3].split(head, 1)
            _require(act_prefix == exp_prefix and exp_prefix.startswith("\\vbox("),
                     f"trip.log line {idx} box dimensions changed: {act_prefix!r} != {exp_prefix!r}")
            delta = abs(float(act_ratio) - float(exp_ratio))
            _require(0 < delta <= 5e-4, f"trip.log line {idx} glue-set deviation out of bounds: {delta}")
            glue_deviations[str(idx)] = {"expected": exp_ratio + "fil", "actual": act_ratio + "fil", "abs_diff": round(delta, 7)}
        else:
            _require(act == exp, f"trip.log line {idx} differs outside allowances: {act!r} != {exp!r}")

    # 5. trip.dvi and trip.typ (dvitype.stdout): only two glue-rounded vertical movements.
    exp_dvi = (fixtures / "trip.dvi").read_bytes()
    act_dvi = (work / "trip.dvi").read_bytes()
    _require(len(act_dvi) == len(exp_dvi) == 2920, "trip.dvi byte length changed")
    _require(act_dvi[1121] == 160 and act_dvi[2535] == 165, "trip.dvi movement opcodes changed")
    down4_exp = int.from_bytes(exp_dvi[1122:1126], "big", signed=True)
    down4_act = int.from_bytes(act_dvi[1122:1126], "big", signed=True)
    y4_exp = int.from_bytes(exp_dvi[2536:2540], "big", signed=True)
    y4_act = int.from_bytes(act_dvi[2536:2540], "big", signed=True)
    _require(down4_exp == 639342208 and down4_act == 639342177, f"Unexpected down4 at 1121: {down4_act}")
    _require(y4_exp == 203921760 and y4_act == 203921756, f"Unexpected y4 at 2535: {y4_act}")
    masked_exp = bytearray(exp_dvi)
    masked_act = bytearray(act_dvi)
    masked_exp[1122:1126] = masked_act[1122:1126] = b"\x00\x00\x00\x00"
    masked_exp[2536:2540] = masked_act[2536:2540] = b"\x00\x00\x00\x00"
    _require(masked_act == masked_exp, "trip.dvi differs outside the two glue-rounded vertical movements")

    exp_typ = (fixtures / "trip.typ").read_text().splitlines()
    act_typ = (work / "dvitype.stdout").read_text().splitlines()
    _require(len(act_typ) == len(exp_typ) == 1214, "trip.typ line count changed")
    typ_allowed = {
        1: "This is DVItype, Version 3.6 (TeX Live 2027/dev)",
        419: f"1121: down4 {down4_act} ",
        1087: f"2535: y4 {y4_act} ",
        1088: f"2540: y0 {y4_act} ",
    }
    for idx, (exp, act) in enumerate(zip(exp_typ, act_typ), 1):
        if idx in typ_allowed:
            _require(act == typ_allowed[idx], f"trip.typ line {idx} unexpected: {act!r}")
        else:
            _require(act == exp, f"trip.typ line {idx} differs outside allowances: {act!r} != {exp!r}")

    return {
        "clock_verified": "1776-07-04 12:00 (literal match in tripin.log, trip.log, trip.dvi, trip.typ)",
        "glue_set_lines": glue_deviations,
        "dvi_movements": {
            "1121_down4": {"expected": down4_exp, "actual": down4_act},
            "2535_y4": {"expected": y4_exp, "actual": y4_act},
        },
        "profile_only_differences": {
            "banner": BANNER,
            "relative_paths": ["./trip.tex", "./tripos.tex"],
            "tripin_strings": {"expected": "1326/23633", "actual": "1374/24704"},
            "tripin_trie_op_capacity": {"expected": 500, "actual": 35111},
            "trip_strings": {"expected": "47/1674, 253/8367", "actual": "44/1626, 261/7296"},
            "trip_hash_capacity": {"expected": "2100", "actual": "15000+0"},
            "trip_hyphenation_exceptions": {
                "expected": "2 out of 307",
                "actual": "1 out of 659",
                "authority": "pristine tex.web §940 remains normative for Webloom; Web2C replaces duplicate exceptions",
            },
        },
    }


def qualify(build, repeat_build, output_root, report_path=None):
    """Qualify the reference oracle across two separate builds and perturbed clock env."""
    build, repeat_build = Path(build).resolve(), Path(repeat_build).resolve()
    _require(build != repeat_build, "Qualification requires two separate build directories")
    primary_report = verify_candidate(build)
    repeat_report = verify_candidate(repeat_build)
    for field in ("lock", "change_chain_sha256", "local_change_sha256", "binary_sha256", "generated_sha256"):
        _require(primary_report[field] == repeat_report[field], f"Repeat build mismatch in {field}")

    work1, results1 = capture(build, output_root)
    work2, results2 = capture(
        repeat_build,
        output_root,
        clock_environment={"TZ": "PST8PDT", "SOURCE_DATE_EPOCH": "946684800", "FORCE_SOURCE_DATE": "1"},
    )
    allowances = validate_allowances(work1, results1)
    validate_allowances(work2, results2)

    compared_artifacts = (
        "trip.tfm", "tmp.pl", "tripin.log", "tripin.stdout", "tripin.stderr",
        "trip.log", "trip.stdout", "trip.stderr", "trip.dvi", "tripos.tex", "8terminal.tex",
        "dvitype.stdout", "dvitype.stderr",
    )
    for name in compared_artifacts:
        _require(results1["artifacts"][name] == results2["artifacts"][name],
                 f"Artifact {name} differed between repeat builds or clock environments")
    # Web2C allocates font_check via malloc and tex.web §552 leaves font_check[null_font]
    # uninitialized before §1322 dumps font_check[0..font_ptr]. Every other byte of trip.fmt
    # must match across both builds and clock environments.
    fmt1 = bytearray((work1 / "trip.fmt").read_bytes())
    fmt2 = bytearray((work2 / "trip.fmt").read_bytes())
    _require(len(fmt1) == len(fmt2) == 135298, "trip.fmt byte length changed")
    _require(int.from_bytes(fmt1[132400:132404], "big", signed=True) == 3, "trip.fmt font_ptr offset changed")
    fmt1[132404:132408] = fmt2[132404:132408] = b"\x00\x00\x00\x00"
    _require(fmt1 == fmt2, "trip.fmt differed outside uninitialized font_check[null_font]")

    qualification = {
        "qualified": True,
        "profile": primary_report["lock"]["profile"],
        "primary_build": str(build),
        "repeat_build": str(repeat_build),
        "binary_sha256": primary_report["binary_sha256"],
        "generated_sha256": primary_report["generated_sha256"],
        "primary_run": str(work1),
        "repeat_run": str(work2),
        "clock_probe_environment": results2["environment"],
        "artifact_sha256": {name: results1["artifacts"][name]["sha256"] for name in compared_artifacts},
        "allowances": allowances,
    }
    for work, results in ((work1, results1), (work2, results2)):
        results.update(qualified=True, status="qualified_reference_oracle", allowances=allowances)
        (work / "result.json").write_text(json.dumps(results, indent=2) + "\n")
    if report_path is None:
        report_path = Path(output_root).resolve() / "qualification.json"
    else:
        report_path = Path(report_path).resolve()
    report_path.write_text(json.dumps(qualification, indent=2) + "\n")
    print(f"qualified_reference_oracle: {report_path}")
    return work1, work2, qualification


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build", type=Path, default=ROOT / "target/reference-build-v1")
    parser.add_argument("--repeat-build", type=Path, default=None)
    parser.add_argument("--output", type=Path, default=ROOT / "target/reference-runs")
    parser.add_argument("--qualify-report", type=Path, default=None)
    args = parser.parse_args()
    try:
        if args.repeat_build is not None:
            qualify(args.build, args.repeat_build, args.output, args.qualify_report)
            return 0
        _, results = capture(args.build, args.output)
    except (ValueError, RuntimeError, OSError, KeyError) as error:
        raise SystemExit(str(error)) from error
    # Single-build capture without repeat-build and allowance verification returns 2.
    return 1 if results["status"] == "failed" else 2


if __name__ == "__main__":
    raise SystemExit(main())

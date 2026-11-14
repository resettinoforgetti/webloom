#!/usr/bin/env python3
"""Exercise capture limits and raw-diff reporting without needing a TeX build."""
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
import unittest

from build_reference import LOCK, verify_candidate
from reference_trip import BANNER, FIXTURES, GLUE_SET_LINES, compare, run, validate_allowances


def make_valid_reference_capture(work: Path):
    """Build a synthetic capture directory from vendor/knuth with exact profile diffs."""
    shutil.copyfile(FIXTURES / "trip.pl", work / "tmp.pl")
    shutil.copyfile(FIXTURES / "tripos.tex", work / "tripos.tex")
    (work / "8terminal.tex").write_bytes(b"")

    in_log = (FIXTURES / "tripin.log").read_text().splitlines()
    in_log[0] = f"{BANNER} (INITEX)  4 JUL 1776 12:00"
    in_log[2] = "(./trip.tex"
    in_log[450] = "1374 strings of total length 24704"
    in_log[460] = "Hyphenation trie of length 273 has 9 ops out of 35111"
    (work / "tripin.log").write_text("\n".join(in_log) + "\n")

    in_fot = (FIXTURES / "tripin.fot").read_text().splitlines()
    in_stdout = [
        f"{BANNER} (INITEX)",
        "**Please type the name of your input file.",
        "**(./trip.tex",
        *in_fot[5:],
    ]
    in_stdout[71] = "1374 strings of total length 24704"
    in_stdout[81] = "Hyphenation trie of length 273 has 9 ops out of 35111"
    (work / "tripin.stdout").write_text("\n".join(in_stdout) + "\n")

    fot = (FIXTURES / "trip.fot").read_text().splitlines()
    stdout = [
        f"{BANNER} (INITEX)",
        "**(./trip.tex ##",
        *fot[3:],
    ]
    (work / "trip.stdout").write_text("\n".join(stdout) + "\n")

    log = (FIXTURES / "trip.log").read_text().splitlines()
    log[0] = f"{BANNER} (preloaded format=trip 1776.7.4)  4 JUL 1776 12:00"
    log[2] = "(./trip.tex ##"
    log[1954] = "(./tripos.tex"
    log[6381] = "(./tripos.tex"
    glue_replacements = {
        1405: "16341.99998fil",
        2019: "16238.99998fil",
        2534: "16330.99998fil",
        3327: "9737.58742fil",
        3865: "16317.99997fil",
        4035: "16343.99998fil",
        4421: "16331.99998fil",
        4743: "16343.99998fil",
    }
    for line_no, new_tail in glue_replacements.items():
        prefix = log[line_no - 1].split(", glue set ", 1)[0]
        log[line_no - 1] = f"{prefix}, glue set {new_tail}"
    log[7297] = " 44 strings out of 1626"
    log[7298] = " 261 string characters out of 7296"
    log[7300] = " 372 multiletter control sequences out of 15000+0"
    log[7302] = " 1 hyphenation exception out of 659"
    (work / "trip.log").write_text("\n".join(log) + "\n")

    dvi = bytearray((FIXTURES / "trip.dvi").read_bytes())
    dvi[1122:1126] = (639342177).to_bytes(4, "big", signed=True)
    dvi[2536:2540] = (203921756).to_bytes(4, "big", signed=True)
    (work / "trip.dvi").write_bytes(bytes(dvi))

    typ = (FIXTURES / "trip.typ").read_text().splitlines()
    typ[0] = "This is DVItype, Version 3.6 (TeX Live 2027/dev)"
    typ[418] = "1121: down4 639342177 "
    typ[1086] = "2535: y4 203921756 "
    typ[1087] = "2540: y0 203921756 "
    (work / "dvitype.stdout").write_text("\n".join(typ) + "\n")

    return rebuild_comparisons(work)


def rebuild_comparisons(work: Path):
    actual_names = {"tripin.fot": "tripin.stdout", "trip.fot": "trip.stdout", "trip.typ": "dvitype.stdout"}
    comparisons = {}
    for name in ("tripin.log", "tripin.fot", "trip.log", "trip.fot", "tripos.tex", "trip.dvi", "trip.typ"):
        comparisons[name] = compare(FIXTURES / name, work / actual_names.get(name, name), work / f"{name}.diff")
    terminal = work / "8terminal.tex"
    comparisons["8terminal.tex"] = {"status": "identical" if terminal.is_file() and terminal.stat().st_size == 0 else "missing_or_nonempty"}
    return {
        "status": "captured_requires_review",
        "steps": {step: {"failure": None} for step in ("pltotf", "tftopl", "tripin", "trip", "dvitype")},
        "comparisons": comparisons,
    }


class CaptureTests(unittest.TestCase):
    def test_preserves_stdin_stdout_stderr_and_nonzero_status(self):
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)
            command = [sys.executable, "-c", "import sys; sys.stdout.buffer.write(sys.stdin.buffer.read()); sys.stderr.write('diagnostic'); sys.exit(3)"]
            result = run(command, work, os.environ.copy(), "capture", b"\x00\xff\n")
            self.assertEqual(result["exit_code"], 3)
            self.assertIsNone(result["failure"])
            self.assertEqual((work / "capture.stdout").read_bytes(), b"\x00\xff\n")
            self.assertEqual((work / "capture.stderr").read_bytes(), b"diagnostic")
            self.assertTrue((work / "capture.json").is_file())

    def test_timeout_retains_partial_output(self):
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)
            command = [sys.executable, "-c", "import time; print('before timeout', flush=True); time.sleep(5)"]
            result = run(command, work, os.environ.copy(), "timeout", seconds=0.2)
            self.assertEqual(result["failure"], "timeout")
            self.assertLess(result["exit_code"], 0)
            self.assertEqual((work / "timeout.stdout").read_bytes(), b"before timeout\n")

    def test_output_limit_includes_generated_files(self):
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)
            command = [sys.executable, "-c", "from pathlib import Path; Path('artifact').write_bytes(b'x'*65536)"]
            result = run(command, work, os.environ.copy(), "limit", limit=8192)
            self.assertEqual(result["failure"], "output_limit")
            self.assertEqual((work / "artifact").stat().st_size, 8192)

    def test_raw_binary_difference_and_prefix_offsets(self):
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)
            expected, actual = work / "expected", work / "actual"
            expected.write_bytes(b"a\n\xffx")
            actual.write_bytes(b"a\n\xfey")
            result = compare(expected, actual, work / "raw.diff")
            self.assertEqual(result["first_byte_offset"], 2)
            self.assertEqual(result["expected_line"], 2)
            self.assertIn(b"\xffx", (work / "raw.diff").read_bytes())
            actual.write_bytes(b"a\n\xffxmore")
            self.assertEqual(compare(expected, actual, work / "prefix.diff")["first_byte_offset"], 4)
            actual.write_bytes(expected.read_bytes())
            self.assertEqual(compare(expected, actual, work / "same.diff")["status"], "identical")
            actual.unlink()
            self.assertEqual(compare(expected, actual, work / "missing.diff")["status"], "missing")

    def test_verify_candidate_checks_lock_and_artifact_hashes(self):
        with tempfile.TemporaryDirectory() as directory:
            build = Path(directory)
            web2c = build / "texk/web2c"
            web2c.mkdir(parents=True)
            lock = json.loads(LOCK.read_text())
            binaries = ("tex", "tangle", "tie", "dvitype", "pltotf", "tftopl")
            generated = ("tex-final.ch", "tex.p", "tex.pool", "tex0.c", "texini.c", "texd.h")
            bin_hashes = {}
            gen_hashes = {}
            for idx, name in enumerate(binaries):
                p = web2c / name
                p.write_bytes(f"bin-{idx}".encode())
                bin_hashes[name] = compare(p, p, build / "unused")["sha256"]
            for idx, name in enumerate(generated):
                p = web2c / name
                p.write_bytes(f"gen-{idx}".encode())
                gen_hashes[name] = compare(p, p, build / "unused")["sha256"]
            report = {
                "status": "built_candidate",
                "lock": lock,
                "local_change_sha256": {item["path"]: item["sha256"] for item in lock["local_changes"]},
                "binary_sha256": bin_hashes,
                "generated_sha256": gen_hashes,
            }
            (build / "webloom-build.json").write_text(json.dumps(report))
            self.assertEqual(verify_candidate(build)["lock"]["profile"], lock["profile"])

            (web2c / "tex").write_bytes(b"tampered")
            with self.assertRaisesRegex(ValueError, "Built reference artifact changed: tex"):
                verify_candidate(build)

    def test_validate_allowances_accepts_documented_profile_and_rejects_typesetting_drifts(self):
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)
            results = make_valid_reference_capture(work)
            allowances = validate_allowances(work, results)
            self.assertEqual(len(allowances["glue_set_lines"]), len(GLUE_SET_LINES))

            # 1. Missing raw diff must fail.
            (work / "trip.log.diff").unlink()
            with self.assertRaisesRegex(ValueError, "Missing retained raw diff for trip.log"):
                validate_allowances(work, results)
            results = make_valid_reference_capture(work)

            # 2. Clock drift in banner or format header must fail.
            log_lines = (work / "trip.log").read_text().splitlines()
            log_lines[0] = f"{BANNER} (preloaded format=trip 1776.7.4)  5 JUL 1776 12:00"
            (work / "trip.log").write_text("\n".join(log_lines) + "\n")
            with self.assertRaisesRegex(ValueError, "trip.log line 1 unexpected"):
                validate_allowances(work, rebuild_comparisons(work))
            results = make_valid_reference_capture(work)

            # 3. Box dimension change on a permitted glue-set line must fail.
            log_lines = (work / "trip.log").read_text().splitlines()
            log_lines[1404] = "\\vbox(16383.99998+0.0)x1000.1, glue set 16341.99998fil"
            (work / "trip.log").write_text("\n".join(log_lines) + "\n")
            with self.assertRaisesRegex(ValueError, "trip.log line 1405 box dimensions changed"):
                validate_allowances(work, rebuild_comparisons(work))
            results = make_valid_reference_capture(work)

            # 4. Out-of-bounds glue-set rounding or glue change on another line must fail.
            log_lines = (work / "trip.log").read_text().splitlines()
            log_lines[1404] = "\\vbox(16383.99998+0.0)x1000.0, glue set 16341.9fil"
            (work / "trip.log").write_text("\n".join(log_lines) + "\n")
            with self.assertRaisesRegex(ValueError, "trip.log line 1405 glue-set deviation out of bounds"):
                validate_allowances(work, rebuild_comparisons(work))
            results = make_valid_reference_capture(work)

            # 5. Memory usage statistic change must fail.
            log_lines = (work / "trip.log").read_text().splitlines()
            log_lines[7299] = " 2826 words of memory out of 3000"
            (work / "trip.log").write_text("\n".join(log_lines) + "\n")
            with self.assertRaisesRegex(ValueError, "trip.log line 7300 differs outside allowances"):
                validate_allowances(work, rebuild_comparisons(work))
            results = make_valid_reference_capture(work)

            # 6. Any DVI byte change outside the two glue-rounded vertical movements must fail.
            dvi = bytearray((work / "trip.dvi").read_bytes())
            dvi[100] ^= 1
            (work / "trip.dvi").write_bytes(bytes(dvi))
            with self.assertRaisesRegex(ValueError, "trip.dvi differs outside the two glue-rounded vertical movements"):
                validate_allowances(work, rebuild_comparisons(work))
            results = make_valid_reference_capture(work)

            # 7. Unrelated DVItype line change must fail.
            typ_lines = (work / "dvitype.stdout").read_text().splitlines()
            typ_lines[15] = "[X]"
            (work / "dvitype.stdout").write_text("\n".join(typ_lines) + "\n")
            with self.assertRaisesRegex(ValueError, "trip.typ line 16 differs outside allowances"):
                validate_allowances(work, rebuild_comparisons(work))


if __name__ == "__main__":
    unittest.main()

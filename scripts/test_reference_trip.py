#!/usr/bin/env python3
"""Exercise capture limits and raw-diff reporting without needing a TeX build."""
import os
from pathlib import Path
import sys
import tempfile
import unittest

from reference_trip import compare, run


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


if __name__ == "__main__":
    unittest.main()

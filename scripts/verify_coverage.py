#!/usr/bin/env python3
"""Validate coverage-ledger ranges against the pinned WEB, not implementation correctness."""
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
source = (ROOT / "vendor/knuth/tex.web").read_text()
sections = re.split(r"(?m)^@(?:\*|(?=\s))", source)[1:]
parts = []
for section, body in enumerate(sections, 1):
    match = re.match(r"\s*\\\[(\d+)\] ([^\n]+)", body)
    if match:
        parts.append((int(match[1]), section, match[2]))

rows = []
for line in (ROOT / "docs/coverage.md").read_text().splitlines():
    if re.match(r"\| \d+ \|", line):
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) != 5:
            raise SystemExit(f"invalid coverage row: {line}")
        rows.append(cells)
if len(rows) != len(parts):
    raise SystemExit(f"expected {len(parts)} WEB parts, found {len(rows)} coverage rows")
for index, ((part, start, title), row) in enumerate(zip(parts, rows)):
    end = parts[index + 1][1] - 1 if index + 1 < len(parts) else len(sections)
    if row[:3] != [str(part), f"{start}–{end}", title]:
        raise SystemExit(f"WEB part {part} coverage range/title mismatch: {row}")
    milestones = row[3].split(", ")
    if not milestones or any(m not in {f"M{i}" for i in range(1, 10)} for m in milestones):
        raise SystemExit(f"WEB part {part} has no valid milestone owner")
    if row[4] not in {"Not started", "Partial", "Implemented", "Non-runtime"}:
        raise SystemExit(f"WEB part {part} has an invalid status")
print(f"Coverage ledger accounts for {len(parts)} parts and {len(sections)} sections; statuses are declarations, not proof.")

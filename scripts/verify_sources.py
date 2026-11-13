#!/usr/bin/env python3
"""Check pinned bytes and WEB section anchors; this is not a TeX runner."""
import hashlib
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
manifest = ROOT / "vendor/SHA256SUMS"
for line in manifest.read_text().splitlines():
    expected, name = line.split("  ", 1)
    actual = hashlib.sha256((ROOT / name).read_bytes()).hexdigest()
    if actual != expected:
        raise SystemExit(f"checksum mismatch: {name}: {actual} != {expected}")

# In this pinned WEB every section delimiter starts a line. Escaped @@ is not
# a delimiter. Cross-checked against TANGLE's starred-section transcript.
parts = re.split(r"(?m)^@(?:\*|(?=\s))", (ROOT / "vendor/knuth/tex.web").read_text())[1:]
anchors = {
    99: "Arithmetic with scaled dimensions.",
    101: "@!scaled = integer",
    104: "@!arith_error:boolean",
    105: "function mult_and_add",
    106: "function x_over_n",
    107: "function xn_over_d",
    108: "function badness",
    109: "@!glue_ratio=real",
    110: "Packed data.",
    111: "mem_max>=max_halfword",
    113: "@!memory_word = record",
    115: "Dynamic memory allocation.",
    116: "lo_mem_max<",
    117: "@!var_used, @!dyn_used",
    118: "@d link(#) == mem[#].hh.rh",
    119: "Declare the procedure called |runaway|",
    120: "function get_avail",
    121: "@d free_avail",
    122: "@d fast_get_avail",
    123: "procedure flush_list",
    124: "@d llink(#)",
    125: "function get_node",
    130: "procedure free_node",
    131: "procedure sort_avail",
    133: "@d type(#)",
    150: "@d glue_spec_size=4",
    162: "Memory layout.",
    163: "@!k:integer",
    164: "rover:=lo_mem_stat_max+1",
    797: "link(end_span):=max_quarterword+1",
    820: "type(last_active):=hyphenated",
    981: "subtype(page_ins_head):=qi(255)",
}
for section, anchor in anchors.items():
    if anchor not in parts[section - 1]:
        raise SystemExit(f"WEB section {section} lost anchor: {anchor}")
print(f"Pinned files and {len(anchors)} anchors verified ({len(parts)} WEB sections).")

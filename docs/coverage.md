# Full WEB coverage ledger

Every part of the pinned source has a milestone owner. M1–M9 refer to
[the roadmap](roadmap.md). `Partial` is deliberately not a compatibility claim;
[the verified map](web-map.md) identifies the implemented sections and tests.
`Non-runtime` identifies the source index, not an omitted engine feature.

`python3 scripts/verify_coverage.py` checks that all ranges and titles match the
source and that no part is missing. It does not infer implementation coverage.
Changes to status require code, tests, and a corresponding verified-map update.

| Part | Sections | Source title | Milestone | Status |
|---|---|---|---|---|
| 1 | 1–16 | Introduction. | M3 | Implemented |
| 2 | 17–24 | The character set. | M3 | Implemented |
| 3 | 25–37 | Input and output. | M3 | Implemented |
| 4 | 38–53 | String handling. | M3 | Implemented |
| 5 | 54–71 | On-line and off-line printing. | M3 | Implemented |
| 6 | 72–98 | Reporting errors. | M3 | Implemented |
| 7 | 99–109 | Arithmetic with scaled dimensions. | M3, M5 | Partial |
| 8 | 110–114 | Packed data. | M5 | Partial |
| 9 | 115–132 | Dynamic memory allocation. | M2, M8 | Partial |
| 10 | 133–161 | Data structures for boxes and their friends. | M5 | Not started |
| 11 | 162–172 | Memory layout. | M5, M8 | Partial |
| 12 | 173–198 | Displaying boxes. | M5 | Not started |
| 13 | 199–202 | Destroying boxes. | M5 | Not started |
| 14 | 203–206 | Copying boxes. | M5 | Not started |
| 15 | 207–210 | The command codes. | M4 | Implemented |
| 16 | 211–219 | The semantic nest. | M4, M6 | Partial |
| 17 | 220–255 | The table of equivalents. | M4 | Partial |
| 18 | 256–267 | The hash table. | M4 | Implemented |
| 19 | 268–288 | Saving and restoring equivalents. | M4 | Partial |
| 20 | 289–296 | Token lists. | M4 | Implemented |
| 21 | 297–299 | Introduction to the syntactic routines. | M4 | Implemented |
| 22 | 300–320 | Input stacks and states. | M4 | Partial |
| 23 | 321–331 | Maintaining the input stacks. | M4 | Partial |
| 24 | 332–365 | Getting the next token. | M4 | Partial |
| 25 | 366–401 | Expanding the next token. | M4 | Implemented |
| 26 | 402–463 | Basic scanning subroutines. | M4 | Implemented |
| 27 | 464–486 | Building token lists. | M4 | Implemented |
| 28 | 487–510 | Conditional processing. | M4 | Implemented |
| 29 | 511–538 | File names. | M3 | Partial |
| 30 | 539–582 | Font metric data. | M5 | Not started |
| 31 | 583–591 | Device-independent file format. | M5 | Not started |
| 32 | 592–643 | Shipping pages out. | M5 | Not started |
| 33 | 644–679 | Packaging. | M5 | Not started |
| 34 | 680–698 | Data structures for math mode. | M7 | Not started |
| 35 | 699–718 | Subroutines for math mode. | M7 | Not started |
| 36 | 719–767 | Typesetting math formulas. | M7 | Not started |
| 37 | 768–812 | Alignment. | M7 | Not started |
| 38 | 813–861 | Breaking paragraphs into lines. | M6 | Not started |
| 39 | 862–890 | Breaking paragraphs into lines, continued. | M6 | Not started |
| 40 | 891–899 | Pre-hyphenation. | M6 | Not started |
| 41 | 900–918 | Post-hyphenation. | M6 | Not started |
| 42 | 919–941 | Hyphenation. | M6 | Not started |
| 43 | 942–966 | Initializing the hyphenation tables. | M6, M8 | Not started |
| 44 | 967–979 | Breaking vertical lists into pages. | M6 | Not started |
| 45 | 980–1028 | The page builder. | M6 | Not started |
| 46 | 1029–1054 | The chief executive. | M5, M6, M7 | Not started |
| 47 | 1055–1135 | Building boxes and lists. | M5, M6 | Not started |
| 48 | 1136–1207 | Building math lists. | M7 | Not started |
| 49 | 1208–1298 | Mode-independent processing. | M4, M7 | Partial |
| 50 | 1299–1329 | Dumping and undumping the tables. | M8 | Not started |
| 51 | 1330–1337 | The main program. | M8 | Not started |
| 52 | 1338–1339 | Debugging. | M9 | Not started |
| 53 | 1340–1378 | Extensions. | M7 | Not started |
| 54 | 1379–1379 | System-dependent changes. | M3, M9 | Partial |
| 55 | 1380–1380 | Index. | M9 | Non-runtime |

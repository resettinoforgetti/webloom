# Verified WEB map

Numbers refer only to the pinned `vendor/knuth/tex.web`, not a change-file output.
Run `python3 scripts/verify_sources.py` to verify hashes and section anchors.
For unnumbered WEB prose sections the table uses their subject, not an invented
formal title. Starred part titles and routine identifiers are taken from source.

| Sections | Source title / subject | Implementation and verification | Status |
|---|---|---|---|
| 1–16 | Introduction; banner; capacity constants; `bad` consistency checks | `io::BANNER`, `IoConfig::check_constants`, `MemoryConfig`; `io_strings_errors.rs` and memory config tests | Implemented |
| 17–24 | The character set; `ASCII_code`; `xord` and `xchr` | `io::CharTranslator` (pristine and 8-bit identity); `io_strings_errors.rs` | Implemented |
| 25–37 | Input and output; `alpha_file`/`byte_file`; `input_ln`; `init_terminal` | `AlphaFileIn`, `ByteFileIn`, `HostFixture`, `IoState::input_ln`, `IoState::init_terminal`; `io_strings_errors.rs` | Implemented |
| 38–53 | String handling; `make_string`; `str_eq_buf`; `str_eq_str`; `get_strings_started` | `IoState` string pool, printable ASCII check (§49), `TEX.POOL` loader and `@$` checksum check; `io_strings_errors.rs` | Implemented |
| 54–71 | On-line and off-line printing; selectors; `print_ln`/`print_char`/`print`/`slow_print`/`print_int`/`print_hex`/`print_roman_int`/`term_input` | `Selector`, `IoState` printing and terminal input routines; `io_strings_errors.rs` | Implemented |
| 72–98 | Reporting errors; `interaction`; `history`; `error`; `int_error`; `normalize_selector`; `fatal_error`; `overflow`; `confusion`; `pause_for_instructions` | `Interaction`, `History`, `ErrorHooks`, `IoState` error and interrupt routines; `io_strings_errors.rs` | Implemented (token/context callbacks via `ErrorHooks`) |
| 99–108 | Arithmetic with scaled dimensions; `half`; `Scaled`; `round_decimals`; `print_scaled`; `mult_and_add`; `x_over_n`; `xn_over_d`; `badness` | `arithmetic.rs` and `IoState::print_scaled`; `arithmetic.rs` tests including all-65,537 fraction round-trip | Implemented (`§§99–108`) |
| 109 | Glue ratios | Discussed in `semantics.md` | Unimplemented (`M5`) |
| 110–113 | Packed data; range checks; conversions; variant records | `memory::Word`; `overlapping_views_have_one_backing_value` | Integer/half/quarter subset; `glue_ratio` absent |
| 115–118 | Dynamic memory allocation; regions; statistics; `avail`/`link`/`info` | `Memory`, `MemPtr`, `MemoryStats`; mixed sequence tests | Implemented allocation subset |
| 119 | Runaway diagnostic dependencies | Explicit terminal `MemoryError` | Token-list runaway text deferred to `M4` |
| 120 | `get_avail` | `Memory::get_avail`; frontier order/exhaustion tests | Implemented |
| 121–122 | `free_avail` / `fast_get_avail` | No dedicated API; `flush_list` handles singleton reclamation | Unimplemented macros, no stubs |
| 123 | `flush_list` | `Memory::flush_list`; order, null, cycles, double reclaim tests | Implemented with precondition validation |
| 124–130 | Variable-size free ring; `get_node`; `free_node` | `memory.rs`; `variable_memory.rs` and independent interval-model tests | Implemented, including merge-only sentinel |
| 133,150 | `type`/`subtype`; glue specification layout | `Memory::initialize`; static glue tests | Layout dependencies only |
| 162–164 | Memory layout; initialization variable; table entries | `Memory::initialize`; `initialization_matches_static_layout` | Memory subset, not full INITEX |
| 241–246 | `fix_date_and_time`; `escape_char`/`new_line_char`; `begin_diagnostic`/`end_diagnostic` | `Clock`, `IoState::fix_date_and_time`, `begin_diagnostic`, `end_diagnostic`; `io_strings_errors.rs` | Implemented I/O subset |
| 300–302,328–329 | `in_state_record`; `begin_file_reading`; `end_file_reading` | `InStateRecord`, `IoState::begin_file_reading_default`, `end_file_reading_default` | File-input subset; token lists in `M4` |
| 511–525,527–538 | File names; `begin_name`/`more_name`/`end_name`; `pack_file_name`; `pack_buffered_name`; `open_fmt_file`; `make_name_string`; `prompt_file_name`; `open_log_file`; `start_input_after_scan` | `IoState` file-name, log-banner, and file-lookup routines; `io_strings_errors.rs` | Implemented (`§526` `scan_file_name` token loop in `M4`) |
| 797,820,981 | `end_span`, `active`, `page_ins_head` initialization | `Memory::initialize`; reserved word assertions | Initialization only |

All of `§§99–108` are now implemented, including `half` (§100), `round_decimals`
(§102), and `print_scaled` (§103) verified by exact round-trip across all 65,537
scaled fractions in `0..=unity`. Section 109 (`glue_ratio`) and surrounding box,
alignment, paragraph, or page algorithms belong to `M5–M7`.

Variable allocation acceptance covers exact rover movement, coalescing,
splitting, whole-block removal, `get_node(2^30)` merge-only behavior, `free_node`
reinsertion order, low-frontier growth, mixed allocations, and exhaustion.
The independent interval model also checks ring links, allocation-kind metadata,
and the complete low-region partition. §126 retains division of the nonnegative
frontier difference. §§131–132 `sort_avail` remain a format-preparation dependency.

Next engine slice: commands, equivalents/hash/save tables, tokens/input stacks,
scanners, expansion, and conditionals (`M4`) in [the roadmap](roadmap.md). The
full [coverage ledger](coverage.md) assigns every source part to a milestone.

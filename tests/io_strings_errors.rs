use webloom::io::{
    AlphaFileIn, BANNER, CARRIAGE_RETURN, CharTranslator, Clock, ErrorHooks, History, HostFixture,
    Interaction, IoConfig, IoState, POOL_NAME, Selector, StrNum, Termination, is_printable_ascii,
};

fn booted_io(terminal_input: &[u8]) -> IoState {
    let host = HostFixture::with_pinned_pool(terminal_input);
    let mut io = IoState::new(IoConfig::default(), host).unwrap();
    assert!(io.get_strings_started().unwrap());
    io.fix_date_and_time();
    io.history = History::Spotless;
    io
}

fn append_ascii_str(io: &mut IoState, text: &[u8]) -> StrNum {
    io.str_room(text.len()).unwrap();
    for &b in text {
        io.append_char(b);
    }
    io.make_string().unwrap()
}

#[test]
fn constant_checks_match_web_bad_cases_and_accept_default_and_trip_profiles() {
    assert_eq!(IoConfig::default().check_constants(), Ok(()));
    assert_eq!(IoConfig::trip().check_constants(), Ok(()));

    let base = IoConfig::default();
    assert_eq!(
        IoConfig {
            half_error_line: 29,
            ..base
        }
        .check_constants(),
        Err(1)
    );
    assert_eq!(
        IoConfig {
            error_line: 50,
            half_error_line: 36,
            ..base
        }
        .check_constants(),
        Err(1)
    );
    assert_eq!(
        IoConfig {
            max_print_line: 59,
            ..base
        }
        .check_constants(),
        Err(2)
    );
    assert_eq!(
        IoConfig {
            dvi_buf_size: 804,
            ..base
        }
        .check_constants(),
        Err(3)
    );
    assert_eq!(
        IoConfig {
            mem_bot: 1,
            mem_top: 1100,
            ..base
        }
        .check_constants(),
        Err(4)
    );
    assert_eq!(
        IoConfig {
            hash_size: 1700,
            hash_prime: 1777,
            ..base
        }
        .check_constants(),
        Err(5)
    );
    assert_eq!(
        IoConfig {
            max_in_open: 128,
            ..base
        }
        .check_constants(),
        Err(6)
    );
    assert_eq!(
        IoConfig {
            mem_bot: 0,
            mem_top: 200,
            ..base
        }
        .check_constants(),
        Err(7)
    );
    assert_eq!(
        IoConfig {
            max_strings: 65536,
            ..base
        }
        .check_constants(),
        Err(17)
    );
    assert_eq!(
        IoConfig {
            buf_size: 65536,
            ..base
        }
        .check_constants(),
        Err(18)
    );
    assert_eq!(
        IoConfig {
            file_name_size: 19,
            ..base
        }
        .check_constants(),
        Err(31)
    );
}

#[test]
fn character_translation_and_first_256_strings_follow_pristine_web() {
    let pristine = CharTranslator::pristine();
    for b in 0..=255_u8 {
        if (32..=126).contains(&b) {
            assert!(is_printable_ascii(b));
            assert_eq!(pristine.xchr(b), b);
            assert_eq!(pristine.xord(b), b);
        } else {
            assert!(!is_printable_ascii(b));
            assert_eq!(pristine.xchr(b), b' ');
            assert_eq!(pristine.xord(b), 127);
        }
    }
    // §24 guarantees that ' ' maps back to 32, not to a control or high byte.
    assert_eq!(pristine.xord(b' '), 32);

    let identity = CharTranslator::identity_eight_bit();
    for b in 0..=255_u8 {
        assert_eq!(identity.xchr(b), b);
        assert_eq!(identity.xord(b), b);
    }

    let io = booted_io(&[]);
    assert_eq!(io.str_bytes(StrNum::from_ascii(0)), b"^^@");
    assert_eq!(io.str_bytes(StrNum::from_ascii(CARRIAGE_RETURN)), b"^^M");
    assert_eq!(io.str_bytes(StrNum::from_ascii(31)), b"^^_");
    assert_eq!(io.str_bytes(StrNum::from_ascii(b' ')), b" ");
    assert_eq!(io.str_bytes(StrNum::from_ascii(b'~')), b"~");
    assert_eq!(io.str_bytes(StrNum::from_ascii(127)), b"^^?");
    assert_eq!(io.str_bytes(StrNum::from_ascii(128)), b"^^80");
    assert_eq!(io.str_bytes(StrNum::from_ascii(255)), b"^^ff");
}

#[test]
fn string_pool_loads_pinned_tex_pool_and_validates_all_pool_failure_modes() {
    let mut io = booted_io(&[]);
    assert_eq!(io.str_ptr, 1301);
    assert_eq!(io.init_str_ptr, 1301);
    assert_eq!(io.init_pool_ptr, io.pool_ptr);
    assert_eq!(io.str_bytes(StrNum::BUFFER_SIZE), b"buffer size");
    assert_eq!(io.str_bytes(StrNum::ROMAN_SPEC), b"m2d5c2l5x2v5i");
    assert_eq!(io.str_bytes(StrNum::EMPTY), b"");
    assert_eq!(io.str_bytes(StrNum::TEXPUT), b"texput");
    assert_eq!(io.str_bytes(StrNum::INITEX_IDENT), b" (INITEX)");

    // Compare strings and buffer slices.
    let s1 = append_ascii_str(&mut io, b"hello");
    let s2 = append_ascii_str(&mut io, b"hello");
    let s3 = append_ascii_str(&mut io, b"world");
    assert!(io.str_eq_str(s1, s2));
    assert!(!io.str_eq_str(s1, s3));
    io.buffer[10..15].copy_from_slice(b"hello");
    assert!(io.str_eq_buf(s1, 10));
    assert!(!io.str_eq_buf(s3, 10));
    io.flush_string();
    assert_eq!(io.str_ptr, usize::from(s3.0));

    // Short pool lines are padded with spaces per §52.
    let mut host = HostFixture::default();
    host.insert_file(POOL_NAME.trim_end(), b"05abc\n*504454778\n".as_slice());
    let mut short_io = IoState::new(IoConfig::default(), host).unwrap();
    assert!(short_io.get_strings_started().unwrap());
    assert_eq!(short_io.str_bytes(StrNum(256)), b"abc  ");

    for (pool_bytes, config, expected_err) in [
        (None, IoConfig::default(), "! I can't read TEX.POOL.\n"),
        (
            Some(b"02hi\n".as_slice()),
            IoConfig::default(),
            "! TEX.POOL has no check sum.\n",
        ),
        (
            Some(b"xxhi\n*504454778\n".as_slice()),
            IoConfig::default(),
            "! TEX.POOL line doesn't begin with two digits.\n",
        ),
        (
            Some(b"05hello\n*504454778\n".as_slice()),
            IoConfig {
                pool_size: 800,
                string_vacancies: 100,
                ..IoConfig::default()
            },
            "! You have to increase POOLSIZE.\n",
        ),
        (
            Some(b"*50445477x\n".as_slice()),
            IoConfig::default(),
            "! TEX.POOL check sum doesn't have nine digits.\n",
        ),
        (
            Some(b"*123456789\n".as_slice()),
            IoConfig::default(),
            "! TEX.POOL doesn't match; TANGLE me again.\n",
        ),
    ] {
        let mut h = HostFixture::default();
        if let Some(bytes) = pool_bytes {
            h.insert_file(POOL_NAME.trim_end(), bytes);
        }
        let mut bad_io = IoState::new(config, h).unwrap();
        assert!(!bad_io.get_strings_started().unwrap());
        assert_eq!(String::from_utf8_lossy(&bad_io.host.term_out), expected_err);
    }
}

#[test]
fn input_ln_and_init_terminal_strip_trailing_blanks_and_enforce_buffer_limits() {
    let mut io = booted_io(b"   \n  \\input paper   \n");
    assert!(io.init_terminal().unwrap());
    assert_eq!(
        String::from_utf8_lossy(&io.host.term_out),
        "**Please type the name of your input file.\n**"
    );
    assert_eq!(io.cur_input.loc_field, 3);
    assert_eq!(
        &io.buffer[io.cur_input.loc_field..io.last],
        b"\\input paper"
    );

    // EOF on terminal during init_terminal.
    let mut eof_io = booted_io(b"");
    assert!(!eof_io.init_terminal().unwrap());
    assert!(
        String::from_utf8_lossy(&eof_io.host.term_out)
            .ends_with("! End of file on the terminal... why?")
    );

    // Multi-line file with trailing spaces and final line without trailing LF (§30).
    let mut f = AlphaFileIn::new_file(b"abc   \ndef".to_vec());
    io.first = 1;
    assert!(io.input_ln(&mut f, false).unwrap());
    assert_eq!(&io.buffer[1..io.last], b"abc");
    assert!(io.input_ln(&mut f, true).unwrap());
    assert_eq!(&io.buffer[1..io.last], b"def");
    assert!(!io.input_ln(&mut f, true).unwrap());
    assert_eq!(io.last, io.first);

    // Buffer overflow before format_ident is set -> FinalEnd; after -> JumpOut with overflow message.
    let small_cfg = IoConfig {
        buf_size: 6,
        ..IoConfig::default()
    };
    let mut pre_fmt = IoState::new(small_cfg, HostFixture::default()).unwrap();
    let mut long_file = AlphaFileIn::new_file(b"1234567\n".to_vec());
    assert_eq!(
        pre_fmt.input_ln(&mut long_file, false),
        Err(Termination::FinalEnd)
    );
    assert_eq!(
        String::from_utf8_lossy(&pre_fmt.host.term_out),
        "Buffer size exceeded!\n"
    );

    let mut post_fmt = IoState::new(small_cfg, HostFixture::with_pinned_pool(&[])).unwrap();
    assert!(post_fmt.get_strings_started().unwrap());
    post_fmt.format_ident = StrNum::INITEX_IDENT;
    let mut long_file2 = AlphaFileIn::new_file(b"1234567\n".to_vec());
    assert_eq!(
        post_fmt.input_ln(&mut long_file2, false),
        Err(Termination::JumpOut)
    );
    assert!(
        String::from_utf8_lossy(&post_fmt.host.term_out)
            .contains("! TeX capacity exceeded, sorry [buffer size=6].")
    );
}

#[test]
fn printing_selectors_wrapping_escaped_strings_and_numerals_match_web() {
    let mut io = booted_io(&[]);
    io.config.max_print_line = 60;
    io.selector = Selector::TermAndLog;

    for _ in 0..60 {
        io.print_char(b'a');
    }
    assert_eq!(io.term_offset, 0);
    assert_eq!(io.file_offset, 0);
    assert_eq!(io.tally, 60);
    assert_eq!(io.host.term_out.len(), 61);
    assert_eq!(io.host.term_out[60], b'\n');
    assert_eq!(io.host.log_file, io.host.term_out);

    // Pseudo-printing into cyclic trick_buf (§58).
    io.host.term_out.clear();
    io.selector = Selector::Pseudo;
    io.tally = 0;
    io.trick_count = 5;
    for b in b"abcdefg" {
        io.print_char(*b);
    }
    assert_eq!(&io.trick_buf[..5], b"abcde");
    assert_eq!(io.tally, 7);

    // NewString selector appends raw character bytes without expanding ^^ (§59).
    io.selector = Selector::NewString;
    io.print(0);
    io.print(i32::from(b'Z'));
    let raw_s = io.make_string().unwrap();
    assert_eq!(io.str_bytes(raw_s), &[0, b'Z']);

    // slow_print expands non-printable bytes inside pooled strings >= 256, whereas print does not.
    io.selector = Selector::TermOnly;
    io.host.term_out.clear();
    io.term_offset = 0;
    io.slow_print(i32::from(raw_s.0));
    assert_eq!(String::from_utf8_lossy(&io.host.term_out), "^^@Z");

    // Out-of-range string numbers print "???".
    io.host.term_out.clear();
    io.term_offset = 0;
    io.print(-1);
    io.print(99999);
    assert_eq!(String::from_utf8_lossy(&io.host.term_out), "??????");

    // print_int, print_two, print_hex, print_roman_int, print_esc.
    for (n, expected) in [
        (0, "0"),
        (42, "42"),
        (-42, "-42"),
        (-100_000_000, "-100000000"),
        (-100_000_001, "-100000001"),
        (-2_147_483_647, "-2147483647"),
        (i32::MIN, "-2147483648"),
        (i32::MAX, "2147483647"),
    ] {
        io.host.term_out.clear();
        io.term_offset = 0;
        io.print_int(n);
        assert_eq!(String::from_utf8_lossy(&io.host.term_out), expected);
    }

    io.host.term_out.clear();
    io.term_offset = 0;
    io.print_two(7);
    io.print_char(b':');
    io.print_two(125);
    io.print_char(b' ');
    io.print_hex(0);
    io.print_char(b' ');
    io.print_hex(0x1A2B);
    assert_eq!(
        String::from_utf8_lossy(&io.host.term_out),
        "07:25 \"0 \"1A2B"
    );

    for (n, roman) in [
        (0, ""),
        (-5, ""),
        (1, "i"),
        (4, "iv"),
        (9, "ix"),
        (49, "xlix"),
        (1776, "mdcclxxvi"),
        (1990, "mcmxc"),
        (2026, "mmxxvi"),
    ] {
        io.host.term_out.clear();
        io.term_offset = 0;
        io.print_roman_int(n);
        assert_eq!(String::from_utf8_lossy(&io.host.term_out), roman);
    }

    io.host.term_out.clear();
    io.term_offset = 0;
    io.escape_char = i32::from(b'\\');
    io.print_esc(StrNum::BATCHMODE);
    io.escape_char = -1;
    io.print_esc(StrNum::BATCHMODE);
    assert_eq!(
        String::from_utf8_lossy(&io.host.term_out),
        "\\batchmodebatchmode"
    );

    // begin_diagnostic and end_diagnostic (§245).
    io.selector = Selector::TermAndLog;
    io.tracing_online = 0;
    io.history = History::Spotless;
    io.begin_diagnostic();
    assert_eq!(io.selector, Selector::LogOnly);
    assert_eq!(io.history, History::WarningIssued);
    io.end_diagnostic(true);
    assert_eq!(io.selector, Selector::TermAndLog);
}

struct CountingHooks {
    deleted: usize,
    context_shown: usize,
    custom_help_shown: usize,
}

impl ErrorHooks for CountingHooks {
    fn show_context(&mut self, _io: &mut IoState) {
        self.context_shown += 1;
    }
    fn get_token(&mut self, _io: &mut IoState) -> Result<(), Termination> {
        self.deleted += 1;
        Ok(())
    }
    fn give_err_help(&mut self, io: &mut IoState) {
        self.custom_help_shown += 1;
        io.print_str(StrNum::ROMAN_SPEC);
        io.print_ln();
    }
}

#[test]
fn interactive_and_scrolled_error_recovery_cover_all_web_branches() {
    // Sequence: '?' (menu), '12' (delete 12 tokens), 'H' (help after deletion), 'H' (already helped), 'S' (scrollmode).
    let mut io = booted_io(b"?\n12\nh\nh\ns\n");
    io.open_log_file().unwrap();
    let mut hooks = CountingHooks {
        deleted: 0,
        context_shown: 0,
        custom_help_shown: 0,
    };
    io.print_err(StrNum::BUFFER_SIZE);
    io.error_with_hooks(&mut hooks).unwrap();
    assert_eq!(hooks.deleted, 12);
    assert_eq!(hooks.context_shown, 2);
    assert_eq!(io.interaction, Interaction::ScrollMode);
    let term = String::from_utf8_lossy(&io.host.term_out);
    assert!(term.contains("Type <return> to proceed"));
    assert!(term.contains("I have just deleted some text, as you asked."));
    assert!(term.contains("Sorry, I already gave what help I could..."));
    assert!(term.contains("OK, entering \\scrollmode..."));

    // 'I' inline insertion and prompted insertion.
    let mut ins_io = booted_io(b"I\\relax\nI\n\\hbox\n");
    ins_io.print_err(StrNum::BUFFER_SIZE);
    ins_io.error().unwrap();
    assert_eq!(
        &ins_io.buffer[ins_io.cur_input.loc_field..=ins_io.cur_input.limit_field],
        b"\\relax"
    );
    ins_io.print_err(StrNum::BUFFER_SIZE);
    ins_io.error().unwrap();
    assert_eq!(
        &ins_io.buffer[ins_io.cur_input.loc_field..=ins_io.cur_input.limit_field],
        b"\\hbox"
    );

    // 'E' option when base_ptr > 0 and input_stack[base_ptr].name_field >= 256.
    let mut edit_io = booted_io(b"e\n");
    let file_str = append_ascii_str(&mut edit_io, b"story.tex");
    edit_io.base_ptr = 1;
    edit_io.input_stack[1].name_field = file_str;
    edit_io.line = 17;
    edit_io.print_err(StrNum::BUFFER_SIZE);
    assert_eq!(edit_io.error(), Err(Termination::JumpOut));
    assert!(
        String::from_utf8_lossy(&edit_io.host.term_out)
            .contains("You want to edit file story.tex at line 17")
    );

    // 100 scrolled errors trigger fatal_error_stop.
    let mut scroll_io = booted_io(&[]);
    scroll_io.interaction = Interaction::NonstopMode;
    for _ in 0..99 {
        scroll_io.print_err(StrNum::BUFFER_SIZE);
        scroll_io.error().unwrap();
    }
    scroll_io.print_err(StrNum::BUFFER_SIZE);
    assert_eq!(scroll_io.error(), Err(Termination::JumpOut));
    assert_eq!(scroll_io.history, History::FatalErrorStop);
    assert!(
        String::from_utf8_lossy(&scroll_io.host.term_out)
            .contains("(That makes 100 errors; please try again.)")
    );

    // confusion before vs after an error message, and pause_for_instructions.
    let mut conf1 = booted_io(&[]);
    assert_eq!(conf1.confusion(StrNum::BUFFER_SIZE), Termination::JumpOut);
    assert!(
        String::from_utf8_lossy(&conf1.host.term_out)
            .contains("! This can't happen (buffer size).")
    );

    let mut conf2 = booted_io(&[]);
    conf2.history = History::ErrorMessageIssued;
    assert_eq!(conf2.confusion(StrNum::BUFFER_SIZE), Termination::JumpOut);
    assert!(
        String::from_utf8_lossy(&conf2.host.term_out)
            .contains("! I can't go on meeting you like this.")
    );

    let mut intr_io = booted_io(b"\n");
    intr_io.interrupt = 1;
    intr_io.check_interrupt(&mut hooks).unwrap();
    assert_eq!(intr_io.interrupt, 0);
    assert!(String::from_utf8_lossy(&intr_io.host.term_out).contains("! Interruption."));
}

#[test]
fn file_name_scanning_log_banner_and_input_lookup_match_part_29() {
    let mut io = booted_io(b"fallback\n");
    io.host.clock = Clock {
        sys_time: 12 * 60,
        sys_day: 4,
        sys_month: 7,
        sys_year: 1776,
    };
    io.fix_date_and_time();
    io.format_ident = StrNum::INITEX_IDENT;

    // Scan "area.1:sub>job.tex extra" -> area="area.1:sub>", name="job", ext=".tex".
    io.begin_name();
    for &b in b"area.1:sub>job.tex extra" {
        if !io.more_name(b).unwrap() {
            break;
        }
    }
    io.end_name().unwrap();
    assert_eq!(io.str_bytes(io.cur_area), b"area.1:sub>");
    assert_eq!(io.str_bytes(io.cur_name), b"job");
    assert_eq!(io.str_bytes(io.cur_ext), b".tex");
    io.pack_cur_name();
    assert_eq!(io.packed_file_name_bytes(), b"area.1:sub>job.tex");

    // open_log_file with first input line in buffer[1..=limit].
    io.buffer[1..=11].copy_from_slice(b"\\input trip");
    io.buffer[12] = CARRIAGE_RETURN;
    io.cur_input.limit_field = 12;
    io.open_log_file().unwrap();
    assert_eq!(io.str_bytes(io.log_name), b"texput.log");
    let log = String::from_utf8_lossy(&io.host.log_file);
    let expected_header = format!("{BANNER} (INITEX)  4 JUL 1776 12:00\n**\\input trip\n");
    assert_eq!(log, expected_header);

    // start_input_after_scan falls back to TeXinputs:, then prompts on missing file.
    io.host
        .insert_file("TeXinputs:fallback.tex", b"Hello TeX\n");
    io.cur_area = StrNum::EMPTY;
    io.cur_name = append_ascii_str(&mut io, b"missing");
    io.cur_ext = StrNum::EMPTY;
    let mut hooks = CountingHooks {
        deleted: 0,
        context_shown: 0,
        custom_help_shown: 0,
    };
    io.start_input_after_scan(&mut hooks).unwrap();
    assert_eq!(hooks.context_shown, 1);
    assert_eq!(io.line, 1);
    assert_eq!(
        &io.buffer[io.cur_input.loc_field..io.cur_input.limit_field],
        b"Hello TeX"
    );
    assert_eq!(io.buffer[io.cur_input.limit_field], CARRIAGE_RETURN);
}

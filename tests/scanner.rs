use webloom::Engine;
use webloom::io::{HostFixture, Interaction, IoConfig, NoopErrorHooks, Selector};
use webloom::memory::MemoryConfig;
use webloom::scanner::{
    EVERY_PAR_TEXT, MACRO, MID_LINE, NO_EXPAND_FLAG, SKIP_BLANKS, ScannerStatus,
};
use webloom::tables::{
    END_LINE_CHAR_CODE, TRACING_MACROS_CODE, TRACING_ONLINE_CODE, Token, cmd, store_new_token,
};

fn make_engine() -> Engine {
    let mut engine = Engine::new(MemoryConfig::default()).expect("engine creation must succeed");
    engine.init_prim().expect("init_prim must succeed");
    engine.io.selector = Selector::TermAndLog;
    engine
}

fn take_term_output(engine: &mut Engine) -> String {
    let out = String::from_utf8(engine.io.host.term_out.clone()).expect("UTF-8 terminal output");
    engine.io.host.term_out.clear();
    engine.io.term_offset = 0;
    out
}

#[test]
fn get_next_state_machine_spaces_comments_braces_and_par() {
    let mut engine = make_engine();

    // Blank line emits \par
    engine.set_line_bytes(b"   ");
    let tok = engine.next_token().unwrap();
    assert_eq!(tok, engine.hash.par_token);
    assert_eq!(engine.io.cur_cmd, cmd::PAR_END);

    // Comment at start of line skips the whole line; let's test mid-line comment and spaces
    engine.set_line_bytes(b"  a  b% comment here");
    // 'a' (leading spaces skipped in NEW_LINE)
    let t1 = engine.next_token().unwrap();
    assert_eq!(t1, Token::from_cmd_chr(cmd::LETTER, b'a'));
    assert_eq!(engine.io.cur_input.state_field, MID_LINE);
    // ' ' (first space in MID_LINE emits SPACER and enters SKIP_BLANKS; second space ignored)
    let t2 = engine.next_token().unwrap();
    assert_eq!(t2, Token::from_cmd_chr(cmd::SPACER, b' '));
    assert_eq!(engine.io.cur_input.state_field, SKIP_BLANKS);
    // 'b'
    let t3 = engine.next_token().unwrap();
    assert_eq!(t3, Token::from_cmd_chr(cmd::LETTER, b'b'));
    // '%' comment skips to end of line without emitting end-of-line space!
    assert!(engine.io.cur_input.loc_field <= engine.io.cur_input.limit_field);

    // Braces update align_state, and end-of-line CAR_RET in MID_LINE emits SPACER(' ')
    engine.eqtb.set_cat_code(b'{', u16::from(cmd::LEFT_BRACE));
    engine.eqtb.set_cat_code(b'}', u16::from(cmd::RIGHT_BRACE));
    let align0 = engine.io.align_state;
    engine.set_line_bytes(b"{x}");
    assert_eq!(
        engine.next_token().unwrap(),
        Token::from_cmd_chr(cmd::LEFT_BRACE, b'{')
    );
    assert_eq!(engine.io.align_state, align0 + 1);
    assert_eq!(
        engine.next_token().unwrap(),
        Token::from_cmd_chr(cmd::LETTER, b'x')
    );
    assert_eq!(
        engine.next_token().unwrap(),
        Token::from_cmd_chr(cmd::RIGHT_BRACE, b'}')
    );
    assert_eq!(engine.io.align_state, align0);
    // End of line in MID_LINE emits space
    assert_eq!(
        engine.next_token().unwrap(),
        Token::from_cmd_chr(cmd::SPACER, b' ')
    );
}

#[test]
fn control_sequences_and_expanded_hat_characters() {
    let mut engine = make_engine();
    engine.eqtb.set_cat_code(b'^', u16::from(cmd::SUP_MARK));

    // Multiletter CS ignores following spaces; single non-letter CS preserves following space
    engine.set_line_bytes(b"\\relax   \\{ x");
    let t_relax = engine.next_token().unwrap();
    assert!(t_relax.is_cs());
    assert_eq!(engine.io.cur_cmd, cmd::RELAX);

    let t_lbrace_cs = engine.next_token().unwrap();
    assert_eq!(
        t_lbrace_cs,
        Token::from_cs((engine.eqtb.layout.single_base + usize::from(b'{')) as u16)
    );
    // Space after \{ is NOT skipped because \{ sets state = MID_LINE
    assert_eq!(
        engine.next_token().unwrap(),
        Token::from_cmd_chr(cmd::SPACER, b' ')
    );
    assert_eq!(
        engine.next_token().unwrap(),
        Token::from_cmd_chr(cmd::LETTER, b'x')
    );

    // ^^ hex and ASCII-64 decoding both in text and inside/after control sequence names (§§352, 355)
    // \^^72el^^61x -> \relax, followed by ^^41 ('A') and ^^8 ('x')
    engine.set_line_bytes(b"\\^^72el^^61x ^^41^^8");
    let t_relax2 = engine.next_token().unwrap();
    assert_eq!(t_relax2, t_relax);
    assert_eq!(
        engine.next_token().unwrap(),
        Token::from_cmd_chr(cmd::LETTER, b'A')
    );
    assert_eq!(
        engine.next_token().unwrap(),
        Token::from_cmd_chr(cmd::LETTER, b'x')
    );

    // Trailing \ at end of line with end_line_char = -1 yields null_cs
    engine.eqtb.set_int_par(END_LINE_CHAR_CODE, -1);
    engine.set_line_bytes(b"\\");
    let t_null = engine.next_token().unwrap();
    assert_eq!(t_null, Token::from_cs(engine.eqtb.layout.null_cs as u16));
    engine.eqtb.set_int_par(END_LINE_CHAR_CODE, 13);
}

#[test]
fn token_list_expansion_parameters_back_input_and_dont_expand() {
    let mut engine = make_engine();
    let mut hooks = NoopErrorHooks;

    // Build a parameter token list: "XY"
    let param_x = engine.memory.get_avail().unwrap();
    engine
        .memory
        .set_info(param_x, Token::from_cmd_chr(cmd::LETTER, b'X').raw())
        .unwrap();
    let _ = store_new_token(
        &mut engine.io,
        &mut engine.memory,
        param_x,
        Token::from_cmd_chr(cmd::LETTER, b'Y'),
        &mut hooks,
    )
    .unwrap();
    engine.scanner.param_stack[0] = param_x;
    engine.scanner.param_ptr = 1;

    // Build a macro body starting with reference count node: [ref=0] -> 'A' -> #1 -> 'B'
    let macro_head = engine.memory.get_avail().unwrap();
    let mut p = macro_head;
    p = store_new_token(
        &mut engine.io,
        &mut engine.memory,
        p,
        Token::from_cmd_chr(cmd::LETTER, b'A'),
        &mut hooks,
    )
    .unwrap();
    p = store_new_token(
        &mut engine.io,
        &mut engine.memory,
        p,
        Token::from_cmd_chr(cmd::OUT_PARAM, 1),
        &mut hooks,
    )
    .unwrap();
    let _ = store_new_token(
        &mut engine.io,
        &mut engine.memory,
        p,
        Token::from_cmd_chr(cmd::LETTER, b'B'),
        &mut hooks,
    )
    .unwrap();

    engine.set_line_bytes(b"Z");
    engine
        .scanner
        .begin_token_list(
            &mut engine.io,
            &mut engine.eqtb,
            &mut engine.hash,
            &mut engine.memory,
            macro_head,
            MACRO,
        )
        .unwrap();
    // For MACRO, caller sets limit_field (param_start) and loc_field = link(macro_head)
    engine.io.cur_input.limit_field = 0;
    engine.io.cur_input.loc_field = usize::from(engine.memory.link(macro_head).unwrap().address());

    assert_eq!(
        engine.next_token().unwrap(),
        Token::from_cmd_chr(cmd::LETTER, b'A')
    );
    assert_eq!(
        engine.next_token().unwrap(),
        Token::from_cmd_chr(cmd::LETTER, b'X')
    );
    assert_eq!(
        engine.next_token().unwrap(),
        Token::from_cmd_chr(cmd::LETTER, b'Y')
    );
    assert_eq!(
        engine.next_token().unwrap(),
        Token::from_cmd_chr(cmd::LETTER, b'B')
    );
    // Next token pops the finished macro (and flushes param_stack[0]) and reads 'Z' from the line!
    assert_eq!(
        engine.next_token().unwrap(),
        Token::from_cmd_chr(cmd::LETTER, b'Z')
    );
    assert_eq!(engine.scanner.param_ptr, 0);
    engine.memory.delete_token_ref(macro_head).unwrap();

    // Test back_input and DONT_EXPAND (\noexpand marker)
    // Create a 2-token list: frozen_dont_expand -> \if
    let if_cs = {
        let name = b"if";
        engine.io.buffer[..name.len()].copy_from_slice(name);
        engine
            .hash
            .id_lookup(
                &mut engine.io,
                &engine.eqtb.layout,
                0,
                name.len(),
                &mut hooks,
            )
            .unwrap()
    };
    let noexp_head = engine.memory.get_avail().unwrap();
    engine
        .memory
        .set_info(
            noexp_head,
            Token::from_cs(engine.eqtb.layout.frozen_dont_expand as u16).raw(),
        )
        .unwrap();
    let _ = store_new_token(
        &mut engine.io,
        &mut engine.memory,
        noexp_head,
        Token::from_cs(if_cs),
        &mut hooks,
    )
    .unwrap();
    engine
        .scanner
        .back_list(
            &mut engine.io,
            &mut engine.eqtb,
            &mut engine.hash,
            &mut engine.memory,
            noexp_head,
        )
        .unwrap();
    let tok = engine.next_token().unwrap();
    assert_eq!(tok, Token::from_cs(if_cs));
    assert_eq!(engine.io.cur_cmd, cmd::RELAX);
    assert_eq!(engine.io.cur_chr, NO_EXPAND_FLAG);

    // Test \tracingmacros > 1 on EVERY_PAR_TEXT
    engine.eqtb.set_int_par(TRACING_MACROS_CODE, 2);
    engine.eqtb.set_int_par(TRACING_ONLINE_CODE, 1);
    engine.eqtb.sync_io_params(&mut engine.io);
    let ep_head = engine.memory.get_avail().unwrap();
    let _ = store_new_token(
        &mut engine.io,
        &mut engine.memory,
        ep_head,
        Token::from_cmd_chr(cmd::LETTER, b'P'),
        &mut hooks,
    )
    .unwrap();
    let _ = take_term_output(&mut engine);
    engine
        .scanner
        .begin_token_list(
            &mut engine.io,
            &mut engine.eqtb,
            &mut engine.hash,
            &mut engine.memory,
            ep_head,
            EVERY_PAR_TEXT,
        )
        .unwrap();
    assert_eq!(take_term_output(&mut engine), "\\everypar->P\n");
    assert_eq!(
        engine.next_token().unwrap(),
        Token::from_cmd_chr(cmd::LETTER, b'P')
    );
    engine.memory.delete_token_ref(ep_head).unwrap();
}

#[test]
fn show_context_runaway_check_outer_validity_and_error_hooks() {
    // Configure terminal with interactive responses: '2' (delete 2 tokens) then '\n' (proceed)
    let host = HostFixture::with_pinned_pool(b"2\n\n");
    let mut engine = Engine::with_io(MemoryConfig::default(), IoConfig::default(), host).unwrap();
    engine.init_prim().unwrap();
    engine.io.selector = Selector::TermOnly;
    engine.io.interaction = Interaction::ErrorStopMode;

    engine.set_line_bytes(b"hello world");
    // Advance 6 chars ("hello ")
    for _ in 0..6 {
        engine.get_token().unwrap();
    }
    let _ = take_term_output(&mut engine);

    // Trigger an error and let interactive '2\n\n' delete 'w' and 'o' via ScannerErrorHooks::get_token
    engine
        .io
        .print_err(webloom::io::pool_str("Undefined control sequence"));
    engine.error().unwrap();
    let out = take_term_output(&mut engine);
    assert!(
        out.contains("<*> hello \n          world"),
        "expected initial context in output, got:\n{out}"
    );
    assert!(
        out.contains("<*> hello wo\n            rld"),
        "expected post-deletion context in output, got:\n{out}"
    );

    // Next token should now be 'r'
    assert_eq!(
        engine.next_token().unwrap(),
        Token::from_cmd_chr(cmd::LETTER, b'r')
    );

    // Test check_outer_validity and runaway when an \outer CS is encountered during Defining
    engine.io.interaction = Interaction::NonstopMode;
    let mut hooks = NoopErrorHooks;
    let outer_cs = {
        let name = b"outermacro";
        engine.io.buffer[..name.len()].copy_from_slice(name);
        engine.hash.no_new_control_sequence = false;
        let cs = engine
            .hash
            .id_lookup(
                &mut engine.io,
                &engine.eqtb.layout,
                0,
                name.len(),
                &mut hooks,
            )
            .unwrap();
        engine.hash.no_new_control_sequence = true;
        engine.eqtb.set_eq_type(usize::from(cs), cmd::OUTER_CALL);
        engine.eqtb.set_equiv(usize::from(cs), 0);
        cs
    };

    let def_ref = engine.memory.get_avail().unwrap();
    let _ = store_new_token(
        &mut engine.io,
        &mut engine.memory,
        def_ref,
        Token::from_cmd_chr(cmd::LETTER, b'Q'),
        &mut hooks,
    )
    .unwrap();
    engine.scanner.def_ref = def_ref;
    engine.scanner.scanner_status = ScannerStatus::Defining;
    engine.scanner.warning_index = outer_cs;

    engine.set_line_bytes(b"\\outermacro");
    let _ = take_term_output(&mut engine);
    let tok = engine.next_token().unwrap();
    // The outer control sequence is replaced by a space and backed up, and a '}' is inserted for recovery!
    assert_eq!(tok, Token::from_cmd_chr(cmd::SPACER, b' '));
    let out = take_term_output(&mut engine);
    assert!(
        out.contains("Runaway definition?\nQ\n! Forbidden control sequence found while scanning definition of \\outermacro."),
        "unexpected runaway output:\n{out}"
    );
    // Next token from recovery is the inserted '}'
    assert_eq!(
        engine.next_token().unwrap(),
        Token(webloom::tables::RIGHT_BRACE_TOKEN + u16::from(b'}'))
    );
    engine.scanner.scanner_status = ScannerStatus::Normal;
    engine.memory.delete_token_ref(def_ref).unwrap();
}

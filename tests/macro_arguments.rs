use webloom::Engine;
use webloom::io::{Interaction, NoopErrorHooks, Selector, Termination};
use webloom::memory::{MemPtr, MemoryConfig};
use webloom::scanner::ScannerStatus;
use webloom::tables::{
    END_MATCH_TOKEN, MATCH_TOKEN, OUT_PARAM_TOKEN, TRACING_MACROS_CODE, TRACING_ONLINE_CODE, Token,
    cmd, store_new_token,
};

const MATCH: Token = Token(MATCH_TOKEN + b'#' as u16);
const ARG1: Token = Token(OUT_PARAM_TOKEN + 1);

fn engine() -> Engine {
    let mut e = Engine::new(MemoryConfig::default()).unwrap();
    e.init_prim().unwrap();
    e.io.selector = Selector::TermAndLog;
    e.io.interaction = Interaction::NonstopMode;
    e.eqtb.set_cat_code(b'{', u16::from(cmd::LEFT_BRACE));
    e.eqtb.set_cat_code(b'}', u16::from(cmd::RIGHT_BRACE));
    e
}

fn tokens(text: &[u8]) -> Vec<Token> {
    text.iter()
        .map(|&c| {
            Token::from_cmd_chr(
                match c {
                    b'{' => cmd::LEFT_BRACE,
                    b'}' => cmd::RIGHT_BRACE,
                    b' ' => cmd::SPACER,
                    _ if c.is_ascii_alphabetic() => cmd::LETTER,
                    _ => cmd::OTHER_CHAR,
                },
                c,
            )
        })
        .collect()
}

// Supply the compiled format of §290 directly; definition scanning is a later slice.
fn define(e: &mut Engine, name: u8, pattern: &[Token], body: &[Token], command: u8) -> MemPtr {
    let head = e.memory.get_avail().unwrap();
    e.memory.set_info(head, 0).unwrap();
    let mut tail = head;
    for &token in pattern
        .iter()
        .chain([Token(END_MATCH_TOKEN)].iter())
        .chain(body)
    {
        tail = store_new_token(&mut e.io, &mut e.memory, tail, token, &mut NoopErrorHooks).unwrap();
    }
    let cs = e.eqtb.layout.single_base + usize::from(name);
    e.eqtb.set_eq_type(cs, command);
    e.eqtb.set_equiv(cs, head.address());
    head
}

fn call(e: &mut Engine, line: &[u8]) {
    e.set_line_bytes(line);
    e.get_token().unwrap();
    assert!((cmd::CALL..=cmd::LONG_OUTER_CALL).contains(&e.io.cur_cmd));
    e.macro_call().unwrap();
}

fn expect(e: &mut Engine, expected: &[Token]) {
    for &token in expected {
        assert_eq!(e.next_token().unwrap(), token);
    }
}

fn output(e: &Engine) -> String {
    String::from_utf8(e.io.host.term_out.clone()).unwrap()
}

#[test]
fn shared_reference_fixtures() {
    for line in include_str!("fixtures/macro-arguments.tsv").lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 4);
        let argument = fields[3]
            .strip_prefix('"')
            .unwrap()
            .strip_suffix('"')
            .unwrap();
        let mut e = engine();
        let mut pattern = vec![MATCH];
        pattern.extend(tokens(fields[1].as_bytes()));
        let head = define(&mut e, b'a', &pattern, &[ARG1], cmd::CALL);
        let allocated = e.memory.stats().dyn_used;
        call(&mut e, format!("\\a {}Z", fields[2]).as_bytes());
        expect(&mut e, &tokens(format!("{argument}Z").as_bytes()));
        assert_eq!(e.memory.stats().dyn_used, allocated, "{}", fields[0]);
        assert_eq!(e.scanner.param_ptr, 0);
        assert_eq!(e.memory.info(head).unwrap(), 0);
        assert_eq!(e.io.align_state, 1_000_000);
    }
}

#[test]
fn empty_macros_and_nested_calls_restore_outer_arguments() {
    let mut e = engine();
    define(&mut e, b'e', &[], &[], cmd::CALL);
    let allocated = e.memory.stats().dyn_used;
    call(&mut e, b"\\e!");
    expect(&mut e, &tokens(b"!"));
    assert_eq!(e.memory.stats().dyn_used, allocated);

    define(&mut e, b'b', &[MATCH], &[ARG1, ARG1], cmd::CALL);
    let b = Token::from_cs((e.eqtb.layout.single_base + usize::from(b'b')) as u16);
    define(&mut e, b'a', &[MATCH], &[b, ARG1, ARG1], cmd::CALL);
    let allocated = e.memory.stats().dyn_used;
    call(&mut e, b"\\a x!");
    assert_eq!(e.next_token().unwrap(), b);
    e.macro_call().unwrap();
    assert_eq!(e.scanner.param_ptr, 2);
    expect(&mut e, &tokens(b"xxx!"));
    assert_eq!(e.scanner.param_ptr, 0);
    assert_eq!(e.memory.stats().dyn_used, allocated);
}

#[test]
fn mandatory_prefix_and_nine_parameters_preserve_order() {
    let mut e = engine();
    let mut pattern = tokens(b"xy");
    pattern.extend([MATCH; 9]);
    let body: Vec<_> = (1..=9).rev().map(|n| Token(OUT_PARAM_TOKEN + n)).collect();
    define(&mut e, b'a', &pattern, &body, cmd::CALL);
    call(&mut e, b"\\a xy123456789!");
    assert_eq!(e.scanner.param_ptr, 9);
    expect(&mut e, &tokens(b"987654321!"));
    assert_eq!(e.scanner.param_ptr, 0);
}

#[test]
fn arguments_are_not_expanded_when_matched_and_category_codes_matter() {
    let mut e = engine();
    define(&mut e, b'b', &[], &tokens(b"EXPANDED"), cmd::CALL);
    let b = Token::from_cs((e.eqtb.layout.single_base + usize::from(b'b')) as u16);
    define(&mut e, b'a', &[MATCH, b], &[ARG1], cmd::CALL);
    call(&mut e, b"\\a x\\b!");
    expect(&mut e, &tokens(b"x!"));

    // The first ! has catcode letter, so only the second (other) ! delimits.
    let mut e = engine();
    define(&mut e, b'a', &[MATCH, tokens(b"!")[0]], &[ARG1], cmd::CALL);
    let first = e.memory.get_avail().unwrap();
    e.memory
        .set_info(
            first,
            Token::from_cs((e.eqtb.layout.single_base + usize::from(b'a')) as u16).raw(),
        )
        .unwrap();
    let mut tail = first;
    let input = [
        Token::from_cmd_chr(cmd::LETTER, b'!'),
        tokens(b"!")[0],
        tokens(b"Z")[0],
    ];
    for token in input {
        tail = store_new_token(&mut e.io, &mut e.memory, tail, token, &mut NoopErrorHooks).unwrap();
    }
    e.scanner
        .back_list(&mut e.io, &mut e.eqtb, &mut e.hash, &mut e.memory, first)
        .unwrap();
    e.get_token().unwrap();
    e.macro_call().unwrap();
    expect(&mut e, &[input[0], input[2]]);
}

#[test]
fn brace_delimiter_is_counted_once_when_reintroduced_in_body() {
    let mut e = engine();
    let mut body = vec![ARG1];
    body.extend(tokens(b"{"));
    define(&mut e, b'a', &[MATCH, tokens(b"{")[0]], &body, cmd::CALL);
    call(&mut e, b"\\a xy{z}!");
    assert_eq!(e.io.align_state, 1_000_000);
    expect(&mut e, &tokens(b"xy{z}!"));
    assert_eq!(e.io.align_state, 1_000_000);
}

#[test]
fn runaway_paragraph_frees_all_arguments_and_restores_scanner_state() {
    let mut e = engine();
    define(&mut e, b'a', &[MATCH, MATCH], &[ARG1], cmd::CALL);
    let allocated = e.memory.stats().dyn_used;
    e.set_line_bytes(b"\\a {first}{second\\par!");
    e.get_token().unwrap();
    e.scanner.scanner_status = ScannerStatus::Absorbing;
    e.scanner.warning_index = 123;
    e.macro_call().unwrap();
    assert_eq!(e.scanner.scanner_status, ScannerStatus::Absorbing);
    assert_eq!(e.scanner.warning_index, 123);
    assert_eq!(e.scanner.param_ptr, 0);
    assert_eq!(e.io.align_state, 1_000_000);
    assert!(output(&e).contains("Paragraph ended before \\a was complete"));
    assert_eq!(e.next_token().unwrap(), e.hash.par_token);
    expect(&mut e, &tokens(b"!"));
    assert_eq!(e.memory.stats().dyn_used, allocated);
}

#[test]
fn long_macros_accept_paragraphs_and_nonlong_accepts_a_par_delimiter() {
    let mut e = engine();
    define(&mut e, b'a', &[MATCH], &[ARG1], cmd::LONG_OUTER_CALL);
    call(&mut e, b"\\a {x\\par y}!");
    expect(&mut e, &tokens(b"x"));
    assert_eq!(e.next_token().unwrap(), e.hash.par_token);
    expect(&mut e, &tokens(b"y!"));
    assert_eq!(e.io.error_count, 0);

    let mut e = engine();
    let par = e.hash.par_token;
    define(
        &mut e,
        b'a',
        &[MATCH, par, tokens(b"!")[0]],
        &[ARG1],
        cmd::CALL,
    );
    call(&mut e, b"\\a x\\par y\\par!!");
    expect(&mut e, &tokens(b"x"));
    assert_eq!(e.next_token().unwrap(), par);
    expect(&mut e, &tokens(b"y!"));
    assert_eq!(e.io.error_count, 0);
}

#[test]
fn prefix_mismatch_and_extra_right_brace_recover() {
    let mut e = engine();
    define(&mut e, b'a', &tokens(b"xy"), &tokens(b"BAD"), cmd::CALL);
    call(&mut e, b"\\a xz!");
    assert!(output(&e).contains("Use of \\a doesn't match its definition"));
    expect(&mut e, &tokens(b"!"));

    let mut e = engine();
    define(&mut e, b'a', &[MATCH], &[ARG1], cmd::LONG_CALL);
    call(&mut e, b"\\a }!");
    assert!(output(&e).contains("Argument of \\a has an extra }"));
    assert!(output(&e).contains("Paragraph ended before \\a was complete"));
    assert_eq!(e.next_token().unwrap(), e.hash.par_token);
    expect(&mut e, &tokens(b"}!"));
    assert_eq!(e.io.align_state, 1_000_000);
    assert_eq!(e.io.error_count, 2);
}

#[test]
fn outer_token_aborts_matching_once_and_is_preserved() {
    let mut e = engine();
    define(&mut e, b'a', &[MATCH, tokens(b"!")[0]], &[ARG1], cmd::CALL);
    define(&mut e, b'b', &[], &[], cmd::OUTER_CALL);
    let b = Token::from_cs((e.eqtb.layout.single_base + usize::from(b'b')) as u16);
    call(&mut e, b"\\a x\\b!");
    assert!(output(&e).contains("Forbidden control sequence found while scanning use of \\a"));
    assert_eq!(e.io.error_count, 1);
    expect(&mut e, &[b, tokens(b"!")[0]]);
    assert_eq!(e.scanner.param_ptr, 0);
}

#[test]
fn tail_calls_reclaim_old_parameters_and_input_levels() {
    let mut e = engine();
    let a = Token::from_cs((e.eqtb.layout.single_base + usize::from(b'a')) as u16);
    let body = [ARG1, a, tokens(b"{")[0], ARG1, tokens(b"}")[0]];
    let head = define(&mut e, b'a', &[MATCH], &body, cmd::CALL);
    call(&mut e, b"\\a x!");
    for _ in 0..100 {
        expect(&mut e, &tokens(b"x"));
        assert_eq!(e.next_token().unwrap(), a);
        e.macro_call().unwrap();
        assert_eq!(e.io.input_ptr, 1);
        assert_eq!(e.scanner.param_ptr, 1);
        assert_eq!(e.memory.info(head).unwrap(), 1);
    }
    assert_eq!(e.scanner.max_param_stack, 1);
    assert!(e.io.max_in_stack <= 2);
}

#[test]
fn parameter_capacity_and_trace_diagnostics() {
    let mut e = engine();
    e.scanner.param_size = 1;
    define(&mut e, b'a', &[MATCH, MATCH], &[ARG1], cmd::CALL);
    e.set_line_bytes(b"\\a xy!");
    e.get_token().unwrap();
    assert_eq!(e.macro_call(), Err(Termination::JumpOut));
    assert!(output(&e).contains("parameter stack size=1"));
    assert_eq!(e.scanner.scanner_status, ScannerStatus::Normal);

    let mut e = engine();
    e.eqtb.set_int_par(TRACING_MACROS_CODE, 1);
    e.eqtb.set_int_par(TRACING_ONLINE_CODE, 1);
    define(&mut e, b'a', &[MATCH], &[ARG1], cmd::CALL);
    call(&mut e, b"\\a {xy}!");
    assert!(output(&e).contains("\\a #1->#1"), "{}", output(&e));
    assert!(output(&e).contains("#1<-xy"));
}

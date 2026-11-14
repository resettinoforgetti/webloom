use webloom::Engine;
use webloom::memory::MemoryConfig;
use webloom::tables::cmd;

fn engine_with_pool() -> Engine {
    let mut engine = Engine::new(MemoryConfig::default()).expect("create engine");
    engine.init_prim().expect("init_prim");
    engine.eqtb.set_cat_code(b'\\', cmd::ESCAPE.into());
    engine.eqtb.set_cat_code(b'{', cmd::LEFT_BRACE.into());
    engine.eqtb.set_cat_code(b'}', cmd::RIGHT_BRACE.into());
    engine.eqtb.set_cat_code(b'#', cmd::MAC_PARAM.into());
    engine.eqtb.set_cat_code(b' ', cmd::SPACER.into());
    engine
}

fn collect_x_chars(engine: &mut Engine, line: &[u8]) -> String {
    engine.set_line_bytes(line);
    let mut out = String::new();
    while engine.get_x_token().is_ok() {
        if engine.io.cur_cmd == cmd::RELAX || engine.io.cur_cmd == cmd::PAR_END {
            break;
        }
        if engine.scanner.cur_cs == 0 && engine.io.cur_chr > 0 && engine.io.cur_chr < 128 {
            out.push(engine.io.cur_chr as u8 as char);
        }
    }
    out
}

#[test]
fn scan_int_handles_decimal_octal_hex_alphabetic_and_negative() {
    let mut engine = engine_with_pool();
    engine.set_line_bytes(b"42 -17 '77 \"1A `A");
    engine.scan_int().unwrap();
    assert_eq!(engine.scanner.cur_val, 42);
    engine.scan_int().unwrap();
    assert_eq!(engine.scanner.cur_val, -17);
    engine.scan_int().unwrap();
    assert_eq!(engine.scanner.cur_val, 0o77);
    engine.scan_int().unwrap();
    assert_eq!(engine.scanner.cur_val, 0x1A);
    engine.scan_int().unwrap();
    assert_eq!(engine.scanner.cur_val, i32::from(b'A'));
}

#[test]
fn scan_dimen_and_scan_glue_parse_units_and_stretch_shrink() {
    let mut engine = engine_with_pool();
    engine.set_line_bytes(b"10pt 2.5pc 65536sp 10pt plus 1fil minus 2pt");
    engine.scan_dimen(false, false, false).unwrap();
    assert_eq!(engine.scanner.cur_val, 10 * 65536);
    engine.scan_dimen(false, false, false).unwrap();
    assert_eq!(engine.scanner.cur_val, 30 * 65536);
    engine.scan_dimen(false, false, false).unwrap();
    assert_eq!(engine.scanner.cur_val, 65536);

    engine.scan_glue(webloom::tables::GLUE_VAL).unwrap();
    let ptr = webloom::memory::MemPtr::from_address(engine.scanner.cur_val as u16);
    let spec = engine.memory.glue_spec(ptr).unwrap();
    assert_eq!(spec.width.raw(), 10 * 65536);
    assert_eq!(spec.stretch.raw(), 65536);
    assert_eq!(spec.stretch_order, 1); // fil
    assert_eq!(spec.shrink.raw(), 2 * 65536);
    assert_eq!(spec.shrink_order, 0);
}

#[test]
fn expansion_dispatch_handles_number_romannumeral_csname_and_jobname() {
    let mut engine = engine_with_pool();
    assert_eq!(collect_x_chars(&mut engine, b"\\number 42\\relax"), "42");
    assert_eq!(
        collect_x_chars(&mut engine, b"\\romannumeral 14\\relax"),
        "xiv"
    );
    assert_eq!(collect_x_chars(&mut engine, b"\\jobname\\relax"), "texput");
    // \csname foo\endcsname defines \foo as \relax if undefined
    assert_eq!(collect_x_chars(&mut engine, b"\\csname foo\\endcsname"), "");
}

#[test]
fn conditionals_evaluate_ifnum_ifdim_ifodd_ifcase_and_nested_branches() {
    let mut engine = engine_with_pool();
    assert_eq!(
        collect_x_chars(&mut engine, b"\\ifnum 3<5 yes\\else no\\fi\\relax"),
        "yes"
    );
    assert_eq!(
        collect_x_chars(&mut engine, b"\\ifnum 5<3 yes\\else no\\fi\\relax"),
        "no"
    );
    assert_eq!(
        collect_x_chars(&mut engine, b"\\ifodd 7 odd\\else even\\fi\\relax"),
        "odd"
    );
    assert_eq!(
        collect_x_chars(
            &mut engine,
            b"\\ifcase 1 zero\\or one\\or two\\else other\\fi\\relax"
        ),
        "one"
    );
    assert_eq!(
        collect_x_chars(
            &mut engine,
            b"\\iftrue \\iffalse A\\else B\\fi \\else C\\fi\\relax"
        ),
        "B"
    );
}

#[test]
fn assignments_execute_def_let_chardef_and_register_arithmetic() {
    let mut engine = engine_with_pool();
    engine
        .set_line_bytes(b"\\def\\greet{hi}\\chardef\\myA=65 \\count0=10 \\advance\\count0 by 5 x");
    engine.do_assignments().unwrap();
    assert_eq!(engine.eqtb.int(engine.eqtb.layout.count_base), 15);
    assert_eq!(collect_x_chars(&mut engine, b"\\greet\\relax"), "hi");
}

#[test]
fn scan_file_name_skips_leading_spaces_and_stops_at_space() {
    let mut engine = engine_with_pool();
    engine.set_line_bytes(b"   plain.tex next");
    engine.scan_file_name().unwrap();
    let name = String::from_utf8_lossy(engine.io.str_bytes(engine.io.cur_name)).into_owned();
    let ext = String::from_utf8_lossy(engine.io.str_bytes(engine.io.cur_ext)).into_owned();
    assert_eq!(format!("{name}{ext}"), "plain.tex");
}

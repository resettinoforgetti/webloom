use webloom::Engine;
use webloom::arithmetic::Scaled;
use webloom::io::{IoConfig, NoopErrorHooks, Selector, pool_str};
use webloom::memory::{
    FIL_ORDER, FILL_ORDER, FILLL_ORDER, GlueSpec, MemPtr, MemoryConfig, NORMAL_ORDER,
};
use webloom::tables::{
    END_LINE_CHAR_CODE, ESCAPE_CHAR_CODE, EqtbLayout, HANG_AFTER_CODE, HMODE, HSIZE_CODE,
    IGNORE_DEPTH, LEVEL_ONE, LEVEL_ZERO, MAG_CODE, MAX_DEAD_CYCLES_CODE, MMODE, SemanticNest,
    TOLERANCE_CODE, TRACING_COMMANDS_CODE, TRACING_RESTORES_CODE, Token, VMODE, cmd, group,
    print_cmd_chr, print_meaning, show_cur_cmd_chr, show_eqtb, show_token_list, store_new_token,
    token_show,
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
fn eqtb_layout_matches_tangle_addresses_and_bad_21_check() {
    let layout = EqtbLayout::new(2100);
    assert_eq!(layout.active_base, 1);
    assert_eq!(layout.single_base, 257);
    assert_eq!(layout.null_cs, 513);
    assert_eq!(layout.hash_base, 514);
    assert_eq!(layout.frozen_control_sequence, 2614);
    assert_eq!(layout.frozen_protection, 2614);
    assert_eq!(layout.frozen_cr, 2615);
    assert_eq!(layout.frozen_end_group, 2616);
    assert_eq!(layout.frozen_right, 2617);
    assert_eq!(layout.frozen_fi, 2618);
    assert_eq!(layout.frozen_end_template, 2619);
    assert_eq!(layout.frozen_endv, 2620);
    assert_eq!(layout.frozen_relax, 2621);
    assert_eq!(layout.end_write, 2622);
    assert_eq!(layout.frozen_dont_expand, 2623);
    assert_eq!(layout.frozen_null_font, 2624);
    assert_eq!(layout.font_id_base, 2624);
    assert_eq!(layout.undefined_control_sequence, 2881);
    assert_eq!(layout.glue_base, 2882);
    assert_eq!(layout.skip_base, 2900);
    assert_eq!(layout.mu_skip_base, 3156);
    assert_eq!(layout.local_base, 3412);
    assert_eq!(layout.par_shape_loc, 3412);
    assert_eq!(layout.output_routine_loc, 3413);
    assert_eq!(layout.every_par_loc, 3414);
    assert_eq!(layout.toks_base, 3422);
    assert_eq!(layout.box_base, 3678);
    assert_eq!(layout.cur_font_loc, 3934);
    assert_eq!(layout.math_font_base, 3935);
    assert_eq!(layout.cat_code_base, 3983);
    assert_eq!(layout.lc_code_base, 4239);
    assert_eq!(layout.uc_code_base, 4495);
    assert_eq!(layout.sf_code_base, 4751);
    assert_eq!(layout.math_code_base, 5007);
    assert_eq!(layout.int_base, 5263);
    assert_eq!(layout.count_base, 5318);
    assert_eq!(layout.del_code_base, 5574);
    assert_eq!(layout.dimen_base, 5830);
    assert_eq!(layout.scaled_base, 5851);
    assert_eq!(layout.eqtb_size, 6106);

    // §290: cs_token_flag (4095) + undefined_control_sequence (781 + hash_size) > 65535 => bad = 21.
    let mut cfg = IoConfig {
        hash_size: 65535 - 4095 - 781,
        ..IoConfig::default()
    };
    assert_eq!(cfg.check_constants(), Ok(()));
    cfg.hash_size += 1;
    assert_eq!(cfg.check_constants(), Err(21));
}

#[test]
fn eqtb_pristine_init_and_date_time() {
    let mut engine = make_engine();
    let layout = engine.eqtb.layout;

    // Region 1 & 2 defaults
    assert_eq!(
        engine.eqtb.eq_type(layout.undefined_control_sequence),
        cmd::UNDEFINED_CS
    );
    assert_eq!(
        engine.eqtb.eq_level(layout.undefined_control_sequence),
        LEVEL_ZERO
    );
    assert_eq!(
        engine.eqtb.eq_type(layout.frozen_dont_expand),
        cmd::DONT_EXPAND
    );

    // Region 3 glue ref count on zero_glue (1 from §164 + 530 from §228)
    let zero_glue = engine.memory.zero_glue();
    assert_eq!(
        engine.memory.glue_ref_count(zero_glue),
        Ok(1 + (layout.local_base - layout.glue_base) as u16)
    );

    // Region 4 catcodes, lccodes, uccodes, sfcodes, mathcodes
    assert_eq!(engine.eqtb.cat_code(b'\\'), cmd::ESCAPE);
    assert_eq!(engine.eqtb.cat_code(b' '), cmd::SPACER);
    assert_eq!(engine.eqtb.cat_code(b'%'), cmd::COMMENT);
    assert_eq!(engine.eqtb.cat_code(13), cmd::CAR_RET);
    assert_eq!(engine.eqtb.cat_code(0), cmd::IGNORE);
    assert_eq!(engine.eqtb.cat_code(127), cmd::INVALID_CHAR);
    assert_eq!(engine.eqtb.cat_code(b'A'), cmd::LETTER);
    assert_eq!(engine.eqtb.cat_code(b'z'), cmd::LETTER);
    assert_eq!(engine.eqtb.cat_code(b'@'), cmd::OTHER_CHAR);
    assert_eq!(engine.eqtb.lc_code(b'A'), u16::from(b'a'));
    assert_eq!(engine.eqtb.uc_code(b'a'), u16::from(b'A'));
    assert_eq!(engine.eqtb.sf_code(b'A'), 999);
    assert_eq!(engine.eqtb.sf_code(b'a'), 1000);
    assert_eq!(engine.eqtb.math_code(b'0'), u16::from(b'0') + 0x7000);
    assert_eq!(engine.eqtb.math_code(b'A'), u16::from(b'A') + 0x7100);

    // Region 5 integers and delcodes
    assert_eq!(engine.eqtb.int_par(MAG_CODE), 1000);
    assert_eq!(engine.eqtb.int_par(TOLERANCE_CODE), 10000);
    assert_eq!(engine.eqtb.int_par(HANG_AFTER_CODE), 1);
    assert_eq!(engine.eqtb.int_par(MAX_DEAD_CYCLES_CODE), 25);
    assert_eq!(engine.eqtb.int_par(ESCAPE_CHAR_CODE), i32::from(b'\\'));
    assert_eq!(engine.eqtb.int_par(END_LINE_CHAR_CODE), 13);
    assert_eq!(engine.eqtb.del_code(b'.'), 0);
    assert_eq!(engine.eqtb.del_code(b'('), -1);

    // §241: fix_date_and_time
    engine.eqtb.fix_date_and_time(&mut engine.io);
    assert_eq!(engine.eqtb.int_par(webloom::tables::TIME_CODE), 12 * 60);
    assert_eq!(engine.eqtb.int_par(webloom::tables::DAY_CODE), 4);
    assert_eq!(engine.eqtb.int_par(webloom::tables::MONTH_CODE), 7);
    assert_eq!(engine.eqtb.int_par(webloom::tables::YEAR_CODE), 1776);
}

#[test]
fn semantic_nest_push_pop_tail_append_and_print_mode() {
    let mut engine = make_engine();
    let mut hooks = NoopErrorHooks;

    assert_eq!(engine.nest.cur_list.mode_field, VMODE);
    assert_eq!(
        engine.nest.cur_list.head_field,
        engine.memory.contrib_head()
    );
    assert_eq!(
        engine.nest.cur_list.tail_field,
        engine.memory.contrib_head()
    );
    assert_eq!(engine.nest.cur_list.aux_int, IGNORE_DEPTH);

    SemanticNest::print_mode(&mut engine.io, VMODE);
    assert_eq!(take_term_output(&mut engine), "vertical mode");
    SemanticNest::print_mode(&mut engine.io, -VMODE);
    assert_eq!(take_term_output(&mut engine), "internal vertical mode");
    SemanticNest::print_mode(&mut engine.io, HMODE);
    assert_eq!(take_term_output(&mut engine), "horizontal mode");
    SemanticNest::print_mode(&mut engine.io, -HMODE);
    assert_eq!(take_term_output(&mut engine), "restricted horizontal mode");
    SemanticNest::print_mode(&mut engine.io, MMODE);
    assert_eq!(take_term_output(&mut engine), "display math mode");
    SemanticNest::print_mode(&mut engine.io, -MMODE);
    assert_eq!(take_term_output(&mut engine), "math mode");
    SemanticNest::print_mode(&mut engine.io, 0);
    assert_eq!(take_term_output(&mut engine), "no mode");

    engine
        .nest
        .push_nest(&mut engine.io, &mut engine.memory, &mut hooks)
        .unwrap();
    engine.nest.cur_list.mode_field = -HMODE;
    engine.nest.cur_list.set_space_factor(1000);
    engine.nest.cur_list.set_clang(0);
    assert_eq!(engine.nest.cur_list.space_factor(), 1000);

    let p = engine.memory.get_avail().unwrap();
    engine.nest.tail_append(&mut engine.memory, p).unwrap();
    assert_eq!(engine.nest.cur_list.tail_field, p);
    assert_eq!(
        engine.memory.head_link(engine.nest.cur_list.head_field),
        Ok(p)
    );

    // Free appended node before pop_nest frees head_field
    engine.memory.free_avail(p).unwrap();
    engine
        .memory
        .set_head_link(engine.nest.cur_list.head_field, MemPtr::NULL)
        .unwrap();
    engine.nest.pop_nest(&mut engine.memory).unwrap();
    assert_eq!(engine.nest.cur_list.mode_field, VMODE);
}

#[test]
fn hash_table_id_lookup_collision_and_primitives() {
    let mut engine = make_engine();
    let mut hooks = NoopErrorHooks;
    let layout = engine.eqtb.layout;

    // Look up existing primitive \relax
    let name = b"relax";
    engine.io.buffer[..name.len()].copy_from_slice(name);
    let relax_cs = engine
        .hash
        .id_lookup(&mut engine.io, &layout, 0, name.len(), &mut hooks)
        .unwrap();
    assert_eq!(engine.eqtb.eq_type(usize::from(relax_cs)), cmd::RELAX);
    assert_eq!(engine.eqtb.equiv(usize::from(relax_cs)), 256);

    // With no_new_control_sequence = true (set at end of init_prim), unknown CS returns undefined_control_sequence
    let unknown = b"nonexistentcs";
    engine.io.buffer[..unknown.len()].copy_from_slice(unknown);
    let undef_cs = engine
        .hash
        .id_lookup(&mut engine.io, &layout, 0, unknown.len(), &mut hooks)
        .unwrap();
    assert_eq!(usize::from(undef_cs), layout.undefined_control_sequence);

    // Allow new control sequences and insert a custom CS
    engine.hash.no_new_control_sequence = false;
    let new_cs = engine
        .hash
        .id_lookup(&mut engine.io, &layout, 0, unknown.len(), &mut hooks)
        .unwrap();
    assert!(usize::from(new_cs) >= layout.hash_base);
    assert!(usize::from(new_cs) < layout.frozen_control_sequence);

    // print_cs and sprint_cs
    engine
        .hash
        .print_cs(&mut engine.io, &engine.eqtb, i32::from(relax_cs));
    assert_eq!(take_term_output(&mut engine), "\\relax ");

    engine
        .hash
        .print_cs(&mut engine.io, &engine.eqtb, layout.null_cs as i32);
    assert_eq!(take_term_output(&mut engine), "\\csname\\endcsname ");

    // Single-letter CS vs single non-letter CS
    let a_cs = (layout.single_base + usize::from(b'a')) as i32;
    engine.hash.print_cs(&mut engine.io, &engine.eqtb, a_cs);
    assert_eq!(take_term_output(&mut engine), "\\a ");

    let pct_cs = (layout.single_base + usize::from(b'%')) as i32;
    engine.hash.print_cs(&mut engine.io, &engine.eqtb, pct_cs);
    assert_eq!(take_term_output(&mut engine), "\\%");
}

#[test]
fn save_stack_local_global_definitions_unsave_and_aftergroup() {
    let mut engine = make_engine();
    let mut hooks = NoopErrorHooks;
    let layout = engine.eqtb.layout;
    let count0 = layout.count_base;

    assert_eq!(engine.eqtb.int(count0), 0);
    assert_eq!(engine.save_stack.cur_level, LEVEL_ONE);

    // Enter simple group
    engine
        .save_stack
        .new_save_level(&mut engine.io, group::SIMPLE_GROUP, &mut hooks)
        .unwrap();
    assert_eq!(engine.save_stack.cur_level, LEVEL_ONE + 1);

    // Local assignment to \count0 = 42
    engine
        .save_stack
        .eq_word_define(&mut engine.io, &mut engine.eqtb, count0, 42, &mut hooks)
        .unwrap();
    assert_eq!(engine.eqtb.int(count0), 42);
    assert_eq!(engine.eqtb.eq_level(count0), LEVEL_ONE + 1);

    // Second local assignment in same group does not grow save_stack
    let sp_before = engine.save_stack.save_ptr;
    engine
        .save_stack
        .eq_word_define(&mut engine.io, &mut engine.eqtb, count0, 99, &mut hooks)
        .unwrap();
    assert_eq!(engine.save_stack.save_ptr, sp_before);
    assert_eq!(engine.eqtb.int(count0), 99);

    // Queue an \aftergroup token
    let tok = Token::from_cmd_chr(cmd::LETTER, b'X');
    engine
        .save_stack
        .save_for_after(&mut engine.io, tok, &mut hooks)
        .unwrap();

    // Unsave restores \count0 = 0 and returns the aftergroup token
    let after_tokens = engine
        .save_stack
        .unsave(
            &mut engine.io,
            &mut engine.eqtb,
            &engine.hash,
            &mut engine.memory,
            &mut hooks,
        )
        .unwrap();
    assert_eq!(engine.save_stack.cur_level, LEVEL_ONE);
    assert_eq!(engine.eqtb.int(count0), 0);
    assert_eq!(after_tokens, vec![tok]);

    // Test global assignment inside group overriding local restoration, with \tracingrestores = 1
    engine.eqtb.set_int_par(TRACING_RESTORES_CODE, 1);
    engine
        .eqtb
        .set_int_par(webloom::tables::TRACING_ONLINE_CODE, 1);
    engine.eqtb.sync_io_params(&mut engine.io);
    engine
        .save_stack
        .new_save_level(&mut engine.io, group::SIMPLE_GROUP, &mut hooks)
        .unwrap();
    engine
        .save_stack
        .eq_word_define(&mut engine.io, &mut engine.eqtb, count0, 10, &mut hooks)
        .unwrap();
    webloom::tables::SaveStack::geq_word_define(&mut engine.io, &mut engine.eqtb, count0, 777);
    let _ = take_term_output(&mut engine);
    let _ = engine
        .save_stack
        .unsave(
            &mut engine.io,
            &mut engine.eqtb,
            &engine.hash,
            &mut engine.memory,
            &mut hooks,
        )
        .unwrap();
    assert_eq!(engine.eqtb.int(count0), 777);
    let trace = take_term_output(&mut engine);
    assert!(
        trace.contains("{retaining \\count0=777}"),
        "unexpected trace: {trace:?}"
    );
}

#[test]
fn token_list_formatting_print_cmd_chr_and_show_eqtb() {
    let mut engine = make_engine();
    let mut hooks = NoopErrorHooks;
    let layout = engine.eqtb.layout;

    // Build a macro token list: #1 -> {a#1}
    let def_ref = engine.memory.get_avail().unwrap();
    let mut p = def_ref;
    p = store_new_token(
        &mut engine.io,
        &mut engine.memory,
        p,
        Token(u16::from(cmd::MATCH) * 256 + u16::from(b'#')),
        &mut hooks,
    )
    .unwrap();
    p = store_new_token(
        &mut engine.io,
        &mut engine.memory,
        p,
        Token(u16::from(cmd::END_MATCH) * 256),
        &mut hooks,
    )
    .unwrap();
    p = store_new_token(
        &mut engine.io,
        &mut engine.memory,
        p,
        Token::from_cmd_chr(cmd::LEFT_BRACE, b'{'),
        &mut hooks,
    )
    .unwrap();
    p = store_new_token(
        &mut engine.io,
        &mut engine.memory,
        p,
        Token::from_cmd_chr(cmd::LETTER, b'a'),
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
        Token::from_cmd_chr(cmd::RIGHT_BRACE, b'}'),
        &mut hooks,
    )
    .unwrap();

    token_show(
        &mut engine.io,
        &engine.eqtb,
        &engine.hash,
        &engine.memory,
        def_ref,
    );
    assert_eq!(take_term_output(&mut engine), "#1->{a#1}");

    engine.io.cur_cmd = cmd::LONG_CALL;
    engine.io.cur_chr = i32::from(def_ref.address());
    print_meaning(
        &mut engine.io,
        &engine.eqtb,
        &engine.hash,
        &engine.memory,
        &[MemPtr::NULL; 5],
    );
    assert_eq!(take_term_output(&mut engine), "\\long macro:\n#1->{a#1}");

    engine.memory.delete_token_ref(def_ref).unwrap();

    // Test print_cmd_chr and show_cur_cmd_chr
    print_cmd_chr(
        &mut engine.io,
        &layout,
        cmd::ASSIGN_DIMEN,
        (layout.dimen_base + HSIZE_CODE) as i32,
    );
    assert_eq!(take_term_output(&mut engine), "\\hsize");

    engine.eqtb.set_int_par(TRACING_COMMANDS_CODE, 1);
    engine
        .eqtb
        .set_int_par(webloom::tables::TRACING_ONLINE_CODE, 1);
    engine.eqtb.sync_io_params(&mut engine.io);
    engine.io.cur_cmd = cmd::RELAX;
    engine.io.cur_chr = 256;
    show_cur_cmd_chr(&mut engine.io, &layout, &mut engine.nest);
    assert_eq!(
        take_term_output(&mut engine),
        "\n{vertical mode: \\relax}\n"
    );

    // Test show_eqtb across regions
    show_eqtb(
        &mut engine.io,
        &engine.eqtb,
        &engine.hash,
        &engine.memory,
        layout.int_base + MAG_CODE,
    );
    assert_eq!(take_term_output(&mut engine), "\\mag=1000");

    // Test print_spec on glue spec with fil/fill/filll orders
    let spec = GlueSpec {
        width: Scaled::UNITY,
        stretch: Scaled::from_raw(Scaled::UNITY.raw() * 2).unwrap(),
        shrink: Scaled::from_raw(Scaled::UNITY.raw() * 3).unwrap(),
        stretch_order: FIL_ORDER,
        shrink_order: FILLL_ORDER,
        ref_count: 0,
    };
    let g = engine.memory.new_null_glue().expect("alloc glue spec");
    engine.memory.set_glue_spec(g, spec).expect("set glue spec");
    engine
        .io
        .print_spec(&engine.memory, i32::from(g.address()), pool_str("pt"));
    assert_eq!(
        take_term_output(&mut engine),
        "1.0pt plus 2.0fil minus 3.0filll"
    );
    let _ = engine.memory.delete_glue_ref(g);

    // Also exercise NORMAL_ORDER and FILL_ORDER
    let spec2 = GlueSpec {
        width: Scaled::ZERO,
        stretch: Scaled::UNITY,
        shrink: Scaled::UNITY,
        stretch_order: NORMAL_ORDER,
        shrink_order: FILL_ORDER,
        ref_count: 0,
    };
    let g2 = engine.memory.new_null_glue().expect("alloc glue spec");
    engine
        .memory
        .set_glue_spec(g2, spec2)
        .expect("set glue spec");
    engine
        .io
        .print_spec(&engine.memory, i32::from(g2.address()), pool_str("mu"));
    assert_eq!(
        take_term_output(&mut engine),
        "0.0mu plus 1.0mu minus 1.0fill"
    );
    let _ = engine.memory.delete_glue_ref(g2);

    // Exercise show_token_list directly with null pointer
    show_token_list(
        &mut engine.io,
        &engine.eqtb,
        &engine.hash,
        &engine.memory,
        0,
        0,
        10,
    );
    assert_eq!(take_term_output(&mut engine), "");

    // Exercise prepare_mag incompatible magnification recovery and par_shape (§275, §288)
    engine.io.interaction = webloom::io::Interaction::NonstopMode;
    engine
        .save_stack
        .prepare_mag(&mut engine.io, &mut engine.eqtb, &mut hooks)
        .unwrap();
    assert_eq!(engine.save_stack.mag_set, 1000);
    engine.eqtb.set_int_par(MAG_CODE, 1200);
    engine
        .save_stack
        .prepare_mag(&mut engine.io, &mut engine.eqtb, &mut hooks)
        .unwrap();
    assert_eq!(engine.eqtb.int_par(MAG_CODE), 1000);

    let shape = engine
        .memory
        .alloc_par_shape(&[(Scaled::ZERO, Scaled::UNITY)])
        .unwrap();
    assert_eq!(
        engine.memory.par_shape_lines(shape),
        Ok(vec![(Scaled::ZERO, Scaled::UNITY)])
    );
    engine
        .save_stack
        .eq_define(
            &mut engine.io,
            &mut engine.eqtb,
            &mut engine.memory,
            layout.par_shape_loc,
            cmd::SHAPE_REF,
            shape.address(),
            &mut hooks,
        )
        .unwrap();
    engine
        .save_stack
        .eq_define(
            &mut engine.io,
            &mut engine.eqtb,
            &mut engine.memory,
            layout.par_shape_loc,
            cmd::SHAPE_REF,
            MemPtr::NULL.address(),
            &mut hooks,
        )
        .unwrap();
}

//! Bounded completion of explicitly selected, unfinished generated stubs.

use super::super::{Mode, plan_with_mode};
use super::{
    BOLT, BOLT_PRINT, BOUNCE, BOUNCE_PRINT, Cli, Cmd, Fixture, Parser, ledger, scryfall, stubgen,
    write,
};
use baylee_cards_codegen::layout;
use std::{fs, path::PathBuf};

/// Keep the shared fixture's identities, cache, scripts and canonical tests;
/// obtain the unfinished source through the real stub renderer.
fn add_stub(fixture: &Fixture, name: &str, explicit_coverage: bool) -> PathBuf {
    let path = fixture.add(name);
    let printing: scryfall::ScryfallCard =
        serde_json::from_slice(&fs::read(fixture.payload(name)).unwrap()).unwrap();
    let ledger = ledger::IndexLedger::from_rows(&baylee_cards_index::ROWS).unwrap();
    let row = ledger.entry_named(name).unwrap();
    let cats = crate::hold_back::compiled_catalogs();
    let cycles = layout::LandCycles::parse("");
    let (_, mut source) = stubgen::render_unimplemented(
        &printing,
        row,
        &ledger,
        &cats,
        &cycles,
        "synthetic fixture: the sentence is not yet read",
    )
    .unwrap();
    if explicit_coverage {
        source = source.replacen(
            "card!(",
            "card!(\n    coverage = Coverage::Unimplemented,",
            1,
        );
    }
    assert!(
        stubgen::adopt_stub(&source).is_some(),
        "a genuine unfinished stub"
    );
    write(&path, source);
    path
}

fn complete(fixture: &Fixture, names: &[&str], check: bool) -> anyhow::Result<()> {
    let names = names.iter().map(ToString::to_string).collect::<Vec<_>>();
    plan_with_mode(
        fixture.dir.path(),
        &fixture.cache,
        &fixture.scripts,
        &names,
        Mode::CompleteStub,
    )?
    .apply(check)
}

fn rejects_selection_without_writes(fixture: &Fixture) {
    let before = [
        fs::read(fixture.source(BOLT)).unwrap(),
        fs::read(fixture.source(BOUNCE)).unwrap(),
    ];
    assert!(complete(fixture, &[BOLT, BOUNCE], false).is_err());
    assert_eq!(fs::read(fixture.source(BOLT)).unwrap(), before[0]);
    assert_eq!(fs::read(fixture.source(BOUNCE)).unwrap(), before[1]);
}

#[test]
fn completion_writes_only_the_selected_unfinished_source() {
    let fixture = Fixture::new();
    let selected = add_stub(&fixture, BOLT, false);
    let sibling = add_stub(&fixture, BOUNCE, false);
    let sibling_before = fs::read(&sibling).unwrap();
    let sentinels = [
        "crates/baylee-cards/src/cards/mod.rs",
        "crates/baylee-cards/src/generated_lines.rs",
        "crates/baylee-cards/src/generated_names.rs",
        "crates/baylee-core/src/generated/index/mod.rs",
    ];
    for relative in sentinels {
        write(
            &fixture.dir.path().join(relative),
            "// completion leaves this alone\n",
        );
    }
    complete(&fixture, &[BOLT], false).unwrap();
    let generated = fs::read_to_string(selected).unwrap();
    assert!(generated.contains(stubgen::OWNED_MARKER));
    assert!(!generated.contains(stubgen::STUB_MARKER));
    assert!(generated.contains("Coverage::Implemented"));
    assert!(generated.contains(BOLT_PRINT));
    assert_eq!(fs::read(sibling).unwrap(), sibling_before);
    for relative in sentinels {
        assert_eq!(
            fs::read_to_string(fixture.dir.path().join(relative)).unwrap(),
            "// completion leaves this alone\n"
        );
    }
}

#[test]
fn absent_and_explicit_unimplemented_coverage_are_both_valid_stub_defaults() {
    for explicit in [false, true] {
        let fixture = Fixture::new();
        let path = add_stub(&fixture, BOLT, explicit);
        complete(&fixture, &[BOLT], false).unwrap();
        assert!(
            fs::read_to_string(path)
                .unwrap()
                .contains("Coverage::Implemented")
        );
    }
}

#[test]
fn accepted_canonical_tests_determine_generated_or_implemented_output() {
    for acceptance in ["absent", "held", "accepted"] {
        let fixture = Fixture::new();
        let path = add_stub(&fixture, BOLT, false);
        match acceptance {
            "absent" => fs::remove_file(fixture.test(BOLT)).unwrap(),
            "held" => {
                let ledger = ledger::IndexLedger::from_rows(&baylee_cards_index::ROWS).unwrap();
                write(
                    &fixture.dir.path().join("data/card-test-holds.tsv"),
                    format!(
                        "{}\tfixture: engine proof is pending\n",
                        ledger.entry_named(BOLT).unwrap().oracle_id
                    ),
                );
            }
            _ => {}
        }
        complete(&fixture, &[BOLT], false).unwrap();
        let source = fs::read_to_string(path).unwrap();
        assert!(source.contains(stubgen::OWNED_MARKER));
        let expected = if acceptance == "accepted" {
            "Coverage::Implemented"
        } else {
            "Coverage::Generated"
        };
        assert!(source.contains(expected), "{acceptance}: {source}");
        let rejected = if acceptance == "accepted" {
            "Coverage::Generated"
        } else {
            "Coverage::Implemented"
        };
        assert!(!source.contains(rejected), "{acceptance}: {source}");
    }
}

#[test]
fn an_unrelated_test_file_does_not_promote_an_untested_stub() {
    let fixture = Fixture::new();
    let path = add_stub(&fixture, BOLT, false);
    let original = fixture.test(BOLT);
    let wrong = fixture
        .dir
        .path()
        .join("crates/baylee-engine/src/engine/card_tests/instants/mv_2/lightning_bolt.rs");
    write(&wrong, fs::read(&original).unwrap());
    fs::remove_file(original).unwrap();
    complete(&fixture, &[BOLT], false).unwrap();
    assert!(
        fs::read_to_string(path)
            .unwrap()
            .contains("Coverage::Generated")
    );
}

#[test]
fn an_invalid_later_script_prevents_completion_of_every_selected_stub() {
    for script_failure in ["absent", "unknown", "unclaimed"] {
        let fixture = Fixture::new();
        add_stub(&fixture, BOLT, false);
        add_stub(&fixture, BOUNCE, true);
        match script_failure {
            "absent" => fs::remove_file(fixture.script(BOUNCE)).unwrap(),
            "unknown" => write(
                &fixture.script(BOUNCE),
                "Name:Unsummon\nTypes:Instant\nA:SP$ UnsupportedInstruction\n",
            ),
            _ => write(
                &fixture.script(BOUNCE),
                "Name:Unsummon\nTypes:Instant\nA:SP$ Draw | NumCards$ 1 | UnreadSemanticKey$ True\n",
            ),
        }
        rejects_selection_without_writes(&fixture);
    }
}

#[test]
fn a_later_source_changed_after_planning_prevents_the_first_write() {
    let fixture = Fixture::new();
    add_stub(&fixture, BOLT, false);
    add_stub(&fixture, BOUNCE, false);
    let first = fs::read(fixture.source(BOLT)).unwrap();
    let names = vec![BOLT.to_owned(), BOUNCE.to_owned()];
    let prepared = plan_with_mode(
        fixture.dir.path(),
        &fixture.cache,
        &fixture.scripts,
        &names,
        Mode::CompleteStub,
    )
    .unwrap();
    let changed = format!(
        "{}\n// changed by another author\n",
        fs::read_to_string(fixture.source(BOUNCE)).unwrap()
    );
    write(&fixture.source(BOUNCE), &changed);
    assert!(prepared.apply(false).is_err());
    assert_eq!(fs::read(fixture.source(BOLT)).unwrap(), first);
    assert_eq!(fs::read_to_string(fixture.source(BOUNCE)).unwrap(), changed);
}

#[test]
fn pending_completion_check_previews_without_changing_source_bytes() {
    let fixture = Fixture::new();
    add_stub(&fixture, BOLT, false);
    add_stub(&fixture, BOUNCE, true);
    let before = [
        fs::read(fixture.source(BOLT)).unwrap(),
        fs::read(fixture.source(BOUNCE)).unwrap(),
    ];
    assert!(complete(&fixture, &[BOLT, BOUNCE], true).is_err());
    assert_eq!(fs::read(fixture.source(BOLT)).unwrap(), before[0]);
    assert_eq!(fs::read(fixture.source(BOUNCE)).unwrap(), before[1]);
}

#[test]
fn pinned_payload_failures_abort_the_entire_completion_selection() {
    for mutation in ["absent", "printing", "oracle"] {
        let fixture = Fixture::new();
        add_stub(&fixture, BOLT, false);
        add_stub(&fixture, BOUNCE, false);
        let path = fixture.payload(BOUNCE);
        if mutation == "absent" {
            fs::remove_file(path).unwrap();
        } else {
            let mut payload: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            let key = if mutation == "printing" {
                "id"
            } else {
                "oracle_id"
            };
            payload[key] = serde_json::Value::String(BOLT_PRINT.to_owned());
            write(&path, payload.to_string());
        }
        rejects_selection_without_writes(&fixture);
    }
}

#[test]
fn a_name_cache_cannot_replace_the_stub_printing_with_a_new_default() {
    let fixture = Fixture::new();
    let path = add_stub(&fixture, BOLT, false);
    let mut payload: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.payload(BOLT)).unwrap()).unwrap();
    fs::remove_file(fixture.payload(BOLT)).unwrap();
    payload["id"] = serde_json::Value::String(BOUNCE_PRINT.to_owned());
    write(
        &fixture.cache.join("lightning_bolt.json"),
        payload.to_string(),
    );
    let before = fs::read(&path).unwrap();
    assert!(complete(&fixture, &[BOLT], false).is_err());
    assert_eq!(fs::read(path).unwrap(), before);
}

#[test]
fn stub_index_oracle_and_printing_header_must_agree_with_the_pinned_identity() {
    for mutation in ["index", "oracle", "header"] {
        let fixture = Fixture::new();
        add_stub(&fixture, BOLT, false);
        let path = add_stub(&fixture, BOUNCE, false);
        let source = fs::read_to_string(&path).unwrap();
        let changed = match mutation {
            "index" => source.replace("index::UNSUMMON", "index::LIGHTNING_BOLT"),
            "oracle" => {
                let ledger = ledger::IndexLedger::from_rows(&baylee_cards_index::ROWS).unwrap();
                source.replace(&ledger.entry_named(BOUNCE).unwrap().oracle_id, BOLT_PRINT)
            }
            _ => source.replacen(
                &format!("Scryfall ID: {BOUNCE_PRINT}"),
                &format!("Scryfall ID: {BOLT_PRINT}"),
                1,
            ),
        };
        assert_ne!(source, changed);
        write(&path, changed);
        rejects_selection_without_writes(&fixture);
    }
}

#[test]
fn completion_refuses_a_stub_outside_its_canonical_path() {
    let fixture = Fixture::new();
    let original = add_stub(&fixture, BOLT, false);
    let wrong = fixture
        .dir
        .path()
        .join("crates/baylee-cards/src/cards/instants/mv_2/lightning_bolt.rs");
    write(&wrong, fs::read(&original).unwrap());
    fs::remove_file(&original).unwrap();
    let before = fs::read(&wrong).unwrap();
    assert!(complete(&fixture, &[BOLT], false).is_err());
    assert_eq!(fs::read(wrong).unwrap(), before);
    assert!(!original.exists());
}

#[test]
fn hand_owned_unimplemented_sources_are_not_generated_stubs() {
    let fixture = Fixture::new();
    let path = add_stub(&fixture, BOLT, true);
    let source = fs::read_to_string(&path).unwrap();
    let adopted = stubgen::adopt_stub(&source).unwrap();
    write(&path, &adopted);
    assert!(complete(&fixture, &[BOLT], false).is_err());
    assert_eq!(fs::read_to_string(path).unwrap(), adopted);
}

#[test]
fn partial_generated_and_implemented_coverage_never_enter_the_stub_completion_route() {
    for coverage in [
        "Coverage::Partial(\"remaining clause\")",
        "Coverage::Generated",
        "Coverage::Implemented",
    ] {
        let fixture = Fixture::new();
        add_stub(&fixture, BOLT, false);
        let path = add_stub(&fixture, BOUNCE, true);
        let source = fs::read_to_string(&path).unwrap();
        write(&path, source.replace("Coverage::Unimplemented", coverage));
        rejects_selection_without_writes(&fixture);
    }
}

#[test]
fn completed_generator_output_cannot_be_completed_again_as_a_stub() {
    let fixture = Fixture::new();
    let path = add_stub(&fixture, BOLT, false);
    complete(&fixture, &[BOLT], false).unwrap();
    let generated = fs::read(&path).unwrap();
    assert!(complete(&fixture, &[BOLT], false).is_err());
    assert!(complete(&fixture, &[BOLT], true).is_err());
    assert_eq!(fs::read(path).unwrap(), generated);
}

#[test]
fn a_hold_for_an_unselected_stub_remains_stale_and_prevents_writes() {
    let fixture = Fixture::new();
    add_stub(&fixture, BOLT, false);
    add_stub(&fixture, BOUNCE, false);
    let ledger = ledger::IndexLedger::from_rows(&baylee_cards_index::ROWS).unwrap();
    write(
        &fixture.dir.path().join("data/card-test-holds.tsv"),
        format!(
            "{}\tfixture: this stub is outside the selected completion\n",
            ledger.entry_named(BOUNCE).unwrap().oracle_id
        ),
    );
    let before = [
        fs::read(fixture.source(BOLT)).unwrap(),
        fs::read(fixture.source(BOUNCE)).unwrap(),
    ];
    assert!(complete(&fixture, &[BOLT], false).is_err());
    assert_eq!(fs::read(fixture.source(BOLT)).unwrap(), before[0]);
    assert_eq!(fs::read(fixture.source(BOUNCE)).unwrap(), before[1]);
}

#[test]
fn duplicate_or_deceptive_stub_markers_never_authorize_completion() {
    for mutation in ["duplicate", "string", "nested_comment", "finished"] {
        let fixture = Fixture::new();
        let path = add_stub(&fixture, BOLT, false);
        let source = fs::read_to_string(&path).unwrap();
        let adopted = stubgen::adopt_stub(&source).unwrap();
        let changed = match mutation {
            "duplicate" => format!("{source}\n{}\n", stubgen::STUB_MARKER),
            "string" => format!(
                "{adopted}\nconst DECOY: &str = {:?};\n",
                stubgen::STUB_MARKER
            ),
            "nested_comment" => format!(
                "// unrelated comment quotes {}\n{adopted}",
                stubgen::STUB_MARKER
            ),
            _ => format!("{}\n{source}", stubgen::OWNED_MARKER),
        };
        write(&path, &changed);
        assert!(
            complete(&fixture, &[BOLT], false).is_err(),
            "accepted {mutation}"
        );
        assert_eq!(fs::read_to_string(path).unwrap(), changed);
    }
}

#[test]
fn a_single_stub_marker_does_not_authorize_two_root_card_macros() {
    let fixture = Fixture::new();
    let path = add_stub(&fixture, BOLT, false);
    let source = fs::read_to_string(&path).unwrap();
    let other_macro = stubgen::adopt_stub(&source).unwrap();
    let changed = format!("{source}\n{other_macro}");
    write(&path, &changed);
    assert!(complete(&fixture, &[BOLT], false).is_err());
    assert_eq!(fs::read_to_string(path).unwrap(), changed);
}

#[test]
fn repeating_a_selected_stub_name_does_not_plan_it_twice() {
    let fixture = Fixture::new();
    let path = add_stub(&fixture, BOLT, false);
    complete(&fixture, &[BOLT, BOLT], false).unwrap();
    assert!(
        fs::read_to_string(path)
            .unwrap()
            .contains("Coverage::Implemented")
    );
}

#[test]
fn complete_stub_cli_excludes_other_modes_but_accepts_repeat_names_and_check() {
    for mode in [
        vec!["--reclaim", BOUNCE],
        vec!["--tables"],
        vec!["--subtypes"],
        vec!["--adopt-stub", BOUNCE],
        vec![
            "--hold-back",
            BOUNCE,
            "--hold-back-reason",
            "fixture review",
        ],
        vec![
            "--hold-back-file",
            "names.txt",
            "--hold-back-reason",
            "fixture review",
        ],
    ] {
        let mut args = vec!["xtask", "codegen", "--complete-stub", BOLT];
        args.extend(mode);
        assert!(Cli::try_parse_from(args).is_err());
    }
    let parsed = Cli::try_parse_from([
        "xtask",
        "codegen",
        "--complete-stub",
        BOLT,
        "--complete-stub",
        BOUNCE,
        "--check",
    ])
    .unwrap();
    assert!(matches!(parsed.cmd, Cmd::Codegen { check: true, .. }));
}

#[cfg(unix)]
#[test]
fn a_symlink_at_the_canonical_stub_path_cannot_write_an_external_file() {
    let fixture = Fixture::new();
    let path = add_stub(&fixture, BOLT, false);
    let original = fs::read(&path).unwrap();
    let external = fixture.dir.path().join("outside-cards.rs");
    write(&external, &original);
    fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink(&external, &path).unwrap();
    assert!(complete(&fixture, &[BOLT], false).is_err());
    assert_eq!(fs::read(external).unwrap(), original);
}

#[cfg(unix)]
#[test]
fn replacing_a_planned_stub_with_a_symlink_prevents_every_write() {
    let fixture = Fixture::new();
    let first = add_stub(&fixture, BOLT, false);
    let later = add_stub(&fixture, BOUNCE, false);
    let first_original = fs::read(&first).unwrap();
    let later_original = fs::read(&later).unwrap();
    let names = vec![BOLT.to_owned(), BOUNCE.to_owned()];
    let prepared = plan_with_mode(
        fixture.dir.path(),
        &fixture.cache,
        &fixture.scripts,
        &names,
        Mode::CompleteStub,
    )
    .unwrap();
    let external = fixture.dir.path().join("outside-cards.rs");
    write(&external, &later_original);
    fs::remove_file(&later).unwrap();
    std::os::unix::fs::symlink(&external, &later).unwrap();
    assert!(prepared.apply(false).is_err());
    assert_eq!(fs::read(first).unwrap(), first_original);
    assert_eq!(fs::read(external).unwrap(), later_original);
}

#[test]
fn a_vanilla_script_completes_only_when_the_normal_renderer_also_reads_it_in_full() {
    for printed_rules in ["", "This still has a rule that is missing from the script."] {
        let fixture = Fixture::new();
        let path = add_stub(&fixture, BOLT, false);
        let mut payload: serde_json::Value =
            serde_json::from_slice(&fs::read(fixture.payload(BOLT)).unwrap()).unwrap();
        payload["oracle_text"] = serde_json::Value::String(printed_rules.to_owned());
        write(&fixture.payload(BOLT), payload.to_string());
        write(
            &fixture.script(BOLT),
            "Name:Lightning Bolt\nTypes:Instant\nManaCost:R\n",
        );
        let before = fs::read(&path).unwrap();
        let result = complete(&fixture, &[BOLT], false);
        assert_eq!(result.is_ok(), printed_rules.is_empty());
        if printed_rules.is_empty() {
            let generated = fs::read_to_string(path).unwrap();
            assert!(generated.contains(stubgen::OWNED_MARKER));
            assert!(generated.contains("Coverage::Implemented"));
        } else {
            assert_eq!(fs::read(path).unwrap(), before);
        }
    }
}

/// An ordinary full codegen run keeps a held stub held, with its finding, even
/// when the reader now takes the whole card. Lifting the hold is a reviewer's
/// act (`--complete-stub` above, or deleting the finding), never a side effect
/// of regenerating the pool, which is how 287 findings went missing and ten
/// held cards were finished unreviewed.
#[test]
fn a_full_run_keeps_a_held_stub_and_its_finding() {
    let fixture = Fixture::new();
    let path = add_stub(&fixture, BOLT, false);
    let held = fs::read_to_string(&path).unwrap();
    let printing: scryfall::ScryfallCard =
        serde_json::from_slice(&fs::read(fixture.payload(BOLT)).unwrap()).unwrap();
    let ledger = ledger::IndexLedger::from_rows(&baylee_cards_index::ROWS).unwrap();
    let row = ledger.entry_named(BOLT).unwrap();
    let cats = crate::hold_back::compiled_catalogs();
    let cycles = layout::LandCycles::parse("");
    let lookup = crate::scriptgen::ScriptLookup::new(
        fixture.scripts.clone(),
        crate::scripts::build_index(&fixture.scripts).unwrap(),
    );
    let full_run = |existing: Option<&str>| {
        stubgen::render_full_run(
            existing,
            &printing,
            row,
            &ledger,
            &cats,
            Some(&lookup),
            None,
            &cycles,
            &|_| false,
        )
        .unwrap()
        .1
    };
    // Without a hold the reader finishes the card: the hold is what stops it.
    let finished = full_run(None);
    assert!(finished.contains(stubgen::OWNED_MARKER), "{finished}");
    let kept = full_run(Some(&held));
    assert!(kept.contains(stubgen::STUB_MARKER), "{kept}");
    assert!(!kept.contains("abilities ="), "{kept}");
    assert_eq!(
        stubgen::held_back_reason(&kept).as_deref(),
        Some("synthetic fixture: the sentence is not yet read")
    );
}

//! Selective promotion of completed reader output after canonical play tests.

use super::{BOLT, BOUNCE, Cli, Cmd, Fixture, Parser, fs, stubgen};
use crate::reclaim::{Mode, planning::plan_with_mode};
use std::path::{Path, PathBuf};

fn generated(fixture: &Fixture, name: &str) -> PathBuf {
    let path = fixture.add(name);
    let source = fs::read_to_string(&path).unwrap();
    let source = source.replace("Coverage::Implemented", "Coverage::Generated");
    write_source(&path, format!("{}\n{source}", stubgen::OWNED_MARKER));
    path
}

fn write_source(path: &Path, source: impl AsRef<[u8]>) {
    super::write(path, source);
}

fn apply(fixture: &Fixture, names: &[&str], check: bool) -> anyhow::Result<()> {
    let names = names.iter().map(ToString::to_string).collect::<Vec<_>>();
    plan_with_mode(
        fixture.dir.path(),
        &fixture.cache,
        &fixture.scripts,
        &names,
        Mode::PromoteGenerated,
    )?
    .apply(check)
}

#[test]
fn promotion_rewrites_only_the_selected_generated_card() {
    let fixture = Fixture::new();
    let selected = generated(&fixture, BOLT);
    let sibling = generated(&fixture, BOUNCE);
    let sibling_before = fs::read(&sibling).unwrap();

    apply(&fixture, &[BOLT], false).unwrap();

    let output = fs::read_to_string(selected).unwrap();
    assert!(output.contains(stubgen::OWNED_MARKER));
    assert!(output.contains("coverage = Coverage::Implemented"));
    assert!(!output.contains("Coverage::Generated"));
    assert_eq!(fs::read(sibling).unwrap(), sibling_before);
}

#[test]
fn promotion_refuses_every_other_ownership_or_coverage_state() {
    for coverage in [
        "Coverage::Implemented",
        "Coverage::Partial(\"unread clause\")",
        "Coverage::Unimplemented",
    ] {
        let fixture = Fixture::new();
        let path = generated(&fixture, BOLT);
        let source = fs::read_to_string(&path).unwrap();
        write_source(&path, source.replace("Coverage::Generated", coverage));
        let before = fs::read(&path).unwrap();
        assert!(apply(&fixture, &[BOLT], false).is_err());
        assert_eq!(fs::read(path).unwrap(), before);
    }

    for stub in [false, true] {
        let fixture = Fixture::new();
        let path = generated(&fixture, BOLT);
        let source = fs::read_to_string(&path).unwrap();
        let source = if stub {
            source.replace(stubgen::OWNED_MARKER, stubgen::STUB_MARKER)
        } else {
            source.replace(&format!("{}\n", stubgen::OWNED_MARKER), "")
        };
        write_source(&path, source);
        let before = fs::read(&path).unwrap();
        assert!(apply(&fixture, &[BOLT], false).is_err());
        assert_eq!(fs::read(path).unwrap(), before);
    }
}

#[test]
fn missing_or_held_tests_prevent_every_promotion_write() {
    for held in [false, true] {
        let fixture = Fixture::new();
        let selected = generated(&fixture, BOLT);
        generated(&fixture, BOUNCE);
        if held {
            let ledger =
                baylee_cards_codegen::ledger::IndexLedger::from_rows(&baylee_cards_index::ROWS)
                    .unwrap();
            super::write(
                &fixture.dir.path().join("data/card-test-holds.tsv"),
                format!(
                    "{}\tfixture: rule verification is pending\n",
                    ledger.entry_named(BOUNCE).unwrap().oracle_id
                ),
            );
        } else {
            fs::remove_file(fixture.test(BOUNCE)).unwrap();
        }
        let before = fs::read(&selected).unwrap();
        assert!(apply(&fixture, &[BOLT, BOUNCE], false).is_err());
        assert_eq!(fs::read(selected).unwrap(), before);
    }
}

#[test]
fn promotion_check_previews_without_writing_and_refuses_an_already_promoted_source() {
    let fixture = Fixture::new();
    let path = generated(&fixture, BOLT);
    let before = fs::read(&path).unwrap();
    assert!(apply(&fixture, &[BOLT], true).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);

    apply(&fixture, &[BOLT], false).unwrap();
    let promoted = fs::read(&path).unwrap();
    assert!(apply(&fixture, &[BOLT], true).is_err());
    assert_eq!(fs::read(path).unwrap(), promoted);
}

#[test]
fn promotion_cli_rejects_other_codegen_modes_and_accepts_repeated_names_with_check() {
    for mode in [
        vec!["--tables"],
        vec!["--registry"],
        vec!["--reclaim", BOUNCE],
        vec!["--complete-stub", BOUNCE],
        vec!["--adopt-stub", BOUNCE],
        vec![
            "--hold-back",
            BOUNCE,
            "--hold-back-reason",
            "review fixture",
        ],
    ] {
        let mut args = vec!["xtask", "codegen", "--promote-generated", BOLT];
        args.extend(mode);
        assert!(Cli::try_parse_from(args).is_err());
    }
    let parsed = Cli::try_parse_from([
        "xtask",
        "codegen",
        "--promote-generated",
        BOLT,
        "--promote-generated",
        BOUNCE,
        "--check",
    ])
    .unwrap();
    assert!(matches!(parsed.cmd, Cmd::Codegen { check: true, .. }));
}

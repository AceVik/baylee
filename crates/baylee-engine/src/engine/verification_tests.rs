//! The card-verification hooks, tested the way their consumer runs them
//! (`docs/verification-hooks.md`): this crate's own test binary, started
//! again as a child process with `BAYLEE_ABILITY_LOG` or `BAYLEE_MUTATE` set
//! and a name filter, exactly as the documented invocation does.
//!
//! A child process rather than a switch inside this one, because both
//! variables are read once per process — the consumer runs one process per
//! mutant, and a test that flipped a value in-process would be testing
//! something the consumer never does.
//!
//! Also here: the pool inventory the L4 check is computed against, written
//! into the log directory when the recorder is on.

use super::testkit::card_index;
use crate::ability_log::{self, Kind, json_str};
use baylee_cards_dsl::{AbilityDef, CardDef, ReplacementRule};
use baylee_core::ids::AbilityRef;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

const BOLT: &str = "4457ed35-7c10-48c8-9776-456485fdf070";
const MOUNTAIN: &str = "a3fb7228-e76b-4e96-a40e-20b5fed75685";
const ANTHEM: &str = "e3886fe8-9b76-4613-8891-4ec74657c087";
const SORCERER: &str = "5e961d15-5972-4e4b-9385-1cd7cd7c6bbe";
const WARDEN: &str = "f3fad295-1af2-4ecc-8546-b121ad6be27b";
const SEASON: &str = "01546b7d-a233-4176-8843-d732074dc5b6";

const BOLT_TEST: &str =
    "engine::card_tests::instants::lightning_bolt_deals_three_to_a_creature_or_a_player";
const ANTHEM_TEST: &str = "engine::card_tests::enchantments::glorious_anthem_pumps_every_creature_its_controller_has_and_no_other";
const SORCERER_TEST: &str =
    "engine::card_tests::creatures::prodigal_sorcerer_taps_to_deal_one_damage_to_any_target";
const WARDEN_TEST: &str =
    "engine::card_tests::creatures::soul_warden_gains_life_for_another_creature_and_not_for_itself";
const SEASON_TEST: &str =
    "engine::card_rider_tests::doubling_season_doubles_a_walkers_starting_loyalty";

/// Runs `tests` in a fresh copy of this test binary with `env` set (or, for
/// `None`, removed), and returns its exit status and output.
///
/// Both variables are always spelled out, so a parent running under either
/// of them (the consumer's own mutant run) cannot leak it into a child.
fn run_child(tests: &[&str], env: &[(&str, Option<&str>)]) -> (Option<i32>, String) {
    let exe = std::env::current_exe().expect("the test binary knows its own path");
    let mut cmd = Command::new(exe);
    cmd.arg("--exact").args(tests).arg("--test-threads=1");
    cmd.env_remove(ability_log::VAR)
        .env_remove(baylee_cards::mutate::VAR);
    for (name, value) in env {
        match value {
            Some(value) => cmd.env(name, value),
            None => cmd.env_remove(name),
        };
    }
    let out = cmd.output().expect("the test binary starts");
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code(), text)
}

/// A directory of this test's own, emptied first.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("baylee-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

// --- L5: the mutation switch ----------------------------------------------------

/// Lightning Bolt's card test passes with the card as printed, fails with its
/// spell ability taken away (by `SPELL` and by its position, the two
/// spellings the contract accepts), and a mutant that names no ability stops
/// the process with `INVALID_MUTANT_EXIT` instead of passing or failing.
#[test]
fn a_mutant_of_a_cards_ability_fails_the_cards_own_test() {
    let bolt = card_index(BOLT).get();

    let (code, out) = run_child(&[BOLT_TEST], &[]);
    assert_eq!(code, Some(0), "unmutated, Bolt's own test passes:\n{out}");
    assert!(out.contains("1 passed"), "and it ran:\n{out}");

    for index in [AbilityRef::SPELL, 0] {
        let spec = format!("{bolt}:{index}");
        let (code, out) = run_child(&[BOLT_TEST], &[(baylee_cards::mutate::VAR, Some(&spec))]);
        assert_eq!(
            code,
            Some(101),
            "with {spec} Bolt deals no damage and its test fails:\n{out}"
        );
        assert!(out.contains("1 failed"), "one test ran and failed:\n{out}");
        assert!(
            out.contains("replaced by Unimplemented"),
            "the mutant says what it took away:\n{out}"
        );
    }

    let spec = format!("{bolt}:7");
    let (code, out) = run_child(&[BOLT_TEST], &[(baylee_cards::mutate::VAR, Some(&spec))]);
    assert_eq!(
        code,
        Some(baylee_cards::mutate::INVALID_MUTANT_EXIT),
        "Bolt has no ability 7, which is neither a survived nor a killed mutant:\n{out}"
    );
}

// --- L4: the firing recorder ----------------------------------------------------

/// The position of the first entry of `def`'s front face that `wanted`
/// accepts.
fn position_of(def: &CardDef, wanted: impl Fn(&AbilityDef) -> bool) -> u32 {
    let at = def
        .abilities_for_face(0)
        .iter()
        .position(wanted)
        .expect("the card prints it");
    u32::try_from(at).expect("a short list")
}

fn line(test: &str, card: &str, index: u32, kind: Kind) -> String {
    format!(
        "{{\"test\":\"{test}\",\"card\":{},\"index\":{index},\"kind\":\"{}\"}}",
        card_index(card).get(),
        kind.name()
    )
}

fn read_lines(dir: &Path, test: &str) -> Vec<String> {
    let path = dir.join(format!("{test}.jsonl"));
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} was not written: {e}", path.display()))
        .lines()
        .map(str::to_owned)
        .collect()
}

/// Five card tests, one of each kind of firing but the static and the
/// replacement a clone does, run with the recorder on: each writes its own
/// file, every line has the contract's shape, the ability that test is
/// about is among them, and no line is written twice.
///
/// Lightning Bolt is cast twice and two Mountains are tapped, which is two
/// resolutions of one spell ability and two activations of one mana ability:
/// one line each. The Mountains are tapped through CR 305.6's intrinsic
/// ability, and the line credits the printed "({T}: Add {R}.)" entry, which
/// is that ability written down.
#[test]
fn the_recorder_writes_each_firing_once_under_its_test() {
    let dir = scratch("ability-log");
    let dir_text = dir.to_str().expect("a UTF-8 temp dir");
    let tests = [
        BOLT_TEST,
        ANTHEM_TEST,
        SORCERER_TEST,
        WARDEN_TEST,
        SEASON_TEST,
    ];
    let (code, out) = run_child(&tests, &[(ability_log::VAR, Some(dir_text))]);
    assert_eq!(
        code,
        Some(0),
        "the five tests pass with the recorder on:\n{out}"
    );
    assert!(out.contains("5 passed"), "and all five ran:\n{out}");

    let season = baylee_cards::by_oracle_id(SEASON).expect("Doubling Season");
    let counters = position_of(season, |a| {
        matches!(
            a,
            AbilityDef::Replacement(ReplacementRule::DoubleCounterPlacement { .. })
        )
    });
    let expected = [
        (
            BOLT_TEST,
            line(BOLT_TEST, BOLT, AbilityRef::SPELL, Kind::Spell),
        ),
        (BOLT_TEST, line(BOLT_TEST, MOUNTAIN, 0, Kind::Mana)),
        (ANTHEM_TEST, line(ANTHEM_TEST, ANTHEM, 0, Kind::Static)),
        (
            SORCERER_TEST,
            line(SORCERER_TEST, SORCERER, 0, Kind::Activated),
        ),
        (WARDEN_TEST, line(WARDEN_TEST, WARDEN, 0, Kind::Triggered)),
        (
            SEASON_TEST,
            line(SEASON_TEST, SEASON, counters, Kind::Replacement),
        ),
    ];
    for (test, want) in &expected {
        let lines = read_lines(&dir, test);
        assert!(
            lines.contains(want),
            "{test} fired {want}, and its file says:\n{}",
            lines.join("\n")
        );
    }
    for test in tests {
        let lines = read_lines(&dir, test);
        let mut unique = lines.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), lines.len(), "{test} wrote a line twice");
        let prefix = format!("{{\"test\":\"{test}\",\"card\":");
        for l in &lines {
            assert!(l.starts_with(&prefix), "{l} is filed under {test}");
        }
    }
    let bolt_lines = read_lines(&dir, BOLT_TEST);
    assert_eq!(
        bolt_lines.len(),
        2,
        "two Bolts and two Mountains are one spell line and one mana line: {bolt_lines:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// --- the inventory --------------------------------------------------------------

/// A variant's name, for the inventory's `variant` field.
const fn variant(ability: &AbilityDef) -> &'static str {
    match ability {
        AbilityDef::Unimplemented => "Unimplemented",
        AbilityDef::Spell { .. } => "Spell",
        AbilityDef::Activated { .. } => "Activated",
        AbilityDef::Triggered { .. } => "Triggered",
        AbilityDef::Ward { .. } => "Ward",
        AbilityDef::Toxic { .. } => "Toxic",
        AbilityDef::ActivatedConditional { .. } => "ActivatedConditional",
        AbilityDef::SagaChapter { .. } => "SagaChapter",
        AbilityDef::Prepared { .. } => "Prepared",
        AbilityDef::Echo { .. } => "Echo",
        AbilityDef::Static(_) => "Static",
        AbilityDef::Replacement(_) => "Replacement",
        AbilityDef::ModalSpell { .. } => "ModalSpell",
        AbilityDef::Suspend { .. } => "Suspend",
        AbilityDef::CopyOnEnterUntilEot { .. } => "CopyOnEnterUntilEot",
        AbilityDef::CopyOnEnter { .. } => "CopyOnEnter",
        AbilityDef::Loyalty { .. } => "Loyalty",
        AbilityDef::ModalTriggered { .. } => "ModalTriggered",
    }
}

/// One card's row: every entry any of its faces answers with, as the
/// recorder would log it.
///
/// An `AbilityRef` names no face, so an entry is one `(index, kind)` and
/// lists the faces that hold it; a back face whose entry at the same
/// position is another kind is another entry.
fn inventory_row(def: &CardDef) -> String {
    let mut entries: Vec<(u32, u32, &'static str, Option<Kind>, Vec<usize>)> = Vec::new();
    for face in 0..def.faces.len().max(1) {
        for (at, ability) in def.abilities_for_face(face).iter().enumerate() {
            let position = u32::try_from(at).expect("a short list");
            let kind = Kind::of(ability);
            let index = if kind == Some(Kind::Spell) {
                AbilityRef::SPELL
            } else {
                position
            };
            if let Some(entry) = entries
                .iter_mut()
                .find(|e| e.0 == index && e.3 == kind && e.2 == variant(ability))
            {
                if !entry.4.contains(&face) {
                    entry.4.push(face);
                }
            } else {
                entries.push((index, position, variant(ability), kind, vec![face]));
            }
        }
    }
    let mut row = format!(
        "{{\"card\":{},\"name\":{},\"oracle_id\":\"{}\",\"implemented\":{},\"abilities\":[",
        def.index.get(),
        json_str(def.name()),
        def.oracle_id,
        def.is_implemented()
    );
    for (n, (index, position, variant, kind, faces)) in entries.iter().enumerate() {
        let kind = kind.map_or_else(|| "null".to_owned(), |k| format!("\"{}\"", k.name()));
        let faces: Vec<String> = faces.iter().map(ToString::to_string).collect();
        let _ = write!(
            row,
            "{}{{\"index\":{index},\"position\":{position},\"variant\":\"{variant}\",\"kind\":{kind},\"faces\":[{}]}}",
            if n == 0 { "" } else { "," },
            faces.join(",")
        );
    }
    row.push_str("]}");
    row
}

/// The whole pool as the L4 check needs it: one row per card, written to
/// `<dir>/pool-inventory.json` when the recorder is on.
///
/// Built and held to its shape on every run, so the file the consumer reads
/// is never produced by code that no test runs. What it asserts about the
/// pool is what the recorder depends on: Lightning Bolt's spell ability is
/// listed under `SPELL`, and every variant the recorder cannot log is
/// listed with a `null` kind rather than left out.
#[test]
fn pool_inventory() {
    let rows: Vec<String> = baylee_cards::all().map(inventory_row).collect();
    let bolt = card_index(BOLT).get();
    let bolt_row = rows
        .iter()
        .find(|r| r.starts_with(&format!("{{\"card\":{bolt},")))
        .expect("Bolt has a row");
    assert!(
        bolt_row.contains(&format!(
            "{{\"index\":{},\"position\":0,\"variant\":\"Spell\",\"kind\":\"spell\",\"faces\":[0]}}",
            AbilityRef::SPELL
        )),
        "{bolt_row}"
    );
    assert!(
        rows.iter()
            .any(|r| r.contains("\"variant\":\"Ward\",\"kind\":null")),
        "ward is in the pool and is listed as not logged"
    );
    if let Some(dir) = ability_log::dir() {
        let path = dir.join("pool-inventory.json");
        let text = format!("[\n{}\n]\n", rows.join(",\n"));
        std::fs::write(&path, text)
            .unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
    }
}

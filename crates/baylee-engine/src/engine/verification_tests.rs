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
use baylee_cards_dsl::{AbilityDef, CardDef, FaceDef, ReplacementRule};
use baylee_core::generated::subtypes::land;
use baylee_core::ids::AbilityRef;
use baylee_core::types::TypeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

const BOLT: &str = "4457ed35-7c10-48c8-9776-456485fdf070";
const MOUNTAIN: &str = "a3fb7228-e76b-4e96-a40e-20b5fed75685";
const ANTHEM: &str = "e3886fe8-9b76-4613-8891-4ec74657c087";
const SORCERER: &str = "5e961d15-5972-4e4b-9385-1cd7cd7c6bbe";
const WARDEN: &str = "f3fad295-1af2-4ecc-8546-b121ad6be27b";
const SEASON: &str = "01546b7d-a233-4176-8843-d732074dc5b6";

const BOLT_TEST: &str = "engine::card_tests::instants::mv_1::lightning_bolt::lightning_bolt_deals_three_to_a_creature_or_a_player";
const ANTHEM_TEST: &str = "engine::card_tests::enchantments::mv_3::glorious_anthem::glorious_anthem_pumps_every_creature_its_controller_has_and_no_other";
const SORCERER_TEST: &str = "engine::card_tests::creatures::mv_3::prodigal_sorcerer::prodigal_sorcerer_taps_to_deal_one_damage_to_any_target";
const WARDEN_TEST: &str = "engine::card_tests::creatures::mv_1::soul_warden::soul_warden_gains_life_for_another_creature_and_not_for_itself";
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

    // The survivor the inventory's `"intrinsic": true` exists for. Mountain's
    // "({T}: Add {R}.)" is the ability CR 305.6 gives every Mountain, and the
    // engine taps a land for its basic land types through that rule
    // (`casting::intrinsic_mana_offer`), not through the printed entry: Bolt
    // is still cast with the entry gone. The run also shows that a survivor
    // says its mutant was in place, so "passed" cannot mean "never applied".
    let spec = format!("{}:0", card_index(MOUNTAIN).get());
    let (code, out) = run_child(&[BOLT_TEST], &[(baylee_cards::mutate::VAR, Some(&spec))]);
    assert_eq!(code, Some(0), "{spec} survives Bolt's test:\n{out}");
    assert!(out.contains("1 passed"), "and the test ran:\n{out}");
    assert!(
        out.contains("replaced by Unimplemented"),
        "with the mutant in place:\n{out}"
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

/// Every name the recorder makes can be created on this system: a test's
/// path holds `::`, which Windows refuses in a file name, and there the
/// recorder writes `_` (`docs/verification-hooks.md`); everywhere else the
/// name is the test's own. Red on Windows when a name keeps a `:`.
#[test]
fn the_recorders_file_names_can_be_created_here() {
    let dir = scratch("ability-log-names");
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    for test in [BOLT_TEST, SEASON_TEST, ability_log::UNNAMED] {
        let name = ability_log::file_name(test);
        if cfg!(windows) {
            assert!(!name.contains(':'), "{name}");
        } else {
            assert_eq!(name, format!("{test}.jsonl"));
        }
        std::fs::write(dir.join(&name), b"{}").unwrap_or_else(|e| panic!("{name}: {e}"));
    }
    let _ = std::fs::remove_dir_all(&dir);
}

fn read_lines(dir: &Path, test: &str) -> Vec<String> {
    // The recorder's own name for it: on Windows `:` is `_`
    // (`docs/verification-hooks.md`).
    let path = dir.join(ability_log::file_name(test));
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

/// Whether `ability`, printed on `face`, is the mana ability CR 305.6 gives a
/// land for its basic land types, distinct from explicitly printed symbols.
///
/// The engine taps a land for its basic land types through that rule
/// (`casting::intrinsic_mana_offer`) whether the entry is there or not, so
/// replacing the entry takes nothing away and its L5 mutant survives by the
/// rules rather than by a hole in the tests. The recorder does credit the
/// entry when the land taps (it is the one ability, printed as reminder
/// text), so L4 holds it like any other.
fn intrinsic(face: Option<&FaceDef>, ability: &AbilityDef) -> bool {
    let Some(face) = face.filter(|f| f.types.contains(TypeSet::LAND)) else {
        return false;
    };
    [
        land::PLAINS,
        land::ISLAND,
        land::SWAMP,
        land::MOUNTAIN,
        land::FOREST,
    ]
    .into_iter()
    .any(|subtype| face.subtypes.contains(&subtype))
        && ability.is_intrinsic_mana_ability()
}

/// One entry of a card's row.
struct Entry {
    index: u32,
    position: u32,
    variant: &'static str,
    kind: Option<Kind>,
    intrinsic: bool,
    faces: Vec<usize>,
}

/// One card's row: every entry any of its faces answers with, as the
/// recorder would log it.
///
/// An `AbilityRef` names no face, so an entry is one `(index, kind)` and
/// lists the faces that hold it; a back face whose entry at the same
/// position is another kind is another entry.
fn inventory_row(def: &CardDef) -> String {
    let mut entries: Vec<Entry> = Vec::new();
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
                .find(|e| e.index == index && e.kind == kind && e.variant == variant(ability))
            {
                if !entry.faces.contains(&face) {
                    entry.faces.push(face);
                }
            } else {
                entries.push(Entry {
                    index,
                    position,
                    variant: variant(ability),
                    kind,
                    intrinsic: intrinsic(def.faces.get(face), ability),
                    faces: vec![face],
                });
            }
        }
    }
    // A Room numbers its abilities by the doors unlocked (CR 709.5):
    // `CardDef::door_abilities`. The contract says how to read that.
    let mut row = format!(
        "{{\"card\":{},\"name\":{},\"oracle_id\":\"{}\",\"implemented\":{},\"room\":{},\"abilities\":[",
        def.index.get(),
        json_str(def.name()),
        def.oracle_id,
        def.is_implemented(),
        def.has_shared_type_line()
    );
    for (n, e) in entries.iter().enumerate() {
        let kind = e
            .kind
            .map_or_else(|| "null".to_owned(), |k| format!("\"{}\"", k.name()));
        let faces: Vec<String> = e.faces.iter().map(ToString::to_string).collect();
        let _ = write!(
            row,
            "{}{{\"index\":{},\"position\":{},\"variant\":\"{}\",\"kind\":{kind},\"intrinsic\":{},\"faces\":[{}]}}",
            if n == 0 { "" } else { "," },
            e.index,
            e.position,
            e.variant,
            e.intrinsic,
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
            "{{\"index\":{},\"position\":0,\"variant\":\"Spell\",\"kind\":\"spell\",\"intrinsic\":false,\"faces\":[0]}}",
            AbilityRef::SPELL
        )),
        "{bolt_row}"
    );
    let mountain = card_index(MOUNTAIN).get();
    let mountain_row = rows
        .iter()
        .find(|r| r.starts_with(&format!("{{\"card\":{mountain},")))
        .expect("Mountain has a row");
    assert!(
        mountain_row.contains(
            "{\"index\":0,\"position\":0,\"variant\":\"Activated\",\"kind\":\"mana\",\"intrinsic\":true,\"faces\":[0]}"
        ),
        "a Mountain's red is CR 305.6's: {mountain_row}"
    );
    assert!(
        rows.iter()
            .any(|r| r.contains("\"variant\":\"Ward\",\"kind\":null")),
        "ward is in the pool and is listed as not logged"
    );
    assert!(
        rows.iter().any(|r| r.contains("\"room\":true")),
        "a Room is in the pool and says so"
    );
    if let Some(dir) = ability_log::dir() {
        let path = dir.join("pool-inventory.json");
        let text = format!("[\n{}\n]\n", rows.join(",\n"));
        std::fs::write(&path, text)
            .unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
    }
}

const TEMPORAL_MASTERY: &str = "5c58b8e6-c572-461e-893e-a8c05f20ba17";
const WARHAMMER: &str = "dba35ac5-7ad3-488a-a006-6b9a1d54eea5";
const BOTTOMLESS_VAULT: &str = "e43413e4-be17-49af-978a-26210d05f52a";
const MASTERY_TEST: &str = "engine::card_tests::sorceries::mv_7::temporal_mastery::temporal_mastery_takes_an_extra_turn_and_exiles_itself";
const WARHAMMER_TEST: &str = "engine::card_tests::artifacts::equipment::mv_3::loxodon_warhammer::loxodon_warhammer_pumps_and_arms_the_creature_it_holds_and_no_other";
const SILOS_TEST: &str = "engine::card_tests::lands::storage::bottomless_vault::a_storage_land_banks_a_counter_only_on_the_upkeeps_it_spent_tapped";

/// Three doors the recorder used to miss (TODO.md, "Verification-hook
/// findings"): a spell that exiles itself as it resolves left the stack
/// before `resolved` looked for it there (Temporal Mastery); an Equipment's
/// grant was noted under the timestamp it had before the attach gave it a
/// new one (Loxodon Warhammer's two statics); and "you may choose not to
/// untap" is read at the untap step, never by the projection (Bottomless Vault).
#[test]
fn the_recorder_logs_a_self_exiling_spell_an_attached_grant_and_an_untap_static() {
    let dir = scratch("ability-log-doors");
    let dir_text = dir.to_str().expect("a UTF-8 temp dir");
    let tests = [MASTERY_TEST, WARHAMMER_TEST, SILOS_TEST];
    let (code, out) = run_child(&tests, &[(ability_log::VAR, Some(dir_text))]);
    assert_eq!(
        code,
        Some(0),
        "the three tests pass with the recorder on:\n{out}"
    );
    assert!(out.contains("3 passed"), "and all three ran:\n{out}");
    let expected = [
        (
            MASTERY_TEST,
            line(
                MASTERY_TEST,
                TEMPORAL_MASTERY,
                AbilityRef::SPELL,
                Kind::Spell,
            ),
        ),
        (
            WARHAMMER_TEST,
            line(WARHAMMER_TEST, WARHAMMER, 1, Kind::Static),
        ),
        (
            WARHAMMER_TEST,
            line(WARHAMMER_TEST, WARHAMMER, 2, Kind::Static),
        ),
        (
            SILOS_TEST,
            line(SILOS_TEST, BOTTOMLESS_VAULT, 0, Kind::Static),
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
    let _ = std::fs::remove_dir_all(&dir);
}

const LICH: &str = "5b7515f2-7a5a-4e2a-9784-6cbacd768172";
const TIME_VAULT: &str = "99d4d99d-cf56-45aa-aa39-a250695612f2";
const ISLAND_SANCTUARY: &str = "7d1769d0-d942-45b3-a31c-2bbe45e68661";
const LICH_TEST: &str = "engine::card_tests::enchantments::lich::lich_put_into_a_graveyard_loses_the_game_even_above_zero_life";
const VAULT_TEST: &str = "engine::card_tests::artifacts::time_vault::skipping_a_turn_untaps_the_vault_as_the_next_turn_begins";
const SANCTUARY_TEST: &str = "engine::card_tests::enchantments::mv_2::island_sanctuary::island_sanctuary_yes_skips_the_turn_based_draw_and_makes_the_restriction";

/// Three more doors the recorder missed (Alpha's L4 measurement): a trigger
/// whose own effect ends its controller's game is taken off the stack as
/// they leave (CR 800.4a), before `resolved` looked for it there (Lich's
/// "you lose the game"); and the two skips offered as a turn or a draw
/// would begin (CR 614.10) are applied by the answer to a question, never
/// through the replacement funnel (Time Vault, Island Sanctuary).
#[test]
fn the_recorder_logs_a_game_ending_trigger_and_the_skips_a_question_applies() {
    let dir = scratch("ability-log-skips");
    let dir_text = dir.to_str().expect("a UTF-8 temp dir");
    let tests = [LICH_TEST, VAULT_TEST, SANCTUARY_TEST];
    let (code, out) = run_child(&tests, &[(ability_log::VAR, Some(dir_text))]);
    assert_eq!(
        code,
        Some(0),
        "the three tests pass with the recorder on:\n{out}"
    );
    assert!(out.contains("3 passed"), "and all three ran:\n{out}");
    let pool = |id| baylee_cards::by_index(card_index(id)).expect("in the pool");
    let dies = position_of(pool(LICH), |a| {
        matches!(a, AbilityDef::Triggered { effects, .. }
            if effects.contains(&baylee_cards_dsl::Effect::LoseGame))
    });
    let skip_turn = position_of(pool(TIME_VAULT), |a| {
        *a == AbilityDef::Replacement(ReplacementRule::SkipTurnToUntapSelf)
    });
    let skip_draw = position_of(pool(ISLAND_SANCTUARY), |a| {
        *a == AbilityDef::Replacement(ReplacementRule::MaySkipDrawStepDraw)
    });
    let expected = [
        (LICH_TEST, line(LICH_TEST, LICH, dies, Kind::Triggered)),
        (
            VAULT_TEST,
            line(VAULT_TEST, TIME_VAULT, skip_turn, Kind::Replacement),
        ),
        (
            SANCTUARY_TEST,
            line(
                SANCTUARY_TEST,
                ISLAND_SANCTUARY,
                skip_draw,
                Kind::Replacement,
            ),
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
    let _ = std::fs::remove_dir_all(&dir);
}

//! Offline fixtures for selective, preflighted reader reclamation.

mod complete_stub;

use super::plan;
use crate::{Cli, Cmd, Parser};
use baylee_cards_codegen::{ledger, scryfall, stubgen};
use std::{
    fs,
    path::{Path, PathBuf},
};

const BOLT: &str = "Lightning Bolt";
const BOUNCE: &str = "Unsummon";
const BOLT_PRINT: &str = "019a0000-0000-7000-8000-000000000001";
const BOUNCE_PRINT: &str = "019a0000-0000-7000-8000-000000000002";

struct Fixture {
    dir: tempfile::TempDir,
    cache: PathBuf,
    scripts: PathBuf,
}

struct CardFixture {
    print: &'static str,
    mana: &'static str,
    color: &'static str,
    oracle: &'static str,
    script: &'static str,
    ability: &'static str,
}

fn card(name: &str) -> CardFixture {
    match name {
        BOLT => CardFixture {
            print: BOLT_PRINT,
            mana: "{R}",
            color: "Red",
            oracle: "Lightning Bolt deals 3 damage to any target.",
            script: "A:SP$ DealDamage | NumDmg$ 3 | ValidTgts$ Any",
            ability: "spell!(&[Effect::DealDamage { amount: Amount::Fixed(3), target: TargetSpec::AnyTarget }], targets = Some(TargetReq::one(TargetSpec::AnyTarget)))",
        },
        BOUNCE => CardFixture {
            print: BOUNCE_PRINT,
            mana: "{U}",
            color: "Blue",
            oracle: "Return target creature to its owner's hand.",
            script: "A:SP$ ChangeZone | Destination$ Hand | ValidTgts$ Creature | Origin$ Battlefield",
            ability: "spell!(&[Effect::ReturnToHand { target: TargetSpec::Object(&Filter::CREATURE) }], targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))",
        },
        _ => panic!("unknown fixture card {name}"),
    }
}

fn write(path: &Path, bytes: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("cache");
        let scripts = dir.path().join("scripts");
        fs::create_dir_all(&cache).unwrap();
        fs::create_dir_all(&scripts).unwrap();
        write(&dir.path().join("data/land-cycles.tsv"), "");
        write(&dir.path().join("data/card-test-holds.tsv"), "");
        Self {
            dir,
            cache,
            scripts,
        }
    }

    fn source(&self, name: &str) -> PathBuf {
        self.dir.path().join(format!(
            "crates/baylee-cards/src/cards/instants/mv_1/{}.rs",
            stubgen::slug(name)
        ))
    }

    fn test(&self, name: &str) -> PathBuf {
        self.dir.path().join(format!(
            "crates/baylee-engine/src/engine/card_tests/instants/mv_1/{}.rs",
            stubgen::slug(name)
        ))
    }

    fn script(&self, name: &str) -> PathBuf {
        self.scripts
            .join(name[..1].to_ascii_lowercase())
            .join(format!("{}.txt", stubgen::slug(name)))
    }

    fn payload(&self, name: &str) -> PathBuf {
        scryfall::printing_cache_path(&self.cache, card(name).print)
    }

    fn add(&self, name: &str) -> PathBuf {
        let ledger = ledger::IndexLedger::from_rows(&baylee_cards_index::ROWS).unwrap();
        let row = ledger.entry_named(name).unwrap();
        let fixture = card(name);
        let source = format!(
            "//! {name} — {} — Instant\n\
             //! Oracle: {}\n\
             //! Set: TST #1 — Reclaim fixture | Scryfall ID: {} | Oracle ID: {}\n\n\
             use baylee_cards_dsl::prelude::*;\n\n\
             card!(index = index::{}, oracle_id = {:?}, scryfall_id = {:?}, \
             color_identity = ColorSet::of(Color::{}), coverage = Coverage::Implemented, \
             faces = &[face!(name = {name:?}, mana_cost = mana!({:?}), types = TypeSet::INSTANT)], \
             abilities = &[{}]);\n",
            fixture.mana,
            fixture.oracle,
            fixture.print,
            row.oracle_id,
            row.constant,
            row.oracle_id,
            fixture.print,
            fixture.color,
            fixture.mana,
            fixture.ability,
        );
        let path = self.source(name);
        write(&path, source);
        let payload = serde_json::json!({
            "id": fixture.print, "oracle_id": row.oracle_id, "name": name,
            "type_line": "Instant", "mana_cost": fixture.mana,
            "oracle_text": fixture.oracle, "layout": "normal", "image_status": "missing",
            "set": "tst", "set_name": "Reclaim fixture", "collector_number": "1",
            "color_identity": [if name == BOLT { "R" } else { "U" }],
        });
        write(&self.payload(name), payload.to_string());
        write(
            &self.script(name),
            format!(
                "Name:{name}\nTypes:Instant\nManaCost:{}\n{}\n",
                fixture.mana.trim_matches(['{', '}']),
                fixture.script,
            ),
        );
        write(
            &self.test(name),
            format!(
                "#[test]\nfn the_selected_card_is_played() {{ let _ = card_index({:?}); }}\n",
                row.oracle_id,
            ),
        );
        path
    }

    fn apply(&self, names: &[&str], check: bool) -> anyhow::Result<()> {
        let names = names.iter().map(ToString::to_string).collect::<Vec<_>>();
        plan(self.dir.path(), &self.cache, &self.scripts, &names)?.apply(check)
    }

    fn rejects_both_unchanged(&self) {
        let before = [
            fs::read(self.source(BOLT)).unwrap(),
            fs::read(self.source(BOUNCE)).unwrap(),
        ];
        assert!(self.apply(&[BOLT, BOUNCE], false).is_err());
        assert_eq!(fs::read(self.source(BOLT)).unwrap(), before[0]);
        assert_eq!(fs::read(self.source(BOUNCE)).unwrap(), before[1]);
    }
}

#[test]
fn reclamation_changes_only_the_explicitly_selected_card_source() {
    let f = Fixture::new();
    let selected = f.add(BOLT);
    let sibling = f.add(BOUNCE);
    let selected_before = fs::read(&selected).unwrap();
    let sibling_before = fs::read(&sibling).unwrap();
    let sentinels = [
        "crates/baylee-cards/src/cards/mod.rs",
        "crates/baylee-cards/src/generated.rs",
        "crates/baylee-cards/src/generated_lines.rs",
        "crates/baylee-cards/src/generated_names.rs",
        "crates/baylee-cards/src/generated_oracle.rs",
        "crates/baylee-cards/src/generated_sides.rs",
        "crates/baylee-cards/src/generated_tokens.rs",
        "crates/baylee-core/src/generated/index/mod.rs",
    ];
    for relative in sentinels {
        write(
            &f.dir.path().join(relative),
            "// untouched registry or table\n",
        );
    }
    f.apply(&[BOLT], false).unwrap();
    let output = fs::read_to_string(selected).unwrap();
    assert_ne!(output.as_bytes(), selected_before);
    assert!(output.contains(stubgen::OWNED_MARKER));
    assert!(output.contains("Coverage::Implemented"));
    assert!(output.contains(BOLT_PRINT));
    assert_eq!(fs::read(sibling).unwrap(), sibling_before);
    for relative in sentinels {
        assert_eq!(
            fs::read_to_string(f.dir.path().join(relative)).unwrap(),
            "// untouched registry or table\n"
        );
    }
}

#[test]
fn a_later_invalid_selection_prevents_every_planned_write() {
    let f = Fixture::new();
    f.add(BOLT);
    f.add(BOUNCE);
    write(
        &f.script(BOUNCE),
        "Name:Unsummon\nTypes:Instant\nA:SP$ UnreadableVerb\n",
    );
    f.rejects_both_unchanged();
}

#[test]
fn missing_or_mismatched_pinned_payloads_never_repin_a_selected_card() {
    for mutation in ["missing", "printing", "oracle"] {
        let f = Fixture::new();
        f.add(BOLT);
        f.add(BOUNCE);
        let path = f.payload(BOUNCE);
        if mutation == "missing" {
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
        f.rejects_both_unchanged();
    }
}

#[test]
fn a_matching_name_cache_may_supply_only_the_exact_pinned_printing() {
    for mismatch in [false, true] {
        let f = Fixture::new();
        f.add(BOLT);
        let mut payload: serde_json::Value =
            serde_json::from_slice(&fs::read(f.payload(BOLT)).unwrap()).unwrap();
        fs::remove_file(f.payload(BOLT)).unwrap();
        if mismatch {
            payload["id"] = serde_json::Value::String(BOUNCE_PRINT.to_owned());
        }
        write(&f.cache.join("lightning_bolt.json"), payload.to_string());
        let before = fs::read(f.source(BOLT)).unwrap();
        let result = f.apply(&[BOLT], false);
        assert_eq!(result.is_err(), mismatch);
        if mismatch {
            assert_eq!(fs::read(f.source(BOLT)).unwrap(), before);
        }
    }
}

#[test]
fn source_oracle_index_and_header_printing_are_authoritative() {
    for mutation in ["oracle", "index", "header"] {
        let f = Fixture::new();
        f.add(BOLT);
        f.add(BOUNCE);
        let path = f.source(BOUNCE);
        let source = fs::read_to_string(&path).unwrap();
        let changed = match mutation {
            "oracle" => {
                let ledger = ledger::IndexLedger::from_rows(&baylee_cards_index::ROWS).unwrap();
                source.replace(&ledger.entry_named(BOUNCE).unwrap().oracle_id, BOLT_PRINT)
            }
            "index" => source.replace("index::UNSUMMON", "index::LIGHTNING_BOLT"),
            _ => source.replacen(
                &format!("Scryfall ID: {BOUNCE_PRINT}"),
                &format!("Scryfall ID: {BOLT_PRINT}"),
                1,
            ),
        };
        assert_ne!(source, changed);
        write(&path, changed);
        f.rejects_both_unchanged();
    }
}

#[test]
fn an_index_decoy_outside_the_root_macro_does_not_supply_its_identity() {
    let f = Fixture::new();
    let path = f.add(BOLT);
    let source = fs::read_to_string(&path).unwrap();
    let changed = format!(
        "// index = index::LIGHTNING_BOLT\n{}",
        source.replace("index::LIGHTNING_BOLT", "index::UNSUMMON")
    );
    write(&path, &changed);
    assert!(f.apply(&[BOLT], false).is_err());
    assert_eq!(fs::read_to_string(path).unwrap(), changed);
}

#[test]
fn a_second_root_card_macro_is_rejected_without_writing() {
    let f = Fixture::new();
    let path = f.add(BOLT);
    let source = fs::read_to_string(&path).unwrap();
    let changed = format!("{source}\n{source}");
    write(&path, &changed);
    assert!(f.apply(&[BOLT], false).is_err());
    assert_eq!(fs::read_to_string(path).unwrap(), changed);
}

#[test]
fn an_existing_source_at_a_noncanonical_path_is_not_moved_or_reclaimed() {
    let f = Fixture::new();
    let original = f.add(BOLT);
    let wrong = f
        .dir
        .path()
        .join("crates/baylee-cards/src/cards/instants/mv_2/lightning_bolt.rs");
    write(&wrong, fs::read(&original).unwrap());
    fs::remove_file(&original).unwrap();
    let before = fs::read(&wrong).unwrap();
    assert!(f.apply(&[BOLT], false).is_err());
    assert_eq!(fs::read(wrong).unwrap(), before);
    assert!(!original.exists());
}

#[test]
fn absent_unknown_and_incompletely_read_scripts_prevent_every_write() {
    for kind in ["missing", "unknown", "unclaimed"] {
        let f = Fixture::new();
        f.add(BOLT);
        f.add(BOUNCE);
        match kind {
            "missing" => fs::remove_file(f.script(BOUNCE)).unwrap(),
            "unknown" => write(
                &f.script(BOUNCE),
                "Name:Unsummon\nTypes:Instant\nA:SP$ UnreadableVerb\n",
            ),
            _ => write(
                &f.script(BOUNCE),
                format!(
                    "Name:Unsummon\nTypes:Instant\n{} | InventedSemanticKey$ True\n",
                    card(BOUNCE).script
                ),
            ),
        }
        f.rejects_both_unchanged();
    }
}

#[test]
fn missing_or_held_canonical_tests_prevent_reclamation() {
    for held in [false, true] {
        let f = Fixture::new();
        f.add(BOLT);
        f.add(BOUNCE);
        if held {
            let ledger = ledger::IndexLedger::from_rows(&baylee_cards_index::ROWS).unwrap();
            write(
                &f.dir.path().join("data/card-test-holds.tsv"),
                format!(
                    "{}\tfixture: rule verification is pending\n",
                    ledger.entry_named(BOUNCE).unwrap().oracle_id
                ),
            );
        } else {
            fs::remove_file(f.test(BOUNCE)).unwrap();
        }
        f.rejects_both_unchanged();
    }
}

#[test]
fn only_implemented_hand_owned_sources_can_be_reclaimed() {
    for coverage in [
        "Coverage::Partial(\"unread clause\")",
        "Coverage::Generated",
        "Coverage::Unimplemented",
    ] {
        let f = Fixture::new();
        f.add(BOLT);
        f.add(BOUNCE);
        let path = f.source(BOUNCE);
        let source = fs::read_to_string(&path).unwrap();
        write(&path, source.replace("Coverage::Implemented", coverage));
        f.rejects_both_unchanged();
    }
    let f = Fixture::new();
    let path = f.add(BOLT);
    let source = fs::read_to_string(&path).unwrap();
    let stub = format!(
        "{}\n{}",
        stubgen::STUB_MARKER,
        source.replace("coverage = Coverage::Implemented,", "")
    );
    write(&path, &stub);
    assert!(f.apply(&[BOLT], false).is_err());
    assert_eq!(fs::read_to_string(path).unwrap(), stub);
}

#[test]
fn check_reports_pending_reclamation_but_preserves_every_source_byte() {
    let f = Fixture::new();
    f.add(BOLT);
    f.add(BOUNCE);
    let before = [
        fs::read(f.source(BOLT)).unwrap(),
        fs::read(f.source(BOUNCE)).unwrap(),
    ];
    assert!(f.apply(&[BOLT, BOUNCE], true).is_err());
    assert_eq!(fs::read(f.source(BOLT)).unwrap(), before[0]);
    assert_eq!(fs::read(f.source(BOUNCE)).unwrap(), before[1]);
}

#[test]
fn already_exact_generated_output_is_idempotent_for_apply_and_check() {
    let f = Fixture::new();
    let path = f.add(BOLT);
    let names = vec![BOLT.to_owned()];
    let prepared = plan(f.dir.path(), &f.cache, &f.scripts, &names).unwrap();
    prepared.apply(false).unwrap();
    let generated = fs::read(&path).unwrap();
    assert!(
        prepared.apply(false).is_err(),
        "an old plan must be rebuilt after any source change"
    );
    assert!(prepared.apply(true).is_err());
    f.apply(&[BOLT], false).unwrap();
    f.apply(&[BOLT], true).unwrap();
    assert_eq!(fs::read(path).unwrap(), generated);
}

#[test]
fn changed_later_source_invalidates_the_whole_plan_before_its_first_write() {
    let f = Fixture::new();
    f.add(BOLT);
    f.add(BOUNCE);
    let first = fs::read(f.source(BOLT)).unwrap();
    let names = vec![BOLT.to_owned(), BOUNCE.to_owned()];
    let prepared = plan(f.dir.path(), &f.cache, &f.scripts, &names).unwrap();
    let changed = format!(
        "{}\n// another writer changed this source\n",
        fs::read_to_string(f.source(BOUNCE)).unwrap()
    );
    write(&f.source(BOUNCE), &changed);
    assert!(prepared.apply(false).is_err());
    assert_eq!(fs::read(f.source(BOLT)).unwrap(), first);
    assert_eq!(fs::read_to_string(f.source(BOUNCE)).unwrap(), changed);
}

#[test]
fn reclaim_cli_rejects_other_codegen_modes_and_accepts_repeated_names_with_check() {
    for mode in [
        vec!["--tables"],
        vec!["--subtypes"],
        vec!["--adopt-stub", BOUNCE],
        vec![
            "--hold-back",
            BOUNCE,
            "--hold-back-reason",
            "review fixture",
        ],
    ] {
        let mut args = vec!["xtask", "codegen", "--reclaim", BOLT];
        args.extend(mode);
        assert!(Cli::try_parse_from(args).is_err());
    }
    let parsed = Cli::try_parse_from([
        "xtask",
        "codegen",
        "--reclaim",
        BOLT,
        "--reclaim",
        BOUNCE,
        "--check",
    ])
    .unwrap();
    assert!(matches!(parsed.cmd, Cmd::Codegen { check: true, .. }));
}

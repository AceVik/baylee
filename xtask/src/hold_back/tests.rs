use super::*;
use crate::{Cli, Parser};
use std::path::PathBuf;

const PRINT: &str = "00000000-0000-4000-8000-000000000001";
const REASON: &str = "review: unsupported clause changes legal play";

struct Fixture {
    dir: tempfile::TempDir,
    cache: PathBuf,
    cards: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("cache");
        let cards = dir.path().join("crates/baylee-cards/src/cards");
        fs::create_dir_all(&cache).unwrap();
        fs::create_dir_all(&cards).unwrap();
        fs::create_dir_all(dir.path().join("data")).unwrap();
        fs::write(dir.path().join("data/land-cycles.tsv"), "").unwrap();
        Self { dir, cache, cards }
    }

    fn add(&self, name: &str, type_line: &str) -> PathBuf {
        let ledger = ledger::IndexLedger::from_rows(&baylee_cards_index::ROWS).unwrap();
        let row = ledger.entry_named(name).unwrap();
        let source = format!(
            "// hand-owned review fixture\ncard!(index = index::{}, oracle_id = {:?}, scryfall_id = {PRINT:?}, coverage = Coverage::Partial(\"bad clause\"));\n",
            row.constant, row.oracle_id,
        );
        let path = self
            .cards
            .join(format!("{}.rs", crate::front_face_slug(name)));
        fs::write(&path, source).unwrap();
        let payload = serde_json::json!({
            "id": PRINT, "oracle_id": row.oracle_id, "name": name,
            "type_line": type_line, "mana_cost": "", "oracle_text": "",
            "layout": "normal", "image_status": "missing",
        });
        fs::write(
            self.cache.join(format!("{}.json", stubgen::slug(name))),
            payload.to_string(),
        )
        .unwrap();
        path
    }

    fn run(&self, names: &[&str], check: bool) -> anyhow::Result<()> {
        run(
            self.dir.path(),
            &self.cache,
            &names.iter().map(ToString::to_string).collect::<Vec<_>>(),
            None,
            REASON,
            check,
        )
    }
}

#[test]
fn hold_back_is_selective_and_suppresses_land_recognition() {
    let f = Fixture::new();
    let plains = f.add("Plains", "Basic Land — Plains");
    let sibling = f.add("Lightning Bolt", "Instant");
    let original = fs::read(&sibling).unwrap();
    fs::write(f.cards.join("mod.rs"), "// registry sentinel\n").unwrap();
    f.run(&["Plains"], false).unwrap();
    let stub = fs::read_to_string(&plains).unwrap();
    assert!(stub.contains(stubgen::STUB_MARKER));
    assert!(stub.contains(REASON));
    assert!(stub.contains(PRINT));
    assert!(!stub.contains("coverage ="), "default is Unimplemented");
    assert!(!stub.contains("abilities ="));
    assert!(!stub.contains("keywords ="));
    assert_eq!(fs::read(sibling).unwrap(), original);
    assert_eq!(
        fs::read_to_string(f.cards.join("mod.rs")).unwrap(),
        "// registry sentinel\n"
    );
    f.run(&["Plains"], true).unwrap();
}

#[test]
fn a_failed_selection_does_not_restore_earlier_cards() {
    let f = Fixture::new();
    let bolt = f.add("Lightning Bolt", "Instant");
    let original = fs::read(&bolt).unwrap();
    assert!(f.run(&["Lightning Bolt", "Plains"], false).is_err());
    assert_eq!(fs::read(bolt).unwrap(), original);
}

#[test]
fn check_reports_pending_restoration_without_writing() {
    let f = Fixture::new();
    let bolt = f.add("Lightning Bolt", "Instant");
    let original = fs::read(&bolt).unwrap();
    assert!(f.run(&["Lightning Bolt"], true).is_err());
    assert_eq!(fs::read(bolt).unwrap(), original);
}

#[test]
fn a_finished_machine_owned_card_must_be_fixed_in_its_reader() {
    let f = Fixture::new();
    let bolt = f.add("Lightning Bolt", "Instant");
    let source = format!(
        "{}\n{}",
        stubgen::OWNED_MARKER,
        fs::read_to_string(&bolt).unwrap()
    );
    for source in [
        source.clone(),
        format!("{}\n{source}", stubgen::STUB_MARKER),
    ] {
        fs::write(&bolt, &source).unwrap();
        let error = f.run(&["Lightning Bolt"], false).unwrap_err().to_string();
        assert!(error.contains("fix its reader"));
        assert_eq!(fs::read_to_string(&bolt).unwrap(), source);
    }
}

#[test]
fn a_different_default_printing_cannot_repin_the_card() {
    let f = Fixture::new();
    let bolt = f.add("Lightning Bolt", "Instant");
    let path = f.cache.join("lightning_bolt.json");
    let payload = fs::read_to_string(&path)
        .unwrap()
        .replace(PRINT, "00000000-0000-4000-8000-000000000002");
    fs::write(&path, payload).unwrap();
    let original = fs::read(&bolt).unwrap();
    let error = f.run(&["Lightning Bolt"], false).unwrap_err().to_string();
    assert!(error.contains("another printing"));
    assert_eq!(fs::read(bolt).unwrap(), original);
}

#[test]
fn a_names_file_skips_comments_and_deduplicates_selection() {
    let f = Fixture::new();
    let bolt = f.add("Lightning Bolt", "Instant");
    fs::write(
        f.dir.path().join("names.txt"),
        "# review slice\n\nLightning Bolt\nLightning Bolt\n",
    )
    .unwrap();
    run(
        f.dir.path(),
        &f.cache,
        &[],
        Some(Path::new("names.txt")),
        REASON,
        false,
    )
    .unwrap();
    assert!(fs::read_to_string(bolt).unwrap().contains(REASON));
}

#[test]
fn cli_requires_a_reason_and_rejects_other_codegen_modes() {
    assert!(Cli::try_parse_from(["xtask", "codegen", "--hold-back", "Plains"]).is_err());
    assert!(Cli::try_parse_from(["xtask", "codegen", "--hold-back-reason", REASON]).is_err());
    for mode in ["--tables", "--subtypes"] {
        assert!(
            Cli::try_parse_from([
                "xtask",
                "codegen",
                "--hold-back",
                "Plains",
                "--hold-back-reason",
                REASON,
                mode
            ])
            .is_err()
        );
    }
    assert!(
        Cli::try_parse_from([
            "xtask",
            "codegen",
            "--hold-back",
            "Plains",
            "--hold-back",
            "Lightning Bolt",
            "--hold-back-reason",
            REASON,
            "--check"
        ])
        .is_ok()
    );
}

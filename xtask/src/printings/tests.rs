//! Offline checks with two printings whose reminder text differs.

use crate::adopt::{refresh_oracle, set_header_line};
use crate::check_text::check_oracle_matches_the_printing;
use crate::validate::PrintingTally;
use baylee_cards_codegen::{layout, scryfall};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

const NAME: &str = "Fixture Armor";
const SLUG: &str = "fixture_armor";
const PINNED: &str = "019a0000-0000-7000-8000-000000000001";
const REPRINT: &str = "019a0000-0000-7000-8000-000000000002";
const ORACLE_ID: &str = "019a0000-0000-7000-8000-000000000003";
const PINNED_ORACLE: &str = "Equip {3} ({3}: Attach to target creature you control.)";
const REPRINT_ORACLE: &str =
    "Equip {3} ({3}: Attach to target creature you control. Equip only as a sorcery.)";

struct Fixture {
    root: tempfile::TempDir,
    card: PathBuf,
}

fn write(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn printing(id: &str, oracle_text: &str, collector_number: &str) -> Value {
    json!({
        "id": id,
        "oracle_id": ORACLE_ID,
        "name": NAME,
        "oracle_text": oracle_text,
        "set": "tst",
        "set_name": "Fixture Set",
        "collector_number": collector_number,
    })
}

fn source(oracle: &str) -> String {
    let set = set_header_line(&printing(PINNED, PINNED_ORACLE, "1")).unwrap();
    format!(
        "//! {NAME} — {{1}} — Artifact — Equipment\n\
         //! Oracle: {oracle}\n\
         {}\n\n\
         // The implementation below the header.\n",
        set.trim_end()
    )
}

impl Fixture {
    fn new(oracle: &str, have_pinned: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        write(&root.path().join("data/acceptance-decks.txt"), "");
        write(&root.path().join("data/card-pool.txt"), NAME);
        let cache = root.path().join("data/scryfall-cache");
        write(
            &cache.join(format!("{SLUG}.json")),
            printing(REPRINT, REPRINT_ORACLE, "2").to_string(),
        );
        if have_pinned {
            write(
                &scryfall::printing_cache_path(&cache, PINNED),
                printing(PINNED, PINNED_ORACLE, "1").to_string(),
            );
        }
        let card = root
            .path()
            .join(layout::SETS_DIR)
            .join(format!("{SLUG}.rs"));
        write(&card, source(oracle));
        Self { root, card }
    }

    fn refresh(&self) -> String {
        refresh_oracle(self.root.path(), false).expect("refresh succeeds against the cache");
        fs::read_to_string(&self.card).unwrap()
    }

    fn validate_oracle(&self) -> (usize, usize) {
        let root = self.root.path();
        let mut tally = PrintingTally::default();
        let mut problems = 0;
        let default = super::cached_printing(root, NAME).unwrap();
        let contents = fs::read_to_string(&self.card).unwrap();
        check_oracle_matches_the_printing(
            root,
            SLUG,
            &contents,
            &default,
            &mut tally,
            &mut problems,
        );
        (tally.oracle, problems)
    }
}

#[test]
fn refresh_keeps_or_restores_the_pinned_oracle_text() {
    for oracle in [PINNED_ORACLE, REPRINT_ORACLE] {
        let fixture = Fixture::new(oracle, true);
        assert_eq!(fixture.refresh(), source(PINNED_ORACLE));
    }
}

#[test]
fn refresh_keeps_the_header_when_the_pinned_printing_is_missing() {
    let fixture = Fixture::new(PINNED_ORACLE, false);
    assert_eq!(fixture.refresh(), source(PINNED_ORACLE));
}

#[test]
fn validation_accepts_the_pinned_oracle_and_rejects_the_reprint_text() {
    for (oracle, expected_problems) in [(PINNED_ORACLE, 0), (REPRINT_ORACLE, 1)] {
        let fixture = Fixture::new(oracle, true);
        assert_eq!(
            fixture.validate_oracle(),
            (1, expected_problems),
            "Oracle header: {oracle}"
        );
    }
}

#[test]
fn validation_reports_the_missing_pinned_printing_without_comparing_the_reprint() {
    let fixture = Fixture::new(REPRINT_ORACLE, false);
    assert_eq!(fixture.validate_oracle(), (0, 1));
}

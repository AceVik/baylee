use super::*;
use flate2::{Compression, write::GzEncoder};
use std::io::Write;
#[test]
fn catalog_uses_only_published_bulk_urls_and_prefers_traditional_draft() -> Result<()> {
    let entries=[("Sealed",format!("{SOURCE_PREFIX}sealed.csv.gz")),("TradDraft",format!("{SOURCE_PREFIX}draft.csv.gz")),("TradDraft","https://private.invalid/other.csv.gz".to_owned())].map(|(mode,url)|json!({"expansion":[{"text":"TEST"}],"format":[{"text":mode}],"game_data":[{"spans":[{"data":{"url":url}}]}]}));
    let sources = catalog_sources(&json!({"results":[{"data":{"datasets":entries}}]}))?;
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].mode, "TradDraft");
    assert_eq!(sources[0].url, format!("{SOURCE_PREFIX}draft.csv.gz"));
    Ok(())
}
fn fixture(csv: &str) -> Result<(tempfile::TempDir, Options)> {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    let cards = root.join("cards.json");
    files::atomic_json(
        &cards,
        &json!({"working_pool_hash":"pool","cards":[{"id":7,"name":"Forest","faces":["Forest"],"held_out":false},{"id":42,"name":"Spell","faces":["Spell"],"held_out":false},{"id":99,"name":"Held","faces":["Held"],"held_out":true}]}),
    )?;
    let catalog = root.join("catalog.json");
    files::atomic_json(
        &catalog,
        &json!({"results":[{"data":{"datasets":[{"expansion":[{"text":"TEST"}],"format":[{"text":"TradDraft"}],"game_data":[{"spans":[{"data":{"url":format!("{SOURCE_PREFIX}fixture.csv.gz")}}]}]}]}}]}),
    )?;
    let mut gzip = GzEncoder::new(
        File::create(root.join("fixture.csv.gz"))?,
        Compression::default(),
    );
    gzip.write_all(csv.as_bytes())?;
    gzip.finish()?;
    let options = Options {
        cards,
        out: root.join("out"),
        rows_per_source: 100,
        decks_per_source: 100,
        sets: String::new(),
        catalog: Some(catalog),
        bulk_dir: Some(root.to_path_buf()),
    };
    Ok((directory, options))
}
#[test]
fn complete_decks_preserve_counts_privacy_and_family_holdouts() -> Result<()> {
    let (_directory, options) = fixture(
        "\u{feff}draft_id,player,deck_Forest,deck_Spell,deck_Held,deck_Unknown\nprivate-family,private-player,20,20,0,0\nprivate-family,private-player,10,20,10,0\nother,private-player,20,20,0,0\nother,private-player,21,19,0,0\nother,private-player,20,19,0,1\nother,private-player,-1,41,0,0\n",
    )?;
    let manifest = import(&options)?;
    let decks = manifest["decks"].as_array().unwrap();
    assert_eq!(decks.len(), 2);
    for deck in decks {
        assert_eq!(deck["split"], "test");
        assert_eq!(deck["holdout_tier"], "unseen_cards_family");
        assert_eq!(deck["main_count"], 40);
        assert_eq!(deck["is_derived"], false);
    }
    assert!(!serde_json::to_string(&manifest)?.contains("private-"));
    let text = std::fs::read_to_string(decks[0]["path"].as_str().unwrap())?;
    assert!(text.contains("20 Forest\n20 Spell\n"));
    assert!(!text.contains("private-"));
    let report = files::read_json(&options.out.join("import-report.json"))?;
    for key in [
        "duplicate",
        "near_duplicate",
        "unsupported_card",
        "invalid_size",
    ] {
        assert_eq!(report[0]["counts"][key], 1);
    }
    Ok(())
}
#[test]
fn budgets_low_coverage_and_source_failures_are_explicit() -> Result<()> {
    let (_directory, mut options) =
        fixture("draft_id,deck_Forest,deck_Spell\na,20,20\nb,bad,20\n")?;
    options.rows_per_source = 1;
    assert_eq!(import(&options)?["decks"].as_array().unwrap().len(), 1);
    options.out = options.out.with_file_name("failure");
    options.rows_per_source = 10;
    assert_eq!(import(&options)?["decks"].as_array().unwrap().len(), 1);
    assert!(
        files::read_json(&options.out.join("import-report.json"))?[0]["error"]
            .as_str()
            .unwrap()
            .contains("invalid digit")
    );
    let (_directory, options) = fixture("deck_Forest,deck_Unknown,deck_Other\n40,0,0\n")?;
    assert_eq!(import(&options)?["decks"].as_array().unwrap().len(), 0);
    assert_eq!(
        files::read_json(&options.out.join("import-report.json"))?[0]["counts"]["skipped_low_coverage_source"],
        1
    );
    Ok(())
}

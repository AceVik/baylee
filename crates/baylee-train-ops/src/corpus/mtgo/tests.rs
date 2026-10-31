use super::*;
use flate2::{Compression, write::GzEncoder};
use std::io::Cursor;
fn archive(source: &std::path::Path, document: &Value) -> Result<()> {
    std::fs::create_dir_all(source)?;
    std::fs::write(source.join("LICENSE"), b"GPL-3.0 fixture\n")?;
    std::fs::write(source.join("revision.txt"), b"pinned-revision\n")?;
    let encoder = GzEncoder::new(
        File::create(source.join("source.tar.gz"))?,
        Compression::default(),
    );
    let mut tar = tar::Builder::new(encoder);
    let bytes = serde_json::to_vec(document)?;
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(
        &mut header,
        "repo/Tournaments/MTGO/modern-fixture.json",
        Cursor::new(bytes),
    )?;
    tar.into_inner()?.finish()?;
    Ok(())
}
fn catalog(path: &std::path::Path) -> Result<()> {
    files::atomic_json(
        path,
        &json!({"working_pool_hash":"pool","existing_decks":[{"main":{"777":60}}],"cards":[{"id":7,"name":"Forest","faces":["Forest"],"held_out":false},{"id":42,"name":"Example Spell","faces":["Example Spell"],"held_out":false},{"id":99,"name":"Held Card","faces":["Held Card"],"held_out":true}]}),
    )
}
#[test]
fn complete_supported_decks_keep_counts_provenance_and_held_out_sideboards() -> Result<()> {
    let root = tempfile::tempdir()?;
    let source = root.path().join("source");
    let cards = root.path().join("cards.json");
    catalog(&cards)?;
    let main = json!([{"CardName":"forest","Count":24},{"CardName":"Example Spell","Count":36}]);
    archive(
        &source,
        &json!({"Tournament":{"Date":"2026-10-28"},"Decks":[{"Player":"private gamer","Mainboard":main,"Sideboard":[{"CardName":"Held Card","Count":1}]},{"Mainboard":main,"Sideboard":[]},{"Mainboard":[{"CardName":"Forest","Count":60}]},{"Mainboard":[{"CardName":"not supported","Count":60}]},{"Mainboard":[{"CardName":"Forest","Count":-1}]},{"Mainboard":[{"CardName":"Forest","Count":56},{"CardName":"Gap One","Count":4}],"Sideboard":[{"CardName":"Gap Two","Count":2},{"CardName":"not supported","Count":1}]}]}),
    )?;
    let out = root.path().join("out");
    let near = root.path().join("near-complete.json");
    let manifest = import(&Options {
        source,
        cards,
        out: out.clone(),
        max_per_format: 1000,
        similarity: 0.85,
        mainboard_only: false,
        near_complete_out: Some(near.clone()),
    })?;
    let decks = manifest["decks"].as_array().unwrap();
    assert_eq!(decks.len(), 1);
    let deck = &decks[0];
    assert_eq!(deck["main_count"], 60);
    assert_eq!(deck["sideboard_count"], 1);
    assert_eq!(deck["split"], "test");
    assert_eq!(deck["holdout_tier"], "unseen_cards");
    assert_eq!(deck["source_revision"], "pinned-revision");
    assert_eq!(deck["source_date"], "2026-10-28");
    assert_eq!(deck["is_derived"], false);
    let content = std::fs::read_to_string(deck["path"].as_str().unwrap())?;
    assert!(content.contains("24 Forest\n36 Example Spell\n"));
    assert!(content.contains("[sideboard]\n1 Held Card\n"));
    assert!(!content.contains("private gamer"));
    assert!(!serde_json::to_string(&manifest)?.contains("private gamer"));
    let report = files::read_json(&out.join("import-report.json"))?;
    for key in ["duplicate", "all_basic_placeholder", "invalid_count"] {
        assert_eq!(report["counts"][key], 1, "{key}");
    }
    // Two lists the pool cannot play: one misses one card, the other three
    // across mainboard and sideboard; both are near complete.
    assert_eq!(report["counts"]["unsupported_card"], 2);
    assert_eq!(report["missing_card_histogram"], json!({"1": 1, "3": 1}));
    assert_eq!(
        report["near_complete_unlocks"],
        json!([["not supported", 2], ["Gap One", 1], ["Gap Two", 1]])
    );
    assert_eq!(
        report["unsupported_card_lists"],
        report["near_complete_unlocks"]
    );
    // Only the list of valid size is kept for substitution, unchanged, with
    // every missing card named; the all-unsupported 60-card list is one card
    // (60 copies) and passes too.
    let near = files::read_json(&near)?;
    assert_eq!(near["contract"], "mtgo_near_complete_v1");
    let kept = near["decks"].as_array().unwrap();
    assert_eq!(kept.len(), 2);
    let gaps = kept
        .iter()
        .find(|deck| deck["missing"].as_array().unwrap().len() == 3)
        .unwrap();
    assert_eq!(gaps["main"], json!([["Forest", 56], ["Gap One", 4]]));
    assert_eq!(
        gaps["sideboard"],
        json!([["Gap Two", 2], ["not supported", 1]])
    );
    assert_eq!(
        gaps["missing"],
        json!(["Gap One", "Gap Two", "not supported"])
    );
    Ok(())
}
#[test]
fn unsupported_sideboard_rejects_whole_deck_unless_projection_is_explicit() -> Result<()> {
    let root = tempfile::tempdir()?;
    let source = root.path().join("source");
    let cards = root.path().join("cards.json");
    catalog(&cards)?;
    archive(
        &source,
        &json!({"decks":[{"mainboard":[{"card_name":"Example Spell","count":60}],"sideboard":[{"card_name":"unsupported","count":1}]}]}),
    )?;
    for projection in [false, true] {
        let manifest = import(&Options {
            source: source.clone(),
            cards: cards.clone(),
            out: root
                .path()
                .join(if projection { "projected" } else { "strict" }),
            max_per_format: 1000,
            similarity: 0.85,
            mainboard_only: projection,
            near_complete_out: None,
        })?;
        assert_eq!(
            manifest["decks"].as_array().unwrap().len(),
            usize::from(projection)
        );
        if projection {
            assert_eq!(manifest["decks"][0]["is_derived"], true);
            assert_eq!(manifest["decks"][0]["sideboard_count"], 0);
        }
    }
    Ok(())
}
#[test]
fn unknown_card_or_invalid_count_rejects_the_entire_multiset() {
    let lookup = BTreeMap::from([(
        "forest".to_owned(),
        Card {
            id: 7,
            name: "Forest".into(),
            held: false,
        },
    )]);
    let Ok(deck) = read_cards(
        &json!([{"CardName":"Missing B","Count":4},{"CardName":"Forest","Count":20},
            {"CardName":"Missing A","Count":4},{"CardName":"Missing B","Count":1}]),
        &lookup,
    ) else {
        panic!("valid counts read");
    };
    assert_eq!(
        deck.missing.into_iter().collect::<Vec<_>>(),
        [("Missing A".to_owned(), 4), ("Missing B".to_owned(), 5)],
        "every distinct missing card is named with its count, not only the first"
    );
    assert_eq!(deck.cards, BTreeMap::from([(7, 20)]));
    for count in [json!(-1), json!(true), json!(101)] {
        assert!(read_cards(&json!([{"CardName":"Forest","Count":count}]), &lookup).is_err());
    }
}

#[test]
fn full_unicode_casefold_preserves_the_original_name_matching_contract() {
    assert_eq!(fold("Straße ﬃ ς"), "strasse ffi σ");
    let lookup = BTreeMap::from([(
        "forest".to_owned(),
        Card {
            id: 7,
            name: "Forest".into(),
            held: false,
        },
    )]);
    assert!(read_cards(&json!([{"CardName":"Foreſt","Count":60}]), &lookup).is_ok());
}

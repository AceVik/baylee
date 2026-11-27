//! Provision the catalog and agent for an end-to-end ranked game.
use baylee_catalog::{Catalog, scryfall};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

/// Advertise one legal card in all three formats and attach a real engine agent.
pub async fn provision(gateway: &super::Gateway) -> tokio::task::JoinHandle<()> {
    seed(gateway).await;
    super::attach_agent(gateway).await
}

/// Populate the authoritative catalog without supplying an execution agent:
/// Forest legal in all three formats, and, where `BAYLEE_E2E_AI_DECKS`
/// names a deck directory (the AI players' test with a real runner), every
/// card of every deck version in it legal in its deck's format, so the host
/// admits the decks a real pilot brings.
pub async fn seed(gateway: &super::Gateway) {
    let catalog = Catalog::connect(&gateway.database_url()).await.unwrap();
    let forest = baylee_cards::by_index(baylee_cards::decks::by_name("Forest").unwrap()).unwrap();
    let mut cards = vec![scryfall::Card {
        id: Uuid::now_v7().to_string(),
        oracle_id: Some(forest.oracle_id.into()),
        lang: "en".into(),
        set: "tst".into(),
        collector_number: "1".into(),
        name: "Forest".into(),
        type_line: Some("Basic Land — Forest".into()),
        legalities: ["standard", "vintage", "commander"]
            .map(|key| (key.into(), "legal".into()))
            .into(),
        ..Default::default()
    }];
    if let Some(dir) = std::env::var_os("BAYLEE_E2E_AI_DECKS") {
        cards.extend(deck_cards(std::path::Path::new(&dir), forest.oracle_id));
    }
    catalog.upsert(&cards).await.unwrap();
    catalog.bump_data_version().await.unwrap();
}

/// Every card of the deck versions in `dir` (`<dir>/<format>/vNNNN-….txt`)
/// but `forest`, each legal in the catalog's key for the formats its decks
/// are of (Highlander is judged by Vintage's verdicts, as the host judges
/// it).
fn deck_cards(dir: &std::path::Path, forest: &str) -> Vec<scryfall::Card> {
    use baylee_account::ranked::Format;
    let mut legal: BTreeMap<&'static str, (String, BTreeSet<&'static str>)> = BTreeMap::new();
    for (format, key) in [
        (Format::Standard, "standard"),
        (Format::Highlander, "vintage"),
        (Format::Commander, "commander"),
    ] {
        for version in baylee_seat::deckbuild::history::versions(dir, format) {
            let deck = baylee_seat::deck::Deck::from_file(&version.deck)
                .unwrap_or_else(|e| panic!("{}: {e:#}", version.deck.display()));
            let rows = deck
                .cards
                .iter()
                .chain(&deck.sideboard)
                .filter_map(|line| baylee_core::deckrow::parse(line).ok().map(|row| row.name));
            for name in rows.chain(deck.commanders.iter().cloned()) {
                let Some(index) = baylee_cards::decks::by_name(&name) else {
                    continue;
                };
                let Some(def) = baylee_cards::by_index(index) else {
                    continue;
                };
                if def.oracle_id == forest {
                    continue;
                }
                let whole = baylee_cards::decks::whole_name(index).map_or(name, str::to_owned);
                legal
                    .entry(def.oracle_id)
                    .or_insert_with(|| (whole, BTreeSet::new()))
                    .1
                    .insert(key);
            }
        }
    }
    legal
        .into_iter()
        .enumerate()
        .map(|(at, (oracle, (name, keys)))| scryfall::Card {
            id: Uuid::now_v7().to_string(),
            oracle_id: Some(oracle.into()),
            lang: "en".into(),
            set: "tst".into(),
            collector_number: (at + 2).to_string(),
            name,
            legalities: keys
                .into_iter()
                .map(|key| (key.into(), "legal".into()))
                .collect(),
            ..Default::default()
        })
        .collect()
}

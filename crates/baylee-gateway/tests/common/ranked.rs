//! Provision the catalog and agent for an end-to-end ranked game.
use baylee_catalog::{Catalog, scryfall};
use uuid::Uuid;

/// Advertise one legal card in all three formats and attach a real engine agent.
pub async fn provision(gateway: &super::Gateway) -> tokio::task::JoinHandle<()> {
    let catalog = Catalog::connect(&gateway.database_url()).await.unwrap();
    let forest = baylee_cards::by_index(baylee_cards::decks::by_name("Forest").unwrap()).unwrap();
    catalog
        .upsert(&[scryfall::Card {
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
        }])
        .await
        .unwrap();
    catalog.bump_data_version().await.unwrap();
    super::attach_agent(gateway).await
}

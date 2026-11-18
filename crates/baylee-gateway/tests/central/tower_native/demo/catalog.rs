//! Real Commander starters; the same reviewed content as an operator publication.
use super::super::{Identity, Principal, Store, Uuid};
use baylee_tower::v2::package::Cohort;
use baylee_tower_store::Publication;

pub(in super::super) fn publication(identity: &Identity) -> Publication {
    let (content, generation) = if identity.rules == "tower-6" {
        (
            baylee_tower_content::runic().unwrap(),
            baylee_tower::v2::generation::RUNIC,
        )
    } else {
        (
            baylee_tower_content::standard().unwrap(),
            baylee_tower::v2::generation::RUN,
        )
    };
    Publication {
        cohort: Cohort::for_generation(identity.clone(), &content.catalog, generation).unwrap(),
        catalog: content.catalog,
        starters: content.starters,
    }
}

pub(in super::super) async fn publish_house(
    store: &Store,
    publisher: &Principal,
    cohort: Uuid,
    publication: &Publication,
) {
    let demo = super::enabled();
    for stage in 1..=if demo { 12 } else { 1 } {
        for (index, starter) in publication.starters.iter().enumerate() {
            let build = baylee_tower_content::development_house(starter, stage);
            store
                .publish_house(
                    publisher,
                    cohort,
                    stage,
                    u8::try_from(index + 1).unwrap(),
                    &build,
                )
                .await
                .unwrap();
        }
    }
}

#[test]
fn playable_demo_starters_use_tested_cards_and_exact_legal_collections() {
    let identity = Identity::native(
        baylee_protocol::execution::Variant::Tower,
        "fixture",
        "fixture",
        23,
    );
    let publication = publication(&identity);
    publication.validate().unwrap();
    baylee_tower_content::validate_catalog(&publication.catalog).unwrap();
    assert_eq!(publication.starters.len(), 50);
    for starter in &publication.starters {
        starter.validate(&publication.catalog).unwrap();
        assert!(starter.formation.is_some());
        assert_eq!(
            starter
                .deck
                .iter()
                .map(|row| u16::from(row.copies))
                .sum::<u16>(),
            39
        );
        assert!(starter.owned.len() >= 23);
    }
}

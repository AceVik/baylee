//! `cards/sorceries/mv_5/ordered_migration.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_cards_dsl::{Color, ColorSet};
use baylee_core::generated::subtypes;

const P0: PlayerId = PlayerId::new(0);

fn ordered_migration() -> CardIndex {
    card_index("ee0a59ba-8d34-43d5-b9a5-42015588e3d5")
}

/// Ordered Migration (ee0a59ba-8d34-43d5-b9a5-42015588e3d5) — {3}{W}{U} Sorcery:
/// "Domain — Create a 1/1 blue Bird creature token with flying for each basic
/// land type among lands you control."
#[test]
fn ordered_migration_creates_one_flying_bird_per_controlled_basic_land_type() {
    // Plains + Plains + Island + Badlands (Swamp Mountain) + Abraded Bluffs
    // = 4 distinct basic land types (Plains, Island, Swamp, Mountain); P1's Forest excluded.
    let mut engine = lk::game(
        SEED,
        &[plains(), plains(), island(), badlands(), abraded_bluffs()],
        &[forest()],
        &[ordered_migration()],
        &[],
    );

    cast_from_hand(&mut engine, P0, ordered_migration());
    lk::settle(&mut engine);

    let birds = tokens_of(&engine, P0);
    assert_eq!(birds.len(), 4);
    for bird in birds {
        assert_eq!(pt(&engine, bird), (1, 1));
        assert!(keywords(&engine, bird).contains(KeywordSet::FLYING));
        let ch = engine.state().object(bird).unwrap().characteristics();
        assert_eq!(ch.colors, ColorSet::from_slice(&[Color::Blue]));
        assert!(ch.subtypes.contains(subtypes::creature::BIRD));
    }
}

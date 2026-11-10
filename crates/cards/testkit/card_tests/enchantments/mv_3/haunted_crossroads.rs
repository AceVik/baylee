//! Tests for Haunted Crossroads.

#[allow(clippy::wildcard_imports)]
use super::*;

// oracle_id = "e9aac98a-6fad-463e-89fa-3ad3a7c7f53a"
fn haunted_crossroads() -> CardIndex {
    card_index("e9aac98a-6fad-463e-89fa-3ad3a7c7f53a")
}

/// {B}: Put target creature card from your graveyard on top of your library.
#[test]
fn haunted_crossroads_puts_target_creature_card_from_your_graveyard_on_top_of_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, gray_ogre())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[haunted_crossroads()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    seed_graveyard(&mut engine, p0, 1);

    cast_from_hand(&mut engine, p0, haunted_crossroads());
    pass_until(&mut engine, stack_is_empty);

    let crossroads = on_battlefield(&engine, p0, haunted_crossroads()).expect("Haunted Crossroads");
    let ogre_in_gy = in_graveyard(&engine, p0, gray_ogre()).expect("Gray Ogre in graveyard");

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: crossroads,
                ability_index: 0,
            },
        )
        .unwrap();
    aim_at(&mut engine, p0, ogre_in_gy);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, gray_ogre()).is_none());
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .expect("library non-empty");
    assert_eq!(
        engine
            .state()
            .object(top)
            .and_then(|o| o.card)
            .map(|c| c.index),
        Some(gray_ogre())
    );
}

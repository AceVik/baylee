//! `cards/instants/mv_5/get_the_point.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "57c704bc-8dff-4b90-b4e1-38d12e41b3d1"
fn get_the_point() -> CardIndex {
    card_index("57c704bc-8dff-4b90-b4e1-38d12e41b3d1")
}

/// Get the Point: "Destroy target creature. Scry 1."
///
/// The scry is asked of the caster over exactly one card, the top of the
/// library; sending it to the bottom is then read off the library itself.
#[test]
fn get_the_point_destroys_a_creature_and_scries_one() {
    let mut engine = table(
        920,
        &[get_the_point()],
        &[swamp(), swamp(), swamp(), swamp(), mountain()],
        &[],
        &[gray_ogre()],
    );
    let ogre = on_battlefield(&engine, them(), gray_ogre()).expect("their Ogre");
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(me()))
        .last()
        .expect("a library");

    cast_from_hand(&mut engine, me(), get_the_point());
    aim_at(&mut engine, me(), ogre);
    pass_until(&mut engine, |e| matches!(e.pending(), Pending::Arrange { .. }));
    let Pending::Arrange { player, cards, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, me(), "the caster scries");
    assert_eq!(cards, vec![top], "one card, the top of the library");
    assert!(
        in_graveyard(&engine, them(), gray_ogre()).is_some(),
        "the destruction came first"
    );
    engine
        .apply(me(), look_answer(&cards, &cards))
        .expect("the card to the bottom");
    settle(&mut engine);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(me())).first(),
        Some(&top),
        "the scried card is now on the bottom"
    );
}

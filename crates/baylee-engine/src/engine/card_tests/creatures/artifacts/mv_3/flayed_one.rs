//! `cards/creatures/artifacts/mv_3/flayed_one.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn flayed_one() -> CardIndex {
    card_index("899415fd-4164-42a5-a0c7-eef1967c5ec5")
}

/// Flayed One — "When this creature enters, mill three cards": its
/// controller's own top three go to their own graveyard.
#[test]
fn flayed_one_mills_its_controller_three_on_entering() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = main_with(&[flayed_one()], &[swamp(), swamp(), swamp()]);
    let top_three: Vec<ObjectId> = library_of(&engine, p0).iter().rev().take(3).copied().collect();
    let library_before = library_of(&engine, p0).len();
    let theirs_before = library_of(&engine, p1).len();

    cast_and_settle(&mut engine, flayed_one());

    let graveyard = engine.state().zones.list(ZoneLocation::Graveyard(p0)).clone();
    assert_eq!(graveyard.len(), 3);
    for card in top_three {
        assert!(graveyard.contains(&card), "the top three were milled");
    }
    assert_eq!(library_of(&engine, p0).len(), library_before - 3);
    assert_eq!(library_of(&engine, p1).len(), theirs_before, "not the opponent's");
    assert!(on_battlefield(&engine, p0, flayed_one()).is_some());
}

/// Lifelink: its 4 combat damage to the opposing player is 4 life for its
/// controller.
#[test]
fn flayed_one_gains_life_equal_to_the_damage_it_deals() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[], &[flayed_one()]);
    let one = on_battlefield(&engine, p0, flayed_one()).expect("seated");
    assert_eq!(pt(&engine, one), (4, 1));
    assert!(keywords(&engine, one).contains(KeywordSet::LIFELINK));

    attack_to_blocks(&mut engine, &[one]);
    engine
        .apply(PlayerId::new(1), PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(life_of(&engine, 1), 16, "four damage");
    assert_eq!(life_of(&engine, 0), 24, "four life from lifelink");
}

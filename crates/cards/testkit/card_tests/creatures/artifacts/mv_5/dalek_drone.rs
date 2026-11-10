//! `cards/creatures/artifacts/mv_5/dalek_drone.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn dalek_drone() -> CardIndex {
    card_index("95d018f4-7f97-4b2c-abb4-cef69031caa1")
}

/// Dalek Drone — flying, menace, and "When this creature enters, destroy
/// target creature an opponent controls. That player loses 3 life." Only the
/// opponent's creature is offered, it dies and its controller loses 3.
#[test]
fn dalek_drone_destroys_an_opponents_creature_and_drains_three() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[dalek_drone()])
        .battlefield(
            0,
            &[swamp(), swamp(), swamp(), swamp(), swamp(), wild_elephant()],
        )
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mine = on_battlefield(&engine, p0, wild_elephant()).expect("mine");
    let theirs = on_battlefield(&engine, p1, gray_ogre()).expect("theirs");
    let life = life_of(&engine, 1);

    cast_from_hand(&mut engine, p0, dalek_drone());
    let options = pass_until_targets(&mut engine, p0);
    assert!(options.contains(&theirs));
    assert!(!options.contains(&mine), "\"an opponent controls\"");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the opponent's creature is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, gray_ogre()).is_some(),
        "destroyed"
    );
    assert_eq!(life_of(&engine, 1), life - 3, "that player loses 3 life");
    assert!(
        on_battlefield(&engine, p0, wild_elephant()).is_some(),
        "mine is untouched"
    );
    let drone = on_battlefield(&engine, p0, dalek_drone()).expect("the Drone is out");
    let kw = keywords(&engine, drone);
    assert!(kw.contains(KeywordSet::FLYING) && kw.contains(KeywordSet::MENACE));
    assert_eq!(pt(&engine, drone), (3, 3));
}

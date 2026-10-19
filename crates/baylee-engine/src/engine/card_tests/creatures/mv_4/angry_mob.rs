//! `cards/creatures/mv_4/angry_mob.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn angry_mob() -> CardIndex {
    card_index("967c2c79-7e49-448d-8d30-b2eb08b21d0a")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// p0: the Mob and a Swamp of its own; p1: two Swamps and an Island.
fn board() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[angry_mob(), swamp()])
        .battlefield(1, &[swamp(), swamp(), island()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, PlayerId::new(0)),
        "p0 reaches its own main"
    );
    engine
}

/// Angry Mob (967c2c79-7e49-448d-8d30-b2eb08b21d0a) — "Trample / During your
/// turn, Angry Mob's power and toughness are each equal to 2 plus the number
/// of Swamps your opponents control. During turns other than yours, Angry
/// Mob's power and toughness are each 2."
#[test]
fn angry_mob_is_2_plus_the_opponents_swamps_on_your_turn_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board();
    let mob = on_battlefield(&engine, p0, angry_mob()).expect("the Mob is out");
    assert!(keywords(&engine, mob).contains(KeywordSet::TRAMPLE));
    assert_eq!(
        pt(&engine, mob),
        (4, 4),
        "2 plus two opposing Swamps; its own Swamp and their Island do not count"
    );

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, mob), (2, 2), "2/2 on the opponent's turn");

    assert!(walk_to_own_main(&mut engine, p0), "p0's next own main");
    assert_eq!(
        pt(&engine, mob),
        (4, 4),
        "and back on its controller's turn"
    );
}

/// The count follows the board: an opposing Swamp that dies takes a point
/// off both numbers.
#[test]
fn angry_mob_shrinks_when_an_opposing_swamp_leaves() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board();
    let mob = on_battlefield(&engine, p0, angry_mob()).expect("the Mob is out");
    let swamp_id = on_battlefield(&engine, p1, swamp()).expect("their Swamp is out");
    assert_eq!(pt(&engine, mob), (4, 4));
    kill(&mut engine, swamp_id);
    assert_eq!(pt(&engine, mob), (3, 3), "one opposing Swamp left");
}

/// Swamps the controller owns never count, however many.
#[test]
fn angry_mob_ignores_its_controllers_own_swamps() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[angry_mob(), swamp(), swamp(), swamp(), grizzly_bears()],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let mob = on_battlefield(&engine, p0, angry_mob()).expect("the Mob is out");
    assert_eq!(pt(&engine, mob), (2, 2), "no opposing Swamp: just the 2");
}

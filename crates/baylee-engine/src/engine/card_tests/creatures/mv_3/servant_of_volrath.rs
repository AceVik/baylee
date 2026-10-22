//! `cards/creatures/mv_3/servant_of_volrath.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{BOOMERANG, GRIZZLY_BEARS};

fn servant_of_volrath() -> CardIndex {
    card_index("4e389b61-7b31-42ba-a19b-2502f63fd34a")
}

/// `Servant of Volrath` ({2}{B}, 3/3 Creature — Minion, `Coverage::Implemented`):
/// `When this creature leaves the battlefield, sacrifice a creature.`
#[test]
fn servant_of_volrath_forces_controller_to_sacrifice_a_creature_when_it_leaves() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[servant_of_volrath(), GRIZZLY_BEARS])
        .battlefield(1, &[island(), island()])
        .hand(1, &[BOOMERANG])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let servant = on_battlefield(&engine, p0, servant_of_volrath()).unwrap();
    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p1, BOOMERANG);
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![servant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, servant_of_volrath()).is_none());
    assert!(on_battlefield(&engine, p0, GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, p0, GRIZZLY_BEARS).is_some());
}

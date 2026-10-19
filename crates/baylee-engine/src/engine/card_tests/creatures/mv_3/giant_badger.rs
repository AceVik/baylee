//! `cards/creatures/mv_3/giant_badger.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::{Phase, Step};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn giant_badger() -> CardIndex {
    card_index("a4109138-5d69-4510-a228-7c1f88c0c8c0")
}

/// Turn one: P0's Gray Ogre (2/2) attacks into P1's Giant Badger (2/2).
/// Returns the engine at the declare-blockers step, blockers not yet declared.
fn ogre_attacks_into_badger() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gray_ogre()])
        .battlefield(1, &[giant_badger()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let ogre = on_battlefield(&engine, P0, gray_ogre()).expect("the Ogre is out");
    let badger = on_battlefield(&engine, P1, giant_badger()).expect("the Badger is out");
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogre, Defender::Player(P1))],
            },
        )
        .expect("the Ogre attacks");
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });
    (engine, ogre, badger)
}

/// Giant Badger (a4109138-5d69-4510-a228-7c1f88c0c8c0) — "Whenever this
/// creature blocks, it gets +2/+2 until end of turn." Blocking makes it a 4/4
/// that survives and kills a 2/2.
#[test]
fn giant_badger_gets_plus_two_plus_two_when_it_blocks() {
    let (mut engine, ogre, badger) = ogre_attacks_into_badger();
    assert_eq!(pt(&engine, badger), (2, 2), "2/2 before blocking");
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(badger, ogre)],
            },
        )
        .expect("the Badger blocks");
    assert!(
        !stack_is_empty(&engine),
        "the block trigger is on the stack"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, badger), (4, 4), "+2/+2 once it blocks");

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        on_battlefield(&engine, P1, giant_badger()).is_some(),
        "two damage does not kill a 4/4"
    );
    assert_eq!(engine.state().object(badger).expect("the Badger").damage, 2);
    assert!(
        on_battlefield(&engine, P0, gray_ogre()).is_none(),
        "four damage kills the 2/2 Ogre"
    );
    assert_eq!(engine.state().players[1].life, 20, "it was blocked");

    // Until end of turn only.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Upkeep
    });
    assert_eq!(pt(&engine, badger), (2, 2), "the bonus wore off");
}

/// Not blocking, no bonus: the trigger is "whenever this creature blocks".
#[test]
fn giant_badger_gets_nothing_when_it_does_not_block() {
    let (mut engine, _ogre, badger) = ogre_attacks_into_badger();
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no block");
    assert!(stack_is_empty(&engine), "nothing triggered");
    assert_eq!(pt(&engine, badger), (2, 2));
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 18);
}

//! `cards/creatures/mv_3/noble_benefactor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{LIGHTNING_BOLT, NOBLE_BENEFACTOR};

/// `Noble Benefactor` ({2}{U}, Creature — Human Cleric 2/2, Partial) lets each
/// player search their library for a card and put it into their hand when it
/// dies.
#[test]
fn noble_benefactor_lets_each_player_search_library_on_death() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[NOBLE_BENEFACTOR, mountain()])
        .hand(0, &[LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand0_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let hand1_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    let benefactor = on_battlefield(&engine, p0, NOBLE_BENEFACTOR).unwrap();
    cast_from_hand(&mut engine, p0, LIGHTNING_BOLT);
    aim_at(&mut engine, p0, benefactor);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards for p0");
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards for p1");
    };
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, NOBLE_BENEFACTOR).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand0_before
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        hand1_before + 1
    );
}

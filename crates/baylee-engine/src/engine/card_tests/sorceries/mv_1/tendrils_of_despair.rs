//! `cards/sorceries/mv_1/tendrils_of_despair.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::TENDRILS_OF_DESPAIR;

/// `Tendrils of Despair` ({B}, Sorcery) sacrifices a creature as an additional
/// cost and makes target opponent discard two cards.
#[test]
fn tendrils_of_despair_sacrifices_creature_and_makes_opponent_discard_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), llanowar_elves()])
        .hand(0, &[TENDRILS_OF_DESPAIR])
        .hand(1, &[swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand1_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let swamp_id = on_battlefield(&engine, p0, swamp()).unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: swamp_id })
        .unwrap();
    cast_with_floating(&mut engine, p0, TENDRILS_OF_DESPAIR);
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards for p1 discard");
    };
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![options[0], options[1]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        hand1_before - 2
    );
}

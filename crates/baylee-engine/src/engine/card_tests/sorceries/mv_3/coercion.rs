//! `cards/sorceries/mv_3/coercion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn coercion() -> CardIndex {
    card_index("68413337-ddb9-46f8-8c8e-f3d2d672c652")
}

#[test]
fn coercion_reveals_target_opponents_hand_and_discards_chosen_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[coercion()])
        .hand(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let opp_bear = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(p1))
        .iter()
        .copied()
        .find(|&id| {
            engine
                .state()
                .object(id)
                .unwrap()
                .card
                .is_some_and(|c| c.index == GRIZZLY_BEARS)
        })
        .unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, coercion());
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![opp_bear],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, GRIZZLY_BEARS).is_some());
}

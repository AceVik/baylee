//! `cards/sorceries/mv_1/imperial_seal.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Imperial Seal: "Search your library for a card, then shuffle and put that card on top. You lose 2 life."
/// Cast off a Swamp, the sorcery prompts a search of the library and places the chosen card on top.
/// Upon resolution, the player's life total is reduced by two from 20 to 18.
#[test]
fn imperial_seal_tutors_card_to_top_and_costs_two_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(134, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[imperial_seal()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, imperial_seal());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected search choice, got {:?}", engine.pending());
    };
    let found = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .last()
            .copied(),
        Some(found),
        "chosen card was placed on top of library"
    );
    assert_eq!(engine.state().players[0].life, 18, "player lost 2 life");
}

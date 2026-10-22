//! `cards/artifacts/mv_2/anvil_of_bogardan.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn anvil_of_bogardan() -> CardIndex {
    card_index("9deb2a0b-f40c-4d13-8321-41dc41448d24")
}

#[test]
fn anvil_of_bogardan_grants_no_max_hand_size_and_triggers_draw_then_discard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[anvil_of_bogardan()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, anvil_of_bogardan());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, anvil_of_bogardan()).is_some());

    // Advance to p1's draw step; Anvil of Bogardan triggers and asks p1 to discard a card.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!();
    };
    let discard = options[0];
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![discard],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        1
    );
}

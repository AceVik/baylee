//! `cards/instants/mv_3/harrow.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn harrow() -> CardIndex {
    card_index("705509e9-a034-4a5a-9c65-66f58748b8a2")
}

/// Harrow (`705509e9-a034-4a5a-9c65-66f58748b8a2`) — {2}{G} Instant:
/// "As an additional cost to cast this spell, sacrifice a land. Search your library for up to two basic land cards, put them onto the battlefield, then shuffle."
#[test]
fn harrow_sacrifices_a_land_and_puts_two_basic_lands_onto_the_battlefield() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[harrow()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let land_to_sac = on_battlefield(&engine, P0, forest()).unwrap();
    cast_from_hand(&mut engine, P0, harrow());
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![land_to_sac],
                },
            )
            .unwrap();
    }

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected library search prompt");
    };
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0], options[1]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Started with 3 Forests, sacrificed 1, put 2 onto the battlefield => 4 Forests on the battlefield.
    let forests_count = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|&&id| engine.state().object(id).unwrap().controller == P0)
        .count();
    assert_eq!(forests_count, 4);
}

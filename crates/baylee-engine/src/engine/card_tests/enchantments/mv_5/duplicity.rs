//! `cards/enchantments/mv_5/duplicity.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn duplicity() -> CardIndex {
    card_index("01072a45-ab1c-4d19-b493-76cf4aae351e")
}

/// Duplicity (`01072a45-ab1c-4d19-b493-76cf4aae351e`) — {3}{U}{U} Enchantment (Partial):
/// "When this enchantment enters, exile the top five cards of your library face down. ... At the beginning of your end step, discard a card."
#[test]
fn duplicity_exiles_five_face_down_on_etb_and_discards_at_end_step() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[duplicity(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let lib_before = engine.state().zones.list(ZoneLocation::Library(P0)).len();
    cast_from_hand(&mut engine, P0, duplicity());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(P0)).len(),
        lib_before - 5
    );
    assert_eq!(engine.state().zones.list(ZoneLocation::Exile(P0)).len(), 5);
    for id in engine.state().zones.list(ZoneLocation::Exile(P0)) {
        assert!(
            engine
                .state()
                .object(*id)
                .unwrap()
                .status
                .contains(Status::FACE_DOWN)
        );
    }

    let hand_before_end = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected end-step discard prompt");
    };
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before_end - 1
    );
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    let before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    reach_their_main_phase(&mut engine, P0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        before + 1
    );
}

//! `cards/artifacts/mv_2/scroll_rack.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn scroll_rack() -> CardIndex {
    card_index("648bd03d-247e-4d6f-87a9-12a67688cbf5")
}

/// `Scroll Rack` ({2}, Artifact, `Coverage::Partial`):
/// activates `{1}, {T}` to draw a card and put a card from your hand on top of
/// your library.
#[test]
fn scroll_rack_activates_to_draw_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), scroll_rack()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, scroll_rack(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards for PutBackOnTop");
    };
    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .last()
            .copied(),
        Some(chosen)
    );
}

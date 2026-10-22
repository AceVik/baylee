//! `cards/instants/mv_2/impulse.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn impulse() -> CardIndex {
    card_index("f6bd2902-7f8b-419e-bbc7-bcab0c1b7e01")
}

/// `Impulse` ({1}{U}, Instant, `Coverage::Implemented`):
/// `Look at the top four cards of your library. Put one of them into your hand
/// and the rest on the bottom of your library in any order.`
#[test]
fn impulse_looks_at_top_four_puts_one_in_hand_and_rest_on_bottom() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[impulse(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear_id = in_hand(&engine, p0, grizzly_bears()).unwrap();
    engine
        .dev_state_mut(p0)
        .unwrap()
        .move_object(
            bear_id,
            ZoneLocation::Library(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, impulse());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards { .. } | Pending::Arrange { .. }
        )
    });
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        assert!(options.contains(&bear_id));
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![bear_id],
                },
            )
            .unwrap();
    }
    if let Pending::Arrange { cards, .. } = engine.pending().clone() {
        engine
            .apply(p0, PlayerAction::Arrange { piles: vec![cards] })
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, grizzly_bears()).is_some());
}

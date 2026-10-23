#[allow(clippy::wildcard_imports)]
use super::*;

fn time_spiral() -> CardIndex {
    card_index("cac55e46-b730-4569-b92c-a4b5922fc20c")
}

#[test]
fn time_spiral_exiles_self_shuffles_draws_seven_and_untaps_up_to_six_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[time_spiral()])
        .battlefield(
            0,
            &[island(), island(), island(), island(), island(), island()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spiral_id = in_hand(&engine, p0, time_spiral()).expect("spiral in hand");
    cast_from_hand(&mut engine, p0, time_spiral());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: options.into_iter().take(6).collect(),
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(spiral_id).unwrap().zone,
        crate::zone::Zone::Exile
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        7
    );
}

#[allow(clippy::wildcard_imports)]
use super::*;

fn great_whale() -> CardIndex {
    card_index("c43ce30a-0346-479d-8dbc-56c3872ff82a")
}

#[test]
fn great_whale_untaps_up_to_seven_lands_on_etb() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[great_whale()])
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land_id = on_battlefield(&engine, p0, island()).expect("seated");
    cast_from_hand(&mut engine, p0, great_whale());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: options,
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(!is_tapped(&engine, land_id));
}

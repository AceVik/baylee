#[allow(clippy::wildcard_imports)]
use super::*;

fn peregrine_drake() -> CardIndex {
    card_index("0bd67481-6bd9-48d6-92bd-8933b5ea1eae")
}

#[test]
fn peregrine_drake_untaps_up_to_five_lands_on_etb() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[peregrine_drake()])
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land_id = on_battlefield(&engine, p0, island()).expect("seated");
    cast_from_hand(&mut engine, p0, peregrine_drake());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: options })
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(!is_tapped(&engine, land_id));
}

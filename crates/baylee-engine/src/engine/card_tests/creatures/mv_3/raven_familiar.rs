#[allow(clippy::wildcard_imports)]
use super::*;

fn raven_familiar() -> CardIndex {
    card_index("47aabd9d-0f68-490a-a2ae-5cd9ef689be6")
}

#[test]
fn raven_familiar_etb_picks_one_of_top_three() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[raven_familiar()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_from_hand(&mut engine, p0, raven_familiar());

    let Pending::ChooseCards { options, .. } = pass_to_card_choice(&mut engine) else {
        panic!("expected ChooseCards for Raven Familiar");
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

    if let Pending::Arrange { cards, .. } = engine.pending().clone() {
        engine
            .apply(p0, PlayerAction::Arrange { piles: vec![cards] })
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    let hand_after = engine.state().zones.list(ZoneLocation::Hand(p0));
    assert_eq!(hand_after.len(), hand_before);
    assert!(hand_after.contains(&chosen));
}

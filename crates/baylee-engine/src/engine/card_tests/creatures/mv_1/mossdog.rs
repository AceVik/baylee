//! `cards/creatures/mv_1/mossdog.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn mossdog_gets_plus_one_plus_one_counter_when_targeted_by_opponent_spell_or_ability() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[card_index("95cc3652-b78b-473b-84cf-0dd3a7006219"), forest()],
        )
        .hand(0, &[index::GIANT_GROWTH])
        .battlefield(1, &[plains(), index::NETTER_EN_DAL])
        .hand(1, &[plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let dog = on_battlefield(
        &engine,
        p0,
        card_index("95cc3652-b78b-473b-84cf-0dd3a7006219"),
    )
    .expect("Mossdog");
    let opp_plains_in_hand = in_hand(&engine, p1, plains()).expect("Plains in p1 hand");

    // Own spell targeting Mossdog does not trigger it.
    cast_from_hand(&mut engine, p0, index::GIANT_GROWTH);
    aim_at(&mut engine, p0, dog);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(dog)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0,
        "own spell does not trigger Mossdog"
    );

    // Opponent ability targeting Mossdog triggers it and adds a +1/+1 counter.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    activate(&mut engine, p1, index::NETTER_EN_DAL, 0);
    aim_at(&mut engine, p1, dog);
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![opp_plains_in_hand],
            },
        )
        .expect("discard Plains");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(dog)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1,
        "opponent ability targeting Mossdog puts a +1/+1 counter on it"
    );
}

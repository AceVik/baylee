//! `cards/creatures/mv_5/bogardan_phoenix.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn bogardan_phoenix() -> CardIndex {
    card_index("72586a7a-3c73-4012-8664-1db0f268eb60")
}

#[test]
fn bogardan_phoenix_returns_once_with_a_death_counter_and_is_exiled_the_second_time() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[bogardan_phoenix()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, bogardan_phoenix());
    pass_until(&mut engine, stack_is_empty);

    let phoenix = on_battlefield(&engine, p0, bogardan_phoenix()).unwrap();
    assert_eq!(pt(&engine, phoenix), (3, 3));
    assert!(keywords(&engine, phoenix).contains(KeywordSet::FLYING));

    kill(&mut engine, phoenix);
    pass_until(&mut engine, stack_is_empty);

    let returned = on_battlefield(&engine, p0, bogardan_phoenix()).unwrap();
    assert_eq!(
        counters_on(&engine, returned, baylee_cards_dsl::counters::DEATH),
        1
    );
    assert!(
        in_graveyard(&engine, p0, bogardan_phoenix()).is_none(),
        "the first death returns it, it is not left in the graveyard"
    );

    // Second death: it had a death counter, so it is exiled instead.
    kill(&mut engine, returned);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, bogardan_phoenix()).is_none(),
        "no second return"
    );
    assert!(
        in_graveyard(&engine, p0, bogardan_phoenix()).is_none(),
        "exiled, not left in the graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(p0)).len(),
        1,
        "the Phoenix is in exile"
    );
}

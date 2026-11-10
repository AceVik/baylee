#[allow(clippy::wildcard_imports)]
use super::*;

fn barrin_s_codex() -> CardIndex {
    card_index("f7c237e6-17a4-4e72-8cc3-456aec738394")
}

#[test]
fn barrin_s_codex_accumulates_page_counters_and_sacrifices_to_draw() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[barrin_s_codex()])
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
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, barrin_s_codex());
    pass_until(&mut engine, stack_is_empty);
    let codex = on_battlefield(&engine, p0, barrin_s_codex()).expect("seated");

    // Advance to p0's next main phase; `pass_until` answers `MayDo` with `YesNo(true)`.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine
            .state()
            .object(codex)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::PAGE),
        0
    );
    reach_their_main_phase(&mut engine, p0);

    assert_eq!(
        engine
            .state()
            .object(codex)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::PAGE),
        1
    );

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: codex,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, barrin_s_codex()).is_none());
    assert!(in_graveyard(&engine, p0, barrin_s_codex()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}

#[test]
fn barrin_s_codex_can_decline_a_counter_and_draws_zero_with_no_counters() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[barrin_s_codex()])
        .battlefield(0, &[island(), island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, barrin_s_codex());
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    reach_main_phase(&mut engine, p0);
    let before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, barrin_s_codex(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, barrin_s_codex()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before
    );
}

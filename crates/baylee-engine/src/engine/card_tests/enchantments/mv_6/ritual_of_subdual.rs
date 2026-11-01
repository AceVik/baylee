//! `cards/enchantments/mv_6/ritual_of_subdual.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn ritual_of_subdual() -> CardIndex {
    card_index("e0587218-206e-41ed-af3c-f06b7a668e90")
}

fn pay_first_upkeep_with_plains_and_island(
    engine: &mut Engine<RegistryLookup>,
    ritual: ObjectId,
) -> (ObjectId, ObjectId) {
    // Upkeep 1 (turn 1): 1 age counter placed -> pay {2} by tapping Plains and Island
    // (each produces {C} instead of {W}/{U}).
    pass_until(engine, |e| matches!(e.pending(), Pending::YesNo { .. }));
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    let pl = on_battlefield(engine, P0, plains()).unwrap();
    let isl = on_battlefield(engine, P0, island()).unwrap();
    tap_mana_where(engine, P0, |id| id == pl);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        0,
        "Plains produces no {{W}} under Ritual of Subdual"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "Plains produces {{C}} under Ritual of Subdual"
    );
    tap_mana_where(engine, P0, |id| id == isl);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        0,
        "Island produces no {{U}} under Ritual of Subdual"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        2,
        "Island produces {{C}} under Ritual of Subdual"
    );
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(engine, |e| {
        e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        engine
            .state()
            .object(ritual)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        1
    );

    (pl, isl)
}

/// Ritual of Subdual (e0587218-206e-41ed-af3c-f06b7a668e90) — {4}{G}{G} Enchantment:
/// - "Cumulative upkeep {2}"
/// - "If a land is tapped for mana, it produces colorless mana instead of any other type."
#[test]
fn ritual_of_subdual_replaces_colored_land_mana_with_colorless_and_scales_cumulative_upkeep() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                ritual_of_subdual(),
                plains(),
                island(),
                swamp(),
                mountain(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);

    let ritual = on_battlefield(&engine, P0, ritual_of_subdual()).unwrap();

    let (pl, isl) = pay_first_upkeep_with_plains_and_island(&mut engine, ritual);

    // Main phase: verify Swamp, Mountain, and Forest also produce {C} instead of {B}, {R}, {G}.
    reach_main_phase(&mut engine, P0);
    let sw = on_battlefield(&engine, P0, swamp()).unwrap();
    let mtn = on_battlefield(&engine, P0, mountain()).unwrap();
    let fst = on_battlefield(&engine, P0, forest()).unwrap();
    tap_mana_where(&mut engine, P0, |id| id == sw || id == mtn || id == fst);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 0);
    assert_eq!(pool.available(ManaColor::Red), 0);
    assert_eq!(pool.available(ManaColor::Green), 0);
    assert_eq!(
        pool.available(ManaColor::Colorless),
        3,
        "Swamp, Mountain, and Forest all produce {{C}}"
    );

    // Upkeep 2 (turn 3): 2nd age counter placed -> pay {4} by tapping 4 lands.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    tap_mana_where(&mut engine, P0, |id| {
        id == pl || id == isl || id == sw || id == mtn
    });
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        engine
            .state()
            .object(ritual)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        2
    );

    // Upkeep 3 (turn 5): 3rd age counter placed -> decline payment -> sacrificed,
    // and lands produce colored mana again.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 5 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 5 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, P0, ritual_of_subdual()).is_none()
            && in_graveyard(&engine, P0, ritual_of_subdual()).is_some(),
        "Ritual of Subdual is sacrificed when cumulative upkeep is declined"
    );
    reach_main_phase(&mut engine, P0);
    tap_mana_where(&mut engine, P0, |id| id == fst);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "Forest produces {{G}} again after Ritual of Subdual leaves the battlefield"
    );
}

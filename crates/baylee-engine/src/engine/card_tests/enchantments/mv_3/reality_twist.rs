//! `cards/enchantments/mv_3/reality_twist.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);

fn reality_twist() -> CardIndex {
    card_index("2c6f9fdb-ee27-4cd8-9d4d-1ea8b2830f73")
}

/// Reality Twist (2c6f9fdb-ee27-4cd8-9d4d-1ea8b2830f73) — {U}{U}{U} Enchantment:
/// - "Cumulative upkeep {1}{U}{U}"
/// - "If tapped for mana, Plains produce {R}, Swamps produce {G}, Mountains produce {W},
///   and Forests produce {B} instead of any other type."
#[test]
fn reality_twist_replaces_plains_swamp_mountain_and_forest_mana_while_leaving_islands_blue_and_has_cumulative_upkeep()
 {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                reality_twist(),
                plains(),
                swamp(),
                mountain(),
                forest(),
                island(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);

    let rt = on_battlefield(&engine, P0, reality_twist()).unwrap();

    // Upkeep 1 (turn 1): cumulative upkeep {1}{U}{U} puts 1 age counter on Reality Twist;
    // P0 taps both Islands (which still produce {U}) and Plains (which produces {R}) to pay {1}{U}{U}.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    let pl = on_battlefield(&engine, P0, plains()).unwrap();
    tap_mana_where(&mut engine, P0, |id| id == pl);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "Plains produces {{R}}"
    );
    let islands = all_on_battlefield(&engine, P0, island());
    for isl in islands {
        tap_mana_where(&mut engine, P0, |id| id == isl);
    }
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        2,
        "Islands are not replaced by Reality Twist and still produce {{U}}"
    );
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        engine
            .state()
            .object(rt)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        1
    );

    // Main phase: Swamp produces {G}, Mountain produces {W}, Forest produces {B}.
    reach_main_phase(&mut engine, P0);
    let sw = on_battlefield(&engine, P0, swamp()).unwrap();
    let mtn = on_battlefield(&engine, P0, mountain()).unwrap();
    let fst = on_battlefield(&engine, P0, forest()).unwrap();
    tap_mana_where(&mut engine, P0, |id| id == sw);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "Swamp produces {{G}}"
    );
    tap_mana_where(&mut engine, P0, |id| id == mtn);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "Mountain produces {{W}}"
    );
    tap_mana_where(&mut engine, P0, |id| id == fst);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "Forest produces {{B}}"
    );

    // Upkeep 2 (turn 3): 2nd age counter placed; P0 declines cumulative upkeep {2}{U}{U}{U}{U}.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, P0, reality_twist()).is_none()
            && in_graveyard(&engine, P0, reality_twist()).is_some(),
        "Reality Twist is sacrificed when cumulative upkeep is declined"
    );
}

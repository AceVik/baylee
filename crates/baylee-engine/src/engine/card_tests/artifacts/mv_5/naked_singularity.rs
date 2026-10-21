//! `cards/artifacts/mv_5/naked_singularity.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);

fn naked_singularity() -> CardIndex {
    card_index("fb5ec55d-4a35-432a-be6e-c295f1b2e603")
}

/// Naked Singularity (fb5ec55d-4a35-432a-be6e-c295f1b2e603) — {5} Artifact:
/// - "Cumulative upkeep {3}"
/// - "If tapped for mana, Plains produce {R}, Islands produce {G}, Swamps produce {W},
///   Mountains produce {U}, and Forests produce {B} instead of any other type."
#[test]
fn naked_singularity_replaces_basic_land_mana_colors_and_has_cumulative_upkeep_three() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                naked_singularity(),
                plains(),
                island(),
                swamp(),
                mountain(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);

    let ns = on_battlefield(&engine, P0, naked_singularity()).unwrap();

    // Upkeep 1 (turn 1): cumulative upkeep {3} puts 1 age counter on Naked Singularity;
    // P0 pays {3} by tapping Plains (produces {R}), Island (produces {G}), and Swamp (produces {W}).
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    let pl = on_battlefield(&engine, P0, plains()).unwrap();
    let isl = on_battlefield(&engine, P0, island()).unwrap();
    let sw = on_battlefield(&engine, P0, swamp()).unwrap();
    tap_mana_where(&mut engine, P0, |id| id == pl);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "Plains produces {{R}}"
    );
    tap_mana_where(&mut engine, P0, |id| id == isl);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "Island produces {{G}}"
    );
    tap_mana_where(&mut engine, P0, |id| id == sw);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "Swamp produces {{W}}"
    );
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        engine
            .state()
            .object(ns)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        1
    );

    // Main phase: tap Mountain -> produces {U}; tap Forest -> produces {B}.
    reach_main_phase(&mut engine, P0);
    let mtn = on_battlefield(&engine, P0, mountain()).unwrap();
    let fst = on_battlefield(&engine, P0, forest()).unwrap();
    tap_mana_where(&mut engine, P0, |id| id == mtn);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "Mountain produces {{U}}"
    );
    tap_mana_where(&mut engine, P0, |id| id == fst);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "Forest produces {{B}}"
    );

    // Upkeep 2 (turn 3): 2nd age counter placed; P0 declines cumulative upkeep {6}.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, P0, naked_singularity()).is_none()
            && in_graveyard(&engine, P0, naked_singularity()).is_some(),
        "Naked Singularity is sacrificed when cumulative upkeep is declined"
    );
}

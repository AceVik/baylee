//! `cards/artifacts/mv_4/gnarled_effigy.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn gnarled_effigy() -> CardIndex {
    card_index("5fa3cc1a-09ee-4152-8c50-ea719510fc20")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Gnarled Effigy (5fa3cc1a-09ee-4152-8c50-ea719510fc20) — {4} Artifact:
/// "{4}, {T}: Put a -1/-1 counter on target creature."
///
/// Requires {4} and tapping; shrinks a 2/2 to 1/1 with a permanent -1/-1
/// counter; a second activation on a subsequent turn kills it via 0 toughness.
#[test]
fn gnarled_effigy_places_minus_one_counters_and_kills_at_zero_toughness() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[gnarled_effigy(), forest(), forest(), forest(), forest()],
        )
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let effigy = on_battlefield(&engine, p0, gnarled_effigy()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    // Without {4} floating, cannot activate directly.
    assert!(
        !priority_offer(&engine).abilities.contains(&(effigy, 0)),
        "requires {{4}} to activate"
    );

    // Pay {4}, {T}: put a -1/-1 counter on opponent's 2/2 Grizzly Bears -> 1/1.
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: effigy,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .expect("target opponent's Grizzly Bears");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, effigy),
        "Gnarled Effigy tapped to pay cost"
    );
    assert_eq!(counters_on(&engine, bears, CounterKind::M1M1), 1);
    assert_eq!(pt(&engine, bears), (1, 1));

    // Next turn: untap and put a second -1/-1 counter -> 0/0 dies to SBA.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: effigy,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, grizzly_bears()).is_some(),
        "second -1/-1 counter killed the 2/2 Grizzly Bears"
    );
}

//! `cards/creatures/artifacts/mv_10/stratadon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn stratadon() -> CardIndex {
    card_index("3b78b672-7773-44c6-9a21-f89c058966ff")
}

fn is_castable(engine: &Engine<RegistryLookup>, card: CardIndex) -> bool {
    let id = in_hand(engine, P0, card).unwrap();
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority");
    };
    legal.castable.contains(&id)
}

/// Stratadon (3b78b672-7773-44c6-9a21-f89c058966ff) — `{10}` 5/5 Artifact Creature — Beast:
/// - Domain — This spell costs `{1}` less to cast for each basic land type among lands you control.
/// - Trample
#[test]
fn stratadon_costs_one_less_per_distinct_basic_land_type_you_control_and_has_trample() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), island(), swamp(), mountain(), forest()])
        .hand(0, &[stratadon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // 5 basic land types controlled -> `{10}` reduced by `{5}` to `{5}`:
    // 4 mana is 1 short, 5 mana is exact.
    tap_all_mana_but(&mut engine, P0, Some(forest()));
    assert!(
        !is_castable(&engine, stratadon()),
        "with 5 basic land types, 4 mana is 1 short of {{5}}"
    );
    tap_all_mana(&mut engine, P0);
    assert!(is_castable(&engine, stratadon()));

    cast_with_floating(&mut engine, P0, stratadon());
    pass_until(&mut engine, stack_is_empty);

    let beast = on_battlefield(&engine, P0, stratadon()).expect("Stratadon entered");
    assert_eq!(pt(&engine, beast), (5, 5));
    assert!(keywords(&engine, beast).contains(KeywordSet::TRAMPLE));
    let types = engine
        .state()
        .object(beast)
        .unwrap()
        .characteristics()
        .types;
    assert!(types.contains(TypeSet::ARTIFACT) && types.contains(TypeSet::CREATURE));
}

#[test]
fn stratadon_ignores_duplicate_and_opponent_basic_land_types() {
    // P0 has 6 Forests (only 1 basic land type -> costs {9}), P1 has all other 4 basic land types.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .battlefield(1, &[plains(), island(), swamp(), mountain()])
        .hand(0, &[stratadon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    assert!(
        !is_castable(&engine, stratadon()),
        "6 Forests only give 1 basic land type ({{9}} cost), and opponent lands do not count"
    );
}

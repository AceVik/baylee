//! `cards/enchantments/mv_3/cold_snap.rs`, played.
//!
//! Oracle:
//! - "Cumulative upkeep {2}"
//! - "At the beginning of each player's upkeep, this enchantment deals damage to that player equal to the number of snow lands they control."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::counters;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "3dcbcf5e-bdbe-46b3-881e-5ac4030c6e38"
fn cold_snap() -> CardIndex {
    card_index("3dcbcf5e-bdbe-46b3-881e-5ac4030c6e38")
}

fn age(engine: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    engine
        .state()
        .object(id)
        .expect("object exists")
        .counters
        .get(counters::AGE)
}

#[test]
fn cold_snap_deals_upkeep_damage_equal_to_snow_lands_controlled() {
    let mut engine = lk::game(
        SEED,
        &[index::FOREST],
        &[
            index::SNOW_COVERED_FOREST,
            index::SNOW_COVERED_FOREST,
            index::FOREST,
        ],
        &[cold_snap()],
        &[],
    );

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
    );
    cast_with_floating(&mut engine, P0, cold_snap());
    pass_until(&mut engine, stack_is_empty);

    // Advance to P1's upkeep: P1 controls 2 snow lands and 1 non-snow land, so P1 takes 2 damage.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(
        engine.state().players[1].life,
        18,
        "Cold Snap deals 2 damage to P1 for the 2 snow lands P1 controls"
    );

    // Advance to P0's upkeep: Cold Snap has cumulative upkeep {2}, and P0 controls 0 snow lands.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });
    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 2)]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(P0, PlayerAction::YesNo(true))
        .expect("pay {2} cumulative upkeep for Cold Snap");
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.phase == Phase::FirstMain
    });

    assert_eq!(
        engine.state().players[0].life,
        20,
        "P0 controls no snow lands so Cold Snap deals 0 damage to P0"
    );
    let snap = on_battlefield(&engine, P0, cold_snap()).expect("Cold Snap remains on battlefield");
    assert_eq!(age(&engine, snap), 1);

    // The upkeep after: a second age counter, so {2} for each is {4}.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });
    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 4)]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut engine, true);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(age(&engine, snap), 2);
    assert_eq!(
        lk::pool_total(&engine, P0),
        0,
        "cumulative upkeep {{2}} with two age counters costs {{4}}"
    );

    // And the upkeep after that, unpaid: Cold Snap is sacrificed.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut engine, false);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, P0, cold_snap()).is_none(),
        "sacrifice it unless you pay its upkeep cost for each age counter on it"
    );
}

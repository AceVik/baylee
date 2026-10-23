//! `cards/creatures/mv_4/floodgate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{
    CHARIOT_OF_THE_SUN, GRIZZLY_BEARS, MERFOLK_OF_THE_PEARL_TRIDENT, SERRA_ANGEL, UNSUMMON,
};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn floodgate() -> CardIndex {
    card_index("f5a85674-fd09-42d8-bba7-5656be25fcea")
}

/// Floodgate (f5a85674-fd09-42d8-bba7-5656be25fcea) — {3}{U} 0/5:
/// "Defender. When this creature has flying, sacrifice it. When this creature
/// leaves the battlefield, it deals damage to each nonblue creature without
/// flying equal to half the number of Islands you control, rounded down."
#[test]
fn floodgate_deals_half_island_count_damage_to_nonblue_nonflying_creatures_when_leaving_battlefield()
 {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[floodgate(), island(), island(), island(), island()])
        .battlefield(1, &[island(), GRIZZLY_BEARS, SERRA_ANGEL])
        .hand(0, &[UNSUMMON])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let gate = on_battlefield(&engine, P0, floodgate()).unwrap();
    let angel = on_battlefield(&engine, P1, SERRA_ANGEL).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, UNSUMMON);
    aim_at(&mut engine, P0, gate);
    pass_until(&mut engine, stack_is_empty);

    // 4 Islands / 2 = 2 damage to nonblue creatures without flying:
    // Grizzly Bears (2/2) dies; Serra Angel (flying) takes 0 damage.
    assert!(on_battlefield(&engine, P0, floodgate()).is_none());
    assert!(on_battlefield(&engine, P1, GRIZZLY_BEARS).is_none());
    assert_eq!(engine.state().object(angel).unwrap().damage, 0);
}

/// "Islands you control": the opponent's Islands do not count. P0 controls two
/// (half is 1), P1 four more: a Grizzly Bears takes 1 and survives; counting
/// all six would have been 3 and killed it.
#[test]
fn floodgate_counts_only_its_controllers_islands() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[floodgate(), island(), island()])
        .battlefield(1, &[island(), island(), island(), island(), GRIZZLY_BEARS])
        .hand(0, &[UNSUMMON])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let gate = on_battlefield(&engine, P0, floodgate()).unwrap();
    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, UNSUMMON);
    aim_at(&mut engine, P0, gate);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, floodgate()).is_none());
    assert_eq!(engine.state().object(bears).unwrap().damage, 1);
}

/// "nonblue creature": a blue creature without flying (Merfolk of the Pearl
/// Trident) takes no damage while the Bears beside it dies.
#[test]
fn floodgate_does_not_damage_a_blue_creature() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[floodgate(), island(), island(), island(), island()])
        .battlefield(1, &[MERFOLK_OF_THE_PEARL_TRIDENT, GRIZZLY_BEARS])
        .hand(0, &[UNSUMMON])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let gate = on_battlefield(&engine, P0, floodgate()).unwrap();
    let merfolk = on_battlefield(&engine, P1, MERFOLK_OF_THE_PEARL_TRIDENT).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, UNSUMMON);
    aim_at(&mut engine, P0, gate);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, GRIZZLY_BEARS).is_none());
    assert_eq!(engine.state().object(merfolk).unwrap().damage, 0);
}

/// "When this creature has flying, sacrifice it.": Chariot of the Sun gives
/// it flying, and it goes to the graveyard (dealing half its controller's two
/// Islands, 1 damage, to the Bears).
#[test]
fn floodgate_is_sacrificed_when_it_has_flying() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[floodgate(), CHARIOT_OF_THE_SUN, island(), island()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let gate = on_battlefield(&engine, P0, floodgate()).unwrap();
    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, CHARIOT_OF_THE_SUN, 0);
    aim_at(&mut engine, P0, gate);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, floodgate()).is_none());
    assert!(in_graveyard(&engine, P0, floodgate()).is_some());
    assert_eq!(engine.state().object(bears).unwrap().damage, 1);
}

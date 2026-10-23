//! `cards/enchantments/auras/mv_1/death_watch.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn death_watch() -> CardIndex {
    card_index("6f5b6cba-3b57-4e6a-afa7-0cb62e5cc7d4")
}

fn savannah_lions() -> CardIndex {
    baylee_core::generated::index::SAVANNAH_LIONS
}

/// Death Watch (`{B}`): "When enchanted creature dies, its controller loses
/// life equal to its power and you gain life equal to its toughness." The
/// enchanted creature is a Savannah Lions (power 2, toughness 1), so the
/// power and the toughness cannot be swapped unseen: its controller loses 2,
/// Death Watch's controller gains 1. A creature that is not enchanted dying
/// first changes nothing.
#[test]
fn death_watch_drains_by_power_and_gains_by_toughness_for_the_enchanted_creature_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp()])
        .battlefield(1, &[savannah_lions(), llanowar_elves()])
        .hand(0, &[death_watch()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let lions = on_battlefield(&engine, p1, savannah_lions()).unwrap();
    let elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    cast_from_hand(&mut engine, p0, death_watch());
    aim_at(&mut engine, p0, lions);
    pass_until(&mut engine, stack_is_empty);

    kill(&mut engine, elf);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        20,
        "not the enchanted creature"
    );
    assert_eq!(engine.state().players[0].life, 20);

    kill(&mut engine, lions);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        18,
        "its controller loses life equal to its power (2)"
    );
    assert_eq!(
        engine.state().players[0].life,
        21,
        "you gain life equal to its toughness (1)"
    );
}

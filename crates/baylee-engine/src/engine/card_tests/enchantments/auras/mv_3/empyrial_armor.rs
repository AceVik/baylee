//! `cards/enchantments/auras/mv_3/empyrial_armor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{EMPYRIAL_ARMOR, GRIZZLY_BEARS};

/// `Empyrial Armor` ({1}{W}{W}, Enchantment — Aura) enchants target creature.
#[test]
fn empyrial_armor_attaches_to_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[GRIZZLY_BEARS, plains(), plains(), plains()])
        .hand(0, &[EMPYRIAL_ARMOR])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, EMPYRIAL_ARMOR);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);

    let armor = on_battlefield(&engine, p0, EMPYRIAL_ARMOR).unwrap();
    assert_eq!(
        engine.state().object(armor).unwrap().attached_to,
        Some(bears)
    );
}

//! `cards/enchantments/mv_4/splintering_wind.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn splintering_wind() -> CardIndex {
    card_index("184f68a1-0c13-42f4-aee6-3ca744f6a30b")
}

/// Splintering Wind ({2}{G}{G} Enchantment):
/// - "{2}{G}: This enchantment deals 1 damage to target creature."
#[test]
fn splintering_wind_deals_one_damage_to_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[splintering_wind(), forest(), forest(), forest()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, splintering_wind(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, llanowar_elves()).is_some());
}

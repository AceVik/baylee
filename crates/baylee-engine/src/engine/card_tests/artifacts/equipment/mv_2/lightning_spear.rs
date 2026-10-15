//! `cards/artifacts/equipment/mv_2/lightning_spear.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn lightning_spear() -> CardIndex {
    card_index("2202faf1-5638-46bf-8bf4-91870b669717")
}

/// Lightning Spear: "Equipped creature gets +1/+0 and has trample.
/// {2}{R}, Sacrifice this Equipment: It deals 3 damage to any target.
/// Equip {1}".
#[test]
fn lightning_spear_gives_plus_one_power_and_trample() {
    equipped_gets(
        lightning_spear(),
        1,
        llanowar_elves(),
        (1, 0),
        KeywordSet::TRAMPLE,
    );
}

#[test]
fn lightning_spear_sacrificed_deals_three_to_a_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), forest(), forest(), lightning_spear()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    // Ability 0 is the Equip, 1 and 2 the statics, 3 the sacrifice.
    activate(&mut engine, p0, lightning_spear(), 3);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 17, "3 damage");
    assert!(
        on_battlefield(&engine, p0, lightning_spear()).is_none(),
        "the Equipment was sacrificed"
    );
    assert!(in_graveyard(&engine, p0, lightning_spear()).is_some());
}

#[test]
fn lightning_spear_sacrificed_can_kill_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), forest(), forest(), lightning_spear()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, lightning_spear(), 3);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_none());
    assert_eq!(engine.state().players[1].life, 20);
}

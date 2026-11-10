//! `cards/creatures/mv_1/armorer_guildmage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Armorer Guildmage` prints `{{B}}, {{T}}: Target creature gets +1/+0 until end of turn.` and `{{G}}, {{T}}: Target creature gets +0/+1 until end of turn.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Armorer Guildmage`, a Swamp, and a 1/1 `llanowar_elves()`.
/// Floating black mana and activating ability 0 targets the elf, taps `Armorer Guildmage`, and increases the elf's power to 2 until end of turn.
#[test]
fn armorer_guildmage_taps_with_black_mana_to_pump_power() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[armorer_guildmage(), swamp(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mage = on_battlefield(&engine, p0, armorer_guildmage()).expect("mage seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf seated");
    assert_eq!(pt(&engine, elf), (1, 1));
    assert!(!is_tapped(&engine, mage));

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1
    );

    activate(&mut engine, p0, armorer_guildmage(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&elf), "elf is a legal target creature");

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("target elf chosen");

    assert!(
        is_tapped(&engine, mage),
        "`Armorer Guildmage` tapped to pay activation cost"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, elf),
        (2, 1),
        "target elf got +1/+0 until end of turn"
    );
}

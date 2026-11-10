//! `cards/instants/mv_1/death_ward.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Death Ward: "Regenerate target creature." Cast ahead of an ordinary
/// destroy (not the "can't be regenerated" kind), the shield saves its
/// target, tapped, in its owner's control.
#[test]
fn death_ward_regenerates_its_target() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[quiet_creature(), plains()])
        .hand(0, &[death_ward()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is seated");
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, death_ward());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("its own Elf is a legal target");
    pass_until(&mut engine, stack_is_empty);

    kill(&mut engine, elf);
    assert_eq!(
        engine.state().object(elf).map(|o| o.zone),
        Some(Zone::Battlefield),
        "the shield saved it"
    );
    assert!(is_tapped(&engine, elf), "regeneration taps the permanent");
}

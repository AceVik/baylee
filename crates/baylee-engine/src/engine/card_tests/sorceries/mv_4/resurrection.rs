//! `cards/sorceries/mv_4/resurrection.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Resurrection: "Return target creature card from your graveyard to the
/// battlefield."
#[test]
fn resurrection_returns_a_creature_from_the_graveyard_to_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), forest(), forest()])
        .hand(0, &[resurrection(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    hand_to_graveyard(&mut engine, p0, quiet_creature());
    let elf_in_gy = in_graveyard(&engine, p0, quiet_creature()).expect("in the graveyard");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, resurrection());
    let menu = aim_at(&mut engine, p0, elf_in_gy);
    assert!(
        menu.contains(&elf_in_gy),
        "the creature card in the graveyard is a legal target: {menu:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_some(),
        "\"to the battlefield\""
    );
    assert!(in_graveyard(&engine, p0, quiet_creature()).is_none());
}

//! `cards/artifacts/mv_1/wooden_sphere.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wooden Sphere: "Whenever a player casts a green spell, you may pay {1}.
/// If you do, you gain 1 life." Any green spell counts, not only an instant
/// or sorcery: Llanowar Elves, a creature spell, is the stimulus here.
#[test]
fn wooden_sphere_offers_to_pay_and_gain_life_off_a_green_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[wooden_sphere(), forest(), forest()])
        .hand(0, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, quiet_creature());
    pays_the_tax_and_gains_a_life(&mut engine, p0);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, quiet_creature()).is_some());
}

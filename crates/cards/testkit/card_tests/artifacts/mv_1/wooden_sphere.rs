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

/// Wooden Sphere: "you may pay {1}" is a choice. Declined, it gains no life and
/// spends nothing of the mana floating in the pool.
#[test]
fn wooden_sphere_declined_gains_nothing_and_spends_nothing() {
    a_rock_declined(wooden_sphere(), forest(), quiet_creature());
}

/// Wooden Sphere: "Whenever *a player* casts a green spell, *you* may pay {1}":
/// the opponent's green spell asks the controller of the Wooden Sphere, who pays
/// and gains the life.
#[test]
fn wooden_sphere_pays_its_controller_off_an_opponents_green_spell() {
    a_rock_pays_off_an_opponents_spell(wooden_sphere(), forest(), forest(), quiet_creature());
}

/// Wooden Sphere: only a green spell is asked about. A spell of another color
/// resolves without a question and the life total stands.
#[test]
fn wooden_sphere_ignores_a_spell_of_another_color() {
    a_rock_ignores_another_color(wooden_sphere(), mountain(), lightning_bolt());
}

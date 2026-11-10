//! `cards/artifacts/mv_1/throne_of_bone.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Throne of Bone: "Whenever a player casts a black spell, you may pay {1}.
/// If you do, you gain 1 life." Dark Ritual, untargeted, is the black
/// stimulus.
#[test]
fn throne_of_bone_offers_to_pay_and_gain_life_off_a_black_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[throne_of_bone(), swamp(), swamp()])
        .hand(0, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, dark_ritual());
    pays_the_tax_and_gains_a_life(&mut engine, p0);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, dark_ritual()).is_some());
}

/// Throne of Bone: "you may pay {1}" is a choice. Declined, it gains no life and
/// spends nothing of the mana floating in the pool.
#[test]
fn throne_of_bone_declined_gains_nothing_and_spends_nothing() {
    a_rock_declined(throne_of_bone(), swamp(), dark_ritual());
}

/// Throne of Bone: "Whenever *a player* casts a black spell, *you* may pay {1}":
/// the opponent's black spell asks the controller of the Throne of Bone, who pays
/// and gains the life.
#[test]
fn throne_of_bone_pays_its_controller_off_an_opponents_black_spell() {
    a_rock_pays_off_an_opponents_spell(throne_of_bone(), swamp(), swamp(), dark_ritual());
}

/// Throne of Bone: only a black spell is asked about. A spell of another color
/// resolves without a question and the life total stands.
#[test]
fn throne_of_bone_ignores_a_spell_of_another_color() {
    a_rock_ignores_another_color(throne_of_bone(), forest(), quiet_creature());
}

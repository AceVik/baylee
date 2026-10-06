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

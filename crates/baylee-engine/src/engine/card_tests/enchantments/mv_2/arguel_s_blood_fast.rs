//! `cards/enchantments/mv_2/arguel_s_blood_fast.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Arguel's Blood Fast: "{1}{B}, Pay 2 life: Draw a card."
///
/// The transform trigger and the back face's sacrifice ability are refused by
/// name; the draw is the card's front half and charges in two currencies at
/// once, which is what the assertion has to read.
#[test]
fn arguel_s_blood_fast_charges_two_life_for_its_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(400, forest())
        .battlefield(0, &[arguel_s_blood_fast(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before = library_size(&engine, p0);
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, arguel_s_blood_fast(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(library_size(&engine, p0), before - 1, "one card drawn");
    assert_eq!(
        engine.state().players[0].life,
        18,
        "and two life paid for it"
    );
}

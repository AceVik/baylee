//! `cards/sorceries/mv_2/regrowth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Regrowth: "Return target card from your graveyard to your hand." Read
/// off an instant, not a creature — "any card" is the whole point beside
/// Raise Dead.
#[test]
fn regrowth_returns_any_card_from_the_graveyard_not_only_a_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[regrowth(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    hand_to_graveyard(&mut engine, p0, lightning_bolt());
    let bolt_in_gy = in_graveyard(&engine, p0, lightning_bolt()).expect("in the graveyard");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, regrowth());
    aim_at(&mut engine, p0, bolt_in_gy);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p0, lightning_bolt()).is_some(),
        "\"any card\" — an instant, not only a creature"
    );
}

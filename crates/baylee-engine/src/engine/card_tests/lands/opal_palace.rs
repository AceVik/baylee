//! `cards/lands/opal_palace.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Opal Palace: "{T}: Add {C}." / "{1}, {T}: Add one mana of any color in your commander's color identity..."
/// Under `Coverage::Partial`, the spend rider adding extra +1/+1 counters to a cast commander is omitted.
/// Activating ability 1 with a black commander produces one black mana in the player's pool.
#[test]
fn opal_palace_produces_commander_identity_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(319, forest())
        .commander(0, &[sheoldred_the_apocalypse()])
        .battlefield(0, &[opal_palace(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let palace = on_battlefield(&engine, p0, opal_palace()).expect("palace deployed");
    tap_mana_except(&mut engine, p0, palace);
    activate(&mut engine, p0, opal_palace(), 1);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, palace));
}

//! `cards/lands/utility/springjack_pasture.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Springjack Pasture: "{T}: Add {C}." / "{4}, {T}: Create a 0/1 white Goat creature token." / "{T}, Sacrifice X Goats: Add X mana of any one color. You gain X life."
/// Under `Coverage::Partial`, the counted Goat sacrifice is unsupported, leaving the mana at 0 and the Goat at 1.
/// Activating ability 0 adds one colorless mana to the pool and leaves the land tapped.
#[test]
fn springjack_pasture_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(136, forest())
        .battlefield(0, &[springjack_pasture()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let pasture =
        on_battlefield(&engine, p0, springjack_pasture()).expect("Springjack Pasture deployed");
    activate(&mut engine, p0, springjack_pasture(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, pasture));
}

/// Springjack Pasture's `{4}, {T}`: one 0/1 white Goat, and nothing at
/// ability 2, where the counted sacrifice would be.
#[test]
fn springjack_pasture_makes_a_goat_for_four() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(136, forest())
        .battlefield(
            0,
            &[springjack_pasture(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let pasture =
        on_battlefield(&engine, p0, springjack_pasture()).expect("Springjack Pasture deployed");
    tap_mana_except(&mut engine, p0, pasture);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(legal.abilities.contains(&(pasture, 1)), "the Goat");
    assert!(
        !legal.abilities.contains(&(pasture, 2)),
        "and no third ability: the X-Goat sacrifice is not written"
    );

    activate(&mut engine, p0, springjack_pasture(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, pasture));
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Goat");
    assert_eq!(pt(&engine, tokens[0]), (0, 1));
    assert!(types(&engine, tokens[0]).contains(TypeSet::CREATURE));
}

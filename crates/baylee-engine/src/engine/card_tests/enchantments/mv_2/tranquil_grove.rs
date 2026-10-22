//! `cards/enchantments/mv_2/tranquil_grove.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{BAD_MOON, CRUSADE};

fn tranquil_grove() -> CardIndex {
    card_index("2f2a3588-b9a6-4c1a-89f8-b11339851b8f")
}

/// `Tranquil Grove` ({1}{G}, Enchantment) pays `{1}{G}{G}` to destroy all other
/// enchantments while leaving itself on the battlefield.
#[test]
fn tranquil_grove_destroys_all_other_enchantments() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[tranquil_grove(), CRUSADE, forest(), forest(), forest()],
        )
        .battlefield(1, &[BAD_MOON])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, tranquil_grove(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, tranquil_grove()).is_some());
    assert!(in_graveyard(&engine, p0, CRUSADE).is_some());
    assert!(in_graveyard(&engine, p1, BAD_MOON).is_some());
}

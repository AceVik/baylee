//! `cards/enchantments/mv_2/leshrac_s_sigil.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn leshrac_s_sigil() -> CardIndex {
    card_index("98388eba-31ab-4302-be9e-ea67ea2b8c1a")
}

/// Leshrac's Sigil (98388eba-31ab-4302-be9e-ea67ea2b8c1a) — {B}{B} Enchantment:
/// "{B}{B}: Return this enchantment to its owner's hand."
#[test]
fn leshrac_s_sigil_casts_and_returns_itself_to_owners_hand_for_two_black() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[leshrac_s_sigil()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana_but(&mut engine, P0, None);
    cast_with_floating(&mut engine, P0, leshrac_s_sigil());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, leshrac_s_sigil()).is_some());

    activate(&mut engine, P0, leshrac_s_sigil(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, leshrac_s_sigil()).is_none()
            && in_hand(&engine, P0, leshrac_s_sigil()).is_some(),
        "activating {{B}}{{B}} returns Leshrac's Sigil to its owner's hand"
    );
}

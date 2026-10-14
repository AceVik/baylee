//! `cards/enchantments/mv_1/phyrexian_reclamation.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn phyrexian_reclamation() -> CardIndex {
    card_index("647ca69e-cc01-4b2b-b376-bee2a98331e8")
}

/// "{1}{B}, Pay 2 life: Return target creature card from your graveyard to
/// your hand."
#[test]
fn phyrexian_reclamation_pays_life_to_return_a_creature_card() {
    let mut e = table(
        7231,
        &[],
        &[phyrexian_reclamation(), index::GRIZZLY_BEARS],
        &[],
    );
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    bury(&mut e, &[bears]);
    let card = in_graveyard(&e, P0, index::GRIZZLY_BEARS).unwrap();
    fill_pool(&mut e);
    activate(&mut e, P0, phyrexian_reclamation(), 0);
    drive(&mut e, &[card], &[], &[], true);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(life(&e, P0), 18);
    assert!(in_hand(&e, P0, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&e, P0, index::GRIZZLY_BEARS).is_none());
}

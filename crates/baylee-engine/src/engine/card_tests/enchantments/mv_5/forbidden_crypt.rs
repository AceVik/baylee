//! `cards/enchantments/mv_5/forbidden_crypt.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{FORBIDDEN_CRYPT, GRIZZLY_BEARS, LIGHTNING_BOLT};

const P0: PlayerId = PlayerId::new(0);

/// Forbidden Crypt (8b34b211-985b-4934-bb3b-8e6672685ac2) — {3}{B}{B} Enchantment:
/// "If a card or token would be put into your graveyard from anywhere, exile it
/// instead."
#[test]
fn forbidden_crypt_exiles_cards_instead_of_putting_them_into_controllers_graveyard() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[FORBIDDEN_CRYPT, GRIZZLY_BEARS, mountain()])
        .hand(0, &[LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, LIGHTNING_BOLT);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, P0, GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, P0, LIGHTNING_BOLT).is_none());
    let exile = engine.state().zones.list(ZoneLocation::Exile(P0));
    assert!(exile.iter().any(
        |&id| engine.state().object(id).unwrap().card.map(|c| c.index) == Some(GRIZZLY_BEARS)
    ));
    assert!(exile.iter().any(
        |&id| engine.state().object(id).unwrap().card.map(|c| c.index) == Some(LIGHTNING_BOLT)
    ));
}

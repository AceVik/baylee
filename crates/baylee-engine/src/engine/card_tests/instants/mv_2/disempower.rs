//! `cards/instants/mv_2/disempower.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{DISEMPOWER, JAYEMDAE_TOME};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Disempower (ff9f4bfe-1585-4eaa-943c-1e85ed79a820) — {1}{W} Instant:
/// "Put target artifact or enchantment on top of its owner's library."
#[test]
fn disempower_puts_target_artifact_or_enchantment_on_top_of_owners_library() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .battlefield(1, &[JAYEMDAE_TOME])
        .hand(0, &[DISEMPOWER])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let tome = on_battlefield(&engine, P1, JAYEMDAE_TOME).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, DISEMPOWER);
    aim_at(&mut engine, P0, tome);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P1),
    );
    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, JAYEMDAE_TOME).is_none());
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(P1))
        .last()
        .unwrap();
    assert_eq!(
        engine.state().object(top).unwrap().card.map(|c| c.index),
        Some(JAYEMDAE_TOME)
    );
}

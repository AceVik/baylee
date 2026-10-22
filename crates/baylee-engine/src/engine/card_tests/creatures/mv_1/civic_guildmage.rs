//! `cards/creatures/mv_1/civic_guildmage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{CIVIC_GUILDMAGE, GRIZZLY_BEARS};

const P0: PlayerId = PlayerId::new(0);

/// Civic Guildmage (6e93cb9a-44c8-49fb-b6a7-bfdf7bd725fe) — {W} 1/1:
/// - "{G}, {T}: Target creature gets +0/+1 until end of turn."
/// - "{U}, {T}: Put target creature you control on top of its owner's library."
#[test]
fn civic_guildmage_boosts_toughness_and_puts_creature_on_top_of_library() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[CIVIC_GUILDMAGE, GRIZZLY_BEARS, forest(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let mage = on_battlefield(&engine, P0, CIVIC_GUILDMAGE).unwrap();
    let bears = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();

    // 1. Activate {G}, {T}: Target creature gets +0/+1 until end of turn.
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, CIVIC_GUILDMAGE, 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (2, 3));

    // 2. Untap Civic Guildmage and activate {U}, {T} to put Grizzly Bears on top of library.
    engine
        .dev_state_mut(P0)
        .unwrap()
        .object_mut(mage)
        .unwrap()
        .status
        .remove(Status::TAPPED);
    engine.refresh_offer();
    activate(&mut engine, P0, CIVIC_GUILDMAGE, 1);
    aim_at(&mut engine, P0, bears);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P0),
    );
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, GRIZZLY_BEARS).is_none());
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(P0))
        .last()
        .unwrap();
    assert_eq!(
        engine.state().object(top).unwrap().card.map(|c| c.index),
        Some(GRIZZLY_BEARS)
    );
}

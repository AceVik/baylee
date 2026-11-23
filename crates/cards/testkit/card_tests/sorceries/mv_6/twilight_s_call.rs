//! `cards/sorceries/mv_6/twilight_s_call.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

// oracle_id = "ae0a1d9c-19cb-42ee-97c3-464e38e84615"
fn twilight_s_call() -> CardIndex {
    card_index("ae0a1d9c-19cb-42ee-97c3-464e38e84615")
}

/// Both players return their creature cards under their own control, while a
/// noncreature card stays in the graveyard. The optional cost supplies instant
/// timing and brings the total payment to eight mana.
#[test]
fn twilight_s_call_with_flash_returns_creatures_for_each_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(); 8])
        .hand(0, &[twilight_s_call(), index::HILL_GIANT, lightning_bolt()])
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p1), "p1 reaches its own main");
    engine
        .apply(p1, PlayerAction::PassPriority)
        .expect("p1 gives p0 priority");
    let mine = hand_to_graveyard(&mut engine, p0, index::HILL_GIANT);
    let theirs = hand_to_graveyard(&mut engine, p1, index::GRIZZLY_BEARS);
    hand_to_graveyard(&mut engine, p0, lightning_bolt());
    tap_all_mana(&mut engine, p0);

    cast_sorcery_with_optional_flash(&mut engine, p0, twilight_s_call());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(on_battlefield(&engine, p0, index::HILL_GIANT), Some(mine));
    assert_eq!(
        on_battlefield(&engine, p1, index::GRIZZLY_BEARS),
        Some(theirs)
    );
    assert!(in_graveyard(&engine, p0, index::HILL_GIANT).is_none());
    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_none());
    assert!(
        in_graveyard(&engine, p0, lightning_bolt()).is_some(),
        "only creature cards return"
    );
}

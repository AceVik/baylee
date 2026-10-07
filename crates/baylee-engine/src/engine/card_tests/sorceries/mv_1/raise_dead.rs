//! `cards/sorceries/mv_1/raise_dead.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Raise Dead: "Return target creature card from your graveyard to your
/// hand." Hand, not the battlefield — the whole difference from
/// Resurrection.
#[test]
fn raise_dead_returns_a_creature_card_from_the_graveyard_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[raise_dead(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    hand_to_graveyard(&mut engine, p0, quiet_creature());
    let elf_in_gy = in_graveyard(&engine, p0, quiet_creature()).expect("in the graveyard");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, raise_dead());
    aim_at(&mut engine, p0, elf_in_gy);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p0, quiet_creature()).is_some(),
        "\"to your hand\""
    );
    assert!(in_graveyard(&engine, p0, quiet_creature()).is_none());
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_none(),
        "hand, not the battlefield"
    );
}

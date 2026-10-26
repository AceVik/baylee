//! `cards/creatures/mv_2/diplomatic_escort.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

/// Diplomatic Escort — {1}{U} 1/1 Creature — Human Spellshaper (Partial).
#[test]
fn diplomatic_escort_casts_as_a_one_one_spellshaper() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[index::DIPLOMATIC_ESCORT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, index::DIPLOMATIC_ESCORT);
    pass_until(&mut engine, stack_is_empty);

    let escort = on_battlefield(&engine, P0, index::DIPLOMATIC_ESCORT).unwrap();
    assert_eq!(pt(&engine, escort), (1, 1));
}

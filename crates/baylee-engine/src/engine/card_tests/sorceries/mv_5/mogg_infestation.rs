//! `cards/sorceries/mv_5/mogg_infestation.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mogg_infestation() -> CardIndex {
    card_index("cfd61b53-83c6-4886-8503-ddcb92dc7f85")
}

/// Mogg Infestation (`cfd61b53-83c6-4886-8503-ddcb92dc7f85`) — {3}{R}{R} Sorcery (Partial):
/// "Destroy all creatures target player controls."
#[test]
fn mogg_infestation_destroys_opposing_creatures() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS, index::LLANOWAR_ELVES])
        .hand(0, &[mogg_infestation()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, mogg_infestation());
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_none());
    assert!(on_battlefield(&engine, P1, index::LLANOWAR_ELVES).is_none());
}

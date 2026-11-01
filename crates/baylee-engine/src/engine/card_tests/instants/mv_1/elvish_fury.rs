//! `cards/instants/mv_1/elvish_fury.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn elvish_fury() -> CardIndex {
    card_index("557d2af0-04c5-461d-826b-7208c4535dc2")
}

/// Elvish Fury (`557d2af0-04c5-461d-826b-7208c4535dc2`) — {G} Instant (Partial):
/// "Buyback {4}. Target creature gets +2/+2 until end of turn."
#[test]
fn elvish_fury_gives_target_creature_plus_two_plus_two_until_end_of_turn() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), index::GRIZZLY_BEARS])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[elvish_fury()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    assert_eq!(pt(&engine, bears), (2, 2));

    cast_from_hand(&mut engine, P0, elvish_fury());
    let land = on_battlefield(&engine, P0, forest()).unwrap();
    let opposing = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    let menu = aim_at(&mut engine, P0, bears);
    assert!(menu.contains(&opposing));
    assert!(!menu.contains(&land));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (4, 4));
    assert_eq!(pt(&engine, opposing), (2, 2));
    assert!(in_graveyard(&engine, P0, elvish_fury()).is_some());

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, bears), (2, 2));
}

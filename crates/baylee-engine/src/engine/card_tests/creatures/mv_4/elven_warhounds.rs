//! `cards/creatures/mv_4/elven_warhounds.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn elven_warhounds() -> CardIndex {
    card_index("ae997fee-2c1f-43c8-9dba-8b409d8e449e")
}

/// Elven Warhounds (`ae997fee-2c1f-43c8-9dba-8b409d8e449e`) — {3}{G} Creature — Dog (Partial):
/// "Whenever this creature becomes blocked by a creature, put that creature on top of its owner's library."
#[test]
fn elven_warhounds_does_not_bounce_a_blocker_instead_of_topdecking_it() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[elven_warhounds()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let hounds = on_battlefield(&engine, P0, elven_warhounds()).unwrap();
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(hounds, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, hounds)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert!(in_hand(&engine, P1, index::GRIZZLY_BEARS).is_none());
    assert!(on_battlefield(&engine, P0, elven_warhounds()).is_some());
}

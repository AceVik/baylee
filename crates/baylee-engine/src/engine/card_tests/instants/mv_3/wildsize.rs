//! Oracle scenarios for Wildsize.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn wildsize() -> CardIndex {
    card_index("e1f439c7-217f-4b20-a77d-b9014c5de8c0")
}

// Checklist: target gets +2/+2 and trample until end of turn; card draw is
// one; actual combat sends excess damage through a chump blocker; the pump
// and keyword expire in the next turn.
#[test]
fn wildsize_pumps_grants_trample_in_combat_and_draws() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), index::GRIZZLY_BEARS])
        .hand(0, &[wildsize()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let target = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let blocker = on_battlefield(&e, p1, llanowar_elves()).unwrap();
    assert_eq!(pt(&e, target), (2, 2));
    assert!(!keywords(&e, target).contains(KeywordSet::TRAMPLE));
    let before = library_size(&e, p0);
    cast_from_hand(&mut e, p0, wildsize());
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![target],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, target), (4, 4));
    assert!(keywords(&e, target).contains(KeywordSet::TRAMPLE));
    assert_eq!(library_size(&e, p0), before - 1);

    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(target, Defender::Player(p1))],
        },
    )
    .unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == p1),
    );
    e.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(blocker, target)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        e.state()
            .object(target)
            .is_some_and(|object| object.damage == 1)
    });
    assert_eq!(
        e.state().players[1].life,
        17,
        "three excess power tramples past the 1/1 blocker"
    );
    assert!(in_graveyard(&e, p1, llanowar_elves()).is_some());
    assert_eq!(e.state().object(target).unwrap().damage, 1);
    reach_their_main_phase(&mut e, p1);
    assert_eq!(pt(&e, target), (2, 2), "the +2/+2 expires at end of turn");
    assert!(!keywords(&e, target).contains(KeywordSet::TRAMPLE));
}

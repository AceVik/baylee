//! Oracle scenarios for Consume Strength.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn consume_strength() -> CardIndex {
    card_index("53fd9fb1-c28f-4d73-bf98-94cdab418f21")
}

#[test]
fn consume_strength_pumps_first_target_and_shrinks_another_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::SWAMP, forest(), forest(), index::GRIZZLY_BEARS])
        .battlefield(1, &[index::HILL_GIANT, index::LLANOWAR_ELVES])
        .hand(0, &[consume_strength()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let bears = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&e, p1, index::HILL_GIANT).unwrap();

    cast_from_hand(&mut e, p0, consume_strength());
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected first target prompt")
    };
    assert!(options.contains(&bears) && options.contains(&giant));
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![bears],
        },
    )
    .unwrap();

    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected second target prompt")
    };
    assert!(
        !options.contains(&bears),
        "Another target creature cannot be the same creature as the first target"
    );
    assert!(options.contains(&giant));
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![giant],
        },
    )
    .unwrap();

    pass_until(&mut e, stack_is_empty);

    assert_eq!(pt(&e, bears), (4, 4), "Grizzly Bears gets +2/+2");
    assert_eq!(pt(&e, giant), (1, 1), "Hill Giant gets -2/-2");

    reach_their_main_phase(&mut e, p1);
    assert_eq!(pt(&e, bears), (2, 2), "+2/+2 expires at end of turn");
    assert_eq!(pt(&e, giant), (3, 3), "-2/-2 expires at end of turn");
}

//! Oracle scenarios for Rotting Giant.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn rotting_giant() -> CardIndex {
    card_index("09bf80cc-1112-4f4c-9431-92d741bf2230")
}

#[test]
fn rotting_giant_exiles_card_from_graveyard_when_blocking_or_is_sacrificed_when_attacking() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::SWAMP)
        .battlefield(0, &[index::SWAMP, index::SWAMP, index::PLAINS])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[rotting_giant()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let plains = on_battlefield(&e, p0, index::PLAINS).unwrap();
    bury(&mut e, &[plains]);

    cast_from_hand(&mut e, p0, rotting_giant());
    pass_until(&mut e, stack_is_empty);

    let giant = on_battlefield(&e, p0, rotting_giant()).unwrap();
    let bear = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();

    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    e.apply(
        p1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bear, Defender::Player(p0))],
        },
    )
    .unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == p0),
    );
    e.apply(
        p0,
        PlayerAction::DeclareBlockers {
            blockers: vec![(giant, bear)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("expected ChooseCards for exiling a card from graveyard");
    };
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![options[0]],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, p0, rotting_giant()).is_some());
    assert_eq!(e.state().object(plains).map(|o| o.zone), Some(Zone::Exile));

    // Advance to P0's next combat with an empty graveyard: attacking sacrifices Rotting Giant.
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(giant, Defender::Player(p1))],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, p0, rotting_giant()).is_none());
    assert!(in_graveyard(&e, p0, rotting_giant()).is_some());
}

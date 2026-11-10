//! Oracle checklist: tap as the activation cost; draw first, then choose and discard exactly one from the resulting hand; a preexisting card remains a valid discard.
#[allow(clippy::wildcard_imports)]
use super::*;

fn thought_courier() -> CardIndex {
    card_index("06ea02f5-f296-4334-bb6f-9cb018e1e150")
}

#[test]
fn courier_draws_before_the_required_discard_and_stays_tapped() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[thought_courier()])
        .hand(0, &[island(), mountain()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let source = on_battlefield(&e, p0, thought_courier()).unwrap();
    // Let the creature be under its controller since the beginning of a turn.
    reach_their_main_phase(&mut e, PlayerId::new(1));
    pass_until(&mut e, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    let drawn = hand_to_library_top(&mut e, p0, mountain());

    activate(&mut e, p0, thought_courier(), 0);
    assert!(
        is_tapped(&e, source),
        "{{T}} is paid before the ability resolves"
    );
    assert!(
        in_hand(&e, p0, mountain()).is_none(),
        "the draw is pending until resolution"
    );
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = e.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    assert!(options.iter().any(|id| {
        e.state()
            .object(*id)
            .is_some_and(|o| o.card.is_some_and(|c| c.index == island()))
    }));
    let island_id = in_hand(&e, p0, island()).unwrap();
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![island_id],
        },
    )
    .unwrap();

    assert!(in_graveyard(&e, p0, island()).is_some());
    assert_eq!(in_hand(&e, p0, mountain()), Some(drawn));
    assert!(is_tapped(&e, source));
}

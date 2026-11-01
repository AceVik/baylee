//! Journey of Discovery entwined: fetch two basics and play three lands.

use super::*;

fn journey_of_discovery() -> CardIndex {
    card_index("1c586d8a-9d1a-48a7-bb3e-9b2c0c329f8d")
}

#[test]
fn journey_of_discovery_entwined_searches_and_grants_two_extra_land_drops() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 6])
        .hand(0, &[journey_of_discovery(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, journey_of_discovery());
    let Pending::ChooseCastMode { options, .. } = e.pending().clone() else {
        panic!("expected modes")
    };
    let mode = options
        .iter()
        .position(|option| option.kind == CastModeKind::Modes(3))
        .expect("entwine is offered");
    e.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = e.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!((min, max), (0, 2));
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: options.into_iter().take(2).collect(),
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(p0)).len(), 3);

    for _ in 0..3 {
        play_land(&mut e, p0, forest());
    }
    assert_eq!(all_on_battlefield(&e, p0, forest()).len(), 9);
}

/// "Reveal them": the two basics found go into the public journal as one
/// `Revealed` by the caster, which is what every other seat's view reads
/// (`resolve::search::reveals`: a filtered search into a hidden zone).
#[test]
fn journey_of_discovery_reveals_the_lands_it_finds() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 3])
        .hand(0, &[journey_of_discovery()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, journey_of_discovery());
    let Pending::ChooseCastMode { options, .. } = e.pending().clone() else {
        panic!("expected modes")
    };
    let mode = options
        .iter()
        .position(|option| {
            option.kind == CastModeKind::Mode(0) || option.kind == CastModeKind::Modes(0b01)
        })
        .expect("the search mode is offered on its own");
    e.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    let found: Vec<ObjectId> = options.into_iter().take(2).collect();
    let since = e.state().journal.last_seq();
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: found.clone(),
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    let mut revealed: Vec<ObjectId> = e
        .state()
        .journal
        .entries()
        .iter()
        .filter(|entry| entry.seq > since)
        .filter_map(|entry| match &entry.event {
            crate::event::GameEvent::Revealed { player, cards } if *player == p0 => {
                Some(cards.clone())
            }
            _ => None,
        })
        .flatten()
        .collect();
    let mut expected = found.clone();
    revealed.sort();
    expected.sort();
    assert_eq!(revealed, expected);
    assert!(
        found
            .iter()
            .all(|&card| e.state().object(card).unwrap().zone == Zone::Hand)
    );
}

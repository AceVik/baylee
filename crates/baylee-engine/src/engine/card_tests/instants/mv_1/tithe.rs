//! `cards/instants/mv_1/tithe.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn tithe() -> CardIndex {
    card_index("318304f5-0929-4cd3-97e6-08ac1ac48aa6")
}

/// Casts Tithe at `p1` and walks to the library search; returns the offer's `max`: how many
/// Plains cards may be taken.
fn tithe_search_max(engine: &mut Engine<RegistryLookup>) -> u8 {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    cast_from_hand(engine, p0, tithe());
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { max, options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(!options.is_empty(), "the library holds Plains");
    max
}

/// `Tithe` ({W}, Instant): "Search your library for a Plains card. If target
/// opponent controls more lands than you, you may search … for an additional
/// Plains card." The opponent controls two lands to one: two Plains may be
/// taken and both reach the hand.
#[test]
fn tithe_searches_for_two_plains_when_the_target_opponent_controls_more_lands() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains()])
        .battlefield(1, &[forest(), forest()])
        .hand(0, &[tithe()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    assert_eq!(tithe_search_max(&mut engine), 2, "an additional Plains");
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0], options[1]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Tithe left the hand and two Plains came in.
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p1)).len(), 0);
}

/// Equal land counts: not "more", so the search is for one Plains only.
#[test]
fn tithe_searches_for_one_plains_when_the_opponent_controls_as_many_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains()])
        .battlefield(1, &[forest()])
        .hand(0, &[tithe()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert_eq!(tithe_search_max(&mut engine), 1, "no additional Plains");
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "one Plains in the hand, Tithe in the graveyard"
    );
}

/// The opponent controls fewer lands than you: one Plains.
#[test]
fn tithe_searches_for_one_plains_when_the_opponent_controls_fewer_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .battlefield(1, &[forest()])
        .hand(0, &[tithe()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert_eq!(tithe_search_max(&mut engine), 1);
}

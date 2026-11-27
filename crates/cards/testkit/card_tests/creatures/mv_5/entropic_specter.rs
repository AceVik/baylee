//! `cards/creatures/mv_5/entropic_specter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as kit;

const P0: PlayerId = PlayerId::new(0);
fn toughness_of(engine: &Engine<RegistryLookup>, object: ObjectId) -> Option<i16> {
    engine
        .state()
        .object(object)
        .and_then(|o| o.characteristics().toughness)
}

const P1: PlayerId = PlayerId::new(1);

fn entropic_specter() -> CardIndex {
    card_index("a9c44443-85c7-4d0a-82ab-36137151e4e5")
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

#[test]
fn entropic_specter_tracks_chosen_opponents_hand_and_forces_discard_on_damage() {
    let mut e = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[entropic_specter()])
        .hand(1, &[swamp(), swamp()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    cast_from_hand(&mut e, P0, entropic_specter());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    e.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut e, stack_is_empty);

    let specter = on_battlefield(&e, P0, entropic_specter()).unwrap();
    assert!(keywords_of(&e, specter).contains(KeywordSet::FLYING));
    assert_eq!(
        (power_of(&e, specter), toughness_of(&e, specter)),
        (Some(2), Some(2))
    );

    // Untap/next P0 turn: attack unblocked -> deals 2, P1 discards 1, Specter shrinks accordingly!
    reach_their_main_phase(&mut e, P1);
    let p1_hand_before = e.state().zones.list(ZoneLocation::Hand(P1)).len();
    reach_their_main_phase(&mut e, P0);
    kit::attack(&mut e, &[specter]);
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!();
    };
    e.apply(
        P1,
        PlayerAction::ChooseObjects {
            objects: vec![options[0]],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    let p1_hand_after = e.state().zones.list(ZoneLocation::Hand(P1)).len();
    assert_eq!(p1_hand_after, p1_hand_before - 1);
    let expected = i16::try_from(p1_hand_after).unwrap();
    assert_eq!(
        (power_of(&e, specter), toughness_of(&e, specter)),
        (Some(expected), Some(expected))
    );
}

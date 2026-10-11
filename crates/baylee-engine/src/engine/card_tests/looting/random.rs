//! Random selection belongs to payment; Gamble shuffles after its discard.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn random_gamble_finishes_its_shuffle_after_leng_places_the_discard_on_top() {
    let mut e = ready(&[index::GAMBLE], &[index::LIBRARY_OF_LENG]);
    cast_with_floating(&mut e, P0, index::GAMBLE);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    let selected = options[0];
    let random_before = e.state().rng.calls();
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![selected],
        },
    )
    .unwrap();
    assert_eq!(
        e.state().rng.calls(),
        random_before,
        "search has not shuffled yet and the sole hand card needs no RNG"
    );
    assert!(matches!(e.pending(), Pending::Arrange { .. }));
    let mut expected_rng = e.state().rng.clone();
    let mut expected = e.state().zones.list(ZoneLocation::Library(P0)).clone();
    expected.push(selected);
    expected_rng.shuffle(&mut expected);
    e.apply(
        P0,
        PlayerAction::Arrange {
            piles: vec![vec![], vec![selected]],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().zones.list(ZoneLocation::Library(P0)), &expected);
    assert_eq!(e.state().rng.calls(), expected_rng.calls());
}

#[test]
fn random_coral_helm_pays_before_resolution_and_never_asks_which_card_to_discard() {
    let mut outcomes = std::collections::BTreeSet::new();
    for seed in 0..24 {
        let mut e = prepared(
            Duel::new(seed, index::FOREST)
                .hand(0, &[index::FOREST, index::GRIZZLY_BEARS, index::ISLAND])
                .battlefield(
                    0,
                    &[
                        index::CORAL_HELM,
                        index::GRIZZLY_BEARS,
                        index::LIBRARY_OF_LENG,
                    ],
                ),
        );
        let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
        let hand = e.state().zones.list(ZoneLocation::Hand(P0)).clone();
        let mut expected_rng = e.state().rng.clone();
        let expected = hand[expected_rng.below(hand.len() as u64) as usize];
        let before = e.state().rng.calls();
        e.refresh_offer();
        e.refresh_offer();
        assert_eq!(
            e.state().rng.calls(),
            before,
            "affordability never consumes randomness"
        );
        activate(&mut e, P0, index::CORAL_HELM, 0);
        aim_at(&mut e, P0, bear);
        assert!(matches!(e.pending(), Pending::Priority { .. }));
        assert_eq!(e.state().object(expected).unwrap().zone, Zone::Graveyard);
        assert_eq!(
            pt(&e, bear),
            (2, 2),
            "payment happened while the effect still waits on the stack"
        );
        outcomes.insert(e.state().object(expected).unwrap().card.unwrap().index);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(pt(&e, bear), (4, 4));
        assert_eq!(e.state().rng.calls(), expected_rng.calls());
    }
    assert_eq!(outcomes.len(), 3, "all hand positions can be selected");
    let e = ready(&[], &[index::CORAL_HELM, index::GRIZZLY_BEARS]);
    let helm = on_battlefield(&e, P0, index::CORAL_HELM).unwrap();
    let Pending::Priority { legal, .. } = e.pending() else {
        unreachable!()
    };
    assert!(
        !legal.abilities.contains(&(helm, 0)),
        "empty hand cannot pay"
    );
}

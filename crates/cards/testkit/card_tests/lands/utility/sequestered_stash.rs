//! `cards/lands/utility/sequestered_stash.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sequestered Stash: "{T}: Add {C}." / "{4}, {T}, Sacrifice this land: Mill five cards. Then you may put an artifact card from your graveyard on top of your library."
/// Under `Coverage::Partial`, placing an artifact from graveyard on top of the library is unsupported.
/// Activating ability 1 for `{4}` and sacrificing this land mills five cards from the library into the graveyard.
#[test]
fn sequestered_stash_sacrifices_to_mill_five_cards() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(135, forest())
        .battlefield(
            0,
            &[sequestered_stash(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let stash =
        on_battlefield(&engine, p0, sequestered_stash()).expect("Sequestered Stash deployed");
    let lib_before = library_size(&engine, p0);

    tap_mana_except(&mut engine, p0, stash);
    activate(&mut engine, p0, sequestered_stash(), 1);

    assert!(in_graveyard(&engine, p0, sequestered_stash()).is_some());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(library_size(&engine, p0), lib_before - 5);
}

#[test]
fn stash_can_decline_or_put_a_newly_milled_artifact_on_top() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for take in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::SEQUESTERED_STASH])
            .hand(0, &[index::MIND_STONE])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let stone = hand_to_library_top(&mut e, p0, index::MIND_STONE);
        let before = library_size(&e, p0);
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Colorless, 4);
        e.refresh_offer();
        activate(&mut e, p0, index::SEQUESTERED_STASH, 1);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: if take { vec![stone] } else { vec![] },
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(library_size(&e, p0), before - 5 + usize::from(take));
        if take {
            assert_eq!(
                e.state().zones.list(ZoneLocation::Library(p0)).last(),
                Some(&stone)
            );
        } else {
            assert_eq!(e.state().object(stone).unwrap().zone, Zone::Graveyard);
        }
    }
}

/// "Then you may put an artifact card from your graveyard on top of your
/// library.": the menu holds the artifact it just milled, and neither a
/// milled land nor an artifact in the opponent's graveyard.
#[test]
fn stash_offers_only_artifact_cards_of_its_controllers_graveyard() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::SEQUESTERED_STASH])
        .hand(0, &[index::MIND_STONE])
        .hand(1, &[index::SOL_RING])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let stash = on_battlefield(&e, p0, index::SEQUESTERED_STASH).unwrap();
    let stone = hand_to_library_top(&mut e, p0, index::MIND_STONE);
    let their_ring = hand_to_graveyard(&mut e, p1, index::SOL_RING);
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 4);
    e.refresh_offer();
    activate(&mut e, p0, index::SEQUESTERED_STASH, 1);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    let milled_lands: Vec<_> = e
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .iter()
        .copied()
        .filter(|id| ![stone, stash].contains(id))
        .collect();
    assert_eq!(
        milled_lands.len(),
        4,
        "the other four milled cards are lands"
    );
    assert!(!options.contains(&their_ring), "not an opponent's artifact");
    assert!(
        milled_lands.iter().all(|land| !options.contains(land)),
        "not a milled non-artifact: {options:?}"
    );
    assert_eq!(options, vec![stone], "the milled artifact alone");
}

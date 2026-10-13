//! Batch 43: naming behavior through real cards.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch43_petra_named_player_chooses_and_matching_card_goes_to_their_hand() {
    for matched in [false, true] {
        let mut e = prepared(&[], &[index::PETRA_SPHINX], &[]);
        let top = *library(&e, P1).last().unwrap();
        activate(&mut e, P0, index::PETRA_SPHINX, 0);
        target(&mut e, vec![], vec![P1]);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCardName { .. })
        });
        assert_eq!(e.pending().asked(), Some(P1));
        name(
            &mut e,
            if matched {
                index::FOREST
            } else {
                index::ISLAND
            },
        );
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().object(top).unwrap().zone,
            if matched { Zone::Hand } else { Zone::Graveyard }
        );
        assert!(e.state().journal.entries().iter().any(|entry|matches!(&entry.event,GameEvent::Revealed{player,cards} if *player==P1&&cards.contains(&top))));
    }
}

#[test]
fn batch43_petrified_hamlet_restricts_names_and_preserves_mana_abilities() {
    let mut e = prepared(&[index::PETRIFIED_HAMLET], &[], &[index::WASTELAND]);
    let land = on_battlefield(&e, P1, index::WASTELAND).unwrap();
    let hamlet = play_land(&mut e, P0, index::PETRIFIED_HAMLET);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    let hash = e.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseCardName {
                card: index::SOL_RING,
                face: 0
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), hash);
    name(&mut e, index::WASTELAND);
    pass_until(&mut e, stack_is_empty);
    assert!(e.state().object(hamlet).unwrap().chosen_name.is_some());
    let choices = e.state().object(land).unwrap().characteristics().types;
    assert!(choices.contains(baylee_core::types::TypeSet::LAND));
    // The affected opponent receives priority to inspect the actual legal offer.
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, legal } = e.pending() else {
        panic!("priority")
    };
    assert_eq!(*player, P1);
    let activated: Vec<_> = legal
        .abilities
        .iter()
        .filter(|(id, _)| *id == land)
        .copied()
        .collect();
    assert!(
        !activated.is_empty(),
        "mana abilities remain and the named land gains another"
    );
    assert!(
        !activated.contains(&(land, 1)),
        "Wasteland's destructive ability is prohibited"
    );
}

#[test]
fn batch43_nebuchadnezzar_announces_x_then_names_and_randomly_reveals_only_that_many() {
    for count in [0, 1, 3, 12] {
        let mut e = prepared(&[], &[index::NEBUCHADNEZZAR], &[]);
        let before = e.state().zones.list(ZoneLocation::Hand(P1)).len();
        activate(&mut e, P0, index::NEBUCHADNEZZAR, 0);
        for _ in 0..3 {
            match e.pending() {
                Pending::ChooseNumber { .. } => {
                    e.apply(P0, PlayerAction::ChooseNumber(count)).unwrap();
                }
                Pending::ChoosePlayer { .. } => {
                    e.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
                }
                Pending::ChooseTargets { .. } => target(&mut e, vec![], vec![P1]),
                _ => break,
            }
        }
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCardName { .. })
        });
        assert_eq!(e.pending().asked(), Some(P0));
        let since = e.state().journal.last_seq();
        name(&mut e, index::FOREST);
        pass_until(&mut e, stack_is_empty);
        let revealed: Vec<_> = e
            .state()
            .journal
            .entries()
            .iter()
            .filter(|entry| entry.seq > since)
            .flat_map(|entry| match &entry.event {
                GameEvent::Revealed { player, cards } if *player == P1 => cards.clone(),
                _ => vec![],
            })
            .collect();
        assert_eq!(revealed.len(), before.min(count as usize));
        for &id in &revealed {
            assert_eq!(e.state().object(id).unwrap().zone, Zone::Graveyard);
        }
        assert_eq!(
            e.state().zones.list(ZoneLocation::Hand(P1)).len(),
            before - revealed.len()
        );
    }
}

#[test]
fn batch43_linked_name_is_not_reused_by_a_new_copy_of_the_same_rules() {
    let mut e = prepared(&[index::PETRIFIED_HAMLET], &[], &[]);
    let source = play_land(&mut e, P0, index::PETRIFIED_HAMLET);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    name(&mut e, index::WASTELAND);
    pass_until(&mut e, stack_is_empty);
    assert!(e.state().object(source).unwrap().chosen_name.is_some());
    let old = e.state().object(source).unwrap().rules_acquisition();
    copy_rules_fixture(&mut e, source, source, false);
    assert_ne!(e.state().object(source).unwrap().rules_acquisition(), old);
    assert!(e.state().object(source).unwrap().chosen_name.is_none());
}

#[test]
fn batch43_old_naming_trigger_keeps_its_link_while_a_temporary_copy_is_active() {
    let mut e = prepared(&[index::PETRIFIED_HAMLET], &[index::FOREST], &[]);
    let forest = on_battlefield(&e, P0, index::FOREST).unwrap();
    let source = play_land(&mut e, P0, index::PETRIFIED_HAMLET);
    copy_rules_fixture(&mut e, source, forest, true);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    name(&mut e, index::WASTELAND);
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state().object(source).unwrap().chosen_name.is_none(),
        "the copied Forest has no linked name"
    );
    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.step == crate::turn::Step::Main
    });
    let object = e.state().object(source).unwrap();
    assert_eq!(object.rules_acquisition(), 0);
    assert_eq!(
        object.chosen_name.unwrap().card(),
        index::WASTELAND,
        "the original linked abilities recover their own choice"
    );
}

/// Nebuchadnezzar on P0's side against a P1 who holds `hand`; P0 has mana for
/// any X. `prepared` seats no P1 hand, and the hand is what these tests read.
fn nebuchadnezzar_against(hand: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::NEBUCHADNEZZAR])
        .hand(1, hand)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    for color in ManaColor::ALL {
        e.dev_state_mut(P0).unwrap().players[0]
            .mana_pool
            .add(color, 30);
    }
    e.refresh_offer();
    e
}

#[test]
fn batch43_nebuchadnezzar_discards_only_the_revealed_cards_that_carry_the_named_name() {
    // "Then that player discards all cards with that name revealed this way":
    // X reveals the whole three-card hand, and the name picks two of the three.
    let mut e = nebuchadnezzar_against(&[index::GRIZZLY_BEARS, index::FOREST, index::FOREST]);
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(P1)).len(), 3);
    activate(&mut e, P0, index::NEBUCHADNEZZAR, 0);
    for _ in 0..3 {
        match e.pending() {
            Pending::ChooseNumber { .. } => {
                e.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
            }
            Pending::ChooseTargets { .. } => target(&mut e, vec![], vec![P1]),
            _ => break,
        }
    }
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    let since = e.state().journal.last_seq();
    name(&mut e, index::FOREST);
    pass_until(&mut e, stack_is_empty);
    let revealed: Vec<_> = e
        .state()
        .journal
        .entries()
        .iter()
        .filter(|entry| entry.seq > since)
        .flat_map(|entry| match &entry.event {
            GameEvent::Revealed { player, cards } if *player == P1 => cards.clone(),
            _ => vec![],
        })
        .collect();
    assert_eq!(
        revealed.len(),
        3,
        "X = 3 reveals all three cards of the hand"
    );
    let bears = in_hand(&e, P1, index::GRIZZLY_BEARS);
    assert!(
        bears.is_some_and(|id| revealed.contains(&id)),
        "the Bears were revealed too: only their name keeps them from the graveyard"
    );
    let forests_discarded = e
        .state()
        .zones
        .list(ZoneLocation::Graveyard(P1))
        .iter()
        .filter(|id| {
            e.state()
                .object(**id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == index::FOREST))
        })
        .count();
    assert_eq!(
        forests_discarded, 2,
        "discards all cards with that name revealed this way"
    );
    assert!(
        in_graveyard(&e, P1, index::GRIZZLY_BEARS).is_none(),
        "a revealed card with another name is not discarded"
    );
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(P1)),
        &[bears.unwrap()],
        "only the Bears are left in the hand"
    );
}

#[test]
fn batch43_nebuchadnezzar_is_activated_only_during_its_controllers_turn() {
    let mut e = nebuchadnezzar_against(&[index::FOREST]);
    let neb = on_battlefield(&e, P0, index::NEBUCHADNEZZAR).unwrap();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(
        legal.abilities.contains(&(neb, 0)),
        "on P0's own turn, untapped, with X = 0 payable, the ability is offered"
    );
    reach_their_main_phase(&mut e, P1);
    e.apply(P1, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, legal } = e.pending() else {
        panic!(
            "expected P0 to hold priority in P1's turn, got {:?}",
            e.pending()
        )
    };
    assert_eq!(*player, P0, "P0 holds priority during P1's turn");
    assert!(
        !is_tapped(&e, neb),
        "Nebuchadnezzar is untapped, so only the turn restriction can be what removes it"
    );
    assert!(
        !legal.abilities.contains(&(neb, 0)),
        "Activate only during your turn: not offered in the opponent's turn"
    );
}

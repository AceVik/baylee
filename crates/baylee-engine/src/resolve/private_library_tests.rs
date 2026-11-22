use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use baylee_core::ids::SeatSet;
use std::sync::Arc;

fn table(count: usize) -> (GameState, ObjectId, Vec<ObjectId>) {
    let mut state =
        GameState::from_preset(&preset(415, &[]), &SyntheticLookup::new(vec![])).unwrap();
    let you = PlayerId::new(0);
    let name = state.names.intern("source");
    let source = state.create_bare(you, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
    let library: Vec<_> = state.zones.list(ZoneLocation::Library(you)).clone();
    for card in library {
        state.zones.remove(card, ZoneLocation::Library(you));
    }
    let cards = (0..count)
        .map(|_| {
            let name = state.names.intern("library card");
            state.create_bare(you, ObjectKind::Card, name, ZoneLocation::Library(you))
        })
        .collect();
    (state, source, cards)
}

fn no_reveal(state: &GameState) {
    assert!(
        !state
            .journal
            .entries()
            .iter()
            .any(|entry| matches!(entry.event, GameEvent::Revealed { .. }))
    );
    assert!(
        !state
            .journal
            .entries()
            .iter()
            .any(|entry| matches!(entry.event, GameEvent::CardsDrawn { .. })),
        "putting a card in hand is not drawing it"
    );
}

#[test]
fn library_or_graveyard_search_requires_consent_before_inspection_or_shuffle() {
    let you = PlayerId::new(0);
    for (graveyard_match, accept) in [(false, false), (false, true), (true, false), (true, true)] {
        let (mut state, source, _) = table(3);
        let mut res = resolution(
            source,
            &[Effect::SearchLibraryOrGraveyard {
                filter: &baylee_cards_dsl::Filter::CREATURE,
                find: &baylee_cards_dsl::Find::BATTLEFIELD,
            }],
            you,
        );
        let before = state.journal.last_seq();
        let graveyard = graveyard_match.then(|| {
            let name = state.names.intern("graveyard creature");
            let card = state.create_bare(you, ObjectKind::Card, name, ZoneLocation::Graveyard(you));
            let object = state.object_mut(card).unwrap();
            Arc::make_mut(&mut object.base).types = baylee_cards_dsl::TypeSet::CREATURE;
            card
        });
        let first = if let Some(card) = graveyard {
            let Flow::Wait(Pending::ChooseCards {
                prompt: ChoicePrompt::FromGraveyard,
                options,
                min: 0,
                ..
            }) = run(&mut state, &mut res)
            else {
                panic!("expected an optional graveyard find");
            };
            assert_eq!(options, [card]);
            resume(&mut state, &mut res, &[])
        } else {
            run(&mut state, &mut res)
        };
        assert!(matches!(
            first,
            Flow::Wait(Pending::YesNo {
                player,
                prompt: YesNoPrompt::SearchLibrary,
                ..
            }) if player == you
        ));
        assert_eq!(state.journal.last_seq(), before);
        let flow = resume_yes_no(&mut state, &mut res, accept);
        if accept {
            let Flow::Wait(Pending::ChooseCards {
                player,
                prompt: ChoicePrompt::SearchLibraryOrGraveyard,
                options,
                max,
                ..
            }) = flow
            else {
                panic!("expected a pick after searching both zones");
            };
            assert_eq!(player, you);
            assert_eq!(options, graveyard.as_slice());
            assert_eq!(max, u8::from(graveyard_match));
            assert!(matches!(
                resume(&mut state, &mut res, graveyard.as_slice()),
                Flow::Complete
            ));
        } else {
            assert!(matches!(flow, Flow::Complete));
        }
        if let Some(card) = graveyard {
            assert_eq!(
                state.object(card).unwrap().zone,
                if accept {
                    crate::zone::Zone::Battlefield
                } else {
                    crate::zone::Zone::Graveyard
                }
            );
        }
        assert_eq!(
            state
                .journal
                .entries()
                .iter()
                .filter(|entry| matches!(
                    entry.event,
                    GameEvent::LibrarySearched { player } if entry.seq > before && player == you
                ))
                .count(),
            usize::from(accept)
        );
        assert_eq!(
            state
                .journal
                .entries()
                .iter()
                .filter(|entry| matches!(
                    entry.event,
                    GameEvent::Shuffled { player, .. } if entry.seq > before && player == you
                ))
                .count(),
            usize::from(accept)
        );
    }
}

#[test]
fn library_or_graveyard_search_obeys_search_prohibitions_and_limits() {
    use baylee_cards_dsl::{Duration, Filter, Find, Modifier};
    use baylee_core::ids::EffectId;

    let you = PlayerId::new(0);
    for modifier in [
        Modifier::OpponentsCantSearch,
        Modifier::OpponentsSearchTop(1),
    ] {
        let (mut state, source, _) = table(3);
        let timestamp = state.next_timestamp();
        state.effects.register(crate::effects::ContinuousEffect {
            id: EffectId::new(0),
            source: None,
            controller: PlayerId::new(1),
            origin: crate::effects::EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp,
            duration: Duration::Indefinitely,
            filter: crate::effects::EffectFilter::Dsl(&Filter::Any),
            modifier,
        });
        let mut res = resolution(
            source,
            &[Effect::SearchLibraryOrGraveyard {
                filter: &Filter::Any,
                find: &Find::HAND,
            }],
            you,
        );
        let first = run(&mut state, &mut res);
        if modifier == Modifier::OpponentsCantSearch {
            assert!(matches!(first, Flow::Complete));
            assert!(!state.journal.entries().iter().any(|entry| matches!(
                entry.event,
                GameEvent::LibrarySearched { player } if player == you
            )));
            continue;
        }
        let top = *state.zones.list(ZoneLocation::Library(you)).last().unwrap();
        assert!(matches!(
            first,
            Flow::Wait(Pending::YesNo {
                prompt: YesNoPrompt::SearchLibrary,
                ..
            })
        ));
        let Flow::Wait(Pending::ChooseCards { options, .. }) =
            resume_yes_no(&mut state, &mut res, true)
        else {
            panic!("expected a search of the permitted segment");
        };
        assert_eq!(options, [top]);
        let Some(AwaitingOp::SearchLibrary { inspected, .. }) = res.awaiting else {
            panic!("expected a suspended library search");
        };
        assert_eq!(inspected, [top]);
    }
}

#[test]
fn required_union_continues_the_same_library_search_before_the_final_shuffle() {
    use baylee_cards_dsl::{Duration, Filter, Find, Modifier};
    use baylee_core::ids::EffectId;

    let you = PlayerId::new(0);
    let (mut state, source, _) = table(3);
    let top = *state.zones.list(ZoneLocation::Library(you)).last().unwrap();
    let name = state.names.intern("named card");
    let graveyard = state.create_bare(you, ObjectKind::Card, name, ZoneLocation::Graveyard(you));
    let modifier = Modifier::OpponentsSearchTop(1);
    let timestamp = state.next_timestamp();
    state.effects.register(crate::effects::ContinuousEffect {
        id: EffectId::new(0),
        source: None,
        controller: PlayerId::new(1),
        origin: crate::effects::EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp,
        duration: Duration::Indefinitely,
        filter: crate::effects::EffectFilter::Dsl(&Filter::Any),
        modifier,
    });
    let mut res = resolution(
        source,
        &[
            Effect::SearchLibraryBeforeShuffle {
                filter: &Filter::Any,
                finds: &[Find::BATTLEFIELD_TAPPED],
                optional: false,
            },
            Effect::SearchLibraryAndGraveyard {
                filter: &Filter::Named("named card"),
                find: &Find::HAND,
            },
        ],
        you,
    );
    let before = state.journal.last_seq();
    let Flow::Wait(Pending::ChooseCards { options, .. }) = run(&mut state, &mut res) else {
        panic!("expected the first search choice");
    };
    assert_eq!(options, [top]);
    let Flow::Wait(Pending::ChooseCards {
        prompt: ChoicePrompt::SearchLibraryOrGraveyard,
        options,
        min: 1,
        ..
    }) = resume(&mut state, &mut res, &[top])
    else {
        panic!("expected the required graveyard find");
    };
    assert_eq!(options, [graveyard]);
    assert!(matches!(
        &res.awaiting,
        Some(AwaitingOp::SearchLibrary { inspected, .. }) if inspected.is_empty()
    ));
    assert!(matches!(
        resume(&mut state, &mut res, &[graveyard]),
        Flow::Complete
    ));
    let order: Vec<_> = state.journal.entries()[before as usize..]
        .iter()
        .filter(|line| {
            matches!(
                line.event,
                GameEvent::LibrarySearched { .. }
                    | GameEvent::Revealed { .. }
                    | GameEvent::Shuffled { .. }
            )
        })
        .map(|line| &line.event)
        .collect();
    assert!(matches!(
        order.as_slice(),
        [
            GameEvent::LibrarySearched { .. },
            GameEvent::Revealed { .. },
            GameEvent::Shuffled { .. }
        ]
    ));
    assert!(
        state
            .zones
            .list(ZoneLocation::Hand(you))
            .contains(&graveyard)
    );
}

#[test]
fn mandatory_unfiltered_search_exiles_exactly_available_cards_for_library_owner() {
    for size in [0, 1, 4] {
        let (mut state, source, cards) = table(size);
        let you = PlayerId::new(0);
        let mut res = resolution(
            source,
            &[Effect::SearchLibraryOf {
                library: PlayerRel::Chosen,
                owner_searches: false,
                filter: &baylee_cards_dsl::Filter::Any,
                mana_value: None,
                finds: &[baylee_cards_dsl::Find::EXILE; 3],
                optional: false,
            }],
            you,
        );
        res.controller = PlayerId::new(1);
        match run(&mut state, &mut res) {
            Flow::Complete => assert_eq!(size, 0),
            Flow::Wait(Pending::ChooseCards {
                player, min, max, ..
            }) => {
                assert_eq!(player, PlayerId::new(1));
                assert_eq!((min, max), (size.min(3) as u8, size.min(3) as u8));
                assert!(matches!(
                    resume(&mut state, &mut res, &cards[..size.min(3)]),
                    Flow::Complete
                ));
            }
            other => panic!("search question: {other:?}"),
        }
        assert_eq!(
            state.zones.list(ZoneLocation::Exile(you)),
            &cards[..size.min(3)]
        );
        assert!(
            state
                .zones
                .list(ZoneLocation::Exile(PlayerId::new(1)))
                .is_empty()
        );
        assert!(
            state
                .zones
                .list(ZoneLocation::Hand(PlayerId::new(1)))
                .is_empty()
        );
    }
}

fn name_for(state: &mut GameState, res: &mut Resolution, name: &str) {
    let name = state.names.intern(name);
    res.subject.named = Some((
        crate::object::PrintedFace::new(baylee_core::ids::CardIndex::new(0), 0).unwrap(),
        name,
    ));
}

#[test]
fn exile_then_reveal_until_name_ignores_the_initial_exiled_match() {
    let (mut state, source, cards) = table(10);
    let you = PlayerId::new(0);
    let name = state.names.intern("wanted");
    for card in [cards[8], cards[1]] {
        state.object_mut(card).unwrap().base_mut().name = name;
    }
    let mut res = resolution(
        source,
        &[
            Effect::ExileTop {
                who: PlayerRel::You,
                count: Amount::Fixed(6),
            },
            Effect::RevealUntilNamed {
                player: PlayerRel::You,
                matched: SearchDest::Hand,
                otherwise: SearchDest::Exile,
            },
        ],
        you,
    );
    name_for(&mut state, &mut res, "wanted");
    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    assert_eq!(state.zones.list(ZoneLocation::Hand(you)), &[cards[1]]);
    assert_eq!(state.zones.list(ZoneLocation::Library(you)), &[cards[0]]);
    let revealed: Vec<_> = state
        .journal
        .entries()
        .iter()
        .filter_map(|entry| match &entry.event {
            GameEvent::Revealed { cards, .. } => Some(cards.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    assert_eq!(revealed, vec![cards[3], cards[2], cards[1]]);
    assert_eq!(state.zones.list(ZoneLocation::Exile(you)).len(), 8);
    assert!(
        !state
            .journal
            .entries()
            .iter()
            .any(|entry| matches!(entry.event, GameEvent::CardsDrawn { .. }))
    );
}

#[test]
fn name_mismatch_branch_runs_only_for_an_existing_nonmatching_card() {
    for (count, matches) in [(0, false), (1, false), (1, true)] {
        let (mut state, source, cards) = table(count);
        let you = PlayerId::new(0);
        let life = state.players[0].life;
        let mut res = resolution(
            source,
            &[Effect::RevealTopByNameThen {
                player: PlayerRel::You,
                matched: SearchDest::Hand,
                otherwise: SearchDest::Graveyard,
                otherwise_effects: &[Effect::Sequence(&[Effect::GainLife {
                    amount: Amount::Fixed(2),
                }])],
            }],
            you,
        );
        name_for(
            &mut state,
            &mut res,
            if matches { "library card" } else { "absent" },
        );
        assert!(matches!(run(&mut state, &mut res), Flow::Complete));
        assert_eq!(
            state.players[0].life,
            life + if count > 0 && !matches { 2 } else { 0 }
        );
        if count > 0 {
            assert!(
                state
                    .zones
                    .list(if matches {
                        ZoneLocation::Hand(you)
                    } else {
                        ZoneLocation::Graveyard(you)
                    })
                    .contains(&cards[0])
            );
        }
    }
}

fn reveal_events(state: &GameState) -> Vec<Vec<ObjectId>> {
    state
        .journal
        .entries()
        .iter()
        .filter_map(|entry| match &entry.event {
            GameEvent::Revealed { cards, .. } => Some(cards.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn public_top_find_reveals_all_and_moves_the_unselected_cards() {
    for pick in [None, Some(4)] {
        let (mut state, source, cards) = table(6);
        let you = PlayerId::new(0);
        let mut res = resolution(
            source,
            &[Effect::RevealTopFind {
                found: baylee_cards_dsl::Find::HAND,
                count: 4,
                filter: &baylee_cards_dsl::Filter::Any,
                otherwise: SearchDest::Graveyard,
            }],
            you,
        );
        let top = vec![cards[5], cards[4], cards[3], cards[2]];
        let Flow::Wait(Pending::ChooseCards {
            player,
            options,
            min,
            max,
            prompt,
            ..
        }) = run(&mut state, &mut res)
        else {
            panic!("an optional choice among all four cards");
        };
        assert_eq!(player, you);
        assert_eq!(options, top);
        assert_eq!((min, max), (0, 1));
        assert_eq!(prompt, ChoicePrompt::PutIntoHand);
        let revealed = reveal_events(&state);
        assert_eq!(revealed, vec![top.clone()]);

        let chosen = pick.map_or_else(Vec::new, |at| vec![cards[at]]);
        assert!(matches!(
            resume(&mut state, &mut res, &chosen),
            Flow::Complete
        ));
        assert_eq!(state.zones.list(ZoneLocation::Hand(you)), &chosen);
        let rest: Vec<_> = top
            .into_iter()
            .filter(|card| !chosen.contains(card))
            .collect();
        assert_eq!(state.zones.list(ZoneLocation::Graveyard(you)), &rest);
        assert_eq!(state.zones.list(ZoneLocation::Library(you)), &cards[..2]);
        assert_eq!(reveal_events(&state), revealed, "no second reveal");
    }
}

#[test]
fn public_top_find_without_a_match_finishes_without_an_empty_question() {
    let (mut state, source, cards) = table(6);
    let you = PlayerId::new(0);
    let mut res = resolution(
        source,
        &[Effect::RevealTopFind {
            found: baylee_cards_dsl::Find::HAND,
            count: 4,
            filter: &baylee_cards_dsl::Filter::CREATURE,
            otherwise: SearchDest::Graveyard,
        }],
        you,
    );
    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    assert_eq!(
        reveal_events(&state),
        vec![vec![cards[5], cards[4], cards[3], cards[2]]]
    );
    assert_eq!(
        state.zones.list(ZoneLocation::Graveyard(you)),
        &cards[2..].iter().rev().copied().collect::<Vec<_>>()
    );
    assert_eq!(state.zones.list(ZoneLocation::Library(you)), &cards[..2]);
    assert!(state.zones.list(ZoneLocation::Hand(you)).is_empty());
}

#[test]
fn private_top_find_still_reveals_only_the_selected_card() {
    let (mut state, source, cards) = table(4);
    let you = PlayerId::new(0);
    let mut res = resolution(
        source,
        &[Effect::LookAtTopFind {
            found: baylee_cards_dsl::Find::HAND,
            count: 3,
            filter: &baylee_cards_dsl::Filter::Any,
            random: false,
        }],
        you,
    );
    assert!(matches!(
        run(&mut state, &mut res),
        Flow::Wait(Pending::ChooseCards { .. })
    ));
    assert!(reveal_events(&state).is_empty());
    let Flow::Wait(Pending::Arrange { cards: rest, .. }) =
        resume(&mut state, &mut res, &[cards[3]])
    else {
        panic!("the two private misses still need an order");
    };
    assert!(matches!(
        resume_arranged(&mut state, &mut res, &[rest]),
        Flow::Complete
    ));
    assert_eq!(reveal_events(&state), vec![vec![cards[3]]]);
    assert_eq!(state.zones.list(ZoneLocation::Hand(you)), &[cards[3]]);
}

#[test]
fn private_top_pick_keeps_suffix_and_orders_remaining_on_top() {
    let (mut state, source, cards) = table(8);
    let you = PlayerId::new(0);
    let mut res = resolution(
        source,
        &[Effect::LookAtTopPickThenReorder {
            count: Amount::Fixed(5),
            pick: 1,
        }],
        you,
    );
    let Flow::Wait(Pending::ChooseCards {
        options, min, max, ..
    }) = run(&mut state, &mut res)
    else {
        panic!("private top choice");
    };
    assert_eq!((min, max), (1, 1));
    assert_eq!(
        options,
        cards[3..].iter().rev().copied().collect::<Vec<_>>()
    );
    let keep = cards[5];
    let Flow::Wait(Pending::Arrange {
        cards: remaining,
        piles,
        ..
    }) = resume(&mut state, &mut res, &[keep])
    else {
        panic!("top order");
    };
    assert_eq!(piles[0].place, ArrangePlace::LibraryTop);
    assert_eq!(remaining.len(), 4);
    let top = vec![cards[3], cards[7], cards[4], cards[6]];
    assert!(matches!(
        resume_arranged(&mut state, &mut res, std::slice::from_ref(&top)),
        Flow::Complete
    ));
    let mut expected = cards[..3].to_vec();
    expected.extend(top.into_iter().rev());
    assert_eq!(state.zones.list(ZoneLocation::Library(you)), &expected);
    assert_eq!(state.zones.list(ZoneLocation::Hand(you)), &[keep]);
    no_reveal(&state);
}

#[test]
fn private_random_exile_is_bounded_and_leaves_deeper_cards_untouched() {
    for count in 0..=10 {
        let (mut state, source, cards) = table(count);
        let you = PlayerId::new(0);
        let mut res = resolution(
            source,
            &[Effect::LookAtTopExileRandom { count: 8, exile: 4 }],
            you,
        );
        let mut replay = state.clone();
        let mut replay_res = res.clone();
        let _ = run(&mut replay, &mut replay_res);
        match run(&mut state, &mut res) {
            Flow::Wait(Pending::Arrange { cards, .. }) => {
                assert!(matches!(
                    resume_arranged(&mut state, &mut res, &[cards]),
                    Flow::Complete
                ));
            }
            Flow::Wait(Pending::ChooseCards {
                options,
                min: 0,
                max: 0,
                ..
            }) => {
                assert_eq!(options.len(), 1);
                assert!(matches!(resume(&mut state, &mut res, &[]), Flow::Complete));
            }
            Flow::Complete => {}
            other => panic!("unexpected random-exile continuation: {other:?}"),
        }
        let exiled = state.zones.list(ZoneLocation::Exile(you));
        assert_eq!(
            exiled,
            replay.zones.list(ZoneLocation::Exile(you)),
            "same seeded sample"
        );
        assert_eq!(exiled.len(), count.min(4));
        assert_eq!(
            exiled
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            exiled.len()
        );
        let untouched = count.saturating_sub(8);
        assert_eq!(
            &state.zones.list(ZoneLocation::Library(you))[..untouched],
            &cards[..untouched]
        );
        no_reveal(&state);
    }
}

#[test]
fn target_hand_owner_chooses_and_orders_on_their_own_library() {
    let (mut state, source, _) = table(0);
    let them = PlayerId::new(1);
    let cards: Vec<_> = (0..5)
        .map(|_| {
            let name = state.names.intern("opponent hand");
            state.create_bare(them, ObjectKind::Card, name, ZoneLocation::Hand(them))
        })
        .collect();
    let before = state.zones.list(ZoneLocation::Library(them)).clone();
    let mut res = resolution(
        source,
        &[Effect::PutFromHandOnTopFor {
            player: PlayerRel::Chosen,
            count: 3,
        }],
        them,
    );
    let Flow::Wait(Pending::ChooseCards {
        player, min, max, ..
    }) = run(&mut state, &mut res)
    else {
        panic!("hand choice");
    };
    assert_eq!(player, them);
    assert_eq!((min, max), (3, 3));
    let Flow::Wait(Pending::Arrange { player, .. }) = resume(&mut state, &mut res, &cards[..3])
    else {
        panic!("hand order");
    };
    assert_eq!(player, them);
    let top = vec![cards[1], cards[2], cards[0]];
    assert!(matches!(
        resume_arranged(&mut state, &mut res, std::slice::from_ref(&top)),
        Flow::Complete
    ));
    let mut expected = before;
    expected.extend(top.into_iter().rev());
    assert_eq!(state.zones.list(ZoneLocation::Library(them)), &expected);
    assert_eq!(state.zones.list(ZoneLocation::Hand(them)), &cards[3..]);
    no_reveal(&state);
}

fn resolution(spell: ObjectId, effects: &[Effect], them: PlayerId) -> Resolution {
    Resolution {
        source: spell,
        on_stack: spell,
        controller: PlayerId::new(0),
        effects: effects.to_vec(),
        pc: 0,
        targets: SmallVec::new(),
        second_targets: SmallVec::new(),
        x: None,
        chosen_player: Some(them),
        target_players: SeatSet::new(),
        event_object: None,
        awaiting: None,
        targeted: true,
        mana_ability: false,
        countered_source: None,
        target_lki: None,
        subject: crate::resolve::SubjectContext::default(),
        text: crate::text_changes::TextChangeMap::IDENTITY,
        event_mana: None,
        retarget_left: None,
        loot: None,
        draw_watch: None,
        this_way_from: None,
        coin_stake: None,
    }
}

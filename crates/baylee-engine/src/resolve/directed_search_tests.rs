use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use baylee_cards_dsl::{Filter, Find};
use baylee_core::ids::SeatSet;

fn table(count: usize) -> (GameState, Resolution, Vec<ObjectId>) {
    let mut state =
        GameState::from_preset(&preset(185, &[]), &SyntheticLookup::new(vec![])).unwrap();
    let you = PlayerId::new(0);
    let them = PlayerId::new(1);
    let name = state.names.intern("source");
    let source = state.create_bare(you, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
    let library = state.zones.list(ZoneLocation::Library(them)).clone();
    for card in library {
        state.zones.remove(card, ZoneLocation::Library(them));
    }
    let cards = (0..count)
        .map(|_| {
            let name = state.names.intern("library card");
            state.create_bare(them, ObjectKind::Card, name, ZoneLocation::Library(them))
        })
        .collect();
    let res = resolution(
        source,
        &[Effect::SearchLibraryFor {
            library: PlayerRel::Chosen,
            receiver: PlayerRel::Chosen,
            count: Amount::Fixed(3),
            filter: &Filter::Any,
            find: &Find::HAND,
            optional: false,
        }],
        them,
    );
    (state, res, cards)
}

#[test]
fn controller_searches_other_library_and_owner_receives_exact_required_cards() {
    for size in [0, 1, 5] {
        let (mut state, mut res, cards) = table(size);
        match run(&mut state, &mut res) {
            Flow::Complete => assert_eq!(size, 0),
            Flow::Wait(Pending::ChooseCards {
                player,
                options,
                min,
                max,
                ..
            }) => {
                assert_eq!(player, PlayerId::new(0));
                assert_eq!((min, max), (size.min(3) as u8, size.min(3) as u8));
                assert_eq!(options.len(), size);
                assert!(matches!(
                    resume(&mut state, &mut res, &cards[..size.min(3)]),
                    Flow::Complete
                ));
            }
            other => panic!("unexpected search {other:?}"),
        }
        assert_eq!(
            state.zones.list(ZoneLocation::Hand(PlayerId::new(1))),
            &cards[..size.min(3)]
        );
        assert!(
            state
                .zones
                .list(ZoneLocation::Hand(PlayerId::new(0)))
                .is_empty()
        );
        assert!(
            !state
                .journal
                .entries()
                .iter()
                .any(|entry| matches!(entry.event, GameEvent::Revealed { .. }))
        );
    }
}

#[test]
fn dynamic_search_collects_above_255_without_moving_partial_selection() {
    let (mut state, mut res, cards) = table(260);
    let Effect::SearchLibraryFor { count, .. } = &mut res.effects[0] else {
        unreachable!();
    };
    *count = Amount::Fixed(260);
    assert!(matches!(
        run(&mut state, &mut res),
        Flow::Wait(Pending::ChooseCards {
            min: 255,
            max: 255,
            ..
        })
    ));
    let Flow::Wait(Pending::ChooseCards {
        options, min, max, ..
    }) = resume(&mut state, &mut res, &cards[..255])
    else {
        panic!("next batch");
    };
    assert_eq!((min, max), (5, 5));
    assert_eq!(options.len(), 5);
    assert_eq!(
        state
            .zones
            .list(ZoneLocation::Library(PlayerId::new(1)))
            .len(),
        260
    );
    assert!(matches!(
        resume(&mut state, &mut res, &options),
        Flow::Complete
    ));
    assert_eq!(
        state.zones.list(ZoneLocation::Hand(PlayerId::new(1))).len(),
        260
    );
    assert!(
        state
            .zones
            .list(ZoneLocation::Library(PlayerId::new(1)))
            .is_empty()
    );
}

#[test]
fn whole_hand_transfer_counts_actual_moves_for_another_players_subsequent_search() {
    let (mut state, mut res, library) = table(4);
    let them = PlayerId::new(1);
    let hand: Vec<_> = (0..2)
        .map(|_| {
            let name = state.names.intern("old hand");
            state.create_bare(them, ObjectKind::Card, name, ZoneLocation::Hand(them))
        })
        .collect();
    let name = state.names.intern("resolving ability");
    let ability = state.create_bare(
        PlayerId::new(0),
        ObjectKind::AbilityOnStack,
        name,
        ZoneLocation::Stack,
    );
    res.on_stack = ability;
    res.effects = vec![
        Effect::PutHandOnTop {
            player: PlayerRel::Chosen,
        },
        Effect::SearchLibraryFor {
            library: PlayerRel::Chosen,
            receiver: PlayerRel::Chosen,
            count: Amount::ThisWay(baylee_cards_dsl::ThisWay::PutIntoLibraryFromHand),
            filter: &Filter::Any,
            find: &Find::HAND,
            optional: false,
        },
    ];
    state
        .journal
        .record(GameEvent::StackObjectResolved { object: ability });
    let Flow::Wait(Pending::Arrange { player, cards, .. }) = run(&mut state, &mut res) else {
        panic!("hand order");
    };
    assert_eq!(player, them);
    assert_eq!(cards, hand);
    let Flow::Wait(Pending::ChooseCards {
        player,
        min,
        max,
        options,
        ..
    }) = resume_arranged(&mut state, &mut res, std::slice::from_ref(&hand))
    else {
        panic!("replacement hand search");
    };
    assert_eq!(player, PlayerId::new(0));
    assert_eq!((min, max), (2, 2));
    assert_eq!(options.len(), 6);
    assert!(state.zones.list(ZoneLocation::Hand(them)).is_empty());
    assert!(matches!(
        resume(&mut state, &mut res, &library[..2]),
        Flow::Complete
    ));
    assert_eq!(state.zones.list(ZoneLocation::Hand(them)), &library[..2]);
    assert_eq!(state.zones.list(ZoneLocation::Library(them)).len(), 4);
    assert!(
        state.journal.entries().iter().any(
            |entry| matches!(entry.event, GameEvent::Shuffled { player, .. } if player == them)
        )
    );
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

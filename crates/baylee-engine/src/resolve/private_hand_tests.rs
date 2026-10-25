use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use baylee_cards_dsl::Filter;
use baylee_core::ids::SeatSet;

fn table() -> (GameState, ObjectId, Vec<ObjectId>) {
    let mut state =
        GameState::from_preset(&preset(183, &[]), &SyntheticLookup::new(vec![])).unwrap();
    let you = PlayerId::new(0);
    let them = PlayerId::new(1);
    let name = state.names.intern("source");
    let source = state.create_bare(you, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
    let cards = (0..5)
        .map(|_| {
            let name = state.names.intern("hand card");
            state.create_bare(them, ObjectKind::Card, name, ZoneLocation::Hand(them))
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
}

#[test]
fn private_discard_above_answer_limit_collects_every_choice_before_discarding() {
    let (mut state, source, _) = table();
    let them = PlayerId::new(1);
    for _ in 0..255 {
        let name = state.names.intern("additional card");
        state.create_bare(them, ObjectKind::Card, name, ZoneLocation::Hand(them));
    }
    let hand = state.zones.list(ZoneLocation::Hand(them)).clone();
    let mut res = resolution(
        source,
        &[Effect::LookHandDiscard {
            player: PlayerRel::Chosen,
            count: Amount::Fixed(999),
            filter: &Filter::Any,
        }],
        them,
    );
    let Flow::Wait(Pending::ChooseCards { min, max, .. }) = run(&mut state, &mut res) else {
        panic!("first batch");
    };
    assert_eq!((min, max), (255, 255));
    let Flow::Wait(Pending::ChooseCards {
        options, min, max, ..
    }) = resume(&mut state, &mut res, &hand[..255])
    else {
        panic!("remaining batch");
    };
    assert_eq!((min, max), (5, 5));
    assert_eq!(options, hand[255..]);
    assert_eq!(
        state.zones.list(ZoneLocation::Hand(them)),
        &hand,
        "no partial discard before all choices"
    );
    assert!(matches!(
        resume(&mut state, &mut res, &options),
        Flow::Complete
    ));
    assert!(state.zones.list(ZoneLocation::Hand(them)).is_empty());
    assert_eq!(state.zones.list(ZoneLocation::Graveyard(them)).len(), 260);
    no_reveal(&state);
}

#[test]
fn random_hand_look_is_private_distinct_deterministic_and_does_not_change_hand() {
    let (mut state, source, hand) = table();
    let before = state
        .zones
        .list(ZoneLocation::Hand(PlayerId::new(1)))
        .clone();
    let mut replay = state.clone();
    let effect = Effect::LookAtRandomFromHand {
        who: PlayerRel::Chosen,
        count: Amount::Fixed(3),
    };
    let mut res = resolution(source, &[effect], PlayerId::new(1));
    let mut replay_res = res.clone();
    let Flow::Wait(Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    }) = run(&mut state, &mut res)
    else {
        panic!("private look");
    };
    assert_eq!(player, PlayerId::new(0));
    assert_eq!((min, max), (0, 0));
    assert_eq!(options.len(), 3);
    assert!(options.iter().all(|id| hand.contains(id)));
    assert_eq!(
        options
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3
    );
    let Flow::Wait(Pending::ChooseCards {
        options: replay_options,
        ..
    }) = run(&mut replay, &mut replay_res)
    else {
        panic!("replayed private look");
    };
    assert_eq!(options, replay_options);
    assert!(matches!(resume(&mut state, &mut res, &[]), Flow::Complete));
    assert_eq!(
        state.zones.list(ZoneLocation::Hand(PlayerId::new(1))),
        &before
    );
    no_reveal(&state);
}

#[test]
fn private_discard_uses_controllers_choice_and_owners_graveyard() {
    let (mut state, source, hand) = table();
    let them = PlayerId::new(1);
    let mut res = resolution(
        source,
        &[Effect::LookHandDiscard {
            player: PlayerRel::Chosen,
            count: Amount::Fixed(2),
            filter: &Filter::Any,
        }],
        them,
    );
    let Flow::Wait(Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    }) = run(&mut state, &mut res)
    else {
        panic!("discard choice");
    };
    assert_eq!(player, PlayerId::new(0));
    assert_eq!((min, max), (2, 2));
    assert_eq!(options, hand);
    assert!(matches!(
        resume(&mut state, &mut res, &hand[..2]),
        Flow::Complete
    ));
    assert_eq!(state.zones.list(ZoneLocation::Hand(them)), &hand[2..]);
    assert_eq!(state.zones.list(ZoneLocation::Graveyard(them)), &hand[..2]);
    assert!(
        state
            .zones
            .list(ZoneLocation::Graveyard(PlayerId::new(0)))
            .is_empty()
    );
    no_reveal(&state);
}

#[test]
fn zero_and_filtered_out_discard_still_privately_inspects_whole_hand() {
    for (count, filter) in [(0, &Filter::Any), (5, &Filter::CREATURE)] {
        let (mut state, source, hand) = table();
        let mut res = resolution(
            source,
            &[Effect::LookHandDiscard {
                player: PlayerRel::Chosen,
                count: Amount::Fixed(count),
                filter,
            }],
            PlayerId::new(1),
        );
        let Flow::Wait(Pending::ChooseCards { min, max, .. }) = run(&mut state, &mut res) else {
            panic!("look even without discard");
        };
        assert_eq!((min, max), (0, 0));
        let Some(AwaitingOp::PrivateHandDiscard { inspected, .. }) = &res.awaiting else {
            panic!("inspection continuation");
        };
        assert_eq!(inspected, &hand);
        assert!(matches!(resume(&mut state, &mut res, &[]), Flow::Complete));
        no_reveal(&state);
    }
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

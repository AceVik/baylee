use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use crate::zone::Zone;
use baylee_cards_dsl::Filter;
use baylee_core::ids::SeatSet;
use baylee_core::types::TypeSet;

fn table() -> GameState {
    GameState::from_preset(&preset(913, &[]), &SyntheticLookup::new(vec![])).unwrap()
}

fn permanent(state: &mut GameState, player: PlayerId, types: TypeSet) -> ObjectId {
    let name = state.names.intern("test permanent");
    let card = state.create_bare(
        player,
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    state.object_mut(card).unwrap().base_mut().types = types;
    card
}

fn resolution(filters: &'static [&'static Filter]) -> Resolution {
    Resolution {
        source: ObjectId::NO_SOURCE,
        on_stack: ObjectId::NO_SOURCE,
        controller: PlayerId::new(0),
        effects: vec![Effect::KeepOneOfEachAndSacrificeRest { filters }],
        pc: 0,
        targets: SmallVec::new(),
        second_targets: SmallVec::new(),
        x: None,
        chosen_player: None,
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

#[test]
fn empty_filter_list_does_not_turn_into_a_board_wipe() {
    let mut state = table();
    let card = permanent(&mut state, PlayerId::new(0), TypeSet::PLANESWALKER);
    let mut res = resolution(&[]);

    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    assert_eq!(state.object(card).unwrap().zone, Zone::Battlefield);
    assert!(res.awaiting.is_none());
}

#[test]
fn one_multi_type_permanent_can_cover_two_categories() {
    let p0 = PlayerId::new(0);
    let mut state = table();
    let both = permanent(&mut state, p0, TypeSet::ARTIFACT.union(TypeSet::CREATURE));
    let artifact = permanent(&mut state, p0, TypeSet::ARTIFACT);
    let creature = permanent(&mut state, p0, TypeSet::CREATURE);
    let walker = permanent(&mut state, p0, TypeSet::PLANESWALKER);
    let mut res = resolution(&[&Filter::ARTIFACT, &Filter::CREATURE]);

    let Flow::Wait(Pending::ChooseCards { options, .. }) = run(&mut state, &mut res) else {
        panic!("an artifact choice was required")
    };
    assert_eq!(options, vec![both, artifact]);
    let Flow::Wait(Pending::ChooseCards { options, .. }) = resume(&mut state, &mut res, &[both])
    else {
        panic!("the same permanent remains a legal creature choice")
    };
    assert_eq!(options, vec![both, creature]);
    assert!(matches!(
        resume(&mut state, &mut res, &[both]),
        Flow::Complete
    ));
    assert_eq!(state.object(both).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.object(artifact).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.object(creature).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.object(walker).unwrap().zone, Zone::Graveyard);
    let kept: Vec<_> = state
        .journal
        .entries()
        .iter()
        .filter_map(|entry| match &entry.event {
            GameEvent::CardsKept { player, cards } if *player == p0 => Some(cards.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        kept,
        vec![vec![both]],
        "the shared choice is published once"
    );
}

#[test]
fn choices_follow_apnap_and_no_victim_moves_before_the_last_answer() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut state = table();
    let p0_cards = [
        permanent(&mut state, p0, TypeSet::ARTIFACT),
        permanent(&mut state, p0, TypeSet::ARTIFACT),
    ];
    let p1_cards = [
        permanent(&mut state, p1, TypeSet::ARTIFACT),
        permanent(&mut state, p1, TypeSet::ARTIFACT),
    ];
    state.turn.active = p1;
    let mut res = resolution(&[&Filter::ARTIFACT]);

    let Flow::Wait(Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    }) = run(&mut state, &mut res)
    else {
        panic!("the active player chooses first")
    };
    assert_eq!(player, p1);
    assert_eq!(prompt, ChoicePrompt::KeepPermanents);
    assert_eq!(options, p1_cards);
    let Flow::Wait(Pending::ChooseCards { player, .. }) =
        resume(&mut state, &mut res, &[p1_cards[0]])
    else {
        panic!("the next player chooses")
    };
    assert_eq!(player, p0);
    for card in p0_cards.into_iter().chain(p1_cards) {
        assert_eq!(state.object(card).unwrap().zone, Zone::Battlefield);
    }

    assert!(matches!(
        resume(&mut state, &mut res, &[p0_cards[1]]),
        Flow::Complete
    ));
    for kept in [p0_cards[1], p1_cards[0]] {
        assert_eq!(state.object(kept).unwrap().zone, Zone::Battlefield);
    }
    for sacrificed in [p0_cards[0], p1_cards[1]] {
        assert_eq!(state.object(sacrificed).unwrap().zone, Zone::Graveyard);
    }
    let announcements: Vec<_> = state
        .journal
        .entries()
        .iter()
        .filter_map(|entry| match entry.event {
            GameEvent::CardsKept { player, .. } => Some(player),
            _ => None,
        })
        .collect();
    assert_eq!(announcements, vec![p1, p0]);
}

#[test]
fn different_keep_choices_change_the_continuation_fingerprint_before_any_move() {
    let p0 = PlayerId::new(0);
    let mut left_state = table();
    let artifacts = [
        permanent(&mut left_state, p0, TypeSet::ARTIFACT),
        permanent(&mut left_state, p0, TypeSet::ARTIFACT),
    ];
    for _ in 0..2 {
        permanent(&mut left_state, p0, TypeSet::CREATURE);
    }
    let mut left = resolution(&[&Filter::ARTIFACT, &Filter::CREATURE]);
    let mut right_state = left_state.clone();
    let mut right = left.clone();
    assert!(matches!(run(&mut left_state, &mut left), Flow::Wait(_)));
    assert!(matches!(run(&mut right_state, &mut right), Flow::Wait(_)));

    let Flow::Wait(Pending::ChooseCards {
        options: left_options,
        ..
    }) = resume(&mut left_state, &mut left, &[artifacts[0]])
    else {
        panic!("the left creature choice")
    };
    let Flow::Wait(Pending::ChooseCards {
        options: right_options,
        ..
    }) = resume(&mut right_state, &mut right, &[artifacts[1]])
    else {
        panic!("the right creature choice")
    };
    assert_eq!(left_options, right_options);
    assert_eq!(left_state.snapshot_hash(), right_state.snapshot_hash());
    assert_ne!(
        left.awaiting.as_ref().unwrap().search_fingerprint(),
        right.awaiting.as_ref().unwrap().search_fingerprint()
    );
    for card in artifacts {
        assert_eq!(left_state.object(card).unwrap().zone, Zone::Battlefield);
        assert_eq!(right_state.object(card).unwrap().zone, Zone::Battlefield);
    }
}

#[test]
fn an_empty_resume_skips_the_category_without_fabricating_a_keep_choice() {
    let p0 = PlayerId::new(0);
    let mut state = table();
    let cards = [
        permanent(&mut state, p0, TypeSet::ARTIFACT),
        permanent(&mut state, p0, TypeSet::ARTIFACT),
    ];
    let mut res = resolution(&[&Filter::ARTIFACT]);
    assert!(matches!(run(&mut state, &mut res), Flow::Wait(_)));

    assert!(matches!(resume(&mut state, &mut res, &[]), Flow::Complete));
    for card in cards {
        assert_eq!(state.object(card).unwrap().zone, Zone::Graveyard);
    }
    assert!(!state.journal.entries().iter().any(|entry| {
        matches!(entry.event, GameEvent::CardsKept { player, .. } if player == p0)
    }));
}

use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use baylee_core::ids::SeatSet;

fn table(count: usize) -> (GameState, Resolution, Vec<ObjectId>) {
    let mut state =
        GameState::from_preset(&preset(184, &[]), &SyntheticLookup::new(vec![])).unwrap();
    let you = PlayerId::new(0);
    let name = state.names.intern("source");
    let source = state.create_bare(you, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
    let cards = (0..count)
        .map(|_| {
            let name = state.names.intern("graveyard card");
            let id = state.create_bare(you, ObjectKind::Card, name, ZoneLocation::Graveyard(you));
            state.object_mut(id).unwrap().card = Some(crate::object::CardRef {
                index: baylee_core::ids::CardIndex::new(1_001_001),
                print: baylee_core::ids::PrintRef::UNKNOWN,
            });
            id
        })
        .collect();
    let res = resolution(
        source,
        &[Effect::RechooseGraveyardCard {
            owner: PlayerRel::You,
            chooser: PlayerRel::Chosen,
            cost: baylee_core::mana!("{G}"),
        }],
        PlayerId::new(1),
    );
    (state, res, cards)
}

#[test]
fn another_player_rechooses_after_actual_payment_and_only_last_card_returns() {
    let (mut state, mut res, cards) = table(3);
    state.players[0].mana_pool.add(ManaColor::Green, 1);
    let Flow::Wait(Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    }) = run(&mut state, &mut res)
    else {
        panic!("graveyard choice");
    };
    assert_eq!(player, PlayerId::new(1));
    assert_eq!((min, max), (1, 1));
    assert_eq!(options, cards);
    assert!(
        matches!(resume(&mut state, &mut res, &cards[..1]), Flow::Wait(Pending::YesNo { player, .. }) if player == PlayerId::new(0))
    );
    let Flow::Wait(Pending::ChooseCards { options, .. }) =
        resume_tax_choice(&mut state, &mut res, true)
    else {
        panic!("second choice");
    };
    assert_eq!(state.players[0].mana_pool.available(ManaColor::Green), 0);
    assert_eq!(options, cards[1..]);
    assert_eq!(
        state.zones.list(ZoneLocation::Graveyard(PlayerId::new(0))),
        &cards
    );
    assert!(matches!(
        resume(&mut state, &mut res, &cards[1..2]),
        Flow::Wait(Pending::YesNo { .. })
    ));
    assert!(matches!(
        resume_tax_choice(&mut state, &mut res, false),
        Flow::Complete
    ));
    assert_eq!(
        state.zones.list(ZoneLocation::Hand(PlayerId::new(0))),
        &[cards[1]]
    );
    assert_eq!(
        state.zones.list(ZoneLocation::Graveyard(PlayerId::new(0))),
        &[cards[0], cards[2]]
    );
}

#[test]
fn exhausted_choices_still_allow_payment_and_keep_the_last_selected_card() {
    let (mut state, mut res, cards) = table(1);
    state.players[0].mana_pool.add(ManaColor::Green, 2);
    assert!(matches!(
        run(&mut state, &mut res),
        Flow::Wait(Pending::ChooseCards { .. })
    ));
    assert!(matches!(
        resume(&mut state, &mut res, &cards),
        Flow::Wait(Pending::YesNo { .. })
    ));
    for _ in 0..2 {
        assert!(matches!(
            resume_tax_choice(&mut state, &mut res, true),
            Flow::Wait(Pending::YesNo { .. })
        ));
    }
    assert_eq!(state.players[0].mana_pool.available(ManaColor::Green), 0);
    assert!(matches!(
        resume_tax_choice(&mut state, &mut res, false),
        Flow::Complete
    ));
    assert_eq!(
        state.zones.list(ZoneLocation::Hand(PlayerId::new(0))),
        &cards
    );
}

#[test]
fn empty_graveyard_has_no_choice_or_payment() {
    let (mut state, mut res, _) = table(0);
    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    let name = state.names.intern("non-card awaiting state-based actions");
    state.create_bare(
        PlayerId::new(0),
        ObjectKind::Card,
        name,
        ZoneLocation::Graveyard(PlayerId::new(0)),
    );
    res.pc = 0;
    assert!(
        matches!(run(&mut state, &mut res), Flow::Complete),
        "a token or copy in the graveyard is not a card"
    );
}

#[test]
fn final_return_rejects_a_card_that_left_and_reentered_the_graveyard() {
    let (mut state, mut res, cards) = table(1);
    let you = PlayerId::new(0);
    assert!(matches!(
        run(&mut state, &mut res),
        Flow::Wait(Pending::ChooseCards { .. })
    ));
    assert!(matches!(
        resume(&mut state, &mut res, &cards),
        Flow::Wait(Pending::YesNo { .. })
    ));
    state
        .move_object(
            cards[0],
            ZoneLocation::Exile(you),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            cards[0],
            ZoneLocation::Graveyard(you),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    assert!(matches!(
        resume_tax_choice(&mut state, &mut res, false),
        Flow::Complete
    ));
    assert!(state.zones.list(ZoneLocation::Hand(you)).is_empty());
    assert_eq!(state.zones.list(ZoneLocation::Graveyard(you)), &cards);
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

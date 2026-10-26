//! Dungeon paths and room programs exercised independently of the UI.
use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use baylee_core::ids::SeatSet;

fn resolution(spell: ObjectId, effects: &[Effect]) -> Resolution {
    Resolution {
        source: spell,
        on_stack: spell,
        controller: PlayerId::new(0),
        effects: effects.to_vec(),
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

fn table() -> GameState {
    GameState::from_preset(&preset(712, &[]), &SyntheticLookup::new(vec![])).unwrap()
}

#[test]
fn every_dungeon_path_reaches_its_final_room_without_skipping_an_arrow() {
    for dungeon in Dungeon::ALL {
        let mut reached = vec![false; dungeon.rooms().len()];
        let mut frontier = vec![0];
        while let Some(room) = frontier.pop() {
            reached[usize::from(room)] = true;
            assert!(!rooms::effects(dungeon, room).0.is_empty());
            for &next in dungeon.next(room) {
                assert!(next > room);
                frontier.push(next);
            }
            if dungeon.next(room).is_empty() {
                assert_eq!(usize::from(room), dungeon.rooms().len() - 1);
            }
        }
        assert!(reached.into_iter().all(|room| room));
    }
}

#[test]
fn venture_offers_three_dungeons_and_only_connected_rooms_then_restarts() {
    let mut state = table();
    let mut res = resolution(ObjectId::NO_SOURCE, &[Effect::VentureIntoTheDungeon]);
    let Some(Pending::ChooseCastMode { options, .. }) =
        venture_into_the_dungeon(&mut state, &mut res)
    else {
        panic!("three choices")
    };
    assert_eq!(options.len(), 3);
    assert!(matches!(resume(&mut state, &mut res, 2), Flow::Complete));
    let (dungeon, source) = state.players[0].dungeon.unwrap();
    let version = state.object(source).unwrap().version;
    assert_eq!(dungeon, Dungeon::Tomb);
    assert_eq!(state.players[0].dungeon_room, 1);
    assert_eq!(state.reflexive.len(), 1, "room waits until resolution ends");
    assert_eq!(state.players[0].life, 20, "room has not resolved inline");
    res.pc = 0;
    let Some(Pending::ChooseCastMode { options, .. }) =
        venture_into_the_dungeon(&mut state, &mut res)
    else {
        panic!("fork")
    };
    assert_eq!(
        options.iter().map(|o| o.kind).collect::<Vec<_>>(),
        vec![
            CastModeKind::DungeonRoom { dungeon, room: 1 },
            CastModeKind::DungeonRoom { dungeon, room: 3 }
        ]
    );
    assert!(matches!(resume(&mut state, &mut res, 1), Flow::Complete));
    res.pc = 0;
    assert!(venture_into_the_dungeon(&mut state, &mut res).is_none());
    assert_eq!(state.players[0].dungeon_room, 5);
    assert_eq!(state.players[0].dungeons_completed, 0);
    assert!(matches!(
        venture_into_the_dungeon(&mut state, &mut res),
        Some(Pending::ChooseCastMode { .. })
    ));
    assert_eq!(
        state.players[0].dungeons_completed, 1,
        "venturing from last room completes even with its trigger pending"
    );
    assert!(state.object(source).is_none());
    assert!(state.damage_source(source, Some(version)).is_some());
}

#[test]
fn tomb_declined_payments_cost_two_life_for_each_player() {
    for discard in [false, true] {
        let mut state = table();
        let mut res = resolution(ObjectId::NO_SOURCE, &[Effect::DungeonTax { discard }]);
        assert!(matches!(run(&mut state, &mut res), Flow::Complete));
        assert_eq!(state.players[0].life, 18);
        assert_eq!(state.players[1].life, 18);
    }
}

#[test]
fn lost_mine_dark_pool_drains_each_opponent_and_gains_once() {
    let mut state = table();
    let mut res = resolution(ObjectId::NO_SOURCE, rooms::effects(Dungeon::LostMine, 4).0);
    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    assert_eq!(state.players[0].life, 21);
    assert_eq!(state.players[1].life, 19);
}

#[test]
fn tomb_final_room_creates_printed_atropal() {
    let mut state = table();
    let mut res = resolution(ObjectId::NO_SOURCE, rooms::effects(Dungeon::Tomb, 4).0);
    let flow = run(&mut state, &mut res);
    assert!(matches!(flow, Flow::Complete | Flow::Enter));
    let token = state
        .battlefield_seen()
        .find_map(|id| state.object(id))
        .unwrap();
    assert_eq!(token.base.power, Some(4));
    assert_eq!(token.base.toughness, Some(4));
    assert!(token.base.keywords.contains(KeywordSet::DEATHTOUCH));
    assert!(
        token
            .base
            .supertypes
            .contains(baylee_core::types::SupertypeSet::LEGENDARY)
    );
}

#[test]
fn oubliette_selects_distinct_permanents_before_sacrificing_them_together() {
    let mut state = table();
    let player = PlayerId::new(0);
    let mut cards = Vec::new();
    for types in [
        baylee_core::types::TypeSet::CREATURE.union(baylee_core::types::TypeSet::ARTIFACT),
        baylee_core::types::TypeSet::ARTIFACT,
        baylee_core::types::TypeSet::LAND,
    ] {
        let name = state.names.intern("Oubliette payment");
        let id = state.create_bare(
            player,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        state.object_mut(id).unwrap().base_mut().types = types;
        cards.push(id);
    }
    let mut res = resolution(ObjectId::NO_SOURCE, &[Effect::DungeonSacrificeThree]);
    assert!(matches!(
        run(&mut state, &mut res),
        Flow::Wait(Pending::ChooseCards { min: 1, .. })
    ));
    let Flow::Wait(Pending::ChooseCards { options, .. }) =
        crate::resolve::resume(&mut state, &mut res, &[cards[0]])
    else {
        panic!("artifact choice")
    };
    assert_eq!(options, vec![cards[1]], "one permanent cannot pay twice");
    assert!(matches!(
        crate::resolve::resume(&mut state, &mut res, &[cards[1]]),
        Flow::Wait(_)
    ));
    assert!(
        cards
            .iter()
            .all(|id| state.object(*id).unwrap().zone == crate::zone::Zone::Battlefield)
    );
    assert!(matches!(
        crate::resolve::resume(&mut state, &mut res, &[cards[2]]),
        Flow::Complete
    ));
    assert!(
        cards
            .iter()
            .all(|id| state.object(*id).unwrap().zone == crate::zone::Zone::Graveyard)
    );
}

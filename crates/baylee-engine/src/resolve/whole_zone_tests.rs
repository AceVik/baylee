use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use baylee_core::ids::SeatSet;

static WHEEL: &[Effect] = &[Effect::DiscardHand {
    who: PlayerRel::EachPlayer,
}];
static TWISTER: &[Effect] = &[Effect::ShuffleIntoLibrary {
    who: PlayerRel::EachPlayer,
    hand: true,
    graveyard: true,
}];
static SELECTION: &[Effect] = &[Effect::ReorderTopLibraryOf {
    who: PlayerRel::Chosen,
    count: 3,
}];

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
    }
}

#[test]
fn suspended_program_hash_includes_remaining_instructions_and_captured_operands() {
    let (state, source) = table();
    let base = resolution(source, WHEEL, PlayerId::new(1));
    let original = base.program_fingerprint();
    assert_eq!(original, base.clone().program_fingerprint());
    let mut altered = base.clone();
    altered.effects = TWISTER.to_vec();
    assert_ne!(original, altered.program_fingerprint());
    altered = base.clone();
    altered.second_targets.push(source);
    assert_ne!(original, altered.program_fingerprint());
    altered = base.clone();
    altered.text.replace(crate::text_changes::TextReplacement {
        kind: baylee_cards_dsl::TextWordKind::Color,
        from: 0,
        to: 1,
    });
    assert_ne!(original, altered.program_fingerprint());
    altered = base;
    let object = state.object(source).unwrap();
    altered.target_lki = Some(vec![TargetLki {
        id: source,
        version: object.version,
        controller: object.controller,
        chars: object.characteristics().clone(),
    }]);
    let historical = altered.program_fingerprint();
    assert_ne!(original, historical);
    altered.target_lki.as_mut().unwrap()[0].version += 1;
    assert_ne!(historical, altered.program_fingerprint());
}

/// Two cards in each player's hand and one in each graveyard.
fn table() -> (GameState, ObjectId) {
    let mut state = GameState::from_preset(&preset(23, &[]), &SyntheticLookup::new(vec![]))
        .expect("a two-seat game");
    for seat in 0..2 {
        let p = PlayerId::new(seat);
        for zone in [
            ZoneLocation::Hand(p),
            ZoneLocation::Hand(p),
            ZoneLocation::Graveyard(p),
        ] {
            let name = state.names.intern("Card");
            state.create_bare(p, ObjectKind::Card, name, zone);
        }
    }
    let name = state.names.intern("Spell");
    let spell = state.create_bare(
        PlayerId::new(0),
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    (state, spell)
}

fn count(state: &GameState, zone: ZoneLocation) -> usize {
    state.zones.list(zone).len()
}

#[test]
fn each_player_discards_every_card_in_their_hand() {
    let (mut state, spell) = table();
    let mut res = resolution(spell, WHEEL, PlayerId::new(1));
    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    for seat in 0..2 {
        let p = PlayerId::new(seat);
        assert_eq!(
            count(&state, ZoneLocation::Hand(p)),
            0,
            "seat {seat}'s hand"
        );
        assert_eq!(
            count(&state, ZoneLocation::Graveyard(p)),
            3,
            "seat {seat}'s graveyard"
        );
    }
    let discards = state
        .journal
        .entries()
        .iter()
        .filter(|e| matches!(e.event, GameEvent::Discarded { .. }))
        .count();
    assert_eq!(discards, 4, "each card is a discard of its own");
}

#[test]
fn each_player_shuffles_hand_and_graveyard_into_their_own_library() {
    let (mut state, spell) = table();
    let before: Vec<usize> = (0..2)
        .map(|s| count(&state, ZoneLocation::Library(PlayerId::new(s))))
        .collect();
    let mut res = resolution(spell, TWISTER, PlayerId::new(1));
    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    for seat in 0..2u8 {
        let p = PlayerId::new(seat);
        assert_eq!(count(&state, ZoneLocation::Hand(p)), 0);
        assert_eq!(count(&state, ZoneLocation::Graveyard(p)), 0);
        assert_eq!(
            count(&state, ZoneLocation::Library(p)),
            before[usize::from(seat)] + 3,
            "seat {seat}'s three cards went into their own library"
        );
    }
}

#[test]
fn the_controller_orders_the_top_of_the_chosen_players_library() {
    let (mut state, spell) = table();
    let them = PlayerId::new(1);
    for _ in 0..4 {
        let name = state.names.intern("Library card");
        state.create_bare(them, ObjectKind::Card, name, ZoneLocation::Library(them));
    }
    let top3: Vec<ObjectId> = state
        .zones
        .list(ZoneLocation::Library(them))
        .iter()
        .rev()
        .take(3)
        .copied()
        .collect();
    let mut res = resolution(spell, SELECTION, them);
    let Flow::Wait(Pending::Arrange { player, cards, .. }) = run(&mut state, &mut res) else {
        panic!("the controller is asked to order the cards");
    };
    assert_eq!(player, PlayerId::new(0), "the controller orders");
    assert_eq!(
        cards, top3,
        "the chosen player's top three, not the controller's"
    );
    let reversed: Vec<ObjectId> = top3.iter().rev().copied().collect();
    assert!(matches!(
        resume_arranged(&mut state, &mut res, std::slice::from_ref(&reversed)),
        Flow::Complete
    ));
    let now: Vec<ObjectId> = state
        .zones
        .list(ZoneLocation::Library(them))
        .iter()
        .rev()
        .take(3)
        .copied()
        .collect();
    assert_eq!(
        now, reversed,
        "put back in the order given, in their library"
    );
}

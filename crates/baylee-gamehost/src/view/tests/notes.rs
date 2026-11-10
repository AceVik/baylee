//! What a question's note may name (`decision_note`, `docs/protocol.md`
//! §"What a question is about"): only a card the asked seat is shown.

use super::*;
use baylee_core::ids::SeatSet;

fn entry(card: CardIndex) -> DeckEntry {
    DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card,
        print: PrintRef::new(0),
    }
}

fn hermetic_study() -> CardIndex {
    baylee_cards::decks::by_name("Hermetic Study").expect("Hermetic Study is in the pool")
}

/// `may_name` reads the zone as well as the face: a card in an opponent's
/// hand or in a library is face up, so [`may_know_card`] alone would name
/// it. A player the seat controls lends it their hand (CR 724.4), and a
/// face-down permanent is named to its controller only (CR 708.5).
#[test]
fn a_source_is_named_only_where_the_seat_is_shown_it() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut preset = mixed_print_preset();
    preset.seats[0].starting_battlefield = vec![entry(island()), entry(island())];
    preset.seats[1].starting_hand = Some(vec![entry(hermetic_study())]);
    let mut state = GameState::from_preset(&preset, &Registry).expect("the table builds");
    let first = |state: &GameState, zone| state.zones.list(zone)[0];
    let their_hand = first(&state, ZoneLocation::Hand(them));
    let my_library = first(&state, ZoneLocation::Library(me));
    let their_library = first(&state, ZoneLocation::Library(them));
    let face_down = first(&state, ZoneLocation::Battlefield);
    state
        .object_mut(face_down)
        .expect("on the battlefield")
        .status
        .insert(baylee_engine::object::Status::FACE_DOWN);
    let face_up = state.zones.list(ZoneLocation::Battlefield)[1];

    let alone = SeatSet::new();
    let mut controlling_them = SeatSet::new();
    controlling_them.insert(them);
    let named = |id: ObjectId, seat: PlayerId, controlled: SeatSet| {
        may_name(state.object(id).expect("in the game"), seat, controlled)
    };
    assert!(
        may_know_card(state.object(their_hand).expect("in hand"), me),
        "face up, so the face alone would name it"
    );
    assert!(
        !named(their_hand, me, alone),
        "an opponent's hand is not shown"
    );
    assert!(named(their_hand, them, alone), "its owner's is");
    assert!(
        named(their_hand, me, controlling_them),
        "and its controller's"
    );
    assert!(
        !named(my_library, me, alone),
        "a library is shown to nobody"
    );
    assert!(!named(their_library, me, controlling_them));
    assert!(
        named(face_down, me, alone),
        "a face-down permanent to its controller"
    );
    assert!(!named(face_down, them, alone), "and to nobody else");
    assert!(named(face_up, them, alone));

    let study = hermetic_study();
    assert!(!card_shown(&state, study, me, alone));
    assert!(card_shown(&state, study, them, alone));
    assert!(card_shown(&state, study, me, controlling_them));
    assert!(
        card_shown(&state, island(), them, alone),
        "the face-up Island is on the battlefield"
    );
}

/// A cast is explained to its caster and to nobody else, and the note names
/// the spell: the caster holds it.
#[test]
fn a_cast_is_explained_to_its_caster_only() {
    let swords = baylee_cards::decks::by_name("Swords to Plowshares").expect("in the pool");
    let plains = baylee_cards::decks::by_name("Plains").expect("in the pool");
    let bears = baylee_cards::decks::by_name("Grizzly Bears").expect("in the pool");
    let mut preset = mixed_print_preset();
    preset.seats[0].starting_hand = Some(vec![entry(swords)]);
    preset.seats[0].starting_battlefield = vec![entry(plains)];
    preset.seats[1].starting_battlefield = vec![entry(bears)];
    let mut engine = Engine::new(&preset, Registry).expect("game starts");
    let view = settle(&mut engine, None);
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let land = view.battlefield_of(me).next().expect("the Plains").id;
    engine
        .apply(me, PlayerAction::ActivateManaAbility { source: land })
        .expect("the Plains taps");
    let spell = view
        .hand
        .iter()
        .find(|card| card.name == "Swords to Plowshares")
        .expect("Swords in hand")
        .id;
    engine
        .apply(me, PlayerAction::CastSpell { card: spell })
        .expect("Swords casts");
    assert!(matches!(engine.pending(), Pending::ChooseTargets { .. }));
    let note = decision_note(&engine, me).expect("the caster is told");
    assert_eq!(note.source, Some(spell));
    assert_eq!(note.printed.map(|(card, _)| card), Some(swords));
    assert!(decision_note(&engine, them).is_none());
}

/// The ping Hermetic Study grants the creature it enchants is printed on
/// the Aura: the note of the ping's target question goes to the activator,
/// names the Bears as the source and points into no card but the Aura, and
/// the opponent is told nothing.
#[test]
fn a_granted_ability_names_a_grantor_the_seat_is_shown() {
    let bears = baylee_cards::decks::by_name("Grizzly Bears").expect("in the pool");
    let mut preset = mixed_print_preset();
    preset.seats[0].starting_battlefield = vec![entry(bears), entry(island()), entry(island())];
    preset.seats[0].starting_hand = Some(vec![entry(hermetic_study())]);
    let mut engine = Engine::new(&preset, Registry).expect("game starts");
    let view = settle(&mut engine, None);
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let creature = view
        .battlefield_of(me)
        .find(|o| o.name == "Grizzly Bears")
        .expect("the Bears")
        .id;
    for land in view.battlefield_of(me).filter(|o| o.name == "Island") {
        engine
            .apply(me, PlayerAction::ActivateManaAbility { source: land.id })
            .expect("an Island taps");
    }
    let aura = view
        .hand
        .iter()
        .find(|card| card.name == "Hermetic Study")
        .expect("in hand")
        .id;
    engine
        .apply(me, PlayerAction::CastSpell { card: aura })
        .expect("the Aura casts");
    settle(&mut engine, Some(creature));
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("seat 0 holds priority");
    };
    let &(source, ability_index) = legal
        .abilities
        .iter()
        .find(|(source, _)| *source == creature)
        .expect("the enchanted Bears may ping");
    engine
        .apply(
            me,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the ping activates");
    assert!(matches!(engine.pending(), Pending::ChooseTargets { .. }));
    let note = decision_note(&engine, me).expect("the activator is told");
    assert_eq!(note.source, Some(creature));
    // A static ability's grant carries no printed provenance today, so the
    // note names no card for it; where one does, it is the Aura.
    assert!(
        note.printed
            .is_none_or(|(card, _)| card == hermetic_study()),
        "the note points into a card other than the grantor: {note:?}"
    );
    let context = targeting_context(&engine, me).expect("and explained");
    assert!(
        context
            .source
            .rules
            .is_none_or(|r| r.card == hermetic_study() || r.card == bears),
        "the context explains the ping with another card's text"
    );
    assert!(decision_note(&engine, them).is_none());
    assert!(targeting_context(&engine, them).is_none());
}

/// A note keeps the asking card only where the seat is shown every card and
/// token it points into ([`as_shown`]): the grantor of a granted ability,
/// the card a copy's modifications are printed on, a token's definition,
/// and the source itself.
#[test]
fn a_note_points_only_into_cards_the_seat_is_shown() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut preset = mixed_print_preset();
    preset.seats[0].starting_battlefield = vec![entry(island()), entry(island())];
    preset.seats[1].starting_hand = Some(vec![entry(hermetic_study())]);
    let mut state = GameState::from_preset(&preset, &Registry).expect("the table builds");
    let source = state.zones.list(ZoneLocation::Battlefield)[0];
    let face_down = state.zones.list(ZoneLocation::Battlefield)[1];
    state
        .object_mut(face_down)
        .expect("on the battlefield")
        .status
        .insert(baylee_engine::object::Status::FACE_DOWN);
    let alone = SeatSet::new();
    let note = |printed: CardIndex| baylee_engine::engine::DecisionNote {
        source: Some(source),
        printed: Some((printed, 0)),
        ability: Some(0),
        x: 2,
        ..baylee_engine::engine::DecisionNote::default()
    };
    let bare = |n: baylee_engine::engine::DecisionNote| (n.source, n.printed, n.ability, n.x);

    // Its own text, on a face-up permanent: named to both seats.
    for seat in [me, them] {
        assert_eq!(
            as_shown(note(island()), &state, seat, alone),
            note(island())
        );
    }
    // A grantor in their hand: not named to me, named to them.
    assert_eq!(
        bare(as_shown(note(hermetic_study()), &state, me, alone)),
        (None, None, None, 2),
        "the X stays, the card does not"
    );
    assert_eq!(
        as_shown(note(hermetic_study()), &state, them, alone),
        note(hermetic_study())
    );
    // A copy's modifications printed on a card I am not shown.
    let copying = baylee_engine::engine::DecisionNote {
        copying: true,
        copy_mods: Some((hermetic_study(), 0, 0)),
        printed: None,
        ..note(island())
    };
    assert_eq!(as_shown(copying.clone(), &state, me, alone).copy_mods, None);
    assert_eq!(as_shown(copying.clone(), &state, them, alone), copying);
    // A token no seat is shown.
    let token = baylee_engine::engine::DecisionNote {
        printed: None,
        token: Some(0),
        ..note(island())
    };
    assert_eq!(as_shown(token, &state, me, alone).token, None);
    // A face-down source: named to its controller only.
    let hidden = baylee_engine::engine::DecisionNote {
        source: Some(face_down),
        ..note(island())
    };
    assert_eq!(as_shown(hidden.clone(), &state, me, alone), hidden);
    assert_eq!(bare(as_shown(hidden, &state, them, alone)).0, None);
}

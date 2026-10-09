//! A report's `#` references never reach a hidden name (window B,
//! `client-core::bugreport::refs`).
//!
//! Held here, on a real game, because a hand-built view cannot hold a
//! library at all: the probe has teeth only where the data it asks about
//! exists. Seat 0 holds Lightning Bolt and draws from a library of Wrath of
//! God, with a Grizzly Bears face down and an Island on the battlefield;
//! seat 1 holds Giant Growth and draws from Llanowar Elves. Every prefix of
//! a name in a library, in the other seat's hand, or of the face-down card
//! (for its own controller too) offers nothing; the Island is found, on
//! the battlefield.

use super::*;
use baylee_client_core::bugreport::refs::{self, CardRef, RefZone};
use baylee_client_core::card_face::CardText;
use baylee_engine::object::Status;

fn named(name: &str) -> CardIndex {
    baylee_cards::decks::by_name(name).unwrap_or_else(|| panic!("{name} is in the pool"))
}

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        card: named(name),
        print: PrintRef::new(0),
    }
}

/// Two seats whose every hidden card has a name of its own.
fn secrets_preset() -> GamePreset {
    let seat = |library: &str, hand: &str, battlefield: Vec<DeckEntry>| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: baylee_core::preset::SeatCapabilities {
            dev_commands: true,
            see_hidden: false,
        },
        deck: (0..40).map(|_| entry(library)).collect(),
        sideboard: vec![],
        commanders: vec![],
        starting_life: None,
        starting_hand: Some(vec![entry(hand), entry(hand)]),
        starting_battlefield: battlefield,
        emblems: vec![],
        team: None,
    };
    GamePreset {
        format: FormatId::Freeform,
        seed: 7,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![print_info("EN", Finish::Normal)],
        seats: vec![
            seat(
                "Wrath of God",
                "Lightning Bolt",
                vec![entry("Island"), entry("Grizzly Bears")],
            ),
            seat("Llanowar Elves", "Giant Growth", vec![]),
        ],
    }
}

fn no_text(_: CardIndex, _: u8) -> Option<CardText> {
    None
}

/// What `query` offers `seat`, as the report's `#` list would.
fn offered(engine: &Engine<Registry>, seat: PlayerId, query: &str) -> Vec<CardRef> {
    let view = seen_by(engine, seat);
    let cards = refs::candidates(&view, None, &no_text);
    refs::best(&cards, query)
        .into_iter()
        .map(|i| cards[i].clone())
        .collect()
}

/// The game, its Grizzly Bears turned face down by the harness.
fn a_table_with_secrets() -> (Engine<Registry>, ObjectId) {
    let mut engine = Engine::new(&secrets_preset(), Registry).expect("game starts");
    let me = PlayerId::new(0);
    let bears = named("Grizzly Bears");
    let morph = seen_by(&engine, me)
        .battlefield
        .iter()
        .find(|o| o.card.is_some_and(|c| c.index == bears))
        .expect("the Bears are on the battlefield")
        .id;
    engine
        .dev_state_mut(me)
        .expect("the test preset grants dev commands")
        .object_mut(morph)
        .expect("the permanent is there")
        .status
        .insert(Status::FACE_DOWN);
    (engine, morph)
}

/// The hidden names this test asks about are really in the game, where it
/// says they are: a probe of absent data proves nothing.
#[test]
fn the_secrets_are_where_the_probe_looks() {
    let (engine, morph) = a_table_with_secrets();
    let state = engine.state();
    let card_of = |id: &ObjectId| state.object(*id).and_then(|o| o.card).map(|c| c.index);
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let lib = state.zones.list(ZoneLocation::Library(me));
    assert!(lib.len() > 30);
    assert!(
        lib.iter()
            .all(|id| card_of(id) == Some(named("Wrath of God")))
    );
    let hand = state.zones.list(ZoneLocation::Hand(me));
    assert!(
        !hand.is_empty()
            && hand
                .iter()
                .all(|id| card_of(id) == Some(named("Lightning Bolt")))
    );
    let theirs = state.zones.list(ZoneLocation::Hand(them));
    assert!(
        !theirs.is_empty()
            && theirs
                .iter()
                .all(|id| card_of(id) == Some(named("Giant Growth")))
    );
    // The controller's own view knows its morph, card and all: the filter,
    // not the view, is what keeps it out of a report.
    let mine = seen_by(&engine, me);
    let shown = mine.object(morph).expect("on the battlefield");
    assert!(shown.card.is_some() && shown.status.is_face_down());
}

/// No prefix of a library card, of the other seat's hand, or of a
/// face-down card offers anything, to either seat; the Island is offered
/// to both, on the battlefield.
#[test]
fn a_reference_never_offers_a_hidden_name() {
    let (engine, morph) = a_table_with_secrets();
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    for (seat, hidden) in [
        // Seat 1: seat 0's library, seat 0's hand, seat 0's morph, its own
        // library.
        (
            them,
            &[
                "W",
                "Wr",
                "Wrath",
                "of G",
                "God",
                "Li",
                "Lightning",
                "Bolt",
                "Gri",
                "Grizzly",
                "Bears",
                "Ll",
                "Llanowar",
                "Elves",
            ][..],
        ),
        // Seat 0: its own library, seat 1's hand and library, and its own
        // morph, which its view names.
        (
            me,
            &[
                "W", "Wrath", "God", "Gi", "Giant", "Growth", "Ll", "Elves", "Gri", "Grizzly",
                "Bears",
            ][..],
        ),
    ] {
        for query in hidden {
            let got = offered(&engine, seat, query);
            assert!(
                got.is_empty(),
                "{seat:?} was offered {:?} for {query:?}",
                got.iter().map(|c| &c.english).collect::<Vec<_>>()
            );
        }
        let island = offered(&engine, seat, "Isl");
        assert_eq!(island.len(), 1, "{seat:?}: the Island, once");
        assert_eq!(island[0].english, "Island");
        assert_eq!(island[0].zone, Some(RefZone::Battlefield));
        assert_eq!(island[0].owner, Some(me));
        assert_ne!(island[0].object, Some(morph));
    }
    // The seat's own hand is its own to name.
    let bolt = offered(&engine, me, "Bolt");
    assert_eq!(bolt.len(), 1);
    assert_eq!(bolt[0].zone, Some(RefZone::Hand));
    // And nothing that is offered is anything but what the view shows.
    for seat in [me, them] {
        let view = seen_by(&engine, seat);
        let shown: std::collections::BTreeSet<CardIndex> =
            view.identities().map(|s| s.card.index).collect();
        for card in refs::candidates(&view, None, &no_text) {
            assert!(
                shown.contains(&card.card),
                "{seat:?}: {} is not in the view",
                card.english
            );
        }
    }
}

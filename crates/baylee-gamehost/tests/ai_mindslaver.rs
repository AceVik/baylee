//! Mindslaver across the real engine boundary: the house controls the other
//! seat's next turn (CR 724.1) and plays it against that player (CR 724.5).
//!
//! The unit tests beside `baylee_ai`'s `controlled` module build each
//! question; this one earns them. The house's seat activates Mindslaver
//! (scripted, since the house has no reading of the card that would make it
//! do so), and from then on both seats are played by the house AI from the
//! views `Session::agent_view` builds: the controlled turn's questions go to
//! the controller (`Engine::decision_actor`) with a view that awaits it and
//! names the controlled player as the question's own.
//!
//! What a good controller does with the turn, and what is checked: their
//! Grizzly Bears walk into this seat's Hill Giant and die there, their Rhox
//! Brute, which the Giant cannot kill, stays home, their Volcanic Hammer goes
//! into their own face, their Gray Ogre and the land they drew stay in hand,
//! and their Mountains are left tapped for this seat's next turn. This seat
//! loses nothing, and no answer of either agent's breaks its question.

// Looks cards up, so this program holds the pool (`baylee-cards-pool`).
use baylee_ai::{AIProfile, HeuristicAgent};
use baylee_cards_pool as _;
use baylee_core::ids::PlayerId;
use baylee_core::preset::{DeckEntry, GamePreset};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::Engine;
use baylee_gamehost::{PlayerView, RegistryLookup, SeatContext, owed_payment, player_view, view};
use baylee_view::{ObjectStatus, Phase, Step};

const HOUSE: PlayerId = PlayerId::new(1);
const THEM: PlayerId = PlayerId::new(0);

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: baylee_cards::decks::by_name(name).expect("registered fixture"),
        print: baylee_core::ids::PrintRef::new(0),
    }
}

/// Seat 1, which takes the first turn, holds Mindslaver over four Plains and
/// a Hill Giant; seat 0 has
/// three Mountains, Grizzly Bears and Rhox Brute out, and a Volcanic Hammer
/// and a Gray Ogre in hand.
fn table() -> GamePreset {
    let mut preset = baylee_cards::decks::probe_preset(41, entry("Plains").card).unwrap();
    for seat in &mut preset.seats {
        seat.starting_hand = Some(vec![]);
        seat.starting_battlefield.clear();
    }
    let cards = |names: &[&str]| names.iter().map(|name| entry(name)).collect::<Vec<_>>();
    preset.seats[usize::from(HOUSE.get())].starting_battlefield = cards(&[
        "Mindslaver",
        "Plains",
        "Plains",
        "Plains",
        "Plains",
        "Hill Giant",
    ]);
    preset.seats[usize::from(THEM.get())].starting_battlefield = cards(&[
        "Mountain",
        "Mountain",
        "Mountain",
        "Grizzly Bears",
        "Rhox Brute",
    ]);
    preset.seats[usize::from(THEM.get())].starting_hand =
        Some(cards(&["Volcanic Hammer", "Gray Ogre"]));
    preset
}

/// The view `seat` answers `pending` from, built as `Session::agent_view`
/// builds it.
fn agent_view(engine: &Engine<RegistryLookup>, seat: PlayerId, seq: u64) -> PlayerView {
    let pending = engine.pending();
    player_view(
        engine.state(),
        seat,
        seq,
        Some(pending),
        &SeatContext {
            awaiting: view::awaiting_for(engine, seat),
            decision_player: pending.asked(),
            controlled_players: engine.controlled_players(seat),
            deciding: view::deciding(engine),
            owed: owed_payment(engine),
            inspected_cards: engine.inspected_cards(),
            ..Default::default()
        },
        &[],
    )
}

/// What `seat` sees of the table, asked nothing.
fn seen(engine: &Engine<RegistryLookup>, seat: PlayerId) -> PlayerView {
    player_view(engine.state(), seat, 0, None, &SeatContext::default(), &[])
}

/// The house's next step toward activating Mindslaver at seat 0, in its
/// own first main phase with an empty stack, or `None` once it is done or
/// when this is not that moment.
fn slave(view: &PlayerView, pending: &Pending, done: &mut bool) -> Option<PlayerAction> {
    if *done || view.seat != HOUSE {
        return None;
    }
    match pending {
        Pending::Priority { legal, .. }
            if view.active == HOUSE
                && view.phase == Phase::FirstMain
                && view.step == Step::Main
                && view.stack.is_empty() =>
        {
            let mindslaver = view
                .battlefield_of(HOUSE)
                .find(|o| o.name == "Mindslaver")?
                .id;
            if legal.abilities.contains(&(mindslaver, 0)) {
                return Some(PlayerAction::ActivateAbility {
                    source: mindslaver,
                    ability_index: 0,
                });
            }
            legal
                .mana_abilities
                .first()
                .map(|&source| PlayerAction::ActivateManaAbility { source })
        }
        Pending::ChooseTargets { player_options, .. } if player_options.contains(&THEM) => {
            *done = true;
            Some(PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![THEM],
            })
        }
        Pending::ChoosePlayer { options, .. } if options.contains(&THEM) => {
            *done = true;
            Some(PlayerAction::ChoosePlayer(THEM))
        }
        _ => None,
    }
}

fn named<'a>(objects: impl Iterator<Item = &'a baylee_view::PublicObject>, name: &str) -> usize {
    objects.filter(|o| o.name == name).count()
}

/// What the controlled turn told whom about its questions
/// (`view::decision_note`).
#[derive(Default)]
struct Notes {
    /// Questions of their turn whose note went to the house.
    of_their_turn: u32,
    /// Of those, the ones that named their Volcanic Hammer.
    their_hammer: u32,
}

/// Plays the table from the start until the controlled turn is over, every
/// question answered by its actor's agent, and returns the house's and their
/// life as that turn began.
///
/// At every question the note of it is built for both seats: only the seat
/// that answers is told, and during the controlled turn that is the house,
/// told about their spells, which their hand lends it (CR 724.4).
fn play_through_the_controlled_turn(
    engine: &mut Engine<RegistryLookup>,
    agents: &[HeuristicAgent; 2],
    notes: &mut Notes,
) -> (i32, i32) {
    let mut activated = false;
    let mut before: Option<(i32, i32)> = None;
    for seq in 0..4000 {
        let pending = engine.pending().clone();
        if matches!(pending, Pending::GameOver(_)) {
            break;
        }
        if let Pending::Mulligan { player, .. } = pending {
            engine.apply(player, PlayerAction::MulliganKeep).unwrap();
            continue;
        }
        let state = engine.state();
        let under_control = engine.may_inspect_private(HOUSE, THEM);
        match (under_control, before) {
            (true, None) => {
                assert!(activated, "control without Mindslaver");
                assert_eq!(state.turn.active, THEM, "the control is of their turn");
                before = Some((
                    state.players[usize::from(HOUSE.get())].life,
                    state.players[usize::from(THEM.get())].life,
                ));
            }
            (false, Some(lives)) => return lives,
            _ => {}
        }
        let actor = engine.decision_actor().expect("somebody is asked");
        if under_control && pending.asked() == Some(THEM) {
            assert_eq!(
                actor, HOUSE,
                "their question is the controller's (CR 723.5)"
            );
        }
        for seat in [HOUSE, THEM] {
            let note = view::decision_note(engine, seat);
            assert!(
                note.is_none() || seat == actor,
                "{seat:?} was told about a question {actor:?} answers: {note:?}"
            );
            if let Some(note) = note.filter(|_| under_control && pending.asked() == Some(THEM)) {
                notes.of_their_turn += 1;
                let hammer = baylee_cards::decks::by_name("Volcanic Hammer");
                notes.their_hammer += u32::from(note.printed.map(|(card, _)| card) == hammer);
            }
        }
        let view = agent_view(engine, actor, seq);
        let action = slave(&view, &pending, &mut activated).unwrap_or_else(|| {
            agents[usize::from(actor.get())].act_with_context(
                &view,
                &pending,
                &engine.decision_context(),
            )
        });
        if std::env::var_os("BAYLEE_HARNESS_TRACE").is_some() {
            let s = engine.state();
            eprintln!(
                "{seq} t{} {:?}/{:?} actor {actor:?} asked {:?}: {action:?}",
                s.turn.number,
                s.turn.phase,
                s.turn.step,
                pending.asked()
            );
        }
        engine
            .apply(actor, action.clone())
            .unwrap_or_else(|err| panic!("{action:?} for {pending:?} was refused: {err}"));
    }
    panic!("the controlled turn never began or never ended (Mindslaver activated: {activated})");
}

#[test]
fn the_house_plays_a_mindslaver_turn_against_its_player() {
    let mut engine = Engine::new(&table(), RegistryLookup).unwrap();
    let agents = [
        HeuristicAgent::new(AIProfile::EXPERT),
        HeuristicAgent::new(AIProfile::EXPERT),
    ];
    let mut notes = Notes::default();
    let (house_life, their_life) =
        play_through_the_controlled_turn(&mut engine, &agents, &mut notes);
    assert!(
        notes.their_hammer > 0 && notes.of_their_turn >= notes.their_hammer,
        "the house was not told about the Hammer it cast for them: {} notes, {} of the Hammer",
        notes.of_their_turn,
        notes.their_hammer
    );
    for (seat, agent) in agents.iter().enumerate() {
        assert_eq!(agent.fallbacks(), 0, "seat {seat} broke a question");
    }
    let table = seen(&engine, HOUSE);
    let mine = seen(&engine, THEM);
    assert_eq!(
        table.seats[usize::from(HOUSE.get())].life,
        house_life,
        "the controlled turn cost this seat life it could have kept"
    );
    assert_eq!(
        named(table.battlefield_of(HOUSE), "Hill Giant"),
        1,
        "their Hammer or their attack killed this seat's Giant"
    );
    assert_eq!(
        named(
            mine.graveyards[usize::from(THEM.get())].iter(),
            "Grizzly Bears"
        ),
        1,
        "their Bears did not die attacking into the Giant"
    );
    assert_eq!(
        named(mine.battlefield_of(THEM), "Rhox Brute"),
        1,
        "their Brute, which the Giant cannot kill, was sent anyway"
    );
    assert_eq!(
        named(
            mine.graveyards[usize::from(THEM.get())].iter(),
            "Volcanic Hammer"
        ),
        1,
        "their Hammer was never cast"
    );
    assert_eq!(
        table.seats[usize::from(THEM.get())].life,
        their_life - 3,
        "their Hammer went somewhere other than their own face"
    );
    assert!(
        mine.hand.iter().any(|card| card.name == "Gray Ogre"),
        "their Gray Ogre was cast, building their board"
    );
    let lands: Vec<&baylee_view::PublicObject> = mine
        .battlefield_of(THEM)
        .filter(|o| o.types.contains(baylee_core::types::TypeSet::LAND))
        .collect();
    assert_eq!(lands.len(), 3, "a land of theirs was played for them");
    assert!(
        lands
            .iter()
            .all(|land| land.status.contains(ObjectStatus::TAPPED)),
        "their Mountains were left untapped for this seat's turn"
    );
}

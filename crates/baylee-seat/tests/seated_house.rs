//! The seated house answers as the table's own house chair (owner,
//! 10.11.2026): from the view a seat is sent, the question's note
//! (`ChoiceRequest::context`) and the deck it brought.
//!
//! Every chair of a hosted table is the house at `sharp`, in-process, and
//! plays the game. At every question it is asked, the same chair is also
//! answered as `baylee-seat --mind house` would answer it: from
//! `Session::view_for_with_context` (the socket view with the clock and log
//! left out, which `house_view` drops anyway), the note of the question the
//! table would send (`Session::reask`) and the seat's own deck list. The two
//! answers are compared, and so is the answer the seat gave before it had
//! a note or a deck (the view alone).

use baylee_cards::decks::{house_deck, preset_for_all};
use baylee_core::ids::{CardIndex, ObjectId, PlayerId};
use baylee_core::preset::{AIProfile, FormatId, GamePreset, SeatController};
use baylee_engine::choice::Pending;
use baylee_engine::engine::DecisionNote;
use baylee_gamehost::Session;
use baylee_gamehost::record::{Line, Source};
use baylee_protocol::v1;
use baylee_seat::{DeckCard, DeckList, GameContext, HouseMind};
use std::collections::BTreeMap;

fn deck(name: &str) -> baylee_cards::decks::LoadedDeck {
    let path = format!("{}/../../data/decks/{name}.txt", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    house_deck(&text).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The deck list a seat that brought `spec` holds.
fn list(spec: &baylee_core::preset::SeatSpec) -> DeckList {
    let counted = |entries: &[baylee_core::preset::DeckEntry]| -> Vec<DeckCard> {
        let mut counts: BTreeMap<_, u32> = BTreeMap::new();
        for entry in entries {
            *counts.entry(entry.card).or_default() += 1;
        }
        counts
            .into_iter()
            .map(|(card, count)| DeckCard { card, count })
            .collect()
    };
    DeckList {
        name: String::new(),
        main: counted(&spec.deck),
        sideboard: counted(&spec.sideboard),
        commanders: counted(&spec.commanders),
    }
}

/// The note the table sends `seat` with its question, if any.
fn note(session: &mut Session, seat: PlayerId) -> Option<DecisionNote> {
    session
        .reask(seat)
        .into_iter()
        .find_map(|envelope| match envelope.msg {
            Some(v1::envelope::Msg::ChoiceRequest(request)) if !request.context.is_empty() => {
                baylee_core::wire::decode(&request.context).ok()
            }
            _ => None,
        })
}

/// A note no honest table sends, on top of what the table said, in one of
/// three shapes: a card the registry does not hold, a real card's face and
/// every index past its list, and a token the registry does not hold. Each
/// has an X at the top of its range and first targets at the reader's
/// bound that name nothing on the table.
fn hostile(said: Option<&DecisionNote>, shape: u32) -> DecisionNote {
    let real = baylee_cards::decks::by_name("Llanowar Elves").expect("in the pool");
    let mut note = said.cloned().unwrap_or_default();
    note.x = u32::MAX;
    note.x_targets = Some(u32::MAX);
    note.ability = Some(u32::MAX);
    note.mode = Some(u32::MAX);
    note.done = Some(u32::MAX);
    note.second_instance = true;
    note.copying = true;
    note.first_targets = (0..DecisionNote::MAX_FIRST_TARGETS)
        .map(|i| ObjectId::new(ObjectId::MAX_SLOT - u32::try_from(i).expect("small"), 7))
        .collect();
    match shape % 3 {
        0 => {
            note.printed = Some((CardIndex::new(u32::MAX), u8::MAX));
            note.copy_mods = Some((CardIndex::new(u32::MAX), u8::MAX, u32::MAX));
        }
        1 => {
            note.printed = Some((real, u8::MAX));
            note.copy_mods = Some((real, u8::MAX, u32::MAX));
        }
        _ => note.token = Some(u16::MAX),
    }
    note
}

fn table(seed: u64, names: &[&str]) -> GamePreset {
    let decks: Vec<_> = names.iter().map(|name| deck(name)).collect();
    let refs: Vec<_> = decks.iter().collect();
    let mut preset = preset_for_all(seed, &refs);
    for spec in &mut preset.seats {
        spec.controller = SeatController::Ai(AIProfile::SHARP);
    }
    preset
}

#[derive(Default)]
struct Agreement {
    asked: u32,
    now: u32,
    before: u32,
    noted: u32,
}

/// Plays `preset` to its end with the table's house, setting the seated
/// house's answer beside every one of its answers, per question kind.
fn compare(game: &str, preset: &GamePreset, tally: &mut BTreeMap<String, Agreement>) {
    let mut session = Session::new_recorded(preset, "seated").expect("the table starts");
    let seats = preset.seats.len();
    session.describe(game.into(), (0..seats).map(|s| s.to_string()).collect());
    let contexts: Vec<GameContext> = (0..seats)
        .map(|s| GameContext {
            game_id: game.into(),
            seat: PlayerId::new(u8::try_from(s).expect("a small table")),
            seats: u8::try_from(seats).expect("a small table"),
            teams: preset.seats.iter().map(|spec| spec.team).collect(),
            format: FormatId::Freeform,
            deck: list(&preset.seats[s]),
            decision_secs: None,
        })
        .collect();
    let mind = HouseMind::new(AIProfile::SHARP);
    for step in 0..20_000 {
        if matches!(session.pending(), Pending::GameOver(_)) {
            break;
        }
        // `pump` answers the lowest awaited seat first.
        let Some(seat) = session.awaited().iter().next() else {
            break;
        };
        let Some((pending, view)) = session.view_for_with_context(seat) else {
            break;
        };
        let said = note(&mut session, seat);
        let context = &contexts[usize::from(seat.get())];
        let now = mind.answer_noted(context, &view, &pending, said.as_ref());
        // A hostile table's note in its place is answered, not panicked on.
        let _ = mind.answer_noted(
            context,
            &view,
            &pending,
            Some(&hostile(said.as_ref(), step)),
        );
        let before = baylee_ai::HeuristicAgent::new(AIProfile::SHARP)
            .with_teams(context.teams.clone())
            .with_seed(baylee_ai::policy_seed(game, seat.get()))
            .act(
                &baylee_seat::house::house_view(view, AIProfile::SHARP_2026_11),
                &pending,
            );
        let _ = session.take_record();
        let _ = session.pump_at_most(1);
        let record = String::from_utf8(session.take_record()).expect("a record is text");
        let Some(applied) =
            record
                .lines()
                .find_map(|line| match serde_json::from_str::<Line>(line).ok()? {
                    Line::Input {
                        seat: s,
                        by: Source::House,
                        action,
                        ..
                    } if s == seat.get() => Some(action),
                    _ => None,
                })
        else {
            continue;
        };
        let kind = format!("{pending:?}")
            .split([' ', '{', '('])
            .next()
            .unwrap_or_default()
            .to_owned();
        let entry = tally.entry(kind).or_default();
        entry.asked += 1;
        entry.now += u32::from(now == applied);
        entry.before += u32::from(before == applied);
        entry.noted += u32::from(said.is_some());
    }
}

/// The seated house now gives the table house's answer nearly everywhere,
/// and far more often than it did with the view alone.
#[test]
fn the_seated_house_answers_as_the_tables_own() {
    let mut tally = BTreeMap::new();
    let pairs: [&[&str]; 4] = [
        &["allytifact", "victory"],
        &["victory", "maik"],
        &["schwarzrand", "weltenbaum"],
        &["maik", "schwarzrand"],
    ];
    for (seed, names) in pairs.iter().enumerate() {
        let seed = u64::try_from(seed).expect("few") + 31;
        compare(
            &format!("seated-house/{seed}"),
            &table(seed, names),
            &mut tally,
        );
    }
    let (mut asked, mut now, mut before) = (0, 0, 0);
    for (kind, a) in &tally {
        eprintln!(
            "{kind:22} asked {:6} noted {:6} agree now {:6} before {:6}",
            a.asked, a.noted, a.now, a.before
        );
        asked += a.asked;
        now += a.now;
        before += a.before;
    }
    eprintln!("all: asked {asked}, agree now {now}, before {before}");
    assert!(asked > 1_000, "the tables asked little");
    assert!(
        now > before,
        "the note and the deck made the seat agree less: {now} against {before}"
    );
    assert!(
        f64::from(now) >= 0.99 * f64::from(asked),
        "the seated house disagreed with the table's in {} of {asked}",
        asked - now
    );
}

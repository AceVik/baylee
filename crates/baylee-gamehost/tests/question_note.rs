//! What a question is about (`DecisionNote`, `ChoiceRequest::context`) is
//! told to the seat it asks and to nobody else, and names the asking card
//! only where that seat can see it (`docs/protocol.md` §"What a question is
//! about").

use baylee_cards::decks::{house_deck, preset_for_all};
use baylee_core::ids::PlayerId;
use baylee_core::preset::{AIProfile, GamePreset, SeatController};
use baylee_engine::choice::Pending;
use baylee_gamehost::Session;
use baylee_protocol::v1;

fn duel(seed: u64) -> GamePreset {
    let decks: Vec<_> = ["allytifact", "victory"]
        .iter()
        .map(|name| {
            let path = format!("{}/../../data/decks/{name}.txt", env!("CARGO_MANIFEST_DIR"));
            let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
            house_deck(&text).unwrap_or_else(|e| panic!("{name}: {e}"))
        })
        .collect();
    let refs: Vec<_> = decks.iter().collect();
    let mut preset = preset_for_all(seed, &refs);
    for spec in &mut preset.seats {
        spec.controller = SeatController::Ai(AIProfile::SHARP);
    }
    preset
}

/// The context a question to `seat` carries on the wire.
fn sent(session: &mut Session, seat: PlayerId) -> Vec<u8> {
    session
        .reask(seat)
        .into_iter()
        .find_map(|envelope| match envelope.msg {
            Some(v1::envelope::Msg::ChoiceRequest(request)) => Some(request.context),
            _ => None,
        })
        .unwrap_or_default()
}

#[test]
fn only_the_asked_seat_is_told_and_only_of_what_it_can_see() {
    let mut noted = 0;
    let mut named = 0;
    for seed in [3, 4] {
        let mut session = Session::new(&duel(seed)).expect("the duel starts");
        session.describe(format!("note/{seed}"), vec!["0".into(), "1".into()]);
        for _ in 0..6_000 {
            if matches!(session.pending(), Pending::GameOver(_)) {
                break;
            }
            for seat in [PlayerId::new(0), PlayerId::new(1)] {
                let note = session.question_note(seat);
                let asked = session.view_for(seat).is_some();
                let wire = sent(&mut session, seat);
                let Some(note) = note else {
                    assert!(wire.is_empty(), "a seat with no note was sent one");
                    continue;
                };
                assert!(asked, "{seat:?} was told about a question it is not asked");
                let read: baylee_engine::engine::DecisionNote =
                    baylee_core::wire::decode(&wire).expect("the note reads off the wire");
                assert_eq!(read, note, "the wire carries the note as built");
                noted += 1;
                if let (Some(source), Some(_)) = (note.source, note.printed) {
                    named += 1;
                    let (_, view) = session.view_for(seat).expect("asked");
                    let seen = view.object(source).is_some_and(|object| {
                        object.card.is_some() || object.rules.is_some() || object.token.is_some()
                    }) || view.hand.iter().any(|card| card.id == source);
                    assert!(
                        seen,
                        "{seat:?} was told the identity of a source it cannot see"
                    );
                }
            }
            let _ = session.pump_at_most(1);
        }
    }
    assert!(noted > 50, "few questions carried a note: {noted}");
    assert!(named > 10, "few notes named their card: {named}");
}

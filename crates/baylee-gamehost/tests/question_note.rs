//! What a question is about (`DecisionNote`, `ChoiceRequest::context`) is
//! told to the seat it asks and to nobody else, and names the asking card
//! only where that seat can see it (`docs/protocol.md` §"What a question is
//! about").

use baylee_cards::decks::{house_deck, preset_for_all};
use baylee_core::ids::{CardIndex, ObjectId, PlayerId};
use baylee_core::preset::{AIProfile, GamePreset, SeatController};
use baylee_engine::choice::Pending;
use baylee_engine::object::Status;
use baylee_engine::state::GameState;
use baylee_engine::zone::Zone;
use baylee_gamehost::Session;
use baylee_protocol::v1;

/// A table of `names`' decks at `sharp`, in teams where `teams` says, so
/// that teammates show each other their hands.
fn table(seed: u64, names: &[&str], teams: &[u8]) -> GamePreset {
    let decks: Vec<_> = names
        .iter()
        .map(|name| {
            let path = format!("{}/../../data/decks/{name}.txt", env!("CARGO_MANIFEST_DIR"));
            let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
            house_deck(&text).unwrap_or_else(|e| panic!("{name}: {e}"))
        })
        .collect();
    let refs: Vec<_> = decks.iter().collect();
    let mut preset = preset_for_all(seed, &refs);
    for (at, spec) in preset.seats.iter_mut().enumerate() {
        spec.controller = SeatController::Ai(AIProfile::SHARP);
        spec.team = teams.get(at).copied();
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

/// Whether `seat` may see which card `id` is, read off the rules rather than
/// the host's own helpers: in its own hand, face up in a zone every player
/// sees (CR 400.2), and face down only as the permanent's controller or as
/// a player allowed to look at the exiled card (CR 708.5, 406.3). These
/// duels control no turns.
fn may_see(state: &GameState, seat: PlayerId, id: ObjectId) -> bool {
    let Some(object) = state.object(id) else {
        return false;
    };
    let face_down = object.status.contains(Status::FACE_DOWN);
    match object.zone {
        Zone::Hand => object.owner == seat,
        Zone::Library | Zone::OutsideGame => false,
        Zone::Exile if face_down => object.may_look_at_exile(seat),
        _ if face_down => object.controller == seat,
        _ => true,
    }
}

/// Whether `seat` sees an object that is `card` or carries its rules.
fn shown(state: &GameState, seat: PlayerId, card: CardIndex) -> bool {
    state.arena.iter().any(|(id, object)| {
        (object.card.is_some_and(|printing| printing.index == card)
            || object
                .printed_face()
                .is_some_and(|face| face.card() == card))
            && may_see(state, seat, id)
    })
}

/// Over two duels and a game of two teams of two, whose teammates show
/// each other their hands: a hand a teammate is shown is still not a zone
/// the note may name a card in.
#[test]
fn only_the_asked_seat_is_told_and_only_of_what_it_can_see() {
    let mut noted = 0;
    let mut named = 0;
    let duel: &[&str] = &["allytifact", "victory"];
    let teams: &[&str] = &["allytifact", "victory", "maik", "schwarzrand"];
    for (seed, names, sides) in [
        (3, duel, &[][..]),
        (4, duel, &[]),
        (5, teams, &[1, 2, 1, 2]),
    ] {
        let preset = table(seed, names, sides);
        let seats: Vec<PlayerId> = (0..preset.seats.len())
            .map(|seat| PlayerId::new(u8::try_from(seat).expect("a small table")))
            .collect();
        let mut session = Session::new(&preset).expect("the table starts");
        session.describe(
            format!("note/{seed}"),
            seats.iter().map(|seat| seat.get().to_string()).collect(),
        );
        for _ in 0..8_000 {
            if matches!(session.pending(), Pending::GameOver(_)) {
                break;
            }
            for &seat in &seats {
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
                if let Some((card, _)) = note.printed {
                    named += 1;
                    let state = session.state();
                    let source = note.source.expect("a named card names its source");
                    assert!(
                        may_see(state, seat, source),
                        "{seat:?} was told the identity of a source it cannot see: {:?}",
                        state.object(source).map(|o| (o.zone, o.owner, o.status))
                    );
                    assert!(
                        shown(state, seat, card),
                        "{seat:?} was pointed into a card it is not shown"
                    );
                }
            }
            let _ = session.pump_at_most(1);
        }
    }
    assert!(noted > 50, "few questions carried a note: {noted}");
    assert!(named > 10, "few notes named their card: {named}");
}

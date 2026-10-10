use super::*;

// ---- a spectator sees the public table and nothing any one seat may see
//
// Every test here reads what a spectator is *sent*: the frames of
// `Session::spectator_snapshot`, decoded as a client decodes them, so a
// leak in the payload, the view or the log is caught on the wire.

use crate::Session;
use crate::session::tests::{a_kept_table, show_to};
use crate::view::SPECTATOR;
use baylee_protocol::v1::{self, Envelope};
use baylee_view::{GameStatic, LogObject, LogTail, SpectatorView};

/// What a spectator is sent, decoded: the payload, the latest view, and
/// every log line.
struct Sent {
    statics: GameStatic,
    view: SpectatorView,
    log: LogTail,
}

fn decode(frames: &[Envelope]) -> Sent {
    let mut statics = None;
    let mut view = None;
    let mut log = LogTail::default();
    for env in frames {
        match &env.msg {
            Some(v1::envelope::Msg::GameStatic(s)) => {
                statics = Some(serde_json::from_slice(&s.static_json).expect("static decodes"));
            }
            Some(v1::envelope::Msg::StateDelta(d)) => {
                view = Some(serde_json::from_slice(&d.view_json).expect("view decodes"));
                if !d.log_json.is_empty() {
                    let part: LogTail = serde_json::from_slice(&d.log_json).expect("log decodes");
                    log.entries.extend(part.entries);
                }
            }
            _ => {}
        }
    }
    Sent {
        statics: statics.expect("a spectator is sent its payload"),
        view: view.expect("a spectator is sent a view"),
        log,
    }
}

fn watched(session: &mut Session) -> Sent {
    session.set_spectators(1);
    decode(&session.spectator_snapshot())
}

/// Every object id a spectator view carries.
fn ids_in(view: &SpectatorView) -> Vec<ObjectId> {
    let mut ids = Vec::new();
    for zone in [&view.battlefield, &view.stack, &view.library_tops] {
        ids.extend(zone.iter().map(|o| o.id));
    }
    for per_seat in [&view.graveyards, &view.exile, &view.command] {
        for zone in per_seat {
            ids.extend(zone.iter().map(|o| o.id));
        }
    }
    ids
}

/// Every id in every hand and library at the table.
fn hidden_ids(state: &baylee_engine::state::GameState) -> Vec<ObjectId> {
    state
        .players
        .iter()
        .flat_map(|p| {
            let mut ids = state.zones.list(ZoneLocation::Hand(p.id)).to_vec();
            ids.extend(state.zones.list(ZoneLocation::Library(p.id)));
            ids
        })
        .collect()
}

/// The spectator's sentinel is no seat: no player has its id and no seat
/// set can hold it, which is what every "is this the viewer's" test the
/// object walk makes relies on.
#[test]
fn the_spectator_is_no_seat() {
    let session = Session::new(&mixed_print_preset()).expect("a game");
    assert!(session.state().players.iter().all(|p| p.id != SPECTATOR));
    assert!(SPECTATOR.get() > SeatSet::MAX_SEAT);
    assert!(!SeatSet::new().contains(SPECTATOR));
}

/// Hands and libraries are counts to a spectator, every one of them.
#[test]
fn a_spectator_is_shown_no_hand_and_no_library_only_their_counts() {
    let mut session = a_kept_table([None; 4], [false; 4]);
    session.pump();
    let sent = watched(&mut session);
    let state = session.state();
    let visible = ids_in(&sent.view);
    let hidden = hidden_ids(state);
    assert!(hidden.len() > 80, "hands and libraries are full");
    for id in &hidden {
        assert!(
            !visible.contains(id),
            "a hidden card reached a spectator: {id:?}"
        );
    }
    for p in &state.players {
        let seat = sent
            .view
            .seats
            .iter()
            .find(|s| s.player == p.id)
            .expect("a seat");
        assert_eq!(
            seat.hand_count,
            state.zones.list(ZoneLocation::Hand(p.id)).len() as u32
        );
        assert_eq!(
            seat.library_count,
            state.zones.list(ZoneLocation::Library(p.id)).len() as u32
        );
    }
}

/// The payload names no printing a spectator has not seen in public: a
/// seat starts with its own deck's printings, a spectator with none, so a
/// decklist cannot be read off the print table.
#[test]
fn a_spectators_print_table_holds_only_what_it_saw() {
    let mut session = a_kept_table([None; 4], [false; 4]);
    session.pump();
    let sent = watched(&mut session);
    let seen: Vec<_> = sent
        .view
        .clone()
        .into_player_view(PlayerId::new(0))
        .prints()
        .chain(sent.log.prints())
        .collect();
    for (i, entry) in sent.statics.prints.iter().enumerate() {
        let print = baylee_core::ids::PrintRef::new(i as u16);
        assert_eq!(
            entry.is_some(),
            seen.contains(&print),
            "print {i}: a spectator holds exactly the printings it was shown"
        );
    }
    let deck_only = session
        .game_static(PlayerId::new(0))
        .prints
        .iter()
        .flatten()
        .count();
    let shown = sent.statics.prints.iter().flatten().count();
    assert!(
        shown < deck_only,
        "seat 0 holds its deck's printings; a spectator fewer"
    );
}

/// A face-down permanent (a morph, a manifest; CR 708.5) and a face-down
/// exiled card are blanks to a spectator, as to every seat but the one who
/// controls them.
#[test]
fn a_face_down_card_is_blank_to_a_spectator() {
    let preset = mixed_print_preset();
    let mut session = Session::new(&preset).expect("a game");
    session.pump();
    let land = session.state().zones.list(ZoneLocation::Battlefield)[0];
    let controller = session.state().object(land).expect("there").controller;
    let mut engine = Engine::new(&preset, Registry).expect("a game");
    engine
        .dev_state_mut(controller)
        .expect("dev commands")
        .object_mut(land)
        .expect("there")
        .status
        .insert(baylee_engine::object::Status::FACE_DOWN);
    let view =
        crate::view::spectator_view(engine.state(), 1, None, SeatSet::new(), SeatSet::new(), &[]);
    let blank = view
        .battlefield
        .iter()
        .find(|o| o.id == land)
        .expect("on the battlefield");
    assert!(
        blank.card.is_none(),
        "a spectator was handed a face-down card's identity"
    );
    assert_eq!(blank.name, "Face-down");
    // Its controller, for contrast, knows it: the blank is the spectator's.
    let theirs = player_view(
        engine.state(),
        controller,
        1,
        None,
        &SeatContext::default(),
        &[],
    );
    assert!(
        theirs
            .battlefield
            .iter()
            .any(|o| o.id == land && o.card.is_some())
    );
}

/// A hand its owner shares with a teammate (#265) is that teammate's to
/// see. A spectator is on no team and is shown it as any other hand: a
/// count.
#[test]
fn a_teammates_shared_hand_is_a_count_to_a_spectator() {
    let mut session = a_kept_table([Some(1), Some(1), Some(2), Some(2)], [false; 4]);
    show_to(&mut session, 0, &[1]).expect("shows");
    let sent = watched(&mut session);
    let hand = session
        .state()
        .zones
        .list(ZoneLocation::Hand(PlayerId::new(0)))
        .to_vec();
    assert!(!hand.is_empty());
    let visible = ids_in(&sent.view);
    assert!(
        hand.iter().all(|id| !visible.contains(id)),
        "a shared hand reached a spectator"
    );
}

/// A card the log names to one seat only (a draw, a card looked at, a card
/// revealed to one player) is `Hidden` to a spectator: no line it is sent
/// names a card that is in a hand or a library, while the seat that drew
/// one is told it by name.
#[test]
fn a_spectator_is_told_no_log_line_about_a_private_card() {
    let mut session = a_kept_table([None; 4], [false; 4]);
    // Played on until somebody has drawn, which is a line named to the
    // drawer alone.
    for _ in 0..400 {
        if session.state().turn.number >= 3 {
            break;
        }
        let seat = session.awaiting_seat().expect("the game goes on");
        let answer = session.timeout_action(seat).expect("an answer");
        session.act(seat, answer).expect("legal");
    }
    let hidden = hidden_ids(session.state());
    let names_hidden = |log: &LogTail| {
        log.entries
            .iter()
            .flat_map(|e| e.event.objects())
            .any(|o| matches!(o, LogObject::Known { id, .. } if hidden.contains(id)))
    };
    let drawer = (0..4).any(|seat| {
        let mut log = LogTail::default();
        for env in session.snapshot(PlayerId::new(seat)) {
            if let Some(v1::envelope::Msg::StateDelta(d)) = &env.msg
                && !d.log_json.is_empty()
            {
                let part: LogTail = serde_json::from_slice(&d.log_json).expect("decodes");
                log.entries.extend(part.entries);
            }
        }
        names_hidden(&log)
    });
    assert!(drawer, "some seat is told a card of its own by name");
    let sent = watched(&mut session);
    assert!(!sent.log.entries.is_empty(), "a spectator is told the log");
    assert!(
        !names_hidden(&sent.log),
        "a spectator was told a private card"
    );
}

/// Word of Command (CR 722.4) shows its caster the target's hand while the
/// caster chooses. A spectator is not the caster.
#[test]
fn word_of_commands_hand_is_not_shown_to_a_spectator() {
    let (mut session, hand, print) = super::command_card::at_the_choice_parts();
    let sent = watched(&mut session);
    let visible = ids_in(&sent.view);
    assert!(
        hand.iter().all(|id| !visible.contains(id)),
        "the target's hand reached a spectator"
    );
    assert!(sent.statics.print(print).is_none(), "nor its printing");
}

/// A searching seat is shown its library's cards (`looking_at`); a spectator
/// is shown none of them, the question being no spectator's.
#[test]
fn a_spectator_is_never_shown_what_a_seat_is_looking_at() {
    let mut session = a_kept_table([None; 4], [false; 4]);
    session.pump();
    // The opening mulligan asks seat 0 about its own hand, which is a
    // `looking_at`-shaped question; the spectator is shown none of it.
    let hand = session
        .state()
        .zones
        .list(ZoneLocation::Hand(PlayerId::new(0)))
        .to_vec();
    let sent = watched(&mut session);
    let visible = ids_in(&sent.view);
    assert!(hand.iter().all(|id| !visible.contains(id)));
}

/// What a spectator view may carry is a fixed list of fields: the next
/// field someone adds to it has to be added here, on purpose.
#[test]
fn a_spectator_view_carries_exactly_the_public_fields() {
    let mut session = Session::new(&mixed_print_preset()).expect("a game");
    session.pump();
    let view = session.spectator_view();
    let json = serde_json::to_value(&view).expect("serializes");
    let mut keys: Vec<_> = json
        .as_object()
        .expect("an object")
        .keys()
        .cloned()
        .collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "active",
            "awaiting",
            "battlefield",
            "combat",
            "command",
            "day_night",
            "deciding",
            "exile",
            "graveyards",
            "library_tops",
            "monarch",
            "phase",
            "seats",
            "seq",
            "stack",
            "step",
            "turn",
        ]
    );
}

/// A spectator has nothing to answer with: the session refuses any action
/// in its name, and a pump never asks it anything.
#[test]
fn a_spectator_cannot_act() {
    let mut session = a_kept_table([None; 4], [false; 4]);
    session.set_spectators(1);
    let routed = session.pump();
    assert!(
        session.act(SPECTATOR, PlayerAction::MulliganKeep).is_err(),
        "an action in a spectator's name was taken"
    );
    assert!(
        routed
            .iter()
            .filter(|(to, _)| *to == SPECTATOR)
            .all(|(_, env)| !matches!(env.msg, Some(v1::envelope::Msg::ChoiceRequest(_)))),
        "a spectator was asked a question"
    );
    assert!(
        routed.iter().any(|(to, _)| *to == SPECTATOR),
        "a pump with a spectator watching tells it the table"
    );
}

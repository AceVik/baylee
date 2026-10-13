//! A replay's projection (`session/project.rs`) held against the live game it
//! replays: what a seat was sent, what a spectator was sent, what no one was
//! sent, and how far each moment is known to be the recorded game's.

use std::collections::BTreeSet;
use std::ops::ControlFlow;

use crate::Session;
use crate::record::tests::{encode, lines, played, table};
use crate::record::{Line, ReplayError, Source, hex};
use crate::replay::{
    Basis, Check, Controller, Divergence, Moment, Perspective, Projection, Stop, Verify, project,
    summary,
};
use crate::session::RegistryLookup;
use baylee_core::ids::{ObjectId, PlayerId, SeatSet};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::Engine;
use baylee_engine::state::GameState;
use baylee_engine::zone::ZoneLocation;
use baylee_protocol::v1::{self, Envelope};
use baylee_view::{GameStatic, LogEntry, LogEvent, LogTail, PlayerView, SpectatorView};

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

fn names() -> Vec<String> {
    vec!["Alice Example".into(), "Bob".into()]
}

/// A projection and every moment it handed on.
fn run(
    record: &[u8],
    perspective: Perspective,
    verify: Verify,
) -> Result<(Projection, Vec<Moment>), ReplayError> {
    let mut moments = Vec::new();
    let projection = project(record, names(), perspective, verify, |moment| {
        moments.push(moment);
        ControlFlow::Continue(())
    })?;
    Ok((projection, moments))
}

fn whole(record: &[u8], perspective: Perspective) -> (Projection, Vec<Moment>) {
    run(record, perspective, Verify::Stop).expect("the record projects")
}

// ---- reading what a moment holds, as a client reads it

fn deltas(moment: &Moment) -> Vec<&v1::StateDelta> {
    moment
        .frames
        .iter()
        .filter_map(|env| match &env.msg {
            Some(v1::envelope::Msg::StateDelta(delta)) => Some(delta),
            _ => None,
        })
        .collect()
}

/// The view a moment tells, when it tells one.
fn seat_view(moment: &Moment) -> Option<PlayerView> {
    let delta = *deltas(moment).last()?;
    Some(serde_json::from_slice(&delta.view_json).expect("a seat's view decodes"))
}

/// Every log tail a moment tells, in order.
fn tails(moment: &Moment) -> Vec<LogTail> {
    deltas(moment)
        .into_iter()
        .filter(|delta| !delta.log_json.is_empty())
        .map(|delta| serde_json::from_slice(&delta.log_json).expect("a log tail decodes"))
        .collect()
}

/// The opening payload a moment carries, if it carries one.
fn statics(moment: &Moment) -> Option<GameStatic> {
    moment.frames.iter().rev().find_map(|env| match &env.msg {
        Some(v1::envelope::Msg::GameStatic(s)) => {
            Some(serde_json::from_slice(&s.static_json).expect("a payload decodes"))
        }
        _ => None,
    })
}

/// The `GameOver` question a moment carries, if it does.
fn game_over(moment: &Moment) -> Option<&[u8]> {
    moment.frames.iter().find_map(|env| match &env.msg {
        Some(v1::envelope::Msg::ChoiceRequest(choice)) => Some(choice.pending_json.as_slice()),
        _ => None,
    })
}

/// A log as a client keeps it: a tail replaces the log from its `from`.
fn append(log: &mut Vec<LogEntry>, tail: LogTail) {
    log.truncate(tail.from as usize);
    log.extend(tail.entries);
}

fn with_seq_of(mut view: PlayerView, other: &PlayerView) -> PlayerView {
    view.seq = other.seq;
    view
}

// ---- the record, as a test edits it

/// The positions among `written` of its inputs.
fn input_positions(written: &[Line]) -> Vec<usize> {
    (0..written.len())
        .filter(|&i| matches!(written[i], Line::Input { .. }))
        .collect()
}

fn n_of(line: &Line) -> u64 {
    match line {
        Line::Input { n, .. } => *n,
        other => panic!("not an input: {other:?}"),
    }
}

/// The `n` of the input before position `at` in the list of inputs, if any.
fn before(written: &[Line], inputs: &[usize], at: usize) -> Option<u64> {
    at.checked_sub(1).map(|i| n_of(&written[inputs[i]]))
}

const WRONG: &str = "0000000000000000";
const OTHER_SEEN: &str = "deadbeefdeadbeef";

fn set_hash(line: &mut Line, to: &str) {
    match line {
        Line::Header { hash, .. } | Line::Input { hash, .. } => *hash = to.to_owned(),
        other => panic!("no hash on {other:?}"),
    }
}

fn set_seen(line: &mut Line, to: Option<&str>) {
    match line {
        Line::Header { seen, .. } | Line::Input { seen, .. } => *seen = to.map(str::to_owned),
        other => panic!("no seen on {other:?}"),
    }
}

// ---- the state of the table at a moment, rebuilt from the record

/// An engine played alongside a projection, input by input, so a test can ask
/// what the table really held at the moment a frame was made.
struct Shadow {
    engine: Engine<RegistryLookup>,
    inputs: std::vec::IntoIter<(u64, u8, PlayerAction)>,
}

impl Shadow {
    fn new(record: &[u8]) -> Self {
        let written = lines(record);
        let Some(Line::Header { preset, .. }) = written.first() else {
            panic!("a record opens with its header");
        };
        let inputs: Vec<_> = written
            .iter()
            .filter_map(|line| match line {
                Line::Input {
                    n, seat, action, ..
                } => Some((*n, *seat, action.clone())),
                _ => None,
            })
            .collect();
        Self {
            engine: Engine::new(preset, RegistryLookup).expect("the preset builds"),
            inputs: inputs.into_iter(),
        }
    }

    /// The table after record input `input` (before the first for `None`).
    fn at(&mut self, input: Option<u64>) -> &GameState {
        while let Some(upto) = input {
            if self
                .inputs
                .as_slice()
                .first()
                .is_none_or(|next| next.0 > upto)
            {
                break;
            }
            let (_, seat, action) = self.inputs.next().expect("peeked");
            self.engine
                .apply(PlayerId::new(seat), action)
                .expect("the recorded input applies");
        }
        self.engine.state()
    }
}

fn hand_ids(state: &GameState, seat: PlayerId) -> Vec<ObjectId> {
    state.zones.list(ZoneLocation::Hand(seat)).clone()
}

fn hand_cards(state: &GameState, seat: PlayerId) -> Vec<baylee_core::ids::CardIndex> {
    hand_ids(state, seat)
        .iter()
        .map(|id| {
            state
                .object(*id)
                .and_then(|o| o.card)
                .expect("a card")
                .index
        })
        .collect()
}

/// Every object id a seat's view names outside its own hand, its looking-at,
/// the hands it is shown and the library tops a rule shows everyone.
fn ids_on_the_table(view: &PlayerView) -> Vec<ObjectId> {
    let mut ids: Vec<ObjectId> = Vec::new();
    for zone in [&view.battlefield, &view.stack] {
        ids.extend(zone.iter().map(|o| o.id));
    }
    for per_seat in [&view.graveyards, &view.exile, &view.command] {
        for zone in per_seat {
            ids.extend(zone.iter().map(|o| o.id));
        }
    }
    ids.extend(view.combat.attackers.iter().map(|a| a.creature));
    ids.extend(view.combat.blockers.iter().map(|b| b.blocker));
    ids.extend(view.combat.bands.iter().flatten());
    ids
}

// ---- the live game, as seat 0's socket was told it

/// The acceptance game played as `played` plays it, keeping what seat 0 was
/// sent and how many inputs the record held when it was.
#[derive(Default)]
struct Live {
    record: Vec<u8>,
    inputs: usize,
    sent: Vec<(usize, Envelope)>,
}

impl Live {
    fn take(&mut self, session: &mut Session, frames: Vec<(PlayerId, Envelope)>) {
        let new = session.take_record();
        self.inputs += lines(&new)
            .iter()
            .filter(|line| matches!(line, Line::Input { .. }))
            .count();
        self.record.extend(new);
        self.sent.extend(
            frames
                .into_iter()
                .filter(|(seat, _)| *seat == ME)
                .map(|(_, env)| (self.inputs, env)),
        );
    }
}

/// [`played`]'s game, with seat 0's frames kept. The player leaves at
/// answer `leave_at`, if it is given; else the player plays to the end.
fn live(seed: u64, leave_at: Option<u32>) -> Live {
    let mut session = Session::new_recorded(&table(seed), "test").expect("the table builds");
    session.describe("g".into(), names());
    session.tell_time(1_000);
    let mut live = Live::default();
    let frames = session.pump();
    live.take(&mut session, frames);
    let mut asked = 0_u32;
    while !matches!(session.pending(), Pending::GameOver(_)) {
        if Some(asked) == leave_at {
            assert!(session.stand_in(ME));
            for _ in 0..16 {
                let frames = session.pump();
                live.take(&mut session, frames);
            }
            break;
        }
        if asked == 20 {
            assert!(session.stand_in(ME));
            assert!(session.hand_back(ME));
        }
        let at = session.asked_at(ME).expect("the player is asked");
        session.tell_time(1_000 + u64::from(asked));
        let frames = if asked % 4 == 3 {
            session
                .answer_by_clock(ME, at)
                .expect("the player is asked")
                .expect("the clock's answer stands")
        } else {
            let action = session.house_action(ME).expect("a question has an answer");
            session.act(ME, action).expect("the player's answer stands")
        };
        live.take(&mut session, frames);
        asked += 1;
    }
    live.take(&mut session, Vec::new());
    live
}

/// The log a live seat was sent held to the log a replay tells.
fn same_log(log: Vec<LogEntry>, projected_log: &[LogEntry]) {
    // A chair changing hands is no input: the record notes it and a
    // replay passes over it, so the live log's two lines about the
    // player leaving and coming back are the only ones a replay lacks.
    let (chairs, game): (Vec<LogEntry>, Vec<LogEntry>) = log.into_iter().partition(|entry| {
        matches!(
            entry.event,
            LogEvent::StandIn { .. } | LogEvent::Returned { .. }
        )
    });
    assert_eq!(chairs.len(), 2, "{chairs:?}");
    // Which lines fold into one (`LogEntry::repeat`) depends on when the
    // log was last sent, since a line sent is never changed: the live
    // game sends once per pump and the replay after every input. So the
    // two are held to the same events, in the same order, each counted
    // as often as it happened.
    let unfolded = |log: &[LogEntry]| -> Vec<(u32, String)> {
        log.iter()
            .flat_map(|entry| {
                let name = format!("{:?}", entry.event);
                let name = name.split([' ', '{']).next().unwrap_or_default().to_owned();
                let once = (entry.turn, format!("{:?} {name}", entry.phase));
                std::iter::repeat_n(once, entry.repeat as usize)
            })
            .collect()
    };
    let (live_events, replayed_events) = (unfolded(&game), unfolded(projected_log));
    let part = live_events
        .iter()
        .zip(&replayed_events)
        .position(|(a, b)| a != b);
    assert_eq!(part, None, "the logs part at event {part:?}");
    assert_eq!(live_events.len(), replayed_events.len());
    let same = game.iter().filter(|e| projected_log.contains(e)).count();
    assert!(
        same * 10 > game.len() * 9,
        "{same} of {} lines are the same",
        game.len()
    );
}

/// What a seat is sent is what the projection hands on, at every input: the
/// whole reason the projection is built from the live path's own functions.
#[test]
fn the_seat_projection_is_what_the_live_seat_was_sent() {
    for (leave_at, ends_in_the_game) in [(Some(60), false), (None, true)] {
        let live = live(3, leave_at);
        let written = lines(&live.record);
        assert_eq!(
            written
                .iter()
                .filter(|line| matches!(line, Line::Input { .. }))
                .count(),
            live.inputs
        );
        assert!(
            written.iter().any(|l| matches!(l, Line::End { .. })),
            "the game ended"
        );
        let (projection, moments) = whole(&live.record, Perspective::Seat(ME));
        assert_eq!(projection.stop, Stop::End);
        assert_eq!(moments.len(), live.inputs + 1);

        let mut compared = 0;
        let mut log = Vec::new();
        let mut last = 0;
        for (inputs, env) in &live.sent {
            let Some(v1::envelope::Msg::StateDelta(delta)) = &env.msg else {
                continue;
            };
            let sent: PlayerView = serde_json::from_slice(&delta.view_json).expect("a view");
            // The projection's view at this input count: the last it told
            // at or before it.
            let (at, projected) = moments[..=*inputs]
                .iter()
                .enumerate()
                .rev()
                .find_map(|(at, moment)| seat_view(moment).map(|view| (at, view)))
                .expect("the opening tells a view");
            assert_eq!(
                with_seq_of(projected, &sent),
                sent,
                "after {inputs} inputs the seat was sent another view than moment {at} tells"
            );
            compared += 1;
            last = last.max(*inputs);
            if !delta.log_json.is_empty() {
                append(
                    &mut log,
                    serde_json::from_slice(&delta.log_json).expect("a tail"),
                );
            }
        }
        assert!(compared > 30, "only {compared} views compared");

        // The log the seat was sent is the log the projection tells, up to
        // the last thing the seat was sent.
        let mut projected_log = Vec::new();
        for moment in &moments[..=last] {
            for tail in tails(moment) {
                append(&mut projected_log, tail);
            }
        }
        assert!(log.len() > 20, "the log is the game's: {} lines", log.len());
        same_log(log, &projected_log);

        if ends_in_the_game {
            assert_eq!(last, live.inputs, "the seat was there to the end");
            let sent_over: Vec<&[u8]> = live
                .sent
                .iter()
                .filter_map(|(_, env)| match &env.msg {
                    Some(v1::envelope::Msg::ChoiceRequest(c))
                        if c.pending_json.starts_with(br#"{"GameOver""#) =>
                    {
                        Some(c.pending_json.as_slice())
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(sent_over.len(), 1, "the live seat was told the end once");
            let projected_over = game_over(moments.last().unwrap()).expect("the end is told");
            assert_eq!(sent_over[0], projected_over);
        } else {
            assert!(last < live.inputs, "the player left, so was told no more");
        }
    }
}

/// Another seat's hand and every library reach a seat as counts and nothing
/// else, in every view the projection hands on.
#[test]
fn a_seat_projection_never_holds_another_hand() {
    let record = played(3).take_record();
    for (me, them) in [(ME, THEM), (THEM, ME)] {
        let mut shadow = Shadow::new(&record);
        let mut seen_views = 0;
        let mut hands_checked = 0;
        let mut at = 0;
        project(
            &record,
            names(),
            Perspective::Seat(me),
            Verify::Stop,
            |moment| {
                shadow.at(moment.input);
                let state = shadow.engine.state();
                let Some(view) = seat_view(&moment) else {
                    return ControlFlow::Continue(());
                };
                at += 1;
                if at % 10 != 1 {
                    return ControlFlow::Continue(());
                }
                seen_views += 1;
                let theirs = hand_ids(state, them);
                assert_eq!(
                    view.seat(them).map(|s| s.hand_count),
                    Some(u32::try_from(theirs.len()).unwrap()),
                    "{me:?} is told a count at {:?}",
                    moment.input
                );
                assert_eq!(
                    view.hand.iter().map(|c| c.id).collect::<Vec<_>>(),
                    hand_ids(state, me),
                    "{me:?} is told its own hand"
                );
                assert!(
                    view.shared_hands
                        .iter()
                        .all(|hand| hand.player != them || hand.public),
                    "a hand nobody showed is shared at {:?}: {:?}",
                    moment.input,
                    view.shared_hands
                );
                assert!(view.hand_shared_with.is_empty() && view.hand_requested.is_empty());
                let on_table = ids_on_the_table(&view);
                // What a seat is looking at is what the engine is asking it
                // about, and only then.
                assert!(
                    view.looking_at.is_empty() || shadow.engine.pending_for(me).is_some(),
                    "{me:?} is shown cards it is not asked about at {:?}",
                    moment.input
                );
                for id in &theirs {
                    hands_checked += 1;
                    assert!(
                        !on_table.contains(id) && !view.looking_at.iter().any(|o| o.id == *id),
                        "{me:?} is told the card {id:?} from {them:?}'s hand at {:?}",
                        moment.input
                    );
                }
                for owner in [me, them] {
                    for id in state.zones.list(ZoneLocation::Library(owner)) {
                        assert!(
                            !on_table.contains(id),
                            "{owner:?}'s library card {id:?} is on {me:?}'s table at {:?}",
                            moment.input
                        );
                    }
                    assert_eq!(
                        view.seat(owner).map(|s| s.library_count),
                        Some(
                            u32::try_from(state.zones.list(ZoneLocation::Library(owner)).len())
                                .unwrap()
                        )
                    );
                }
                ControlFlow::Continue(())
            },
        )
        .expect("projects");
        assert!(seen_views > 10, "{seen_views} views checked");
        assert!(hands_checked > 30, "{hands_checked} hand cards checked");
    }
}

/// A spectator is told the public table: a type with no hand to leak, an
/// payload with a printing only for what it saw.
#[test]
fn the_public_projection_has_no_hand_and_no_decklist() {
    let record = played(3).take_record();
    let (projection, moments) = whole(&record, Perspective::Public);
    assert_eq!(projection.stop, Stop::End);

    let first = statics(&moments[0]).expect("the opening payload leads");
    assert!(matches!(
        moments[0].frames.first().and_then(|e| e.msg.as_ref()),
        Some(v1::envelope::Msg::GameStatic(_))
    ));
    assert!(!first.prints.is_empty(), "the game has printings");

    let given = |statics: &GameStatic| -> std::collections::BTreeSet<u16> {
        statics
            .prints
            .iter()
            .enumerate()
            .filter(|(_, p)| p.is_some())
            .map(|(i, _)| u16::try_from(i).unwrap())
            .collect()
    };
    let mut views = 0;
    let mut shown = std::collections::BTreeSet::new();
    let mut last_statics = first;
    for moment in &moments {
        for delta in deltas(moment) {
            views += 1;
            let raw: serde_json::Value = serde_json::from_slice(&delta.view_json).unwrap();
            let view: SpectatorView =
                serde_json::from_value(raw.clone()).expect("a spectator view");
            assert_eq!(
                serde_json::to_value(&view).unwrap(),
                raw,
                "the view carries nothing a spectator's type has no field for"
            );
            for key in [
                "hand",
                "looking_at",
                "shared_hands",
                "hand_requests",
                "owed",
            ] {
                assert!(
                    raw.get(key).is_none(),
                    "a seat's field {key} is in the view"
                );
            }
            shown.extend(
                view.into_player_view(ME)
                    .prints()
                    .map(baylee_core::ids::PrintRef::get),
            );
        }
        for tail in tails(moment) {
            shown.extend(tail.prints().map(baylee_core::ids::PrintRef::get));
        }
        // A printing is in the payload only once the spectator has seen a
        // card of it, in the view or the log of this very moment or before.
        if let Some(statics) = statics(moment) {
            assert!(
                given(&statics).is_subset(&shown),
                "a printing no one saw went to the spectator at {:?}",
                moment.input
            );
            last_statics = statics;
        }
    }
    assert!(views > 50);
    // At the end the payload holds what was seen, and the rest of both
    // decklists stays out of it.
    let given = given(&last_statics);
    assert!(!given.is_empty());
    assert_eq!(given, shown, "the print table is what was seen, no more");
    assert!(
        given.len() < last_statics.prints.len(),
        "the whole decklist went out to the spectator"
    );
}

/// All hands: the other seat's hand beside the seat's own, true to the
/// engine's at that input, with its printings in the payload by then.
#[test]
fn the_all_hands_projection_shows_the_other_hand_as_the_engine_held_it() {
    let record = played(3).take_record();
    let mut shadow = Shadow::new(&record);
    let mut payload: Option<GameStatic> = None;
    let mut checked = 0;
    let mut non_empty = 0;
    project(
        &record,
        names(),
        Perspective::AllHands(ME),
        Verify::Stop,
        |moment| {
            let state = shadow.at(moment.input);
            if let Some(statics) = statics(&moment) {
                payload = Some(statics);
            }
            let Some(view) = seat_view(&moment) else {
                return ControlFlow::Continue(());
            };
            let entry = view
                .shared_hands
                .iter()
                .find(|hand| hand.player == THEM)
                .unwrap_or_else(|| panic!("no hand shown at {:?}", moment.input));
            assert_eq!(
                entry.cards.iter().map(|c| c.card.index).collect::<Vec<_>>(),
                hand_cards(state, THEM),
                "at {:?}",
                moment.input
            );
            assert_eq!(
                entry.cards.iter().map(|c| c.id).collect::<Vec<_>>(),
                hand_ids(state, THEM)
            );
            checked += 1;
            non_empty += usize::from(!entry.cards.is_empty());
            let payload = payload.as_ref().expect("the payload comes first");
            for card in &entry.cards {
                assert!(
                    payload
                        .prints
                        .get(card.card.print.get() as usize)
                        .is_some_and(Option::is_some),
                    "{:?}'s printing is not in the payload at {:?}",
                    card.name,
                    moment.input
                );
            }
            assert_eq!(
                view.hand.iter().map(|c| c.id).collect::<Vec<_>>(),
                hand_ids(state, ME)
            );
            ControlFlow::Continue(())
        },
    )
    .expect("projects");
    assert!(checked > 30 && non_empty > 20, "{checked}, {non_empty}");

    // And the seat projection of the same record shows no such hand: the
    // difference is the perspective, not the record.
    let (_, seat) = whole(&record, Perspective::Seat(ME));
    assert!(seat.iter().filter_map(seat_view).all(|view| {
        view.shared_hands
            .iter()
            .all(|hand| hand.player != THEM || hand.public)
    }));
}

// ---- Shown: a seat's own cards by consent, the public table otherwise

/// A set of seats.
fn seats(of: &[PlayerId]) -> SeatSet {
    let mut set = SeatSet::new();
    for &seat in of {
        set.insert(seat);
    }
    set
}

/// Every number under an `id` key and every one under an `index` key
/// anywhere in a JSON value: the objects and the cards a payload names.
fn names_in(value: &serde_json::Value, ids: &mut BTreeSet<u64>, cards: &mut BTreeSet<u64>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, inner) in map {
                match (key.as_str(), inner.as_u64()) {
                    ("id", Some(n)) => {
                        ids.insert(n);
                    }
                    ("index", Some(n)) => {
                        cards.insert(n);
                    }
                    _ => {}
                }
                names_in(inner, ids, cards);
            }
        }
        serde_json::Value::Array(items) => {
            for inner in items {
                names_in(inner, ids, cards);
            }
        }
        _ => {}
    }
}

/// The objects and cards the views and log tails of `moment` name.
fn named_by(moment: &Moment, ids: &mut BTreeSet<u64>, cards: &mut BTreeSet<u64>) {
    for delta in deltas(moment) {
        for bytes in [&delta.view_json, &delta.log_json] {
            if !bytes.is_empty() {
                let json: serde_json::Value = serde_json::from_slice(bytes).expect("json");
                names_in(&json, ids, cards);
            }
        }
    }
}

/// The printings a payload holds.
fn printings(statics: &GameStatic) -> BTreeSet<usize> {
    statics
        .prints
        .iter()
        .enumerate()
        .filter(|(_, p)| p.is_some())
        .map(|(i, _)| i)
        .collect()
}

/// A seat that does not share its own cards is the public table drawn from
/// it, at every input: no hand, nothing looked at, the spectators' log and
/// the spectators' print table, and nothing of the seat's hand anywhere.
#[test]
#[allow(clippy::too_many_lines)] // one scenario, read top to bottom
fn a_seat_shown_without_its_own_cards_is_the_public_table_drawn_from_it() {
    let record = played(3).take_record();
    let (_, public) = whole(&record, Perspective::Public);
    for (drawn, other) in [(THEM, ME), (ME, THEM)] {
        let (projection, moments) = whole(
            &record,
            Perspective::Shown {
                seat: drawn,
                own: false,
                hands: SeatSet::new(),
            },
        );
        assert_eq!(projection.stop, Stop::End);
        assert_eq!(moments.len(), public.len());

        let mut shadow = Shadow::new(&record);
        let (mut shown_log, mut public_log) = (Vec::new(), Vec::new());
        let (mut shown_ids, mut shown_cards) = (BTreeSet::new(), BTreeSet::new());
        let (mut public_ids, mut public_cards) = (BTreeSet::new(), BTreeSet::new());
        let (mut shown_statics, mut public_statics): (Option<GameStatic>, Option<GameStatic>) =
            (None, None);
        let (mut views, mut secret_cards, mut hidden_ids) = (0, 0, 0);
        for (moment, spectated) in moments.iter().zip(&public) {
            let state = shadow.at(moment.input);
            assert_eq!(moment.input, spectated.input);
            assert_eq!(
                moment.frames.is_empty(),
                spectated.frames.is_empty(),
                "the two streams tell at the same inputs ({:?})",
                moment.input
            );
            // Only the kinds of frame a spectator is told; a question only
            // as the end.
            for env in &moment.frames {
                match &env.msg {
                    Some(v1::envelope::Msg::GameStatic(_) | v1::envelope::Msg::StateDelta(_)) => {}
                    Some(v1::envelope::Msg::ChoiceRequest(choice)) => assert!(
                        choice.pending_json.starts_with(br#"{"GameOver""#),
                        "a question went to a seat shown no cards at {:?}",
                        moment.input
                    ),
                    other => panic!("an unexpected frame at {:?}: {other:?}", moment.input),
                }
            }

            // The log is the spectators' log, tail for tail.
            assert_eq!(
                tails(moment),
                tails(spectated),
                "at {:?} the log is not the spectators'",
                moment.input
            );
            for tail in tails(moment) {
                append(&mut shown_log, tail);
            }
            for tail in tails(spectated) {
                append(&mut public_log, tail);
            }
            assert_eq!(shown_log, public_log);

            // The print table is the spectators', drawn from the seat.
            assert_eq!(statics(moment).is_some(), statics(spectated).is_some());
            if let Some(statics) = statics(moment) {
                assert_eq!(statics.your_seat, drawn);
                shown_statics = Some(statics);
            }
            if let Some(statics) = statics(spectated) {
                public_statics = Some(statics);
            }
            if let (Some(shown), Some(public)) = (&shown_statics, &public_statics) {
                assert_eq!(
                    printings(shown),
                    printings(public),
                    "a printing the spectators lack at {:?}",
                    moment.input
                );
                assert!(printings(shown).len() < shown.prints.len(), "the decklist");
            }

            named_by(moment, &mut shown_ids, &mut shown_cards);
            named_by(spectated, &mut public_ids, &mut public_cards);
            assert!(
                shown_ids.is_subset(&public_ids) && shown_cards.is_subset(&public_cards),
                "the shown stream names an object or card the spectators were not shown at {:?}",
                moment.input
            );

            let Some(view) = seat_view(moment) else {
                continue;
            };
            views += 1;
            let raw = deltas(spectated).last().copied().expect("a view");
            let spectator: SpectatorView = serde_json::from_slice(&raw.view_json).unwrap();
            assert_eq!(
                with_seq_of(spectator.into_player_view(drawn), &view),
                view,
                "the view is the spectators' table drawn from {drawn:?} at {:?}",
                moment.input
            );
            assert_eq!(view.seat, drawn);
            assert!(view.hand.is_empty() && view.looking_at.is_empty());
            assert!(view.hand_shared_with.is_empty() && view.hand_requested.is_empty());
            assert!(view.hand_requests.is_empty() && view.owed.is_none());
            assert!(
                view.shared_hands.iter().all(|hand| hand.public),
                "a private hand in {:?}",
                view.shared_hands
            );

            // The seat's hand, from the engine: not an object the views
            // name, not a card nobody saw.
            let hand = hand_ids(state, drawn);
            let on_table = ids_on_the_table(&view);
            for id in &hand {
                hidden_ids += 1;
                assert!(
                    !on_table.contains(id)
                        && !view.library_tops.iter().any(|o| o.id == *id)
                        && !view
                            .shared_hands
                            .iter()
                            .any(|h| h.cards.iter().any(|c| c.id == *id)),
                    "{drawn:?}'s hand card {id:?} is in the view at {:?}",
                    moment.input
                );
            }
            for index in hand_cards(state, drawn) {
                if !public_cards.contains(&u64::from(index.get())) {
                    secret_cards += 1;
                    assert!(
                        !shown_cards.contains(&u64::from(index.get())),
                        "{drawn:?}'s card {index:?}, which no spectator saw, is in the stream at {:?}",
                        moment.input
                    );
                }
            }
            // And the other seat is only counts too.
            assert_eq!(
                view.seat(other).map(|s| s.hand_count),
                Some(u32::try_from(hand_ids(state, other).len()).unwrap())
            );
        }
        assert!(views > 50, "{views} views");
        assert!(
            hidden_ids > 100,
            "{hidden_ids} hand cards held against the views"
        );
        assert!(
            secret_cards > 100,
            "{secret_cards} hand cards no spectator saw"
        );
    }
}

/// `Seat(p)` is `Shown { p, own, no hands }`, and `AllHands(p)` is the same
/// with every other seat's hand: frame for frame, for a human's chair and for
/// the house's.
#[test]
fn shown_with_its_own_cards_is_the_seat_projection_and_all_hands() {
    let record = played(3).take_record();
    for (drawn, other) in [(ME, THEM), (THEM, ME)] {
        let frames = |perspective| -> Vec<(Option<u64>, Vec<Envelope>)> {
            whole(&record, perspective)
                .1
                .into_iter()
                .map(|moment| (moment.input, moment.frames))
                .collect()
        };
        let seat = frames(Perspective::Seat(drawn));
        assert!(seat.iter().filter(|(_, f)| !f.is_empty()).count() > 50);
        assert_eq!(
            frames(Perspective::Shown {
                seat: drawn,
                own: true,
                hands: SeatSet::new(),
            }),
            seat,
            "{drawn:?}: own, no hands is Seat"
        );
        // Naming the drawn seat's own hand among the hands adds nothing.
        assert_eq!(
            frames(Perspective::Shown {
                seat: drawn,
                own: true,
                hands: seats(&[drawn]),
            }),
            seat,
            "{drawn:?}: its own hand among the hands"
        );
        let all = frames(Perspective::AllHands(drawn));
        assert_ne!(all, seat, "the other hand makes a difference");
        assert_eq!(
            frames(Perspective::Shown {
                seat: drawn,
                own: true,
                hands: seats(&[other]),
            }),
            all,
            "{drawn:?}: own and the other hand is AllHands"
        );
    }
}

/// A seat's own cards withheld and another's hand shown: that hand is the
/// engine's at every input, with its printings earned, and the seat's own is
/// nowhere.
#[test]
fn a_hand_is_shown_beside_a_table_that_withholds_the_seats_own() {
    let record = played(3).take_record();
    for (drawn, other) in [(ME, THEM), (THEM, ME)] {
        let mut shadow = Shadow::new(&record);
        let mut payload: Option<GameStatic> = None;
        let (mut checked, mut non_empty) = (0, 0);
        let (mut ids, mut cards) = (BTreeSet::new(), BTreeSet::new());
        project(
            &record,
            names(),
            Perspective::Shown {
                seat: drawn,
                own: false,
                hands: seats(&[other]),
            },
            Verify::Stop,
            |moment| {
                let state = shadow.at(moment.input);
                if let Some(statics) = statics(&moment) {
                    assert_eq!(statics.your_seat, drawn);
                    payload = Some(statics);
                }
                named_by(&moment, &mut ids, &mut cards);
                let Some(view) = seat_view(&moment) else {
                    return ControlFlow::Continue(());
                };
                assert!(view.hand.is_empty() && view.looking_at.is_empty());
                let shown: Vec<_> = view
                    .shared_hands
                    .iter()
                    .filter(|hand| hand.player == other)
                    .collect();
                assert_eq!(
                    shown.len(),
                    1,
                    "one entry for the hand at {:?}",
                    moment.input
                );
                assert!(!shown[0].public, "a consented hand is not a public one");
                assert_eq!(
                    shown[0].cards.iter().map(|c| c.id).collect::<Vec<_>>(),
                    hand_ids(state, other)
                );
                assert_eq!(
                    shown[0]
                        .cards
                        .iter()
                        .map(|c| c.card.index)
                        .collect::<Vec<_>>(),
                    hand_cards(state, other),
                    "at {:?}",
                    moment.input
                );
                checked += 1;
                non_empty += usize::from(!shown[0].cards.is_empty());
                let payload = payload.as_ref().expect("the payload comes first");
                for card in &shown[0].cards {
                    assert!(
                        payload
                            .prints
                            .get(card.card.print.get() as usize)
                            .is_some_and(Option::is_some),
                        "{:?}'s printing is not in the payload at {:?}",
                        card.name,
                        moment.input
                    );
                }
                // The seat's own hand: no entry for it, no id of it.
                assert!(view.shared_hands.iter().all(|hand| hand.player != drawn));
                let mine = hand_ids(state, drawn);
                let on_table = ids_on_the_table(&view);
                for id in &mine {
                    assert!(
                        !on_table.contains(id),
                        "{drawn:?}'s own hand card {id:?} is shown at {:?}",
                        moment.input
                    );
                }
                ControlFlow::Continue(())
            },
        )
        .expect("projects");
        assert!(checked > 30 && non_empty > 20, "{checked}, {non_empty}");
    }
}

// ---- how far a moment is the recorded game

/// The record's own hashes make every moment exact; a record that carries
/// none still is the game when its public digests say so.
#[test]
fn an_untouched_record_is_exact_to_the_end_and_one_without_hashes_is_seen() {
    let record = played(3).take_record();
    let written = lines(&record);
    let inputs = input_positions(&written);
    let last = n_of(&written[*inputs.last().unwrap()]);

    let (projection, moments) = whole(&record, Perspective::Seat(ME));
    assert_eq!(projection.stop, Stop::End);
    assert_eq!(projection.inputs, inputs.len() as u64);
    assert_eq!(projection.exact_to, Some(last));
    assert_eq!(projection.seen_to, Some(last));
    assert_eq!(projection.diverged, None);
    assert!(moments.iter().all(|m| m.check == Check::Exact));

    let mut blanked = written.clone();
    for line in &mut blanked {
        if matches!(line, Line::Header { .. } | Line::Input { .. }) {
            set_hash(line, WRONG);
        }
    }
    let (projection, moments) = whole(&encode(&blanked), Perspective::Seat(ME));
    assert_eq!(projection.stop, Stop::End);
    assert_eq!(projection.inputs, inputs.len() as u64);
    assert_eq!(projection.exact_to, None);
    assert_eq!(projection.seen_to, Some(last));
    assert_eq!(projection.diverged, None);
    assert_eq!(moments.len(), inputs.len() + 1);
    assert!(moments.iter().all(|m| m.check == Check::Seen));
}

/// One wrong hash beside a right digest is one moment known by its digest;
/// the next input's hash is right again and the replay is exact once more.
#[test]
fn a_wrong_hash_with_the_right_digest_is_seen_for_that_input_only() {
    let record = played(3).take_record();
    let mut written = lines(&record);
    let inputs = input_positions(&written);
    let k = inputs.len() / 2;
    set_hash(&mut written[inputs[k]], WRONG);
    let (projection, moments) = whole(&encode(&written), Perspective::Seat(ME));
    assert_eq!(projection.stop, Stop::End);
    assert_eq!(projection.diverged, None);
    // Moment `i` is after input `i`; moment 0 is the opening.
    for (i, moment) in moments.iter().enumerate() {
        let want = if i == k + 1 {
            Check::Seen
        } else {
            Check::Exact
        };
        assert_eq!(moment.check, want, "moment {i}");
    }
    let last = n_of(&written[*inputs.last().unwrap()]);
    assert_eq!(projection.exact_to, Some(last));
    assert_eq!(projection.seen_to, Some(last));
}

/// A digest and a hash that both disagree is a divergence: it stops the
/// projection or is noted, and the moments before it stand either way.
#[test]
fn a_digest_and_a_hash_that_both_differ_are_a_divergence_found_by_the_digest() {
    let record = played(3).take_record();
    let mut written = lines(&record);
    let inputs = input_positions(&written);
    let k = inputs.len() / 2;
    let at = n_of(&written[inputs[k]]);
    let previous = before(&written, &inputs, k);
    set_hash(&mut written[inputs[k]], WRONG);
    set_seen(&mut written[inputs[k]], Some(OTHER_SEEN));
    let record = encode(&written);
    let divergence = Divergence {
        input: Some(at),
        by: Basis::Seen,
    };

    // Stopped: everything before stands, and the input it stopped at is not
    // handed on.
    let (projection, moments) = whole(&record, Perspective::Seat(ME));
    assert_eq!(projection.stop, Stop::Diverged { input: Some(at) });
    assert_eq!(projection.diverged, Some(divergence));
    assert_eq!(projection.exact_to, previous);
    assert_eq!(projection.seen_to, previous);
    assert_eq!(projection.inputs, k as u64 + 1);
    assert_eq!(moments.len(), k + 1, "the opening and {k} inputs");
    assert_eq!(moments.last().unwrap().input, previous);
    assert!(moments.iter().all(|m| m.check == Check::Exact));

    // Noted: the projection goes to the game's end, unverified from there.
    let (projection, moments) = run(&record, Perspective::Seat(ME), Verify::Note).unwrap();
    assert_eq!(projection.stop, Stop::End);
    assert_eq!(projection.diverged, Some(divergence));
    assert_eq!(projection.exact_to, previous);
    assert_eq!(projection.seen_to, previous);
    assert_eq!(projection.inputs, inputs.len() as u64);
    assert_eq!(moments.len(), inputs.len() + 1);
    for (i, moment) in moments.iter().enumerate() {
        let want = if i <= k {
            Check::Exact
        } else {
            Check::Unverified
        };
        assert_eq!(moment.check, want, "moment {i}");
    }
    // What the player asked to see is still the game the record plays: the
    // frames after the divergence are the ones an untouched record gives.
    let (_, untouched) = whole(
        &encode(&lines(&played(3).take_record())),
        Perspective::Seat(ME),
    );
    for (a, b) in moments.iter().zip(&untouched) {
        assert_eq!(a.frames, b.frames);
    }
}

/// A record from before digests has only its hashes to be held to: a hash
/// that differs is a divergence by the hash, and one at the opening stops the
/// projection before a single frame.
#[test]
fn a_record_with_no_digests_and_wrong_hashes_diverges_by_the_hash_before_any_frame() {
    let mut written = lines(&played(3).take_record());
    for line in &mut written {
        if matches!(line, Line::Header { .. } | Line::Input { .. }) {
            set_seen(line, None);
            set_hash(line, WRONG);
        }
    }
    let record = encode(&written);
    let divergence = Divergence {
        input: None,
        by: Basis::Hash,
    };

    let (projection, moments) = whole(&record, Perspective::Seat(ME));
    assert_eq!(projection.stop, Stop::Diverged { input: None });
    assert_eq!(projection.diverged, Some(divergence));
    assert_eq!(projection.inputs, 0);
    assert_eq!(projection.exact_to, None);
    assert_eq!(projection.seen_to, None);
    assert!(moments.is_empty(), "not a frame of an unknown game");

    let (projection, moments) = run(&record, Perspective::Public, Verify::Note).unwrap();
    assert_eq!(projection.stop, Stop::End);
    assert_eq!(projection.diverged, Some(divergence));
    assert_eq!(projection.exact_to, None);
    assert_eq!(projection.seen_to, None);
    assert!(moments.len() > 10);
    assert!(moments.iter().all(|m| m.check == Check::Unverified));
}

/// With digests removed, a hash wrong from the middle on is a divergence by
/// the hash at that input, not by a digest the record does not hold.
#[test]
fn a_hash_wrong_where_the_record_has_no_digest_is_found_by_the_hash() {
    let mut written = lines(&played(3).take_record());
    let inputs = input_positions(&written);
    let k = inputs.len() / 3;
    let at = n_of(&written[inputs[k]]);
    for line in &mut written {
        if matches!(line, Line::Header { .. } | Line::Input { .. }) {
            set_seen(line, None);
        }
    }
    set_hash(&mut written[inputs[k]], WRONG);
    let (projection, moments) = whole(&encode(&written), Perspective::Seat(ME));
    assert_eq!(projection.stop, Stop::Diverged { input: Some(at) });
    assert_eq!(
        projection.diverged,
        Some(Divergence {
            input: Some(at),
            by: Basis::Hash
        })
    );
    assert_eq!(projection.seen_to, None, "the record holds no digest");
    assert_eq!(projection.exact_to, before(&written, &inputs, k));
    assert_eq!(moments.len(), k + 1);
}

// ---- refused, cut, cancelled

/// `written` with the input at position `k` among its inputs given to the
/// other of the two seats, who is not the one asked.
fn answered_by_the_wrong_seat(written: &mut [Line], k: usize) {
    let inputs = input_positions(written);
    let Line::Input { seat, .. } = &mut written[inputs[k]] else {
        unreachable!()
    };
    *seat = 1 - *seat;
}

#[test]
fn an_input_this_build_refuses_stops_the_projection_before_its_moment() {
    let mut written = lines(&played(3).take_record());
    let inputs = input_positions(&written);
    let k = inputs.len() / 2;
    let at = n_of(&written[inputs[k]]);
    let previous = before(&written, &inputs, k);
    answered_by_the_wrong_seat(&mut written, k);
    for perspective in [Perspective::Seat(ME), Perspective::Public] {
        let (projection, moments) = whole(&encode(&written), perspective);
        assert_eq!(
            projection.stop,
            Stop::Refused { input: at },
            "{perspective:?}"
        );
        assert_eq!(projection.inputs, k as u64);
        assert_eq!(projection.diverged, None);
        assert_eq!(moments.len(), k + 1, "the refused input has no moment");
        assert_eq!(moments.last().unwrap().input, previous);
        assert_eq!(projection.exact_to, previous);
    }
}

/// A record that names a seat the table lacks is refused like any other
/// input the engine will not take, and does not take the process down.
#[test]
fn an_input_for_a_seat_the_table_lacks_is_refused() {
    let mut written = lines(&played(3).take_record());
    let inputs = input_positions(&written);
    let k = inputs.len() / 2;
    let at = n_of(&written[inputs[k]]);
    let Line::Input { seat, .. } = &mut written[inputs[k]] else {
        unreachable!()
    };
    *seat = 7;
    let (projection, _) = whole(&encode(&written), Perspective::Seat(ME));
    assert_eq!(projection.stop, Stop::Refused { input: at });
}

#[test]
fn a_record_without_its_end_is_cut_and_never_tells_the_end() {
    let record = played(5).take_record();
    // Cut mid-line: the part before the cut is played, the half line is not.
    let cut = &record[..record.len() * 2 / 3];
    let whole_lines = cut.iter().rposition(|&b| b == b'\n').expect("whole lines");
    let inputs = lines(&cut[..=whole_lines])
        .iter()
        .filter(|line| matches!(line, Line::Input { .. }))
        .count();
    assert!(inputs > 20);
    for perspective in [
        Perspective::Seat(ME),
        Perspective::Public,
        Perspective::AllHands(ME),
    ] {
        let (projection, moments) = whole(cut, perspective);
        assert_eq!(projection.stop, Stop::Cut, "{perspective:?}");
        assert_eq!(projection.inputs, inputs as u64);
        assert_eq!(moments.len(), inputs + 1);
        assert!(
            moments.iter().all(|m| game_over(m).is_none()),
            "{perspective:?}: a game that did not end was told to have ended"
        );
        assert!(moments.iter().all(|m| m.check == Check::Exact));
    }

    // A header alone is the game before its first input.
    let header = encode(&lines(&record)[..1]);
    let (projection, moments) = whole(&header, Perspective::Seat(ME));
    assert_eq!(projection.stop, Stop::Cut);
    assert_eq!(projection.inputs, 0);
    assert_eq!(moments.len(), 1);
    assert_eq!(moments[0].input, None);
    assert_eq!(moments[0].check, Check::Exact);
}

#[test]
fn a_sink_that_breaks_cancels_the_projection_where_it_stood() {
    let record = played(3).take_record();
    for (break_at, inputs) in [(5_usize, 4_u64), (1, 0)] {
        let mut calls = 0;
        let projection = project(
            &record,
            names(),
            Perspective::Seat(ME),
            Verify::Stop,
            |_| {
                calls += 1;
                if calls == break_at {
                    ControlFlow::Break(())
                } else {
                    ControlFlow::Continue(())
                }
            },
        )
        .expect("projects");
        assert_eq!(projection.stop, Stop::Cancelled);
        assert_eq!(calls, break_at, "no moment is handed on after a break");
        assert_eq!(projection.inputs, inputs);
    }
}

/// What a record that is not a record, a seat the table lacks and an end the
/// replayed game never reached are: errors, not stops.
#[test]
fn a_projection_that_cannot_begin_or_cannot_agree_on_the_end_is_an_error() {
    let record = played(3).take_record();
    let written = lines(&record);
    for perspective in [
        Perspective::Seat(PlayerId::new(7)),
        Perspective::AllHands(PlayerId::new(2)),
    ] {
        assert!(matches!(
            run(&record, perspective, Verify::Stop),
            Err(ReplayError::NoSuchSeat { .. })
        ));
    }
    assert!(matches!(
        run(b"", Perspective::Public, Verify::Stop),
        Err(ReplayError::NoHeader)
    ));
    let end = written
        .iter()
        .find(|line| matches!(line, Line::End { .. }))
        .cloned()
        .expect("an end");
    let Line::End { n, .. } = end else {
        unreachable!()
    };
    let mut early = written[..12].to_vec();
    early.push(end);
    assert!(matches!(
        run(&encode(&early), Perspective::Seat(ME), Verify::Stop),
        Err(ReplayError::NotOver { n: at }) if at == n
    ));
}

// ---- what a projection is made of

/// A moment that changed nothing visible tells nothing: no two views the
/// projection tells in a row are one view, and every input has its moment.
#[test]
fn a_moment_that_changed_nothing_tells_nothing() {
    let record = played(3).take_record();
    let written = lines(&record);
    let inputs = input_positions(&written);
    for perspective in [
        Perspective::Seat(ME),
        Perspective::Seat(THEM),
        Perspective::AllHands(ME),
    ] {
        let (projection, moments) = whole(&record, perspective);
        assert_eq!(moments.len(), inputs.len() + 1, "{perspective:?}");
        assert_eq!(projection.inputs, inputs.len() as u64);
        // Moment `i` is after the `i`-th input.
        assert_eq!(moments[0].input, None);
        for (moment, &position) in moments[1..].iter().zip(&inputs) {
            assert_eq!(moment.input, Some(n_of(&written[position])));
        }
        let quiet = moments.iter().filter(|m| m.frames.is_empty()).count();
        assert!(
            quiet > 10,
            "{perspective:?}: the dedupe saved {quiet} moments"
        );

        let told: Vec<&Moment> = moments.iter().filter(|m| !m.frames.is_empty()).collect();
        for pair in told.windows(2) {
            let [earlier, later] = pair else {
                unreachable!()
            };
            let (earlier_view, later_view) = (seat_view(earlier), seat_view(later));
            let (Some(a), Some(b)) = (earlier_view, later_view) else {
                panic!("a moment told a frame and no view");
            };
            let same_view = a == with_seq_of(b, &a);
            let news = tails(later).iter().any(|tail| !tail.entries.is_empty())
                || game_over(later).is_some();
            assert!(
                !same_view || news,
                "{perspective:?}: moments {:?} and {:?} tell one view",
                earlier.input,
                later.input
            );
        }
    }

    let (_, moments) = whole(&record, Perspective::Public);
    assert_eq!(moments.len(), inputs.len() + 1);
    let told: Vec<&Moment> = moments.iter().filter(|m| !m.frames.is_empty()).collect();
    assert!(
        told.len() + 10 < moments.len(),
        "the spectator's stream is deduped"
    );
    for pair in told.windows(2) {
        let [earlier, later] = pair else {
            unreachable!()
        };
        let view = |m: &Moment| -> SpectatorView {
            serde_json::from_slice(&deltas(m).last().expect("a view").view_json).unwrap()
        };
        let (a, mut b) = (view(earlier), view(later));
        b.seq = a.seq;
        let news =
            tails(later).iter().any(|tail| !tail.entries.is_empty()) || game_over(later).is_some();
        assert!(
            a != b || news,
            "moments {:?} and {:?} tell one view",
            earlier.input,
            later.input
        );
    }
}

/// Nothing in a projection depends on when, where or how often it is made.
#[test]
fn a_projection_is_the_same_every_time() {
    let record = played(3).take_record();
    for perspective in [
        Perspective::Seat(ME),
        Perspective::Seat(THEM),
        Perspective::Public,
        Perspective::AllHands(ME),
    ] {
        let (a, first) = whole(&record, perspective);
        let (b, second) = whole(&record, perspective);
        assert_eq!(a, b);
        assert_eq!(first.len(), second.len());
        let mut frames = 0;
        for (x, y) in first.iter().zip(&second) {
            assert_eq!((x.input, x.at, x.check), (y.input, y.at, y.check));
            assert_eq!(x.frames.len(), y.frames.len());
            frames += x.frames.len();
            for (ex, ey) in x.frames.iter().zip(&y.frames) {
                // The wire form: what the engine link would carry.
                assert_eq!(ex, ey, "{perspective:?} at {:?}", x.input);
                assert_eq!(format!("{ex:?}"), format!("{ey:?}"));
            }
        }
        assert!(frames > 50, "{perspective:?}: {frames} frames");
    }
}

// ---- the list's summary

#[test]
fn a_summary_reads_a_record_without_playing_it() {
    let record = played(3).take_record();
    let written = lines(&record);
    let Some(Line::End {
        winners,
        reason,
        turn,
        at: ended_at,
        ..
    }) = written.iter().find(|line| matches!(line, Line::End { .. }))
    else {
        panic!("the game ended");
    };
    let Some(Line::Header { build, preset, .. }) = written.first() else {
        panic!("a header");
    };

    let summed = summary(&record).expect("summarises");
    assert!(summed.complete);
    assert_eq!(&summed.build, build);
    assert_eq!(&summed.winners, winners);
    assert_eq!(&summed.reason, reason);
    assert!(!summed.reason.is_empty());
    assert_eq!(summed.turn, *turn);
    assert!(summed.turn.is_some());
    assert_eq!(summed.inputs, input_positions(&written).len() as u64);
    assert_eq!(summed.started, 1_000);
    assert_eq!(summed.ended, *ended_at);
    assert!(summed.ended >= summed.started);

    assert_eq!(summed.seats.len(), 2);
    assert_eq!(summed.seats[0].controller, Controller::You);
    // `table` seats the house at its default level.
    assert_eq!(
        summed.seats[1].controller,
        Controller::House("steady".into())
    );
    for (seat, spec) in summed.seats.iter().zip(&preset.seats) {
        assert_eq!(seat.cards, spec.deck.len());
        assert_eq!(seat.team, spec.team);
        assert!(
            !seat.colours.is_empty() && seat.colours.chars().all(|c| "WUBRG".contains(c)),
            "{:?}",
            seat.colours
        );
        let mut order: Vec<char> = seat.colours.chars().collect();
        order.sort_by_key(|c| "WUBRG".find(*c));
        order.dedup();
        assert_eq!(
            order.iter().collect::<String>(),
            seat.colours,
            "WUBRG order, once each"
        );
        assert_eq!(
            seat.commander,
            spec.commanders.first().map(|entry| entry.card)
        );
        assert!(
            seat.commander.is_some(),
            "the acceptance decks are commander decks"
        );
    }

    // The first input is not an input of a game that is over: a record cut
    // before the end is not complete, and has no winner to name.
    let cut: Vec<Line> = written
        .iter()
        .filter(|line| !matches!(line, Line::End { .. }))
        .cloned()
        .collect();
    let summed = summary(&encode(&cut)).expect("summarises");
    assert!(!summed.complete);
    assert!(summed.winners.is_empty() && summed.reason.is_empty());
    assert_eq!(summed.turn, None);
    assert_eq!(summed.inputs, input_positions(&written).len() as u64);

    let half = &record[..record.len() / 2];
    assert!(!summary(half).expect("a cut record summarises").complete);
    assert!(matches!(
        summary(b"nonsense\n"),
        Err(ReplayError::Unreadable { .. })
    ));
    assert!(matches!(summary(b""), Err(ReplayError::NoHeader)));
}

/// Every kind of input the record holds is played by the projection: it takes
/// the clock's and the stand-in's answers as the live game did.
#[test]
fn the_record_the_projection_plays_holds_every_kind_of_input() {
    let written = lines(&played(3).take_record());
    for source in [Source::Seat, Source::Clock, Source::House, Source::StandIn] {
        assert!(
            written
                .iter()
                .any(|l| matches!(l, Line::Input { by, .. } if *by == source)),
            "no {source:?} input"
        );
    }
    assert_eq!(hex(0), WRONG, "the blank hash is a hash's width");
}

/// How long the game log was after each input of `record`, played alongside
/// on a session of this test's own: the first entry is before any.
fn log_lengths(record: &[u8]) -> Vec<usize> {
    let written = lines(record);
    let Some(Line::Header { preset, .. }) = written.first() else {
        panic!("a record opens with its header");
    };
    let mut session = Session::new(preset).expect("the preset builds");
    session.describe("replay".into(), names());
    let mut lengths = vec![session.log.len()];
    for line in &written {
        if let Line::Input {
            at,
            seat,
            by,
            action,
            ..
        } = line
        {
            session.tell_time(*at);
            assert!(session.replayed(PlayerId::new(*seat), *by, action.clone()));
            // Told at once, so that no later line folds into it: as the
            // projection tells it.
            session.log.seal(session.log.len());
            lengths.push(session.log.len());
        }
    }
    lengths
}

/// The log lines an input wrote are told with that input's moment, though
/// the view it left is the view before: a moment is quiet only when it
/// changed nothing of what the perspective can read, log included.
#[test]
fn a_log_line_with_no_new_view_is_still_told_at_its_input() {
    let record = played(5).take_record();
    let lengths = log_lengths(&record);
    for perspective in [Perspective::Seat(ME), Perspective::Public] {
        let (_, moments) = whole(&record, perspective);
        assert_eq!(moments.len(), lengths.len());
        let mut told = 0;
        let mut previous: Option<PlayerView> = None;
        let mut log_only = 0;
        for (i, moment) in moments.iter().enumerate() {
            let tail = tails(moment).pop();
            let view = if perspective == Perspective::Public {
                deltas(moment).last().map(|d| {
                    serde_json::from_slice::<SpectatorView>(&d.view_json)
                        .expect("a spectator view")
                        .into_player_view(ME)
                })
            } else {
                seat_view(moment)
            };
            if let (Some(tail), Some(view)) = (&tail, &view) {
                told = told.max(tail.from as usize + tail.entries.len());
                if let Some(before) = &previous
                    && *before == with_seq_of(view.clone(), before)
                {
                    log_only += 1;
                }
            }
            if view.is_some() {
                previous = view;
            }
            assert_eq!(
                told, lengths[i],
                "{perspective:?}: after {:?} the log holds {} lines and {told} were told",
                moment.input, lengths[i]
            );
        }
        assert!(
            log_only > 0,
            "{perspective:?}: no moment wrote only a log line"
        );
    }
}

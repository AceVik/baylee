//! A projection job (`projection.rs`): a stored record in, the tape one
//! perspective of it is sent out, in gzip members, ending on its `stop`.

use super::*;
use baylee_protocol::tape::{self, Basis, Check, Divergence, Perspective, Source, StopKind};
use std::io::Read as _;

const BUILD: &str = "test-build";
const RECORDED_BY: &str = "recorded-build";

/// The acceptance duel played to its end by seat 0, recorded as a gateway
/// stores a hosted game.
fn recorded_duel() -> Vec<u8> {
    let me = PlayerId::new(0);
    let mut session = Session::new_recorded(&duel(0), RECORDED_BY).expect("the duel builds");
    session.describe("g".into(), vec!["You".into(), "House".into()]);
    session.tell_time(5_000);
    session.pump();
    for _ in 0..5_000 {
        if matches!(session.pending(), Pending::GameOver(_)) {
            break;
        }
        let action = session.house_action(me).expect("the player is asked");
        session.act(me, action).expect("the answer stands");
    }
    assert!(matches!(session.pending(), Pending::GameOver(_)));
    session.take_record()
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut gz = GzEncoder::new(Vec::new(), flate2::Compression::default());
    gz.write_all(bytes).expect("compressing into memory");
    gz.finish().expect("compressing into memory")
}

fn order(record: Vec<u8>) -> v1::ProjectRecord {
    v1::ProjectRecord {
        job: "replay-x".into(),
        record,
        perspective: v1::project_record::Perspective::Seat as i32,
        seat: 0,
        names: vec!["You".into(), "House".into()],
        verify: v1::project_record::Verify::Stop as i32,
        game_id: "g".into(),
        recorded_by: String::new(),
        own: true,
        hands: Vec::new(),
    }
}

/// What a job sent: its chunks, and the tape they hold.
struct Sent {
    chunks: Vec<v1::TapeChunk>,
    stop: tape::Stop,
    lines: Vec<tape::Line>,
}

fn job(order: &v1::ProjectRecord) -> Sent {
    let mut chunks = Vec::new();
    let stop = crate::projection::run(order, BUILD, &mut |chunk| chunks.push(chunk));
    // The pieces in order: numbered from 0, the last one last, each one
    // gzip member that stands on its own.
    for (i, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk.seq as usize, i);
        assert_eq!(chunk.job, "replay-x");
        assert_eq!(chunk.last, i + 1 == chunks.len(), "chunk {i}");
        let mut alone = String::new();
        flate2::read::MultiGzDecoder::new(chunk.data.as_slice())
            .read_to_string(&mut alone)
            .expect("a chunk is a gzip member");
        assert!(alone.ends_with('\n'), "a chunk ends on a line");
    }
    let stored: Vec<u8> = chunks.iter().flat_map(|c| c.data.iter().copied()).collect();
    let mut text = String::new();
    flate2::read::MultiGzDecoder::new(stored.as_slice())
        .read_to_string(&mut text)
        .expect("the chunks are one gzip stream");
    let lines: Vec<tape::Line> = text
        .lines()
        .map(|line| tape::parse_line(line).expect("a tape line"))
        .collect();
    let Some(tape::Line::Stop(last)) = lines.last() else {
        panic!("the tape does not end on its stop");
    };
    assert_eq!(*last, stop, "the stop returned is the stop sent");
    assert_eq!(
        lines
            .iter()
            .filter(|line| matches!(line, tape::Line::Stop(_)))
            .count(),
        1
    );
    assert!(
        matches!(lines.first(), Some(tape::Line::Header(_))),
        "the header leads"
    );
    Sent {
        chunks,
        stop,
        lines,
    }
}

fn header(sent: &Sent) -> &tape::Header {
    let Some(tape::Line::Header(header)) = sent.lines.first() else {
        unreachable!()
    };
    header
}

fn views(sent: &Sent) -> Vec<&tape::View> {
    sent.lines
        .iter()
        .filter_map(|line| match line {
            tape::Line::View(view) => Some(view),
            _ => None,
        })
        .collect()
}

/// The whole job is a refusal: a header and a stop, nothing between.
fn refused(sent: &Sent) {
    assert_eq!(sent.lines.len(), 2, "{:?}", sent.lines);
    assert_eq!(sent.stop, tape::Stop::bare(StopKind::Refused));
    assert_eq!(sent.chunks.len(), 1);
}

/// The record's input lines, as JSON.
fn record_inputs(record: &[u8]) -> Vec<serde_json::Value> {
    String::from_utf8(record.to_vec())
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .filter(|line| line["kind"] == "input")
        .collect()
}

/// A record with the digest and hash of the `k`-th input set to something
/// else, as a record of another build would hold.
fn altered_at(record: &[u8], k: usize) -> Vec<u8> {
    let mut seen_inputs = 0;
    let mut out = String::new();
    for line in String::from_utf8(record.to_vec()).unwrap().lines() {
        let mut value: serde_json::Value = serde_json::from_str(line).unwrap();
        if value["kind"] == "input" {
            if seen_inputs == k {
                value["hash"] = "0000000000000000".into();
                value["seen"] = "deadbeefdeadbeef".into();
            }
            seen_inputs += 1;
        }
        out.push_str(&value.to_string());
        out.push('\n');
    }
    out.into_bytes()
}

/// A hosted game's seat projection comes back whole: a header that says what
/// it is, the opening payload, a view per change, the end and the stop.
#[test]
fn a_seat_projection_is_sent_as_a_tape_that_ends_on_its_stop() {
    let record = recorded_duel();
    let inputs = record_inputs(&record);
    assert!(inputs.len() > 100);
    let sent = job(&order(gzip(&record)));

    // The same order gives the same bytes: members, not just lines.
    let again = job(&order(gzip(&record)));
    assert_eq!(sent.chunks, again.chunks);
    assert!(
        sent.chunks.len() > 1,
        "a whole game is more than one member"
    );
    let header = header(&sent);
    assert_eq!(header.tape, 1);
    assert_eq!(header.tape, tape::TAPE_VERSION);
    assert_eq!(header.view, baylee_view::VIEW_VERSION);
    assert_eq!(header.build, BUILD);
    assert_eq!(header.source, Source::Hosted);
    assert_eq!(header.perspective, Perspective::Seat);
    assert_eq!(header.seat, 0);
    assert_eq!(header.game, "g");
    assert_eq!(
        header.recorded_by, RECORDED_BY,
        "the build that played it, from the record's own header"
    );

    let last_input = inputs.last().unwrap()["n"].as_u64().unwrap();
    assert_eq!(
        sent.stop,
        tape::Stop {
            stop: StopKind::End,
            inputs: inputs.len() as u64,
            exact_to: Some(last_input),
            seen_to: Some(last_input),
            diverged: None,
        }
    );

    // The opening payload comes before the first view, and names the seats.
    let first_static = sent
        .lines
        .iter()
        .position(|line| matches!(line, tape::Line::Static { .. }))
        .expect("an opening payload");
    let first_view = sent
        .lines
        .iter()
        .position(|line| matches!(line, tape::Line::View(_)))
        .expect("a view");
    assert!(first_static < first_view);
    let tape::Line::Static { json, .. } = &sent.lines[first_static] else {
        unreachable!()
    };
    assert!(json.get().contains("\"House\""), "the order's names");

    // Every view says which input it follows and that the build is the
    // recording's own, to the bit.
    let shown = views(&sent);
    assert!(shown.len() > 100);
    assert_eq!(shown[0].n, None);
    assert!(shown.iter().all(|view| view.check == Check::Exact));
    assert!(shown[1..].iter().all(|view| view.n.is_some()));
    assert!(shown.windows(2).all(|pair| pair[0].n <= pair[1].n));
    assert!(
        shown.iter().all(|view| {
            serde_json::from_str::<baylee_view::PlayerView>(view.json.get()).is_ok()
        })
    );

    // The game's end is told once, after the last view, before the stop.
    let overs: Vec<usize> = sent
        .lines
        .iter()
        .enumerate()
        .filter(|(_, line)| matches!(line, tape::Line::Over { .. }))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(overs.len(), 1);
    let last_view = sent
        .lines
        .iter()
        .rposition(|line| matches!(line, tape::Line::View(_)))
        .unwrap();
    assert_eq!(overs[0], last_view + 1);
    assert_eq!(overs[0] + 2, sent.lines.len());
    let tape::Line::Over { json, .. } = &sent.lines[overs[0]] else {
        unreachable!()
    };
    assert!(json.get().starts_with(r#"{"GameOver""#));

    // It is the very projection the host would make itself.
    let mut direct = Vec::new();
    baylee_gamehost::replay::project(
        &record,
        vec!["You".into(), "House".into()],
        baylee_gamehost::replay::Perspective::Seat(PlayerId::new(0)),
        baylee_gamehost::replay::Verify::Stop,
        |moment| {
            for env in &moment.frames {
                if let Some(v1::envelope::Msg::StateDelta(delta)) = &env.msg {
                    direct.push((moment.input, delta.seq, delta.view_json.clone()));
                }
            }
            std::ops::ControlFlow::Continue(())
        },
    )
    .expect("projects");
    assert_eq!(direct.len(), shown.len());
    for ((n, seq, json), view) in direct.iter().zip(&shown) {
        assert_eq!((*n, *seq), (view.n, view.seq));
        assert_eq!(std::str::from_utf8(json).unwrap(), view.json.get());
    }
}

/// A record stored as several gzip members is one record.
#[test]
fn a_record_in_several_gzip_members_is_projected_whole() {
    let record = recorded_duel();
    let half = record[..record.len() / 2]
        .iter()
        .rposition(|&b| b == b'\n')
        .unwrap()
        + 1;
    let mut stored = gzip(&record[..half]);
    stored.extend(gzip(&record[half..]));
    let one = job(&order(gzip(&record)));
    let two = job(&order(stored));
    assert_eq!(two.stop.stop, StopKind::End);
    assert_eq!(one.stop, two.stop);
    assert_eq!(views(&one).len(), views(&two).len());
}

/// What a record cannot be read as, or a perspective that cannot be had, is a
/// header and a refusal: the job still ends on its stop.
#[test]
fn a_job_that_cannot_project_is_a_header_and_a_refusal() {
    let record = recorded_duel();

    // Not gzip.
    let mut plain = order(record.clone());
    plain.recorded_by = "the-gateways-note".into();
    let sent = job(&plain);
    refused(&sent);
    assert_eq!(
        header(&sent).recorded_by,
        "the-gateways-note",
        "a record that cannot be read leaves the order's note"
    );
    refused(&job(&order(b"not a record at all".to_vec())));
    refused(&job(&order(Vec::new())));

    // Gzip of nothing is no record.
    refused(&job(&order(gzip(b""))));
    // Gzip of a first line that is no header gets no frame either: a
    // record that never began is refused, not cut.
    let garbled = job(&order(gzip(b"nonsense\n")));
    assert_eq!(garbled.lines.len(), 2, "{:?}", garbled.lines);
    assert_eq!(garbled.stop, tape::Stop::bare(StopKind::Refused));

    // No perspective asked, or a seat the table does not have.
    let mut unspecified = order(gzip(&record));
    unspecified.perspective = v1::project_record::Perspective::Unspecified as i32;
    refused(&job(&unspecified));
    let mut unknown = order(gzip(&record));
    unknown.perspective = 99;
    refused(&job(&unknown));
    for seat in [2, 7, 300] {
        let mut order = order(gzip(&record));
        order.seat = seat;
        refused(&job(&order));
    }
}

/// A spectator's projection is the public table: its header says so, the seat
/// asked for is no matter, and no view is a seat's.
#[test]
fn a_public_projection_is_headed_public_at_seat_zero() {
    let record = recorded_duel();
    let mut asked = order(gzip(&record));
    asked.perspective = v1::project_record::Perspective::Public as i32;
    asked.seat = 1;
    let sent = job(&asked);
    let header = header(&sent);
    assert_eq!(header.perspective, Perspective::Public);
    assert_eq!(header.seat, 0);
    assert_eq!(sent.stop.stop, StopKind::End);
    let shown = views(&sent);
    assert!(shown.len() > 50);
    for view in shown {
        let value: serde_json::Value = serde_json::from_str(view.json.get()).unwrap();
        let spectator: baylee_view::SpectatorView =
            serde_json::from_value(value.clone()).expect("a spectator's view");
        assert_eq!(serde_json::to_value(spectator).unwrap(), value);
        assert!(value.get("hand").is_none());
    }
}

/// A record the build cannot confirm stops the tape where it stood, or goes
/// on, unverified, when the player asked to play on.
#[test]
fn a_record_that_diverges_stops_or_goes_on_by_the_order() {
    let record = recorded_duel();
    let inputs = record_inputs(&record);
    let k = inputs.len() / 2;
    let at = inputs[k]["n"].as_u64().unwrap();
    let before = inputs[k - 1]["n"].as_u64().unwrap();
    let altered = altered_at(&record, k);

    let stopped = job(&order(gzip(&altered)));
    assert_eq!(
        stopped.stop,
        tape::Stop {
            stop: StopKind::Diverged,
            inputs: k as u64 + 1,
            exact_to: Some(before),
            seen_to: Some(before),
            diverged: Some(Divergence {
                input: Some(at),
                by: Basis::Seen
            }),
        }
    );
    let shown = views(&stopped);
    assert_eq!(shown.last().unwrap().n, Some(before));
    assert!(
        !stopped
            .lines
            .iter()
            .any(|line| matches!(line, tape::Line::Over { .. })),
        "a tape that stopped never tells the end"
    );

    let mut note = order(gzip(&altered));
    note.verify = v1::project_record::Verify::Note as i32;
    let noted = job(&note);
    assert_eq!(noted.stop.stop, StopKind::End);
    assert_eq!(
        noted.stop.diverged,
        Some(Divergence {
            input: Some(at),
            by: Basis::Seen
        })
    );
    assert_eq!(noted.stop.inputs, inputs.len() as u64);
    for view in views(&noted) {
        let want = if view.n.is_some_and(|n| n >= at) {
            Check::Unverified
        } else {
            Check::Exact
        };
        assert_eq!(view.check, want, "after {:?}", view.n);
    }
}

/// A record cut off before its end is a tape that stops where the record did.
#[test]
fn a_record_cut_off_is_a_tape_that_stops_cut() {
    let record = recorded_duel();
    let cut = &record[..record.len() / 2];
    let sent = job(&order(gzip(cut)));
    assert_eq!(sent.stop.stop, StopKind::Cut);
    assert!(sent.stop.inputs > 20);
    assert_eq!(sent.stop.diverged, None);
    assert!(
        !sent
            .lines
            .iter()
            .any(|line| matches!(line, tape::Line::Over { .. }))
    );
    assert!(views(&sent).len() > 20);
}

// ---- which hidden cards a tape may hold (docs/protocol.md §"Replays")

/// The views a tape holds as seat views.
fn player_views(sent: &Sent) -> Vec<baylee_view::PlayerView> {
    views(sent)
        .iter()
        .map(|view| serde_json::from_str(view.json.get()).expect("a seat's view"))
        .collect()
}

/// What the host itself tells for `perspective`, the views' bytes and the
/// input each follows.
fn direct_views(
    record: &[u8],
    perspective: baylee_gamehost::replay::Perspective,
) -> Vec<(Option<u64>, String)> {
    let mut direct = Vec::new();
    baylee_gamehost::replay::project(
        record,
        vec!["You".into(), "House".into()],
        perspective,
        baylee_gamehost::replay::Verify::Stop,
        |moment| {
            for env in &moment.frames {
                if let Some(v1::envelope::Msg::StateDelta(delta)) = &env.msg {
                    direct.push((
                        moment.input,
                        String::from_utf8(delta.view_json.clone()).unwrap(),
                    ));
                }
            }
            std::ops::ControlFlow::Continue(())
        },
    )
    .expect("projects");
    direct
}

fn bytes_of(sent: &Sent) -> Vec<(Option<u64>, String)> {
    views(sent)
        .iter()
        .map(|view| (view.n, view.json.get().to_owned()))
        .collect()
}

fn seats_of(of: &[u8]) -> baylee_core::ids::SeatSet {
    let mut set = baylee_core::ids::SeatSet::new();
    for &seat in of {
        set.insert(PlayerId::new(seat));
    }
    set
}

/// An order that withholds the drawn seat's own cards: the header says so, no
/// view holds a hand or a looked-at card, and the views are the host's
/// public table drawn from that seat, byte for byte.
#[test]
fn an_order_that_withholds_the_seats_own_cards_sends_the_public_table_drawn_from_it() {
    let record = recorded_duel();
    for seat in [0_u32, 1] {
        let mut full = order(gzip(&record));
        full.seat = seat;
        let full = job(&full);
        assert!(header(&full).own);
        assert!(
            player_views(&full).iter().any(|view| !view.hand.is_empty()),
            "the seat's own tape holds its hand, or this proves nothing"
        );

        let mut masked = order(gzip(&record));
        masked.seat = seat;
        masked.own = false;
        let sent = job(&masked);
        let head = header(&sent);
        assert_eq!(head.perspective, Perspective::Seat);
        assert_eq!(head.seat, seat as u8);
        assert!(!head.own);
        assert!(head.hands.is_empty());
        assert_eq!(sent.stop.stop, StopKind::End);

        let shown = player_views(&sent);
        assert!(shown.len() > 50);
        for view in &shown {
            assert!(view.hand.is_empty(), "a hand in a masked tape");
            assert!(view.looking_at.is_empty());
            assert!(view.shared_hands.iter().all(|hand| hand.public));
            assert_eq!(u32::from(view.seat.get()), seat);
        }
        assert_eq!(
            bytes_of(&sent),
            direct_views(
                &record,
                baylee_gamehost::replay::Perspective::Shown {
                    seat: PlayerId::new(seat as u8),
                    own: false,
                    hands: seats_of(&[]),
                }
            )
        );
        // The payload is the spectators' print table, drawn from the seat.
        let tape::Line::Static { json, .. } = sent
            .lines
            .iter()
            .find(|line| matches!(line, tape::Line::Static { .. }))
            .expect("an opening payload")
        else {
            unreachable!()
        };
        let payload: baylee_view::GameStatic = serde_json::from_str(json.get()).unwrap();
        assert_eq!(u32::from(payload.your_seat.get()), seat);
        assert!(
            payload.prints.iter().filter(|p| p.is_some()).count() < payload.prints.len() / 2,
            "the whole decklist went out in a masked tape's payload"
        );
        // And the log is not the seat's: no line the log holds for the seat
        // alone.
        let masked_log = sent
            .lines
            .iter()
            .filter_map(|line| match line {
                tape::Line::View(view) => view.log.as_ref().map(|log| log.get().to_owned()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let full_log = full
            .lines
            .iter()
            .filter_map(|line| match line {
                tape::Line::View(view) => view.log.as_ref().map(|log| log.get().to_owned()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(!masked_log.is_empty());
        assert_ne!(masked_log, full_log, "the seat's own log in a masked tape");
    }
}

/// `own` omitted is `false` (proto3): an order that does not say is masked.
/// The gateway always sets it; this is the failure that costs nothing.
#[test]
fn an_order_that_leaves_own_out_is_masked() {
    let record = recorded_duel();
    let omitted = v1::ProjectRecord {
        job: "replay-x".into(),
        record: gzip(&record),
        perspective: v1::project_record::Perspective::Seat as i32,
        seat: 0,
        names: vec!["You".into(), "House".into()],
        verify: v1::project_record::Verify::Stop as i32,
        game_id: "g".into(),
        ..Default::default()
    };
    assert!(!omitted.own && omitted.hands.is_empty());
    let sent = job(&omitted);
    assert!(!header(&sent).own);
    let shown = player_views(&sent);
    assert!(shown.len() > 50);
    assert!(shown.iter().all(|view| view.hand.is_empty()));
}

/// All hands with hands `[1]`: the header says so, and each view has seat 1's
/// hand beside the seat's own, as the host tells it.
#[test]
fn an_all_hands_order_shows_the_hands_it_names() {
    let record = recorded_duel();
    let mut asked = order(gzip(&record));
    asked.perspective = v1::project_record::Perspective::AllHands as i32;
    asked.hands = vec![1];
    let sent = job(&asked);
    let head = header(&sent);
    assert_eq!(head.perspective, Perspective::AllHands);
    assert_eq!(head.seat, 0);
    assert!(head.own);
    assert_eq!(head.hands, vec![1]);
    assert_eq!(sent.stop.stop, StopKind::End);

    let shown = player_views(&sent);
    assert!(shown.len() > 50);
    let mut held = 0;
    for view in &shown {
        let hands: Vec<_> = view.shared_hands.iter().filter(|h| !h.public).collect();
        assert_eq!(hands.len(), 1);
        assert_eq!(hands[0].player.get(), 1);
        held += usize::from(!hands[0].cards.is_empty());
    }
    assert!(held > 30);
    assert!(
        shown.iter().any(|view| !view.hand.is_empty()),
        "its own hand"
    );
    let want = direct_views(
        &record,
        baylee_gamehost::replay::Perspective::AllHands(PlayerId::new(0)),
    );
    assert_eq!(bytes_of(&sent), want);

    // The same hands asked of a plain seat tape are the same views; only the
    // header's label differs.
    let mut seat = order(gzip(&record));
    seat.hands = vec![1];
    let labelled = job(&seat);
    assert_eq!(header(&labelled).perspective, Perspective::Seat);
    assert_eq!(header(&labelled).hands, vec![1]);
    assert_eq!(bytes_of(&labelled), want);

    // Hands asked of a seat whose own cards are withheld: seat 0 masked, the
    // hand of seat 1 beside it.
    let mut masked = order(gzip(&record));
    masked.perspective = v1::project_record::Perspective::AllHands as i32;
    masked.own = false;
    masked.hands = vec![1];
    let sent = job(&masked);
    assert!(!header(&sent).own);
    assert_eq!(header(&sent).hands, vec![1]);
    assert!(
        player_views(&sent)
            .iter()
            .all(|view| view.hand.is_empty() && view.shared_hands.iter().any(|h| !h.public))
    );
}

/// The hands of an order are cleaned before they reach the header or the
/// views: sorted, each once, never the drawn seat, never past a set's bound.
#[test]
fn the_hands_of_an_order_are_sorted_deduplicated_and_bounded() {
    let record = recorded_duel();
    // Drawn seat 1: its own number (1), a duplicate (0 twice), numbers that
    // are no seat and past a set's bound (16, 99, 300).
    let mut asked = order(gzip(&record));
    asked.perspective = v1::project_record::Perspective::AllHands as i32;
    asked.seat = 1;
    asked.hands = vec![99, 0, 1, 300, 16, 0, 15];
    let sent = job(&asked);
    assert_eq!(header(&sent).hands, vec![0, 15]);
    for view in player_views(&sent) {
        let owners: Vec<_> = view.shared_hands.iter().map(|h| h.player.get()).collect();
        assert_eq!(owners, vec![0], "only a seat the table has is shown");
    }

    // Nothing left of the hands: the seat's own tape, as `Seat` tells it.
    let mut none = order(gzip(&record));
    none.hands = vec![0, 16, 99, 300];
    let sent = job(&none);
    assert!(header(&sent).hands.is_empty());
    assert_eq!(
        bytes_of(&sent),
        bytes_of(&job(&order(gzip(&record)))),
        "a hand of the drawn seat is no hand"
    );
}

/// The public perspective has no seat's cards to withhold or to add: `own`
/// and `hands` change nothing about it, not even its header.
#[test]
fn a_public_order_ignores_own_and_hands() {
    let record = recorded_duel();
    let mut plain = order(gzip(&record));
    plain.perspective = v1::project_record::Perspective::Public as i32;
    let plain = job(&plain);
    let mut odd = order(gzip(&record));
    odd.perspective = v1::project_record::Perspective::Public as i32;
    odd.own = false;
    odd.hands = vec![1];
    let odd = job(&odd);
    assert_eq!(header(&odd), header(&plain));
    assert!(header(&odd).own && header(&odd).hands.is_empty());
    assert_eq!(bytes_of(&odd), bytes_of(&plain));
}

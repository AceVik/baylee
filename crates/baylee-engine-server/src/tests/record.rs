//! The game's record: pieces, flushes, and the minds declared into it.

use super::*;

/// The game's record reaches the gateway whole (#315): in pieces as it
/// grows, the last of them before `GameEnded`, and the pieces together
/// replay the game the runner played.
#[test]
fn the_record_goes_to_the_gateway_before_the_game_ends() {
    let me = PlayerId::new(0);
    let mut runner = EngineRunner::new();
    let mut out = setup(&mut runner, &duel(0));
    out.extend(sit(&mut runner, 0));
    for _ in 0..5_000 {
        if runner.finished() {
            break;
        }
        let action = runner
            .session()
            .and_then(|s| s.house_action(me))
            .expect("the player is asked while the game goes on");
        let step = act(&mut runner, 0, &action);
        let sent = step
            .iter()
            .any(|e| matches!(e.msg, Some(v1::envelope::Msg::GameRecordChunk(_))));
        if !sent {
            // Nothing held back that was due: what is left waiting is
            // under the bound.
            let pending = runner.session().unwrap().record_pending();
            assert!(pending < RECORD_CHUNK_BYTES, "{pending} bytes held back");
        }
        out.extend(step);
    }
    assert!(runner.finished(), "the game ended");
    let pieces: Vec<&v1::GameRecordChunk> = out
        .iter()
        .filter_map(|e| match &e.msg {
            Some(v1::envelope::Msg::GameRecordChunk(c)) => Some(c),
            _ => None,
        })
        .collect();
    assert!(
        pieces.len() > 1,
        "a whole game is sent in more than one piece"
    );
    let seqs: Vec<u32> = pieces.iter().map(|c| c.seq).collect();
    assert_eq!(
        seqs,
        (0..u32::try_from(pieces.len()).unwrap()).collect::<Vec<_>>()
    );
    let last = pieces.iter().filter(|c| c.last).count();
    assert_eq!(last, 1, "one last piece");
    assert!(pieces.last().unwrap().last, "the last piece is last");
    let kinds: Vec<bool> = out
        .iter()
        .filter_map(|e| match &e.msg {
            Some(v1::envelope::Msg::GameRecordChunk(_)) => Some(false),
            Some(v1::envelope::Msg::GameEnded(_)) => Some(true),
            _ => None,
        })
        .collect();
    assert_eq!(
        kinds.last(),
        Some(&true),
        "GameEnded comes after the record"
    );
    assert_eq!(kinds.iter().filter(|&&k| k).count(), 1);

    let stored: Vec<u8> = pieces.iter().flat_map(|c| c.data.iter().copied()).collect();
    let mut record = Vec::new();
    std::io::Read::read_to_end(
        &mut flate2::read::MultiGzDecoder::new(&stored[..]),
        &mut record,
    )
    .expect("the pieces are one gzip stream");
    let replayed = baylee_gamehost::record::replay(&record).expect("the record replays");
    assert!(replayed.ended);
    assert_eq!(
        replayed.engine.snapshot_hash(),
        runner.session().unwrap().snapshot_hash()
    );
    let text = String::from_utf8(record).unwrap();
    assert!(!text.contains("\"You\""), "the record names no seat");

    // Each piece but the last waited for the bound, and each is a gzip
    // member of its own.
    for piece in &pieces {
        let mut inside = Vec::new();
        std::io::Read::read_to_end(
            &mut flate2::read::GzDecoder::new(&piece.data[..]),
            &mut inside,
        )
        .expect("a gzip member");
        if !piece.last {
            assert!(inside.len() >= RECORD_CHUNK_BYTES, "piece {}", piece.seq);
        }
    }
}

/// The bound is inclusive: exactly [`RECORD_CHUNK_BYTES`] is a piece.
#[test]
fn a_piece_is_due_at_exactly_its_bound() {
    assert!(!record_due(0));
    assert!(!record_due(RECORD_CHUNK_BYTES - 1));
    assert!(record_due(RECORD_CHUNK_BYTES));
    assert!(record_due(RECORD_CHUNK_BYTES + 1));
}

/// The gateway's ask for the record as it stands (#323).
fn flush(runner: &mut EngineRunner, nonce: u64) -> Vec<Envelope> {
    runner.handle(
        Envelope {
            msg: Some(v1::envelope::Msg::FlushRecord(v1::FlushRecord {
                game_id: "g1".to_string(),
                nonce,
            })),
        },
        &[],
    )
}

/// The record pieces among `out`, in order.
fn pieces(out: &[Envelope]) -> Vec<v1::GameRecordChunk> {
    out.iter()
        .filter_map(|e| match &e.msg {
            Some(v1::envelope::Msg::GameRecordChunk(c)) => Some(c.clone()),
            _ => None,
        })
        .collect()
}

/// One gzip member, unpacked; panics on anything else, trailing bytes
/// included, so a piece that needs its neighbour to be read fails here.
fn member(data: &[u8]) -> Vec<u8> {
    let mut gz = flate2::read::GzDecoder::new(data);
    let mut inside = Vec::new();
    std::io::Read::read_to_end(&mut gz, &mut inside).expect("a gzip member");
    assert!(gz.into_inner().is_empty(), "one member, nothing after it");
    inside
}

/// The record the pieces make, each unpacked on its own.
fn unpacked(pieces: &[v1::GameRecordChunk]) -> Vec<u8> {
    pieces.iter().flat_map(|p| member(&p.data)).collect()
}

/// Plays the house's answers for seat 0, `steps` of them.
fn play_a_little(runner: &mut EngineRunner, steps: usize) -> Vec<Envelope> {
    let me = PlayerId::new(0);
    let mut out = Vec::new();
    for _ in 0..steps {
        let action = runner
            .session()
            .and_then(|s| s.house_action(me))
            .expect("the player is asked while the game goes on");
        out.extend(act(runner, 0, &action));
    }
    out
}

/// What answers a seat, as its socket declares it, wrapped the way the
/// socket sends it.
fn declare(runner: &mut EngineRunner, seat: u32, model: &str) -> Vec<Envelope> {
    let inner = Envelope {
        msg: Some(v1::envelope::Msg::SeatMind(v1::SeatMind {
            kind: v1::seat_mind::Kind::LlmApi as i32,
            provider: "anthropic".into(),
            model: model.into(),
            effort: "high".into(),
            level: String::new(),
        })),
    };
    runner.handle(
        Envelope {
            msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                seat,
                envelope: prost::Message::encode_to_vec(&inner).into(),
            })),
        },
        &[],
    )
}

/// A seat's declared mind goes into the record, before the curtain as
/// after, and to nobody at the table: it answers no frame. One shaped
/// like a key is dropped, and the record holds no trace of it. A swap
/// mid-game is written where it came; the record still replays.
#[test]
fn a_declared_mind_goes_into_the_record_and_to_no_seat() {
    let key = format!("claude-opus-5-5sk-ant-{}", "Z".repeat(30));
    let mut runner = EngineRunner::new();
    setup(&mut runner, &duel(0));
    attach(&mut runner, 0);
    assert!(runner.curtain_pending());
    assert!(declare(&mut runner, 0, &key).is_empty());
    assert!(declare(&mut runner, 0, "claude-opus-5-5").is_empty());
    ready(&mut runner, 0);
    runner.tell_time(runner.entrance_deadline().expect("the entrance"));
    runner.finish_entrance();
    play_a_little(&mut runner, 4);
    assert!(declare(&mut runner, 0, "claude-sonnet-5-5").is_empty());
    play_a_little(&mut runner, 4);
    let record = unpacked(&pieces(&flush(&mut runner, 1)));
    let text = String::from_utf8(record.clone()).unwrap();
    assert!(!text.contains("ZZZZ"), "the key-shaped one is not written");
    let models: Vec<String> = text
        .lines()
        .filter_map(
            |l| match serde_json::from_str::<baylee_gamehost::record::Line>(l).ok()? {
                baylee_gamehost::record::Line::DeclaredMind { seat: 0, mind, .. } => {
                    Some(mind.model)
                }
                _ => None,
            },
        )
        .collect();
    assert_eq!(models, ["claude-opus-5-5", "claude-sonnet-5-5"]);
    let replayed = baylee_gamehost::record::replay(&record).expect("the record replays");
    assert_eq!(
        replayed.engine.snapshot_hash(),
        runner.session().unwrap().snapshot_hash()
    );
}

/// A new socket is credited with nothing the last one declared: one that
/// answers before it says what it is ends the declaration where its first
/// answer stands, and one that says it again is written again. A resync,
/// the same socket that lagged, changes nothing.
#[test]
fn a_new_socket_that_answers_undeclared_ends_the_last_declaration() {
    use baylee_gamehost::record::{Line, MindKind, Source};
    let mut runner = EngineRunner::new();
    setup(&mut runner, &duel(0));
    attach(&mut runner, 0);
    declare(&mut runner, 0, "claude-opus-5-5");
    ready(&mut runner, 0);
    runner.tell_time(runner.entrance_deadline().expect("the entrance"));
    runner.finish_entrance();
    play_a_little(&mut runner, 3);
    runner.handle(
        Envelope {
            msg: Some(v1::envelope::Msg::SeatAttached(v1::SeatAttached {
                seat: 0,
                resync: true,
            })),
        },
        &[],
    );
    play_a_little(&mut runner, 2);
    detach(&mut runner, 0);
    attach(&mut runner, 0);
    play_a_little(&mut runner, 2);
    declare(&mut runner, 0, "claude-opus-5-5");
    play_a_little(&mut runner, 2);
    let record = unpacked(&pieces(&flush(&mut runner, 1)));
    let lines: Vec<Line> = record
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_slice(l).expect("a line"))
        .collect();
    let said: Vec<(usize, MindKind)> = lines
        .iter()
        .enumerate()
        .filter_map(|(at, l)| match l {
            Line::DeclaredMind { seat: 0, mind, .. } => Some((at, mind.kind)),
            _ => None,
        })
        .collect();
    assert_eq!(
        said.iter().map(|(_, kind)| *kind).collect::<Vec<_>>(),
        [MindKind::LlmApi, MindKind::Undeclared, MindKind::LlmApi],
        "{said:?}"
    );
    let undeclared = said[1].0;
    let seat_inputs_before = lines[..undeclared]
        .iter()
        .filter(|l| {
            matches!(
                l,
                Line::Input {
                    seat: 0,
                    by: Source::Seat,
                    ..
                }
            )
        })
        .count();
    assert_eq!(
        seat_inputs_before, 5,
        "the first socket's answers, resync included"
    );
    assert!(matches!(
        lines.get(undeclared + 1),
        Some(Line::Input {
            seat: 0,
            by: Source::Seat,
            ..
        })
    ));
    baylee_gamehost::record::replay(&record).expect("the record replays");
}

/// A flush sends what has gathered as one piece that is not `last`, and
/// then the acknowledgement with the gateway's nonce, in that order on
/// the socket; the piece replays to exactly the game as it stands. It
/// moves nothing: no frame for a seat, the same hash, the same question.
#[test]
fn a_flush_sends_what_has_gathered_and_then_says_so() {
    let mut runner = EngineRunner::new();
    let mut out = setup(&mut runner, &duel(0));
    out.extend(sit(&mut runner, 0));
    out.extend(play_a_little(&mut runner, 6));
    assert!(
        pieces(&out).is_empty(),
        "a few answers are under one size-due piece"
    );
    let session = runner.session().unwrap();
    assert!(session.record_pending() > 0, "the record has gathered");
    let (hash, asked) = (session.snapshot_hash(), session.asked_at(PlayerId::new(0)));

    let flushed = flush(&mut runner, 7);
    assert_eq!(flushed.len(), 2, "{flushed:?}");
    let Some(v1::envelope::Msg::GameRecordChunk(piece)) = &flushed[0].msg else {
        panic!("the piece first: {flushed:?}");
    };
    assert_eq!(
        (piece.game_id.as_str(), piece.seq, piece.last),
        ("g1", 0, false)
    );
    assert_eq!(
        flushed[1].msg,
        Some(v1::envelope::Msg::RecordFlushed(v1::RecordFlushed {
            game_id: "g1".to_string(),
            nonce: 7,
            pieces: 1,
        })),
        "then the acknowledgement, after the piece"
    );
    let session = runner.session().unwrap();
    assert_eq!(session.record_pending(), 0, "nothing left waiting");
    assert_eq!(session.snapshot_hash(), hash, "the game did not move");
    assert_eq!(session.asked_at(PlayerId::new(0)), asked);

    let replayed =
        baylee_gamehost::record::replay(&member(&piece.data)).expect("the piece replays");
    assert!(!replayed.ended);
    assert_eq!(
        replayed.engine.snapshot_hash(),
        hash,
        "to the moment of the flush"
    );
}

/// Every flush is answered, because the gateway waits on the answer:
/// with nothing waiting, before the game is built and after it ended
/// the answer is the acknowledgement alone, never an empty piece.
#[test]
fn a_flush_with_nothing_waiting_is_answered_with_the_acknowledgement_alone() {
    let ack = |nonce: u64, pieces: u32, game_id: &str| Envelope {
        msg: Some(v1::envelope::Msg::RecordFlushed(v1::RecordFlushed {
            game_id: game_id.to_string(),
            nonce,
            pieces,
        })),
    };
    let mut runner = EngineRunner::new();
    assert_eq!(
        flush(&mut runner, 1),
        [ack(1, 0, "")],
        "before the game is built"
    );

    setup(&mut runner, &duel(0));
    sit(&mut runner, 0);
    play_a_little(&mut runner, 2);
    let first = flush(&mut runner, 2);
    assert_eq!(pieces(&first).len(), 1);
    assert_eq!(
        flush(&mut runner, 3),
        [ack(3, 1, "g1")],
        "nothing gathered since the last flush"
    );

    let out = act(&mut runner, 0, &PlayerAction::Concede);
    assert!(runner.finished());
    assert_eq!(pieces(&out).last().map(|p| p.last), Some(true));
    assert_eq!(
        flush(&mut runner, 4),
        [ack(4, 2, "g1")],
        "after the last piece there is nothing more to send"
    );
}

/// Record that waits [`RECORD_FLUSH_MS`] goes without being asked for,
/// counted from when it began to wait, not from the last piece; not
/// while the curtain is down, and never a moment early.
#[test]
fn the_record_goes_by_time_once_it_has_waited_long_enough() {
    const T0: u64 = 1_700_000_000_000;
    let mut runner = EngineRunner::new();
    runner.tell_time(T0);
    assert_eq!(runner.record_deadline(), None, "no game, no record");
    setup(&mut runner, &duel(0));
    assert!(runner.curtain_pending());
    assert_eq!(runner.record_deadline(), None, "not while seats load");
    runner.tell_time(T0 + RECORD_FLUSH_MS);
    assert!(runner.flush_record_due().is_empty());

    runner.tell_time(T0);
    sit(&mut runner, 0);
    let due = T0 + RECORD_FLUSH_MS;
    assert_eq!(
        runner.record_deadline(),
        Some(due),
        "the header has waited since the game was built"
    );
    runner.tell_time(due - 1);
    assert!(runner.flush_record_due().is_empty(), "never early");
    runner.tell_time(due);
    let sent = pieces(&runner.flush_record_due());
    assert_eq!(sent.len(), 1);
    assert!(!sent[0].last);
    assert_eq!(runner.record_deadline(), None, "nothing waits now");
    assert!(
        runner.flush_record_due().is_empty(),
        "a stale timer does nothing"
    );

    // Bytes that begin to wait at T1 are due at T1 + RECORD_FLUSH_MS,
    // however much more joins them before then.
    let t1 = due + 5 * RECORD_FLUSH_MS;
    runner.tell_time(t1);
    play_a_little(&mut runner, 1);
    assert_eq!(runner.record_deadline(), Some(t1 + RECORD_FLUSH_MS));
    runner.tell_time(t1 + RECORD_FLUSH_MS / 2);
    play_a_little(&mut runner, 1);
    assert_eq!(runner.record_deadline(), Some(t1 + RECORD_FLUSH_MS));

    // A flush the gateway asked for starts the wait over.
    flush(&mut runner, 9);
    assert_eq!(runner.record_deadline(), None);

    act(&mut runner, 0, &PlayerAction::Concede);
    assert!(runner.finished());
    assert_eq!(runner.record_deadline(), None, "the game is over");
}

/// A tiny deterministic generator, so the property below is the same
/// every run.
struct XorShift(u64);

impl XorShift {
    fn below(&mut self, n: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % n
    }
}

/// Flushes at any moment change nothing but where the pieces break
/// (#323). One game played twice, answer for answer and millisecond for
/// millisecond: once sending only size-due pieces, once also flushing at
/// random moments and on the timer. Each piece is a gzip member on its
/// own; at every flush the pieces so far replay to exactly the game as
/// it stands; and the two records are the same bytes.
#[test]
fn flushes_at_any_moment_change_nothing_but_where_the_pieces_break() {
    let me = PlayerId::new(0);
    for seed in [0x9e37_79b9_7f4a_7c15_u64, 0x2545_f491_4f6c_dd1d] {
        let mut rng = XorShift(seed);
        let mut plain = EngineRunner::new();
        let mut flushed = EngineRunner::new();
        let (mut out_plain, mut out_flushed) = (Vec::new(), Vec::new());
        for (runner, out) in [
            (&mut plain, &mut out_plain),
            (&mut flushed, &mut out_flushed),
        ] {
            runner.tell_time(1_700_000_000_000);
            out.extend(setup(runner, &duel(0)));
            out.extend(sit(runner, 0));
        }
        let mut now = plain.now;
        let (mut asked, mut checked) = (0, 0);
        for _ in 0..5_000 {
            if flushed.finished() {
                break;
            }
            let action = flushed
                .session()
                .and_then(|s| s.house_action(me))
                .expect("the player is asked while the game goes on");
            now += rng.below(20_000);
            plain.tell_time(now);
            flushed.tell_time(now);
            out_plain.extend(act(&mut plain, 0, &action));
            out_flushed.extend(act(&mut flushed, 0, &action));
            out_flushed.extend(flushed.flush_record_due());
            assert_eq!(
                plain.session().unwrap().snapshot_hash(),
                flushed.session().unwrap().snapshot_hash()
            );
            if !flushed.finished() && rng.below(25) == 0 {
                asked += 1;
                out_flushed.extend(flush(&mut flushed, asked));
                let so_far = unpacked(&pieces(&out_flushed));
                let replayed =
                    baylee_gamehost::record::replay(&so_far).expect("the record replays");
                assert_eq!(
                    replayed.engine.snapshot_hash(),
                    flushed.session().unwrap().snapshot_hash(),
                    "flush {asked} reaches the moment it was asked at"
                );
                checked += 1;
            }
        }
        assert!(plain.finished() && flushed.finished(), "the game ended");
        assert!(checked >= 5, "only {checked} flushes checked");
        let (a, b) = (pieces(&out_plain), pieces(&out_flushed));
        assert!(
            b.len() > a.len() + checked,
            "{} pieces against {}",
            b.len(),
            a.len()
        );
        for list in [&a, &b] {
            let seqs: Vec<u32> = list.iter().map(|p| p.seq).collect();
            assert_eq!(
                seqs,
                (0..u32::try_from(list.len()).unwrap()).collect::<Vec<_>>()
            );
            assert_eq!(list.iter().filter(|p| p.last).count(), 1);
            assert!(list.last().unwrap().last);
        }
        assert_eq!(unpacked(&a), unpacked(&b), "the same record");
        let stored: Vec<u8> = b.iter().flat_map(|p| p.data.iter().copied()).collect();
        let mut whole = Vec::new();
        std::io::Read::read_to_end(
            &mut flate2::read::MultiGzDecoder::new(&stored[..]),
            &mut whole,
        )
        .expect("the pieces stored in order are one gzip stream");
        assert_eq!(whole, unpacked(&a));
    }
}

/// A game can end with nothing left to send (the last take drained it):
/// the last piece is then an empty gzip member, which a reader of the
/// pieces in order passes over, and `seq` still counts on.
#[test]
fn an_empty_last_piece_still_ends_the_record() {
    let mut runner = EngineRunner::new();
    let piece = |env: Envelope| match env.msg {
        Some(v1::envelope::Msg::GameRecordChunk(c)) => c,
        other => panic!("{other:?}"),
    };
    let first = piece(runner.record_chunk(b"a line\n", false));
    let last = piece(runner.record_chunk(b"", true));
    assert_eq!((first.seq, last.seq), (0, 1));
    assert!(last.last && !last.data.is_empty());
    let mut record = Vec::new();
    std::io::Read::read_to_end(
        &mut flate2::read::MultiGzDecoder::new(&[first.data, last.data].concat()[..]),
        &mut record,
    )
    .expect("one gzip stream");
    assert_eq!(record, b"a line\n");
}

/// A preset that does not describe a game has to say so. The gateway
/// cannot read a `Pending` and would otherwise hold a table open forever
/// waiting for a game that was never built.
#[test]
fn a_game_that_cannot_start_says_so_rather_than_hanging() {
    let mut runner = EngineRunner::new();
    let out = runner.handle(
        Envelope {
            msg: Some(v1::envelope::Msg::GameSetup(v1::GameSetup {
                game_id: "g1".to_string(),
                preset_json: b"not a preset".to_vec(),
                seat_names: Vec::new(),
            })),
        },
        &[],
    );
    assert!(
        matches!(
            out.first().map(|e| &e.msg),
            Some(Some(v1::envelope::Msg::GameEnded(_)))
        ),
        "a broken setup was swallowed"
    );
    assert!(!runner.ready());
}

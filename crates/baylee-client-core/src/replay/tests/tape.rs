//! The tape in memory: reading, writing, the two doors, the lookups.

use baylee_protocol::v1::{self, Envelope};
use baylee_view::LogTail;

use super::{
    Script, VIEW, at, entries, game_over, header_line, is_game_over, json, spectator_of, tail,
    view_of, with_lines,
};
use crate::bugreport::gzip;
use crate::replay::tape::{Tape, TapeMoment, TapeRefusal};
use crate::replay::{Check, Header, Source, StopLine, StopReason};
use crate::test_support::statics;
use baylee_core::ids::PlayerId;
use baylee_view::Phase;

/// A script exercising everything a tape has: statics (one before the
/// first step, one mid-way), a curtain, log lines, an over line, a stop that
/// says more than `end`.
fn busy() -> Script {
    let mut script = Script::new(vec![
        at(1, Phase::Beginning).lines(2).n(1),
        at(1, Phase::FirstMain).check(Check::Seen).n(2),
        at(1, Phase::Combat).lines(1).check(Check::Seen).n(3),
        at(2, Phase::Beginning)
            .lines(3)
            .check(Check::Unverified)
            .n(4),
        at(2, Phase::FirstMain).n(5),
    ]);
    script.statics_before = vec![0, 2];
    script.curtain_before = Some(1);
    script.over = true;
    script.stop = Some("diverged");
    script
}

/// A tape written and read back is the same tape: statics, curtain, log,
/// over and stop included.
#[test]
fn a_written_tape_reads_back_equal() {
    let tape = busy().tape();
    assert_eq!(tape.frames.len(), 5);
    assert!(tape.curtain.is_some());
    assert!(tape.over().is_some());
    assert_eq!(tape.stop.stop, StopReason::Diverged);

    assert_eq!(Tape::read(&tape.write()).expect("gzip"), tape);
    assert_eq!(Tape::read_lines(&tape.write_lines()).expect("lines"), tape);
}

/// What the writer says is what the reader was given, line for line: nothing
/// is dropped or reordered on the way out.
#[test]
fn a_written_tape_is_the_text_it_was_read_from() {
    let script = busy();
    let tape = script.tape();
    let written = String::from_utf8(tape.write_lines()).expect("text");
    let given: Vec<serde_json::Value> = script
        .text()
        .lines()
        .map(|l| serde_json::from_str(l).expect("json"))
        .collect();
    let back: Vec<serde_json::Value> = written
        .lines()
        .map(|l| serde_json::from_str(l).expect("json"))
        .collect();
    assert_eq!(back, given);
}

/// The moments a local projection hands its sink: a static and the first
/// view with a log, a curtain, more views, then the end with a question that
/// is not the end beside it.
fn moments() -> Vec<TapeMoment> {
    let delta = |i: usize, spec: &crate::replay::tests::Spec, log: Option<LogTail>| Envelope {
        msg: Some(v1::envelope::Msg::StateDelta(v1::StateDelta {
            game_id: String::new(),
            seq: i as u64 + 1,
            view_json: json(&view_of(i, spec, true)).into_bytes(),
            log_json: log.map(|t| json(&t).into_bytes()).unwrap_or_default(),
        })),
    };
    let steps = [
        at(1, Phase::Beginning),
        at(1, Phase::FirstMain),
        at(2, Phase::Beginning),
        at(2, Phase::FirstMain),
    ];
    let ask = |pending: &baylee_engine::choice::Pending| Envelope {
        msg: Some(v1::envelope::Msg::ChoiceRequest(v1::ChoiceRequest {
            game_id: String::new(),
            seq: 9,
            pending_json: json(pending).into_bytes(),
        })),
    };
    vec![
        TapeMoment {
            input: None,
            at: 100,
            check: Check::Exact,
            frames: vec![
                Envelope {
                    msg: Some(v1::envelope::Msg::GameStatic(v1::GameStaticMsg {
                        game_id: String::new(),
                        view_version: VIEW,
                        static_json: json(&statics(2)).into_bytes(),
                    })),
                },
                delta(0, &steps[0], Some(tail(0, 2))),
                Envelope {
                    msg: Some(v1::envelope::Msg::Curtain(v1::Curtain {})),
                },
            ],
        },
        TapeMoment {
            input: Some(1),
            at: 200,
            check: Check::Seen,
            frames: vec![delta(1, &steps[1], None)],
        },
        TapeMoment {
            input: Some(2),
            at: 300,
            check: Check::Seen,
            frames: vec![delta(2, &steps[2], Some(tail(2, 3)))],
        },
        TapeMoment {
            input: Some(3),
            at: 400,
            check: Check::Unverified,
            frames: vec![delta(3, &steps[3], Some(tail(3, 6))), ask(&game_over())],
        },
    ]
}

fn header() -> Header {
    Header {
        tape: crate::replay::TAPE_VERSION,
        view: VIEW,
        build: "b".into(),
        source: Source::Local,
        perspective: crate::replay::TapePerspective::Seat,
        seat: 0,
        game: "g".into(),
        recorded_by: "b".into(),
        own: true,
        hands: Vec::new(),
    }
}

fn stop() -> StopLine {
    StopLine {
        stop: StopReason::End,
        inputs: 3,
        exact_to: Some(1),
        seen_to: None,
        diverged: None,
    }
}

/// Moments built into a tape are the tape that their written lines read
/// back as: the local door and the hosted door cannot differ.
#[test]
fn a_tape_from_moments_is_the_tape_its_lines_read_as() {
    let tape = Tape::from_moments(header(), moments(), stop()).expect("a tape");
    assert_eq!(tape.frames.len(), 4);
    assert_eq!(tape.curtain, Some((100, 1)));
    assert_eq!(tape.over_at, Some(400));
    assert_eq!(tape.stop, stop());
    assert_eq!(tape.log().len(), 6);
    assert_eq!(
        tape.frames.iter().map(|f| f.n).collect::<Vec<_>>(),
        [None, Some(1), Some(2), Some(3)]
    );
    assert_eq!(
        tape.frames.iter().map(|f| f.check).collect::<Vec<_>>(),
        [Check::Exact, Check::Seen, Check::Seen, Check::Unverified]
    );
    assert_eq!(Tape::read(&tape.write()).expect("read back"), tape);
}

/// A question that is not the end is not kept; the end is, with its time.
#[test]
fn from_moments_keeps_only_the_end_of_the_questions() {
    let ask = |pending: &baylee_engine::choice::Pending| Envelope {
        msg: Some(v1::envelope::Msg::ChoiceRequest(v1::ChoiceRequest {
            game_id: String::new(),
            seq: 1,
            pending_json: json(pending).into_bytes(),
        })),
    };
    let moment = |frames| TapeMoment {
        input: None,
        at: 77,
        check: Check::Exact,
        frames,
    };
    let other = baylee_engine::choice::Pending::ChoosePlayer {
        player: PlayerId::new(0),
        options: Vec::new(),
    };

    let tape = Tape::from_moments(header(), [moment(vec![ask(&other)])], stop()).expect("a tape");
    assert!(tape.over().is_none());
    assert_eq!(tape.over_at, None);

    let tape = Tape::from_moments(
        header(),
        [moment(vec![ask(&other), ask(&game_over())])],
        stop(),
    )
    .expect("a tape");
    assert!(is_game_over(
        &tape.over().expect("an end").expect("decodes")
    ));
    assert_eq!(tape.over_at, Some(77));
}

/// A static of another view version is refused, naming both versions.
#[test]
fn from_moments_refuses_a_static_of_another_view_version() {
    let moment = TapeMoment {
        input: None,
        at: 1,
        check: Check::Exact,
        frames: vec![Envelope {
            msg: Some(v1::envelope::Msg::GameStatic(v1::GameStaticMsg {
                game_id: String::new(),
                view_version: VIEW + 1,
                static_json: json(&statics(1)).into_bytes(),
            })),
        }],
    };
    assert_eq!(
        Tape::from_moments(header(), [moment], stop()).expect_err("refused"),
        TapeRefusal::OtherView {
            tape: VIEW + 1,
            ours: VIEW,
        }
    );
}

/// Two gzip members read as the tape they spell, and a tape cut inside its
/// last member reads the whole lines before the cut and says it was cut.
#[test]
fn a_multi_member_tape_reads_whole_and_a_torn_one_reads_cut() {
    let script = Script::new(with_lines(&[1, 2, 0, 1, 3]));
    let text = script.text();
    let whole = Tape::read_lines(text.as_bytes()).expect("a tape");
    assert_eq!(whole.stop.stop, StopReason::End);

    // Header, the first two steps | the last three and the stop line.
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let (first, second) = lines.split_at(3);
    let first = gzip(first.concat().as_bytes());
    let mut file = first.clone();
    file.extend(gzip(second.concat().as_bytes()));
    assert_eq!(Tape::read(&file).expect("two members"), whole);

    // Cut inside the second member: what had come out of it is read up to
    // its last whole line. Without the stop line (never inflated before the
    // last bytes) the tape says it was cut; it is never a tape that only
    // looks whole.
    let mut cut_short = 0;
    let mut fewer = 0;
    for cut in first.len() + 1..file.len() {
        let torn = Tape::read(&file[..cut]).expect("a torn tape still reads");
        match torn.stop.stop {
            StopReason::Cut => {
                cut_short += 1;
                assert_eq!(torn.stop, crate::replay::wire::cut(), "cut at {cut}");
                assert!(torn.frames.len() >= 2, "cut at {cut}");
                assert!(torn.frames.len() <= whole.frames.len(), "cut at {cut}");
                fewer += usize::from(torn.frames.len() < whole.frames.len());
                assert_eq!(
                    torn.frames[..],
                    whole.frames[..torn.frames.len()],
                    "cut at {cut}"
                );
            }
            // Only the trailer is missing: every line is there.
            StopReason::End => assert_eq!(torn, whole, "cut at {cut}"),
            other => panic!("cut at {cut} stops as {other:?}"),
        }
    }
    assert!(cut_short > 10, "{cut_short} cuts read as cut");
    assert!(fewer > 0, "no cut lost a step");
    // The second member missing altogether is two steps and a cut.
    let torn = Tape::read(&file[..first.len()]).expect("the first member");
    assert_eq!(torn.frames.len(), 2);
    assert_eq!(torn.stop, crate::replay::wire::cut());
}

/// A tape without its stop line reads as cut in transit, with no inputs.
#[test]
fn a_missing_stop_line_reads_as_cut() {
    let mut script = Script::new(with_lines(&[1, 1]));
    script.stop = None;
    let tape = script.tape();
    assert_eq!(tape.frames.len(), 2);
    assert_eq!(tape.stop, crate::replay::wire::cut());
    assert_eq!(tape.stop.stop, StopReason::Cut);
}

/// A stop line reads whole: the reason and the places the checks reached.
#[test]
fn a_stop_line_reads_whole() {
    let mut text = Script::new(with_lines(&[1])).text();
    text.truncate(text.rfind(r#"{"kind":"stop""#).expect("a stop line"));
    text.push_str(
        r#"{"kind":"stop","stop":"diverged","inputs":812,"exact_to":10,"seen_to":811,"diverged":{"input":12,"by":"hash"}}"#,
    );
    let tape = Tape::read_lines(text.as_bytes()).expect("a tape");
    assert_eq!(tape.stop.stop, StopReason::Diverged);
    assert_eq!(tape.stop.inputs, 812);
    assert_eq!(tape.stop.exact_to, Some(10));
    assert_eq!(tape.stop.seen_to, Some(811));
    let found = tape.stop.diverged.expect("a divergence");
    assert_eq!(found.input, Some(12));
    assert_eq!(found.by, crate::replay::wire::DivergedBy::Hash);
}

/// What a tape is refused for, and in whose words.
#[test]
fn a_tape_is_refused_for_its_version_or_its_lines() {
    let steps = Script::new(with_lines(&[1])).text();
    let body = steps.split_once('\n').expect("a header").1;
    let read = |header: String| Tape::read_lines(format!("{header}\n{body}").as_bytes());

    assert_eq!(
        read(header_line(2, VIEW, "seat", 0)).expect_err("another tape"),
        TapeRefusal::OtherTape(2)
    );
    let other = read(header_line(1, VIEW + 1, "seat", 0)).expect_err("another view");
    assert_eq!(
        other,
        TapeRefusal::OtherView {
            tape: VIEW + 1,
            ours: VIEW,
        }
    );
    let said = other.to_string();
    assert!(said.contains(&(VIEW + 1).to_string()), "{said}");
    assert!(said.contains(&VIEW.to_string()), "{said}");

    // A header too short to be a header still names its version.
    assert_eq!(
        Tape::read_lines(br#"{"kind":"tape","tape":2}"#).expect_err("named"),
        TapeRefusal::OtherTape(2)
    );
    assert_eq!(
        Tape::read_lines(format!(r#"{{"kind":"tape","tape":1,"view":{}}}"#, VIEW + 1).as_bytes())
            .expect_err("named"),
        TapeRefusal::OtherView {
            tape: VIEW + 1,
            ours: VIEW,
        }
    );
}

/// What is no tape says so: another file, an empty one, an empty gzip.
#[test]
fn what_is_no_tape_is_not_a_tape() {
    assert_eq!(
        Tape::read_lines(b"").expect_err("empty"),
        TapeRefusal::NotATape
    );
    assert_eq!(
        Tape::read_lines(b"\n \n").expect_err("blank"),
        TapeRefusal::NotATape
    );
    assert_eq!(
        Tape::read_lines(b"hello\n").expect_err("text"),
        TapeRefusal::NotATape
    );
    // A tape's line, but not its header.
    let body = Script::new(with_lines(&[0])).text();
    let first_view = body.lines().nth(1).expect("a view line");
    assert_eq!(
        Tape::read_lines(first_view.as_bytes()).expect_err("no header"),
        TapeRefusal::NotATape
    );
    assert_eq!(
        Tape::read(&gzip(b"")).expect_err("empty gzip"),
        TapeRefusal::NotATape
    );
    assert_eq!(
        Tape::read(b"not gzip at all").expect_err("not gzip"),
        TapeRefusal::NotATape
    );
}

/// A line this format has no kind for is named by its number.
#[test]
fn an_unknown_line_is_named_by_its_number() {
    let text = Script::new(with_lines(&[1, 1])).text();
    let mut lines: Vec<&str> = text.lines().collect();
    lines.insert(2, r#"{"kind":"bogus","at":1}"#);
    assert_eq!(
        Tape::read_lines(lines.join("\n").as_bytes()).expect_err("unreadable"),
        TapeRefusal::Unreadable { line: 3 }
    );
    // A view line without its seq is not a view line either.
    let mut lines: Vec<&str> = text.lines().collect();
    lines.insert(3, r#"{"kind":"view","at":1,"json":{}}"#);
    assert_eq!(
        Tape::read_lines(lines.join("\n").as_bytes()).expect_err("unreadable"),
        TapeRefusal::Unreadable { line: 4 }
    );
}

/// Two frames repeating one view and `seq` carry the parts of one long log
/// tail: one step, the whole log on it.
#[test]
fn split_log_tails_merge_into_one_step() {
    let spec = at(1, Phase::FirstMain);
    let view = json(&view_of(6, &spec, true));
    let head = header_line(1, VIEW, "seat", 0);
    let line = |seq: u64, view: &str, log: &LogTail| {
        format!(
            r#"{{"kind":"view","at":5,"n":1,"check":"exact","seq":{seq},"json":{view},"log":{}}}"#,
            json(log)
        )
    };
    let next = json(&view_of(7, &at(1, Phase::Combat), true));
    let text = [
        head,
        line(7, &view, &tail(0, 3)),
        line(7, &view, &tail(3, 5)),
        // The same view under another seq is another step.
        line(8, &next, &tail(5, 6)),
    ]
    .join("\n");
    let tape = Tape::read_lines(text.as_bytes()).expect("a tape");

    assert_eq!(tape.frames.len(), 2);
    assert_eq!(tape.frames[0].log_len, 5);
    assert_eq!(tape.frames[1].log_len, 6);
    assert_eq!(tape.log_to(0), tail(0, 5));
    assert_eq!(tape.tail_at(0), Some(tail(0, 5)));
    assert_eq!(tape.tail_at(1), Some(tail(5, 6)));
    // The same view under the same seq, with the same lines, written again
    // later is read back as the one it is.
    assert_eq!(Tape::read_lines(&tape.write_lines()).expect("back"), tape);
}

/// The log at step `i` is what a client appending tails by `from` holds
/// after step `i`; a tail is exactly what the step adds, none when it adds
/// nothing.
#[test]
fn the_log_at_a_step_is_what_a_client_appending_tails_holds() {
    let adds = [2, 0, 3, 1, 0, 0, 4];
    let tape = Script::new(with_lines(&adds)).tape();
    let mut held = Vec::new();
    let mut before = 0;
    for (i, added) in adds.iter().enumerate() {
        match tape.tail_at(i) {
            Some(tail) => {
                assert!(*added > 0, "step {i}");
                assert_eq!(tail.from as usize, before, "step {i}");
                assert_eq!(tail.entries, entries(before as u32, before as u32 + added));
                held.truncate(tail.from as usize);
                held.extend(tail.entries);
            }
            None => assert_eq!(*added, 0, "step {i}"),
        }
        before += *added as usize;
        assert_eq!(held.len(), before, "step {i}");
        let told = tape.log_to(i);
        assert_eq!(told.from, 0);
        assert_eq!(told.entries, held, "step {i}");
    }
    assert_eq!(tape.log(), &held[..]);
    // Past the last step there is nothing to say.
    assert_eq!(tape.tail_at(adds.len()), None);
}

/// A static is told at the first step after its line, and the static at a
/// step is the newest at or before it.
#[test]
fn statics_arrive_and_hold() {
    let mut script = Script::new(with_lines(&[0, 0, 0, 0, 0]));
    script.statics_before = vec![1, 3];
    let tape = script.tape();

    let arrives: Vec<bool> = (0..5).map(|i| tape.static_arrives_at(i)).collect();
    assert_eq!(arrives, [false, true, false, true, false]);

    let prints = |i| tape.statics_at(i).map(|s| s.expect("decodes").prints.len());
    assert_eq!(
        (0..5).map(prints).collect::<Vec<_>>(),
        [None, Some(1), Some(1), Some(2), Some(2)]
    );
    assert!(tape.statics_at(9).is_none());
}

/// Two statics before one step: the newest is the step's, and it arrives
/// once.
#[test]
fn two_statics_before_a_step_are_one_arrival_of_the_newest() {
    let mut script = Script::new(with_lines(&[0, 0, 0]));
    script.statics_before = vec![1, 1];
    let tape = script.tape();
    assert_eq!(
        (0..3)
            .map(|i| tape.static_arrives_at(i))
            .collect::<Vec<_>>(),
        [false, true, false]
    );
    assert_eq!(
        tape.statics_at(1)
            .expect("a static")
            .expect("decodes")
            .prints
            .len(),
        2
    );
    // Written and read back, both are still there, in order.
    let back = Tape::read_lines(&tape.write_lines()).expect("back");
    assert_eq!(back, tape);
}

/// A seat tape's view is the view as written, hand and all.
#[test]
fn a_seat_tape_shows_the_view_as_written() {
    let script = Script::new(with_lines(&[0, 0]));
    let tape = script.tape();
    for (i, spec) in script.steps.iter().enumerate() {
        let view = tape.view(i).expect("decodes");
        assert_eq!(view, view_of(i, spec, true));
        assert_eq!(view.hand.len(), 1);
    }
    assert!(tape.view(2).is_err());
}

/// The public table is a spectator's, drawn from seat 0, with no hand at
/// all.
#[test]
fn a_public_tape_shows_a_spectators_table_from_seat_zero() {
    let mut script = Script::new(vec![at(2, Phase::Combat), at(3, Phase::Ending)]);
    script.perspective = "public";
    let tape = script.tape();
    assert!(tape.public());
    for (i, spec) in script.steps.iter().enumerate() {
        let view = tape.view(i).expect("decodes");
        let expect = spectator_of(&view_of(i, spec, false)).into_player_view(PlayerId::new(0));
        assert_eq!(view, expect);
        assert_eq!(view.seat, PlayerId::new(0));
        assert!(view.hand.is_empty());
        assert_eq!(view.turn, spec.turn);
    }
    assert!(!Script::new(with_lines(&[0])).tape().public());
}

/// A switch of perspective keeps the place by input: the last step whose
/// input is at or before it, steps before any input counting.
#[test]
fn step_at_input_is_the_last_step_at_or_before_the_input() {
    let steps = vec![
        at(1, Phase::Beginning),
        at(1, Phase::FirstMain).n(1),
        at(1, Phase::Combat).n(1),
        at(2, Phase::Beginning).n(4),
        at(2, Phase::FirstMain).n(9),
    ];
    let tape = Script::new(steps).tape();
    assert_eq!(tape.step_at_input(None), 0);
    assert_eq!(tape.step_at_input(Some(0)), 0);
    assert_eq!(tape.step_at_input(Some(1)), 2);
    assert_eq!(tape.step_at_input(Some(3)), 2);
    assert_eq!(tape.step_at_input(Some(4)), 3);
    assert_eq!(tape.step_at_input(Some(8)), 3);
    assert_eq!(tape.step_at_input(Some(9)), 4);
    assert_eq!(tape.step_at_input(Some(1000)), 4);
}

/// The frames keep what the lines said of them.
#[test]
fn frames_keep_their_time_input_check_seq_and_place() {
    let tape = busy().tape();
    let f = &tape.frames[3];
    assert_eq!(f.at, 1030);
    assert_eq!(f.n, Some(4));
    assert_eq!(f.check, Check::Unverified);
    assert_eq!(f.seq, 4);
    assert_eq!(f.place.turn, 2);
    assert_eq!(f.place.phase, Phase::Beginning);
    assert_eq!(f.place.active, PlayerId::new(0));
    assert_eq!(tape.curtain, Some((1010, 1)));
    assert_eq!(tape.over_at, Some(1050));
    assert!(is_game_over(
        &tape.over().expect("an end").expect("decodes")
    ));
}

/// The payloads stay as the wire wrote them (§4: "never re-encoded"): a tape
/// built from a view as a serialiser spells it (keys in field order) writes
/// and reads back as the same tape.
#[test]
fn a_written_tape_keeps_its_payloads_as_written() {
    let spec = at(1, Phase::FirstMain);
    let moment = TapeMoment {
        input: None,
        at: 5,
        check: Check::Exact,
        frames: vec![Envelope {
            msg: Some(v1::envelope::Msg::StateDelta(v1::StateDelta {
                game_id: String::new(),
                seq: 1,
                // A serialiser's spelling, not the sorted one.
                view_json: serde_json::to_vec(&view_of(0, &spec, true)).expect("writes"),
                log_json: Vec::new(),
            })),
        }],
    };
    let tape = Tape::from_moments(header(), [moment], stop()).expect("a tape");
    assert_eq!(Tape::read(&tape.write()).expect("read back"), tape);
}

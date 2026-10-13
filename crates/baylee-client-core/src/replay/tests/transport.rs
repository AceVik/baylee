//! The transport: what each command shows, how a replay paces itself.

use std::time::Duration;

use baylee_view::Phase;

use super::{at, timeline_of, turns_of_three, with_lines};
use crate::replay::timeline::Timeline;
use crate::replay::transport::{Command, Out, Speed, Transport, dwell};

/// `n` quiet steps in one turn.
fn quiet(n: usize) -> Timeline {
    timeline_of(with_lines(&vec![0; n]))
}

/// Ticks `transport` by `dt` until it shows something, at most `limit`
/// times: how many ticks it took, and what it showed.
fn until_shown(
    transport: &mut Transport,
    timeline: &Timeline,
    dt: Duration,
    limit: usize,
) -> (usize, Option<Out>) {
    for tick in 1..=limit {
        if let Some(out) = transport.apply(Command::Tick(dt), timeline) {
            return (tick, Some(out));
        }
    }
    (limit, None)
}

fn near(a: Duration, b: f32) -> bool {
    (a.as_secs_f32() - b).abs() < 0.001
}

/// One step forward is a `Show` (shown as live); one back is a `Jump`.
#[test]
fn one_forward_shows_and_one_back_jumps() {
    let timeline = quiet(5);
    let mut transport = Transport::at(2);
    assert_eq!(
        transport.apply(Command::Step(1), &timeline),
        Some(Out::Show(3))
    );
    assert_eq!(transport.at, 3);
    assert_eq!(
        transport.apply(Command::Step(-1), &timeline),
        Some(Out::Jump(2))
    );
    assert_eq!(transport.at, 2);
    // Further than one is a jump, and clamps.
    assert_eq!(
        transport.apply(Command::Step(2), &timeline),
        Some(Out::Jump(4))
    );
    assert_eq!(
        transport.apply(Command::Step(-5), &timeline),
        Some(Out::Jump(0))
    );
}

/// The ends hold: back at 0 and forward at the last step show nothing and
/// stay.
#[test]
fn stepping_past_the_ends_does_nothing() {
    let timeline = quiet(3);
    let mut transport = Transport::at(0);
    assert_eq!(transport.apply(Command::Step(-1), &timeline), None);
    assert_eq!(transport.at, 0);
    let mut transport = Transport::at(2);
    assert_eq!(transport.apply(Command::Step(1), &timeline), None);
    assert_eq!(transport.at, 2);
}

/// A step, forward or back, at the ends or not, pauses.
#[test]
fn a_step_pauses() {
    let timeline = quiet(4);
    for (from, by) in [(1, 1), (1, -1), (0, -1), (3, 1)] {
        let mut transport = Transport::at(from);
        transport.apply(Command::Play, &timeline);
        assert!(transport.playing, "playing from {from}");
        transport.apply(Command::Step(by), &timeline);
        assert!(!transport.playing, "step {by} from {from}");
    }
}

/// Every move that is not one step on is a jump, and a jump to where the
/// replay stands is no move.
#[test]
fn turns_phases_seeks_and_the_ends_jump() {
    let timeline = timeline_of(turns_of_three(3));
    let last = timeline.last();
    assert_eq!(last, 8);

    let mut t = Transport::at(1);
    assert_eq!(t.apply(Command::Turn(1), &timeline), Some(Out::Jump(3)));
    assert_eq!(t.apply(Command::Turn(-1), &timeline), Some(Out::Jump(0)));
    assert_eq!(t.apply(Command::Turn(2), &timeline), Some(Out::Jump(6)));
    assert_eq!(t.apply(Command::NextPhase, &timeline), Some(Out::Jump(7)));
    assert_eq!(t.apply(Command::PrevPhase, &timeline), Some(Out::Jump(6)));
    assert_eq!(t.apply(Command::PrevPhase, &timeline), Some(Out::Jump(5)));
    assert_eq!(t.apply(Command::Seek(2), &timeline), Some(Out::Jump(2)));
    assert_eq!(t.apply(Command::Seek(2), &timeline), None);
    assert_eq!(t.apply(Command::Seek(99), &timeline), Some(Out::Jump(last)));
    assert_eq!(t.at, last);
    assert_eq!(t.apply(Command::Start, &timeline), Some(Out::Jump(0)));
    assert_eq!(t.apply(Command::Start, &timeline), None);
    assert_eq!(t.apply(Command::End, &timeline), Some(Out::Jump(last)));
    assert_eq!(t.apply(Command::End, &timeline), None);
}

/// A turn on from the last turn goes to the last step; back from the first
/// step goes nowhere; a turn pauses.
#[test]
fn a_turn_past_the_ends_clamps_and_pauses() {
    let timeline = timeline_of(turns_of_three(2));
    let mut t = Transport::at(4);
    assert_eq!(t.apply(Command::Turn(1), &timeline), Some(Out::Jump(5)));
    assert_eq!(t.apply(Command::Turn(1), &timeline), None);
    let mut t = Transport::at(0);
    assert_eq!(t.apply(Command::Turn(-1), &timeline), None);
    t.apply(Command::Play, &timeline);
    assert!(t.playing);
    t.apply(Command::Turn(1), &timeline);
    assert!(!t.playing);
}

/// Seeking, jumping to the end and the start while playing keep or end the
/// play as the end does: the end pauses.
#[test]
fn the_end_pauses() {
    let timeline = quiet(4);
    let mut t = Transport::at(0);
    t.apply(Command::Play, &timeline);
    t.apply(Command::End, &timeline);
    assert!(!t.playing);
}

/// How long a step stays: 0.3 s bare, 0.9 s with a line, a quarter second
/// per line more up to 2 s, 0.6 s more at a turn's first step.
#[test]
fn a_steps_dwell_grows_with_its_lines_and_its_turn() {
    let timeline = timeline_of(vec![
        at(1, Phase::FirstMain),
        at(1, Phase::FirstMain),
        at(1, Phase::FirstMain).lines(1),
        at(1, Phase::FirstMain).lines(2),
        at(1, Phase::FirstMain).lines(5),
        at(1, Phase::FirstMain).lines(6),
        at(2, Phase::FirstMain).lines(1),
        at(2, Phase::FirstMain).lines(30),
        at(3, Phase::FirstMain),
    ]);
    let expect = [0.9, 0.3, 0.9, 1.15, 1.9, 2.0, 1.5, 2.0, 0.9];
    for (i, secs) in expect.iter().enumerate() {
        let got = dwell(&timeline, i);
        assert!(near(got, *secs), "step {i}: {got:?}, wanted {secs}");
    }
    // A step the timeline does not have has no lines and is no turn's start.
    assert!(near(dwell(&timeline, 40), 0.3));
}

/// At 1× a step is shown when its dwell has passed, and not a tick before.
#[test]
fn at_one_times_a_step_follows_when_its_dwell_has_passed() {
    // Step 0 opens the turn with a line: 0.9 + 0.6 = 1.5 s. Step 1: 0.3 s.
    let timeline = timeline_of(vec![
        at(1, Phase::FirstMain).lines(1),
        at(1, Phase::FirstMain),
        at(1, Phase::FirstMain),
    ]);
    let tick = Duration::from_millis(100);
    let mut t = Transport::at(0);
    t.apply(Command::Play, &timeline);
    let (ticks, shown) = until_shown(&mut t, &timeline, tick, 100);
    assert_eq!((ticks, shown), (15, Some(Out::Show(1))));
    assert_eq!(t.at, 1);

    // Step 1 is bare, 0.3 s: a tick of 0.29 s is too early, 0.01 s more is
    // not.
    assert_eq!(
        t.apply(Command::Tick(Duration::from_millis(290)), &timeline),
        None
    );
    assert_eq!(
        t.apply(Command::Tick(Duration::from_millis(20)), &timeline),
        Some(Out::Show(2))
    );
}

/// The dwell of a step with one line and no turn opening: nine ticks of a
/// tenth, never eight.
#[test]
fn a_step_with_a_line_stays_nine_tenths() {
    let timeline = timeline_of(vec![
        at(1, Phase::FirstMain),
        at(1, Phase::FirstMain).lines(1),
        at(1, Phase::FirstMain),
        at(1, Phase::FirstMain),
    ]);
    let mut t = Transport::at(1);
    t.apply(Command::Play, &timeline);
    let tick = Duration::from_millis(100);
    assert_eq!(
        until_shown(&mut t, &timeline, tick, 100),
        (9, Some(Out::Show(2)))
    );
}

/// At 8× the same dwell takes an eighth of the time; at 0.5× twice as long.
#[test]
fn speed_divides_the_dwell() {
    let timeline = timeline_of(vec![
        at(1, Phase::FirstMain),
        at(1, Phase::FirstMain).lines(1),
        at(1, Phase::FirstMain),
        at(1, Phase::FirstMain),
    ]);
    // 0.9 s of dwell: nine ticks of a tenth at 1× ...
    let mut t = Transport::at(1);
    t.apply(Command::Play, &timeline);
    let one = Duration::from_millis(100);
    assert_eq!(until_shown(&mut t, &timeline, one, 100).0, 9);

    // ... nine ticks of an eighth of that at 8×.
    let mut t = Transport::at(1);
    t.speed = Speed::Eight;
    t.apply(Command::Play, &timeline);
    let eighth = Duration::from_micros(12_500);
    assert_eq!(until_shown(&mut t, &timeline, eighth, 100).0, 9);
    // Where a tenth of a second at 8× is 0.8 s of dwell: not enough, then
    // enough.
    let mut t = Transport::at(1);
    t.speed = Speed::Eight;
    t.apply(Command::Play, &timeline);
    assert_eq!(until_shown(&mut t, &timeline, one, 100).0, 2);

    // At 0.5× nine tenths takes eighteen ticks of a tenth.
    let mut t = Transport::at(1);
    t.speed = Speed::Half;
    t.apply(Command::Play, &timeline);
    assert_eq!(until_shown(&mut t, &timeline, one, 100).0, 18);
}

/// A paused replay does not move on ticks.
#[test]
fn a_paused_replay_ignores_ticks() {
    let timeline = quiet(3);
    let mut t = Transport::at(0);
    assert_eq!(
        t.apply(Command::Tick(Duration::from_secs(60)), &timeline),
        None
    );
    assert_eq!(t.at, 0);
}

/// Playing reaches the last step and pauses by itself; a long frame does not
/// skip a step nobody saw; later ticks show nothing.
#[test]
fn playing_reaches_the_last_step_and_pauses() {
    let timeline = quiet(3);
    let mut t = Transport::at(0);
    t.apply(Command::Play, &timeline);
    let long = Duration::from_secs(100);
    assert_eq!(t.apply(Command::Tick(long), &timeline), Some(Out::Show(1)));
    assert!(t.playing);
    assert_eq!(t.apply(Command::Tick(long), &timeline), Some(Out::Show(2)));
    assert!(!t.playing, "the last step pauses");
    assert_eq!(t.apply(Command::Tick(long), &timeline), None);
    assert_eq!(t.at, 2);
}

/// Play or toggle at the last step starts over from the first; toggle
/// elsewhere flips playing and shows nothing.
#[test]
fn play_at_the_end_starts_over_and_toggle_flips() {
    let timeline = quiet(4);
    for command in [Command::Toggle, Command::Play] {
        let mut t = Transport::at(3);
        assert_eq!(
            t.apply(command, &timeline),
            Some(Out::Jump(0)),
            "{command:?}"
        );
        assert!(t.playing, "{command:?}");
        assert_eq!(t.at, 0);
    }
    let mut t = Transport::at(1);
    assert_eq!(t.apply(Command::Toggle, &timeline), None);
    assert!(t.playing);
    assert_eq!(t.apply(Command::Toggle, &timeline), None);
    assert!(!t.playing);
    assert_eq!(t.apply(Command::Play, &timeline), None);
    assert!(t.playing);
    assert_eq!(t.apply(Command::Pause, &timeline), None);
    assert!(!t.playing);
    assert_eq!(t.at, 1);
}

/// With nothing to play a transport shows nothing and does not play.
#[test]
fn an_empty_timeline_plays_nothing() {
    let mut t = Transport::default();
    let empty = Timeline::default();
    for command in [
        Command::Play,
        Command::Step(1),
        Command::End,
        Command::Seek(3),
    ] {
        assert_eq!(t.apply(command, &empty), None, "{command:?}");
        assert!(!t.playing);
    }
}

/// Faster and slower walk 0.5, 1, 2, 4, 8 and stop at the ends.
#[test]
fn speeds_walk_their_ladder_and_stop_at_its_ends() {
    let timeline = quiet(3);
    let mut t = Transport::default();
    assert_eq!(t.speed, Speed::One);
    let mut seen = vec![t.speed];
    for _ in 0..5 {
        assert_eq!(t.apply(Command::Faster, &timeline), None);
        seen.push(t.speed);
    }
    assert_eq!(
        seen,
        [
            Speed::One,
            Speed::Two,
            Speed::Four,
            Speed::Eight,
            Speed::Eight,
            Speed::Eight
        ]
    );
    let mut down = vec![];
    for _ in 0..6 {
        t.apply(Command::Slower, &timeline);
        down.push(t.speed);
    }
    assert_eq!(
        down,
        [
            Speed::Four,
            Speed::Two,
            Speed::One,
            Speed::Half,
            Speed::Half,
            Speed::Half
        ]
    );
    assert_eq!(Speed::ALL.map(Speed::factor), [0.5, 1.0, 2.0, 4.0, 8.0]);
    assert_eq!(
        Speed::ALL.map(Speed::label),
        ["0.5×", "1×", "2×", "4×", "8×"]
    );
}

/// Cues sound up to 2× and not at 4× and 8×.
#[test]
fn cues_sound_up_to_two_times() {
    assert_eq!(
        Speed::ALL.map(Speed::cues),
        [true, true, true, false, false]
    );
}

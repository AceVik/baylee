//! The timeline: turns, phases, bands, the scrubber's two maps.

use baylee_core::ids::PlayerId;
use baylee_view::Phase;

use super::{Script, at, timeline_of, turns_of_three, with_lines};
use crate::replay::timeline::{Band, Timeline, Turn};
use crate::replay::{Check, StopReason};

/// Seven steps over three turns; turn 2 opens in the phase turn 1 ended in,
/// so only the turn tells a new phase there.
///
/// ```text
/// step  0     1     2     3     4     5     6
/// turn  1     1     1     2     2     2     3
/// phase Beg   Beg   Main  Main  Main  Comb  Beg
/// ```
fn seven() -> Timeline {
    timeline_of(vec![
        at(1, Phase::Beginning),
        at(1, Phase::Beginning),
        at(1, Phase::FirstMain),
        at(2, Phase::FirstMain),
        at(2, Phase::FirstMain),
        at(2, Phase::Combat),
        at(3, Phase::Beginning),
    ])
}

/// Consecutive steps of one turn number are one cell of the scrubber.
#[test]
fn consecutive_steps_of_a_turn_form_one_cell() {
    let timeline = seven();
    assert_eq!(timeline.len(), 7);
    assert_eq!(timeline.last(), 6);
    assert_eq!(
        timeline.turns(),
        [
            Turn {
                turn: 1,
                active: PlayerId::new(1),
                first: 0,
                len: 3,
            },
            Turn {
                turn: 2,
                active: PlayerId::new(0),
                first: 3,
                len: 3,
            },
            Turn {
                turn: 3,
                active: PlayerId::new(1),
                first: 6,
                len: 1,
            },
        ]
    );
    assert_eq!(timeline.turns()[1].end(), 6);
    assert_eq!(
        (0..7)
            .map(|i| timeline.turn_of(i).map(|t| t.turn))
            .collect::<Vec<_>>(),
        [1, 1, 1, 2, 2, 2, 3].map(Some)
    );
    assert!(
        timeline.turn_of(7).is_some_and(|t| t.turn == 3),
        "past the end is the last turn"
    );
    assert_eq!(
        (0..7).map(|i| timeline.starts_turn(i)).collect::<Vec<_>>(),
        [true, false, false, true, false, false, true]
    );
}

/// Back mid-turn goes to this turn's first step, back at a turn's first goes
/// to the previous turn's, back at the very first goes nowhere.
#[test]
fn prev_turn_is_this_turns_start_then_the_one_before() {
    let timeline = seven();
    let prev: Vec<_> = (0..7).map(|i| timeline.prev_turn(i)).collect();
    assert_eq!(
        prev,
        [None, Some(0), Some(0), Some(0), Some(3), Some(3), Some(3)]
    );
}

/// Forward goes to the next turn's first step; from the last turn nowhere.
#[test]
fn next_turn_is_the_next_turns_start_and_none_from_the_last() {
    let timeline = seven();
    let next: Vec<_> = (0..7).map(|i| timeline.next_turn(i)).collect();
    assert_eq!(
        next,
        [Some(3), Some(3), Some(3), Some(6), Some(6), Some(6), None]
    );
}

/// Phases begin where the phase or the turn changes, and every turn's first
/// step is one, even when the phase is the one the turn before ended in.
#[test]
fn phases_begin_where_phase_or_turn_changes() {
    let timeline = seven();
    assert_eq!(timeline.phases(), [0, 2, 3, 5, 6]);
    for turn in timeline.turns() {
        assert!(
            timeline.phases().contains(&turn.first),
            "turn {}",
            turn.turn
        );
    }
}

/// Next phase is the next phase's first step; back is this phase's first
/// step mid-phase, the previous phase's at a phase's first, nothing at 0.
#[test]
fn next_and_prev_phase_walk_the_phase_starts() {
    let timeline = seven();
    assert_eq!(
        (0..7).map(|i| timeline.next_phase(i)).collect::<Vec<_>>(),
        [Some(2), Some(2), Some(3), Some(5), Some(5), Some(6), None]
    );
    assert_eq!(
        (0..7).map(|i| timeline.prev_phase(i)).collect::<Vec<_>>(),
        [None, Some(0), Some(0), Some(2), Some(3), Some(3), Some(5)]
    );
}

/// A step's mark holds its place, its check, the lines it adds and its
/// input.
#[test]
fn a_mark_holds_place_check_lines_and_input() {
    let timeline = timeline_of(vec![
        at(1, Phase::Beginning).lines(2).n(1),
        at(1, Phase::FirstMain).check(Check::Seen).n(4),
        at(2, Phase::Combat).lines(3).check(Check::Unverified),
    ]);
    let marks: Vec<_> = (0..3).map(|i| *timeline.step(i).expect("a step")).collect();
    assert_eq!(marks.iter().map(|m| m.lines).collect::<Vec<_>>(), [2, 0, 3]);
    assert_eq!(
        marks.iter().map(|m| m.n).collect::<Vec<_>>(),
        [Some(1), Some(4), None]
    );
    assert_eq!(
        marks.iter().map(|m| m.check).collect::<Vec<_>>(),
        [Check::Exact, Check::Seen, Check::Unverified]
    );
    assert_eq!(marks[2].place.turn, 2);
    assert_eq!(marks[2].place.phase, Phase::Combat);
    assert!(timeline.step(3).is_none());
}

/// Runs of seen and of unverified steps are bands; exact steps make none;
/// two weaker checks side by side are two bands.
#[test]
fn weaker_runs_are_bands_and_exact_steps_are_not() {
    use Check::{Exact, Seen, Unverified};
    let checks = [
        Exact, Seen, Seen, Unverified, Unverified, Exact, Seen, Exact, Unverified,
    ];
    let steps = checks
        .iter()
        .map(|c| at(1, Phase::FirstMain).check(*c))
        .collect();
    let timeline = timeline_of(steps);
    assert_eq!(
        timeline.bands(),
        [
            Band {
                from: 1,
                to: 3,
                check: Seen,
            },
            Band {
                from: 3,
                to: 5,
                check: Unverified,
            },
            Band {
                from: 6,
                to: 7,
                check: Seen,
            },
            Band {
                from: 8,
                to: 9,
                check: Unverified,
            },
        ]
    );
    assert!(timeline_of(with_lines(&[0, 1, 2])).bands().is_empty());
}

/// A band reaching the last step ends at the length, one past the last.
#[test]
fn a_band_to_the_last_step_ends_at_the_length() {
    let timeline = timeline_of(vec![
        at(1, Phase::FirstMain),
        at(1, Phase::FirstMain).check(Check::Seen),
        at(1, Phase::FirstMain).check(Check::Seen),
    ]);
    assert_eq!(
        timeline.bands(),
        [Band {
            from: 1,
            to: 3,
            check: Check::Seen,
        }]
    );
}

/// The scrubber's ends are the first and last step, and the middle of every
/// step maps back to the step, however many there are.
#[test]
fn the_scrubber_maps_fractions_to_steps_and_back() {
    for count in [1, 2, 3, 7, 10, 13, 40] {
        let timeline = timeline_of(with_lines(&vec![0; count]));
        assert_eq!(timeline.step_at(0.0), 0, "{count} steps");
        assert_eq!(timeline.step_at(1.0), count - 1, "{count} steps");
        assert_eq!(timeline.step_at(-3.0), 0, "{count} steps");
        assert_eq!(timeline.step_at(7.0), count - 1, "{count} steps");
        let mut before = -1.0;
        for i in 0..count {
            let f = timeline.fraction_of(i);
            assert!(f > before && (0.0..=1.0).contains(&f), "{count} steps, {i}");
            before = f;
            assert_eq!(timeline.step_at(f), i, "{count} steps, step {i}");
        }
    }
}

/// An empty timeline has no steps and maps everything to 0.
#[test]
fn an_empty_timeline_maps_to_zero() {
    let timeline = Timeline::default();
    assert!(timeline.is_empty());
    assert_eq!(timeline.len(), 0);
    assert_eq!(timeline.last(), 0);
    assert_eq!(timeline.step_at(0.5), 0);
    assert!(timeline.turns().is_empty());
    assert_eq!(timeline.next_turn(0), None);
}

/// A tape that stops at the game's end has nothing to say about stopping;
/// every other stop is named.
#[test]
fn stopped_names_every_stop_but_the_end() {
    let stopped = |stop| {
        let mut script = Script::new(turns_of_three(1));
        script.stop = stop;
        Timeline::of(&script.tape()).stopped()
    };
    assert_eq!(stopped(Some("end")), None);
    assert_eq!(stopped(Some("cut")), Some(StopReason::Cut));
    assert_eq!(stopped(Some("diverged")), Some(StopReason::Diverged));
    assert_eq!(stopped(Some("refused")), Some(StopReason::Refused));
    assert_eq!(stopped(Some("cancelled")), Some(StopReason::Cancelled));
    assert_eq!(stopped(Some("too_large")), Some(StopReason::TooLarge));
    // No stop line at all is a cut.
    assert_eq!(stopped(None), Some(StopReason::Cut));
}

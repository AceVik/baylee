//! Tests of the replay module (`.claude/b8/REPLAY-DESIGN.md` §13, Lane B).
//!
//! The helpers here write tapes as JSON Lines text, the way the wire does,
//! so a test reads the tape through the same door a gateway's does.

mod library;
mod tape;
mod timeline;
mod transport;

use baylee_core::ids::PlayerId;
use baylee_view::{LogEntry, LogTail, Phase, PlayerView, Step};

use super::{Check, Tape};
use crate::test_support::{ViewBuilder, revealed_line, statics};

/// The view version this build reads.
pub(super) const VIEW: u32 = baylee_view::VIEW_VERSION;

/// The log line number `k`: unique by its object slot and its time.
pub(super) fn entry(k: u32) -> LogEntry {
    let mut line = revealed_line(0, k, 0);
    line.at = u64::from(k);
    line
}

/// The log lines `from..to`.
pub(super) fn entries(from: u32, to: u32) -> Vec<LogEntry> {
    (from..to).map(entry).collect()
}

/// A tail of lines `from..to`, told from `from`.
pub(super) fn tail(from: u32, to: u32) -> LogTail {
    LogTail {
        from,
        entries: entries(from, to),
    }
}

/// `value` as one line of JSON in the writer's own spelling: keys sorted,
/// nothing between tokens. A tape built from it reads back byte for byte as
/// the tape writes it (see `tape::a_written_tape_keeps_its_payloads_as_written`
/// for the spelling a serialiser gives a struct).
pub(super) fn json<T: serde::Serialize>(value: &T) -> String {
    let value = serde_json::to_value(value).expect("serialises");
    serde_json::to_string(&value).expect("writes")
}

/// The header line of a tape.
pub(super) fn header_line(tape: u32, view: u32, perspective: &str, seat: u8) -> String {
    format!(
        r#"{{"kind":"tape","tape":{tape},"view":{view},"build":"b","source":"local","perspective":"{perspective}","seat":{seat},"game":"g","recorded_by":"b"}}"#
    )
}

/// A step of a scripted tape: where the game stands and what the step adds.
#[derive(Clone, Copy)]
pub(super) struct Spec {
    pub turn: u32,
    pub phase: Phase,
    pub check: Check,
    /// How many log lines the step adds.
    pub lines: u32,
    pub n: Option<u64>,
}

/// A step in `turn`'s `phase`: exact, no lines, no input.
pub(super) fn at(turn: u32, phase: Phase) -> Spec {
    Spec {
        turn,
        phase,
        check: Check::Exact,
        lines: 0,
        n: None,
    }
}

impl Spec {
    pub(super) fn check(mut self, check: Check) -> Self {
        self.check = check;
        self
    }

    pub(super) fn lines(mut self, lines: u32) -> Self {
        self.lines = lines;
        self
    }

    pub(super) fn n(mut self, n: u64) -> Self {
        self.n = Some(n);
        self
    }
}

/// Steps in turn 1's main phase, one per entry of `lines`, each adding that
/// many log lines.
pub(super) fn with_lines(lines: &[u32]) -> Vec<Spec> {
    lines
        .iter()
        .map(|k| at(1, Phase::FirstMain).lines(*k))
        .collect()
}

/// One step per turn, `turns` in all, three steps (beginning, main, combat)
/// each.
pub(super) fn turns_of_three(turns: u32) -> Vec<Spec> {
    (1..=turns)
        .flat_map(|t| {
            [
                at(t, Phase::Beginning),
                at(t, Phase::FirstMain),
                at(t, Phase::Combat),
            ]
        })
        .collect()
}

/// The view step `i` shows: seq `i + 1`, the spec's place, active seat
/// `turn % 2`, a hand when the tape is a seat's.
pub(super) fn view_of(i: usize, spec: &Spec, hand: bool) -> PlayerView {
    let builder = ViewBuilder::new(2);
    let builder = if hand {
        builder.with_hand(vec![("Lightning Bolt", 1, 5)])
    } else {
        builder
    };
    let mut view = builder.build();
    view.seq = i as u64 + 1;
    view.turn = spec.turn;
    view.phase = spec.phase;
    view.step = match spec.phase {
        Phase::Beginning => Step::Upkeep,
        Phase::FirstMain | Phase::SecondMain => Step::Main,
        Phase::Combat => Step::DeclareAttackers,
        Phase::Ending => Step::End,
    };
    view.active = PlayerId::new((spec.turn % 2) as u8);
    view
}

/// What the spectators are told of `view`: its public half.
pub(super) fn spectator_of(view: &PlayerView) -> baylee_view::SpectatorView {
    baylee_view::SpectatorView {
        seq: view.seq,
        turn: view.turn,
        phase: view.phase,
        step: view.step,
        active: view.active,
        awaiting: view.awaiting,
        deciding: view.deciding,
        monarch: view.monarch,
        day_night: view.day_night,
        seats: view.seats.clone(),
        battlefield: view.battlefield.clone(),
        stack: view.stack.clone(),
        graveyards: view.graveyards.clone(),
        exile: view.exile.clone(),
        command: view.command.clone(),
        combat: view.combat.clone(),
        library_tops: view.library_tops.clone(),
        revealed_hands: view.shared_hands.clone(),
    }
}

/// The game's result, as the tape's `over` line carries it.
pub(super) fn result() -> baylee_engine::win::GameResult {
    baylee_engine::win::GameResult {
        winner: Some(baylee_engine::win::Victor::Player(PlayerId::new(0))),
        reason: baylee_engine::win::EndReason::LastPlayerStanding,
    }
}

/// The game's end, as the tape's `over` line carries it.
pub(super) fn game_over() -> baylee_engine::choice::Pending {
    baylee_engine::choice::Pending::GameOver(result())
}

/// Whether `pending` is the end of the game with [`result`].
pub(super) fn is_game_over(pending: &baylee_engine::choice::Pending) -> bool {
    matches!(pending, baylee_engine::choice::Pending::GameOver(r) if *r == result())
}

/// A tape as text, built from steps.
pub(super) struct Script {
    pub steps: Vec<Spec>,
    /// Before which steps a `static` line comes (the k-th one has `k + 1`
    /// prints).
    pub statics_before: Vec<usize>,
    /// Before which step the curtain line comes (`steps.len()`: after all).
    pub curtain_before: Option<usize>,
    /// Whether an `over` line comes.
    pub over: bool,
    /// The `stop` line's reason (`None`: no stop line, a tape cut in transit).
    pub stop: Option<&'static str>,
    /// `seat` or `public`.
    pub perspective: &'static str,
}

impl Script {
    pub(super) fn new(steps: Vec<Spec>) -> Self {
        Self {
            steps,
            statics_before: Vec::new(),
            curtain_before: None,
            over: false,
            stop: Some("end"),
            perspective: "seat",
        }
    }

    pub(super) fn text(&self) -> String {
        let public = self.perspective == "public";
        let mut out = vec![header_line(1, VIEW, self.perspective, 0)];
        let mut told = 0;
        let mut statics_sent = 0u16;
        for (i, spec) in self.steps.iter().enumerate() {
            let at = 1000 + 10 * i as u64;
            if self.curtain_before == Some(i) {
                out.push(format!(r#"{{"kind":"curtain","at":{at}}}"#));
            }
            for _ in self.statics_before.iter().filter(|b| **b == i) {
                statics_sent += 1;
                out.push(format!(
                    r#"{{"kind":"static","at":{at},"json":{}}}"#,
                    json(&statics(statics_sent))
                ));
            }
            let view = view_of(i, spec, !public);
            let view = if public {
                json(&spectator_of(&view))
            } else {
                json(&view)
            };
            let log = if spec.lines == 0 {
                "null".to_owned()
            } else {
                let t = tail(told, told + spec.lines);
                told += spec.lines;
                json(&t)
            };
            let n = spec.n.map_or_else(|| "null".to_owned(), |n| n.to_string());
            let check = match spec.check {
                Check::Exact => "exact",
                Check::Seen => "seen",
                Check::Unverified => "unverified",
            };
            out.push(format!(
                r#"{{"kind":"view","at":{at},"n":{n},"check":"{check}","seq":{},"json":{view},"log":{log}}}"#,
                i + 1
            ));
        }
        let end = 1000 + 10 * self.steps.len() as u64;
        if self.curtain_before == Some(self.steps.len()) {
            out.push(format!(r#"{{"kind":"curtain","at":{end}}}"#));
        }
        if self.over {
            out.push(format!(
                r#"{{"kind":"over","at":{end},"json":{}}}"#,
                json(&game_over())
            ));
        }
        if let Some(stop) = self.stop {
            out.push(format!(
                r#"{{"kind":"stop","stop":"{stop}","inputs":9,"exact_to":2,"seen_to":null,"diverged":null}}"#
            ));
        }
        let mut text = out.join("\n");
        text.push('\n');
        text
    }

    pub(super) fn tape(&self) -> Tape {
        Tape::read_lines(self.text().as_bytes()).expect("a tape")
    }
}

/// The timeline of a tape of `steps`.
pub(super) fn timeline_of(steps: Vec<Spec>) -> super::Timeline {
    super::Timeline::of(&Script::new(steps).tape())
}

//! The question boundary (commits c55a8c6eb and a5eb549ca): records written
//! before it.
//!
//! From c55a8c6eb the engine no longer asks for an order between damage
//! effects that cannot change the event (protection on two creatures, a
//! redirection onto protection: `damage::order_matters`), and from a5eb549ca
//! it asks nothing once the game is decided (`end_a_decided_game`). Neither
//! moves a hash at a position both builds reach, but a record written before
//! holds answers to those questions. A later build no longer asks them: its
//! hash parts from the record at the answer before a removed damage-order
//! question and meets it again after the record's answers to it, and it is
//! over where the record still answers. Such a record is converted by the
//! runtime that wrote it (`docs/engine-internals.md` §"The question
//! boundary"), so the audit names the boundary instead of a divergence.
//! `engine::recorded_lockstep_tests` finds the records a set holds that
//! cross it.

use super::{Line, PlayerId};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::Engine;
use baylee_gamehost::RegistryLookup;

/// The first commit of the boundary: no damage-order question that cannot
/// change the event. The second, a5eb549ca, asks nothing in a decided game.
pub const QUESTION_BOUNDARY: &str = "c55a8c6eb";

/// How a refusal of a record from before [`QUESTION_BOUNDARY`] begins, when
/// it opens with this build's hash: the conversion stops on it rather than
/// leaving the game out, and `--llm-bc-skip-unreplayable` does not take it.
pub const QUESTION_REFUSAL: &str = "record written before question boundary c55a8c6eb:";

/// The rule a refusal names.
pub const QUESTION_RULE: &str =
    "convert records written before c55a8c6eb with the runtime that wrote them";

/// Which digest a record is held to: the engine hash (an exact audit) or
/// the public table (`seen`, a record of another build).
#[derive(Clone, Copy)]
enum Digest {
    Exact,
    Seen,
}

/// What a record from before the boundary holds that this build no longer
/// asks.
struct Crossing {
    build: String,
    /// Answers to damage-order questions this build no longer asks.
    orders: usize,
    /// Answers given after the game was decided.
    after_the_end: usize,
    /// The first input after which this build parts from the record.
    first: u64,
}

fn hex(hash: u64) -> String {
    format!("{hash:016x}")
}

/// Whether `action` answers a question, as opposed to taking a turn's
/// action, passing, or setting automation: the only kind of input a build
/// before the boundary took after the game was decided.
fn answers_a_question(action: &PlayerAction) -> bool {
    matches!(
        action,
        PlayerAction::ChooseManaAbility { .. }
            | PlayerAction::ChooseDamageSource { .. }
            | PlayerAction::ChooseDamageEffect { .. }
            | PlayerAction::AllocatePrevention { .. }
            | PlayerAction::ChooseObjects { .. }
            | PlayerAction::ChooseTargets { .. }
            | PlayerAction::ChooseTargetBatch { .. }
            | PlayerAction::Arrange { .. }
            | PlayerAction::ChooseCardType(_)
            | PlayerAction::ChooseColor(_)
            | PlayerAction::ChooseSubtype(_)
            | PlayerAction::ChooseCardName { .. }
            | PlayerAction::ChooseMode(_)
            | PlayerAction::ChooseNumber(_)
            | PlayerAction::ChoosePlayer(_)
            | PlayerAction::YesNo(_)
    )
}

/// One recorded input.
struct Input {
    n: u64,
    seat: u8,
    action: PlayerAction,
    wrote: Option<String>,
}

/// Plays `record` on this build, held to `digest`, and finds where it
/// crosses the boundary; `None` when it departs from this build anywhere
/// else, or nowhere.
///
/// Two departures are the boundary's, and only these. The record's digest
/// parts from this build's after an input, the record's next inputs answer
/// damage-order questions, and after the last of them it carries this
/// build's digest again: a build before c55a8c6eb asked that order and
/// reached, once answered, the board this build reaches without asking.
/// Or this build's game is over while the record goes on, with answers to
/// questions only, to its end line: a build before a5eb549ca asked them
/// after the game was decided, on the board this build ends on.
fn crossing(record: &[u8], digest: Digest) -> Option<Crossing> {
    let mut lines = record
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .map(serde_json::from_slice::<Line>);
    let Some(Ok(Line::Header {
        build,
        preset,
        hash,
        seen,
        ..
    })) = lines.next()
    else {
        return None;
    };
    let of = |engine: &Engine<RegistryLookup>| match digest {
        Digest::Exact => hex(engine.snapshot_hash()),
        Digest::Seen => hex(baylee_gamehost::record::seen(engine.state())),
    };
    let mut engine = Engine::new(&preset, RegistryLookup).ok()?;
    let opening = match digest {
        Digest::Exact => Some(hash),
        Digest::Seen => seen,
    };
    if opening.as_deref() != Some(of(&engine).as_str()) {
        return None;
    }
    let mut inputs = Vec::new();
    let mut ended = false;
    for line in lines {
        match line.ok()? {
            Line::Input {
                n,
                seat,
                action,
                hash,
                seen,
                ..
            } => inputs.push(Input {
                n,
                seat,
                action,
                wrote: match digest {
                    Digest::Exact => Some(hash),
                    Digest::Seen => seen,
                },
            }),
            Line::End { .. } => ended = true,
            Line::Header { .. } => return None,
            Line::Chair { .. } | Line::DeclaredMind { .. } => {}
        }
    }
    let mut found = Crossing {
        build,
        orders: 0,
        after_the_end: 0,
        first: 0,
    };
    let mut crossed = false;
    let mut at = 0;
    while let Some(input) = inputs.get(at) {
        if matches!(engine.pending(), Pending::GameOver(_)) {
            let rest = &inputs[at..];
            if !ended || !rest.iter().all(|input| answers_a_question(&input.action)) {
                return None;
            }
            found.after_the_end = rest.len();
            if !crossed {
                found.first = input.n;
            }
            return Some(found);
        }
        engine
            .apply(PlayerId::new(input.seat), input.action.clone())
            .ok()?;
        let now = of(&engine);
        if input.wrote.as_deref() == Some(now.as_str()) {
            at += 1;
            continue;
        }
        // The answers to the order questions this build skipped.
        let mut answer = at + 1;
        loop {
            let next = inputs.get(answer)?;
            if !matches!(next.action, PlayerAction::ChooseDamageEffect { .. }) {
                return None;
            }
            if next.wrote.as_deref() == Some(now.as_str()) {
                break;
            }
            answer += 1;
        }
        found.orders += answer - at;
        if !crossed {
            found.first = input.n;
            crossed = true;
        }
        at = answer + 1;
    }
    (crossed && (!ended || matches!(engine.pending(), Pending::GameOver(_)))).then_some(found)
}

impl Crossing {
    fn what(&self) -> String {
        format!(
            "from input {}, it answers {} damage-order question(s) this build no longer asks \
             (c55a8c6eb) and {} question(s) asked after the game was decided (a5eb549ca)",
            self.first, self.orders, self.after_the_end,
        )
    }
}

/// Why a record that opens with this build's hash and whose exact replay
/// diverged or was refused is one written before [`QUESTION_BOUNDARY`], or
/// `None` when it departs for anything else.
pub(super) fn refusal(record: &[u8]) -> Option<String> {
    let found = crossing(record, Digest::Exact)?;
    Some(format!(
        "{QUESTION_REFUSAL} written by {build}, converted by {this}: {what}; {QUESTION_RULE}",
        build = found.build,
        this = baylee_gamehost::record::this_build(),
        what = found.what(),
    ))
}

/// The same for a record of another build, held to its public table: the
/// reason a seen replay gives, so that it stays one `--llm-bc-skip-unreplayable`
/// may leave out, as for any rule this build plays differently.
pub(super) fn seen_refusal(record: &[u8]) -> Option<String> {
    let found = crossing(record, Digest::Seen)?;
    Some(format!(
        "seen replay audit: written before question boundary {QUESTION_BOUNDARY} by {build}: \
         {what}; {QUESTION_RULE}",
        build = found.build,
        what = found.what(),
    ))
}

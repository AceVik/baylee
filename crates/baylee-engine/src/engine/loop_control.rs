//! The driver's response to a detected loop, separate from turn progression.
use super::{CardLookup, EndReason, Engine, GameEvent, GameResult, Pending};
use baylee_core::preset::LoopPolicy;

/// All persistent detector state, copied or restored as one unit.
///
/// Keeping it out of the turn driver's other latches lets a compiled policy
/// later replace the whole unit, including its counters and saved signature.
#[derive(Clone, Debug, Default)]
pub(super) struct LoopControl {
    /// Trigger suppression after a break, until the stack has drained.
    pub(super) breaking_loop: bool,
    /// Repetition across answers, including its hidden-state signature.
    pub(super) action_loops: crate::loops::LoopWatch,
    /// Diagnostic count; the journal records the authoritative detections.
    pub(super) loops_broken: u32,
}

impl LoopControl {
    /// A hidden-information fork must not retain a signature of the real game.
    #[cfg(any(test, feature = "fork"))]
    pub(super) const fn allows_fork(&self) -> bool {
        // Exhaustive: a new detector field needs an explicit fork decision.
        let Self {
            breaking_loop,
            action_loops,
            loops_broken: _,
        } = self;
        !*breaking_loop && !action_loops.holds_signature()
    }
}

impl<L: CardLookup> Engine<L> {
    /// Applies the house rule for an endless loop. Returns `true` when the
    /// game ended and the caller must stop.
    ///
    /// `RunOnceThenBreak` breaks the loop rather than ending the game: the
    /// abilities feeding it stop reaching the stack, the stack drains, and
    /// play continues from whatever the loop left behind. That is the
    /// "resolve it once, then break it" house rule — the loop's effect has
    /// happened, it just stops happening forever. A card that starts the
    /// same loop again next upkeep is broken again next upkeep; the game
    /// keeps moving, which is the point.
    ///
    /// Detecting a loop *while a break is still in force* means the break
    /// did not take: the loop is driven by something other than triggers —
    /// replacement effects, state-based actions — and withholding triggers
    /// changed nothing. Repeating the attempt would be a slower hang, so
    /// that case falls back to the Comprehensive Rules answer (CR 104.4b:
    /// a draw), as does `CompRulesDraw` from the start.
    ///
    /// The watch is reset either way: whatever comes next is a new
    /// question, and Brent's saved state is about the loop just handled.
    pub(crate) fn on_loop_detected(&mut self, period: u64) -> bool {
        let break_it = self.house_rules.loop_policy == LoopPolicy::RunOnceThenBreak
            && !self.loop_control.breaking_loop;
        self.state.journal.record(GameEvent::LoopDetected {
            period,
            broken: break_it,
        });
        self.loop_control.action_loops = crate::loops::LoopWatch::default();
        if break_it {
            self.loop_control.loops_broken += 1;
            self.loop_control.breaking_loop = true;
            self.trigger_queue.clear();
            self.trigger_scan_seq = self.state.journal.last_seq();
            return false;
        }
        let result = GameResult {
            winner: None,
            reason: EndReason::Draw,
        };
        self.pending = Pending::GameOver(result);
        self.awaiting_answer = true;
        self.state
            .journal
            .record(GameEvent::GameWon { winner: None });
        true
    }

    /// How many endless loops this game has broken (diagnostic; the
    /// journal's `LoopDetected` entries are authoritative).
    #[must_use]
    pub const fn loops_broken(&self) -> u32 {
        self.loop_control.loops_broken
    }
}

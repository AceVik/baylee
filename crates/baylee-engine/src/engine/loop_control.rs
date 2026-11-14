//! The driver's response to a detected loop, separate from turn progression.
use super::{CardLookup, EndReason, Engine, GameEvent, GameResult, Pending};
mod policy;
pub use policy::{DetectLoops, LoopDetection, LoopSegment, NoLoopDetection};

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
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
        let Some(break_it) = self.loop_control.detected(self.house_rules.loop_policy) else {
            return false;
        };
        self.state.journal.record(GameEvent::LoopDetected {
            period,
            broken: break_it,
        });
        if break_it {
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
    pub fn loops_broken(&self) -> u32 {
        self.loop_control.broken_count()
    }
}

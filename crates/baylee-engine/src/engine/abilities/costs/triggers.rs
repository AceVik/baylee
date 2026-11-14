//! Trigger collection between complete cost parts.
use super::{CardLookup, Engine};

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    /// Detect journal triggers without stacking, pruning, or running SBAs.
    /// Append only so an unsuccessful payment can restore its queue length.
    pub(in crate::engine::abilities) fn capture_cost_triggers(&mut self) {
        // A completed cost part can change power before the next part sacrifices
        // its source. Refresh without SBAs, and only between complete parts so
        // all sacrifices within one part keep the same pre-event projection.
        self.state.refresh_characteristics();
        if self.loop_control.is_breaking() || self.trigger_scan_seq == self.state.journal.last_seq()
        {
            return;
        }
        self.trigger_queue.extend(crate::trigger::collect(
            &self.state,
            &self.lookup,
            self.trigger_scan_seq,
        ));
        self.trigger_scan_seq = self.state.journal.last_seq();
        crate::resolve::linked_reanimation::prune(&mut self.state);
    }
}

//! Trigger collection between complete cost parts.
use super::{CardLookup, Engine};

impl<L: CardLookup> Engine<L> {
    /// Detect journal triggers without stacking, pruning, or running SBAs.
    /// Append only so an unsuccessful payment can restore its queue length.
    pub(in crate::engine::abilities) fn capture_cost_triggers(&mut self) {
        if self.breaking_loop || self.trigger_scan_seq == self.state.journal.last_seq() {
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

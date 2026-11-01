//! Additive full-choice context for new offline consumers. Legacy views stay stable.
use super::{Pending, PlayerId, PlayerView, Session};

impl Session {
    /// Authorized deciding view, including the owner's current cast and targets.
    ///
    /// Unlike [`Self::view_for`], this opt-in method includes the same choice
    /// context as a socket snapshot. It does not add clocks, shared hands or logs.
    #[must_use]
    pub fn view_for_with_context(&self, seat: PlayerId) -> Option<(Pending, PlayerView)> {
        self.table().view_for_with_context(seat)
    }

    /// Authorized passive view with owner-only cast/target context when present.
    ///
    /// [`Self::observe_view`] is deliberately unchanged for archived consumers.
    #[must_use]
    pub fn observe_view_with_context(&self, seat: PlayerId) -> PlayerView {
        self.table().observe_view_with_context(seat)
    }

    /// Authorized deciding view with persistent owner-only announced payment
    /// source. Opt-in consumers may distinguish this from cancellation rights.
    /// Existing [`Self::view_for_with_context`] bytes are unchanged.
    #[must_use]
    pub fn view_for_with_payment_context(&self, seat: PlayerId) -> Option<(Pending, PlayerView)> {
        self.table().view_for_with_payment_context(seat)
    }

    /// Passive owner view with the same persistent announced source. Default,
    /// socket and archived context projections do not opt into this field.
    #[must_use]
    pub fn observe_view_with_payment_context(&self, seat: PlayerId) -> PlayerView {
        self.table().observe_view_with_payment_context(seat)
    }

    /// Experimental authorized announcement facts, including settled X. No
    /// existing encoder, socket or payment-context projection opts into these.
    #[must_use]
    pub fn view_for_with_announcement_context(
        &self,
        seat: PlayerId,
    ) -> Option<(Pending, PlayerView)> {
        self.table().view_for_with_announcement_context(seat)
    }

    /// Passive counterpart of [`Self::view_for_with_announcement_context`].
    #[must_use]
    pub fn observe_view_with_announcement_context(&self, seat: PlayerId) -> PlayerView {
        self.table().observe_view_with_announcement_context(seat)
    }
}

#[cfg(test)]
mod tests;

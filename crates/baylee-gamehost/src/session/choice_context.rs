//! Additive full-choice context for new offline consumers. Legacy views stay stable.
use super::{Pending, PlayerId, PlayerView, Session};

impl Session {
    /// Authorized deciding view, including the owner's current cast and targets.
    ///
    /// Unlike [`Self::view_for`], this opt-in method includes the same choice
    /// context as a socket snapshot. It does not add clocks, shared hands or logs.
    #[must_use]
    pub fn view_for_with_context(&self, seat: PlayerId) -> Option<(Pending, PlayerView)> {
        self.view_for(seat)
            .map(|(pending, view)| (pending, self.attach_choice_context(seat, view)))
    }

    /// Authorized passive view with owner-only cast/target context when present.
    ///
    /// [`Self::observe_view`] is deliberately unchanged for archived consumers.
    #[must_use]
    pub fn observe_view_with_context(&self, seat: PlayerId) -> PlayerView {
        self.attach_choice_context(seat, self.observe_view(seat))
    }

    /// Authorized deciding view with persistent owner-only announced payment
    /// source. Opt-in consumers may distinguish this from cancellation rights.
    /// Existing [`Self::view_for_with_context`] bytes are unchanged.
    #[must_use]
    pub fn view_for_with_payment_context(&self, seat: PlayerId) -> Option<(Pending, PlayerView)> {
        self.view_for_with_context(seat).map(|(pending, mut view)| {
            view.announced_cast = self.engine.announced_cast(seat);
            (pending, view)
        })
    }

    /// Passive owner view with the same persistent announced source. Default,
    /// socket and archived context projections do not opt into this field.
    #[must_use]
    pub fn observe_view_with_payment_context(&self, seat: PlayerId) -> PlayerView {
        let mut view = self.observe_view_with_context(seat);
        view.announced_cast = self.engine.announced_cast(seat);
        view
    }

    /// Experimental authorized announcement facts, including settled X. No
    /// existing encoder, socket or payment-context projection opts into these.
    #[must_use]
    pub fn view_for_with_announcement_context(
        &self,
        seat: PlayerId,
    ) -> Option<(Pending, PlayerView)> {
        self.view_for_with_payment_context(seat)
            .map(|(pending, view)| (pending, self.attach_announcement_context(seat, view)))
    }

    /// Passive counterpart of [`Self::view_for_with_announcement_context`].
    #[must_use]
    pub fn observe_view_with_announcement_context(&self, seat: PlayerId) -> PlayerView {
        self.attach_announcement_context(seat, self.observe_view_with_payment_context(seat))
    }

    fn attach_announcement_context(&self, seat: PlayerId, mut view: PlayerView) -> PlayerView {
        if let Some((source, x)) = self.engine.announced_spell_context(seat) {
            view.announced_cast = Some(source);
            view.announced_x = x;
        }
        view
    }

    fn attach_choice_context(&self, seat: PlayerId, mut view: PlayerView) -> PlayerView {
        view.targeting = crate::view::targeting_context(&self.engine, seat);
        view.casting = self.engine.cancellable_cast(seat);
        view
    }
}

#[cfg(test)]
mod tests;

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

    fn attach_choice_context(&self, seat: PlayerId, mut view: PlayerView) -> PlayerView {
        view.targeting = crate::view::targeting_context(&self.engine, seat);
        view.casting = self.engine.cancellable_cast(seat);
        view
    }
}

#[cfg(test)]
mod tests;

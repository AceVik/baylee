//! When a card could be cast, as far as a seat's own view can tell.
//!
//! The engine answers this properly in `casting::timing_allows`, and what it
//! offers in `LegalActions::castable` is the authority. This is the *other*
//! half of the client's hand: [`crate::manaplan`] finds the lands that would
//! pay for a card the engine has **not** offered yet, and the client lights
//! that card and taps them on the player's behalf. That offer was made on
//! affordability alone. It never asked what turn it was.
//!
//! So a sorcery in hand lit up, and could be clicked, on an opponent's turn,
//! in the middle of combat, and — the case the owner reported — with a spell
//! still on the stack. The lands came tapped, and then the engine refused the
//! cast that was supposed to follow, which is worse than never offering: the
//! mana empties at the end of the step and the turn's lands are gone.
//!
//! # What a view cannot see
//!
//! Two effects move this line and travel in no view: Teferi's `{2}{U}` static
//! putting an opponent's every spell back to sorcery speed, and the +1 that
//! gives its controller's sorceries flash. Both are read off `GameState` by
//! the engine and neither is projected. This module is therefore
//! *conservative in the direction that costs the player nothing they cannot
//! get back*: with sorceries-have-flash running, a sorcery simply is not lit
//! until its mana is floating, and the player taps the lands themselves. The
//! opposite mistake is the one that spends a turn.

use baylee_core::types::TypeSet;
use baylee_view::{Phase, PlayerView};

/// Whether this seat is inside a window a sorcery could be cast in
/// (CR 307.1): a main phase of its own turn, with an empty stack.
///
/// Priority is the fourth condition and is not checked here, because the only
/// caller is inside a `Pending::Priority` addressed to this seat — the engine
/// asking the question *is* the seat having priority.
#[must_use]
pub fn sorcery_window(view: &PlayerView) -> bool {
    matches!(view.phase, Phase::FirstMain | Phase::SecondMain)
        && view.active == view.seat
        && view.stack.is_empty()
}

/// Whether a card with these characteristics could be cast right now.
///
/// `flash` is the printed keyword (CR 702.8a), which the caller looks up in
/// the card registry — a `HandObject` carries types but no keywords, and a
/// view has no reason to grow a field for something the client already has
/// the card for.
#[must_use]
pub fn allows(view: &PlayerView, types: TypeSet, flash: bool) -> bool {
    types.contains(TypeSet::INSTANT) || flash || sorcery_window(view)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{ViewBuilder, token};
    use baylee_core::ids::PlayerId;
    use baylee_view::Step;

    /// A seat in its own first main phase with nothing on the stack.
    fn open() -> PlayerView {
        ViewBuilder::new(2).build()
    }

    #[test]
    fn a_seats_own_empty_main_phase_is_a_sorcery_window() {
        assert!(sorcery_window(&open()));
    }

    #[test]
    fn a_spell_on_the_stack_closes_it() {
        let mut view = open();
        view.stack.push(token(9, 0, "Lightning Bolt", 0, 0));
        assert!(
            !sorcery_window(&view),
            "CR 307.1 wants an empty stack, and the reported fault was a \
             sorcery offered over one"
        );
        assert!(
            !allows(&view, TypeSet::SORCERY, false),
            "so the sorcery is not offered either"
        );
        assert!(
            allows(&view, TypeSet::INSTANT, false),
            "but an instant still is — that is the whole point of the window"
        );
        assert!(
            allows(&view, TypeSet::CREATURE, true),
            "and so is a creature with flash (CR 702.8a)"
        );
    }

    #[test]
    fn somebody_elses_turn_closes_it_too() {
        let mut view = open();
        view.active = PlayerId::new(1);
        assert!(!sorcery_window(&view));
        assert!(!allows(&view, TypeSet::CREATURE, false));
        assert!(allows(&view, TypeSet::INSTANT, false));
    }

    #[test]
    fn and_so_does_a_step_that_is_not_a_main_phase() {
        let mut view = open();
        view.phase = Phase::Combat;
        view.step = Step::DeclareBlockers;
        assert!(!sorcery_window(&view));
        assert!(
            !allows(&view, TypeSet::ARTIFACT, false),
            "an artifact is sorcery-speed like any other permanent (CR 301.1)"
        );
    }

    /// The second main phase is a main phase.
    ///
    /// Worth its own test only because the predicate is a `matches!` over an
    /// enum, and a `matches!` that lost an arm would pass every other test
    /// here.
    #[test]
    fn the_second_main_phase_is_one_as_well() {
        let mut view = open();
        view.phase = Phase::SecondMain;
        assert!(sorcery_window(&view));
    }
}

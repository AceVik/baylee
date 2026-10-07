//! The hand as a drawer on a phone (the owner's of 07.10.2026, DESIGN-v8
//! WA11): on a phone held sideways the hand zone takes 202 px of a 386-px
//! window, more than half of it; retracted, the hand gives that height back
//! to the table and only the actions bar and a tab with the hand's count
//! stay at the bottom edge.
//!
//! Whether it stands open is two things: what the player last did with it
//! (a tap or a swipe on the tab, the key), remembered for the game, and
//! whether the question in front of them is **answered from the hand**, in
//! which case it opens by itself for as long as the question stands and
//! goes back to what the player had afterwards.
//!
//! Answered from the hand (decided here, WA11): the opening hand (keep or
//! mulligan, and the cards a mulligan bottoms), a discard, and a choice of
//! cards any of which is in the hand. **Not** a priority with something
//! castable: that is nearly every priority of a player's own main phase,
//! and a drawer that opened on each would be a drawer that is never shut —
//! the tab says how many cards are castable instead, and one tap opens it.

use baylee_engine::choice::Pending;
use baylee_view::PlayerView;

/// How long the drawer takes to slide, seconds (the brief's 0.25–0.3).
pub const SLIDE_SECS: f32 = 0.28;

/// The drawer: what the player last did with it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HandDrawer {
    /// Open by the player's own hand. Retracted at the start: the table
    /// gets the phone's height until a question needs the hand or the
    /// player opens it.
    pub chosen_open: bool,
}

impl HandDrawer {
    /// Whether it stands open with `pending` in front of the player.
    #[must_use]
    pub fn open(self, view: Option<&PlayerView>, pending: Option<&Pending>) -> bool {
        self.chosen_open || needs_the_hand(view, pending)
    }

    /// A tap or a swipe on the tab, or the key: open becomes shut and shut
    /// open — and remembered. While a question holds it open, a request to
    /// shut it is the player's for afterwards: it shuts once the question
    /// is answered, and stays open while the question stands (the answer
    /// is in it).
    pub fn toggle(&mut self, view: Option<&PlayerView>, pending: Option<&Pending>) {
        let shown = self.open(view, pending);
        self.chosen_open = !shown;
    }

    /// A swipe: up opens, down shuts (remembered as a toggle is).
    pub fn swipe(&mut self, up: bool) {
        self.chosen_open = up;
    }
}

/// Whether `pending` is answered from the hand (see the module).
#[must_use]
pub fn needs_the_hand(view: Option<&PlayerView>, pending: Option<&Pending>) -> bool {
    let Some(pending) = pending else {
        return false;
    };
    match pending {
        Pending::Mulligan { .. }
        | Pending::MulliganBottom { .. }
        | Pending::DiscardChoice { .. } => true,
        Pending::ChooseCards { options, .. } => {
            view.is_some_and(|view| view.hand.iter().any(|card| options.contains(&card.id)))
        }
        _ => false,
    }
}

/// How far open the drawer is drawn this frame, 0 shut to 1 open: eased
/// toward `open` over [`SLIDE_SECS`], or at once under reduced motion.
#[must_use]
pub fn slide(shown: f32, open: bool, dt: f32, still: bool) -> f32 {
    let target = if open { 1.0 } else { 0.0 };
    if still {
        return target;
    }
    let step = dt / SLIDE_SECS;
    if shown < target {
        (shown + step).min(target)
    } else {
        (shown - step).max(target)
    }
}

/// The eased fraction a linear `shown` is drawn at: smoothstep, so the
/// drawer leaves and lands softly.
#[must_use]
pub fn eased(shown: f32) -> f32 {
    let u = shown.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ViewBuilder;
    use baylee_core::ids::PlayerId;

    fn priority() -> Pending {
        Pending::Priority {
            player: PlayerId::new(0),
            legal: Box::default(),
        }
    }

    /// Opened by the questions answered from the hand, by them alone, and
    /// shut again after them as the player had it; a toggle is remembered.
    #[test]
    fn the_drawer_opens_for_the_hand_s_questions_and_remembers_the_player_s_choice() {
        let view = ViewBuilder::new(2).with_hand(vec![("Opt", 1, 1)]).build();
        let in_hand = view.hand[0].id;
        let drawer = HandDrawer::default();
        assert!(!drawer.open(Some(&view), None), "shut at the start");
        assert!(
            !drawer.open(Some(&view), Some(&priority())),
            "not for a priority"
        );
        let discard = Pending::DiscardChoice {
            player: PlayerId::new(0),
            count: 1,
        };
        assert!(
            drawer.open(Some(&view), Some(&discard)),
            "a discard opens it"
        );
        let mulligan = Pending::Mulligan {
            player: PlayerId::new(0),
            taken: 0,
            next_is_free: true,
            can_take: true,
        };
        assert!(drawer.open(Some(&view), Some(&mulligan)));
        let choose = |options| Pending::ChooseCards {
            player: PlayerId::new(0),
            options,
            min: 1,
            max: 1,
            prompt: baylee_engine::choice::ChoicePrompt::PutBackOnTop,
            total: None,
        };
        let from_hand = choose(vec![in_hand]);
        assert!(
            drawer.open(Some(&view), Some(&from_hand)),
            "cards from the hand"
        );
        let from_library = choose(vec![baylee_core::ids::ObjectId::new(999, 0)]);
        assert!(
            !drawer.open(Some(&view), Some(&from_library)),
            "not a search"
        );

        let mut drawer = HandDrawer::default();
        drawer.toggle(Some(&view), Some(&priority()));
        assert!(
            drawer.open(Some(&view), Some(&priority())),
            "opened by hand"
        );
        drawer.toggle(Some(&view), Some(&priority()));
        assert!(!drawer.open(Some(&view), None), "and shut by hand");
        // Held open by a discard, a tap asks it shut for afterwards.
        drawer.toggle(Some(&view), Some(&discard));
        assert!(
            drawer.open(Some(&view), Some(&discard)),
            "the answer is in it"
        );
        assert!(
            !drawer.open(Some(&view), Some(&priority())),
            "shut afterwards"
        );
    }

    /// It slides in the brief's time and lands exactly; reduced motion is
    /// the cut.
    #[test]
    fn the_drawer_slides_in_its_time_and_snaps_under_reduced_motion() {
        let mut shown = 0.0;
        let mut frames = 0;
        while shown < 1.0 {
            shown = slide(shown, true, 1.0 / 60.0, false);
            frames += 1;
        }
        assert!(
            (15..=18).contains(&frames),
            "{frames} frames: 0.25 to 0.3 s"
        );
        assert!((shown - 1.0).abs() < f32::EPSILON);
        assert!((slide(1.0, false, 1.0 / 60.0, true)).abs() < f32::EPSILON);
        assert!((eased(0.5) - 0.5).abs() < 1e-6);
        assert!(eased(0.0).abs() < 1e-6 && (eased(1.0) - 1.0).abs() < 1e-6);
    }
}

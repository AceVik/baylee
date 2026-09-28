//! Renderer-independent target choices and bounded batch confirmation.
use crate::Interaction;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::PlayerView;

/// The two distinct namespaces legal targeting may name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// A card or stack object.
    Object(ObjectId),
    /// A seat (including self and teammates when legal).
    Player(PlayerId),
}

/// All legal options, without a client-side opponent filter.
#[must_use]
pub fn options(pending: &Pending) -> Vec<Target> {
    match pending {
        Pending::ChooseTargets {
            options,
            player_options,
            ..
        } => player_options
            .iter()
            .copied()
            .map(Target::Player)
            .chain(options.iter().copied().map(Target::Object))
            .collect(),
        _ => Vec::new(),
    }
}

/// Two-stage confirmation: a selection must already be valid. A batch is
/// explicitly confirmed each time and never persisted as a preference.
#[must_use]
pub fn batch_answer(interaction: &Interaction, view: &PlayerView) -> Option<PlayerAction> {
    let count = view.targeting.as_ref()?.batch_count;
    if count < 2 {
        return None;
    }
    let (objects, players) = match interaction.confirm()? {
        PlayerAction::ChooseTargets { objects, players } => (objects, players),
        PlayerAction::ChooseObjects { objects }
            if matches!(interaction.pending(), Pending::ChooseTargets { .. }) =>
        {
            (objects, Vec::new())
        }
        _ => return None,
    };
    Some(PlayerAction::ChooseTargetBatch {
        objects,
        players,
        count,
    })
}

/// Bounded visible list; all targets remain reachable through pages.
pub const PAGE_SIZE: usize = 8;
/// Navigation indices never collide with an engine's target index.
pub const PREVIOUS: usize = usize::MAX;
/// Next page control.
pub const NEXT: usize = usize::MAX - 1;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{ViewBuilder, token};
    use baylee_engine::choice::TargetPrompt;

    fn pending() -> Pending {
        Pending::ChooseTargets {
            player: PlayerId::new(0),
            options: vec![ObjectId::new(10, 0)],
            player_options: (0..6).map(PlayerId::new).collect(),
            min: 1,
            max: 1,
            reason: TargetPrompt::Targets,
        }
    }

    #[test]
    fn all_seats_including_self_and_objects_stay_distinct() {
        let choices = options(&pending());
        assert_eq!(choices.len(), 7);
        assert_eq!(choices[0], Target::Player(PlayerId::new(0)));
        assert_eq!(choices[5], Target::Player(PlayerId::new(5)));
        assert_eq!(choices[6], Target::Object(ObjectId::new(10, 0)));
    }

    #[test]
    fn batch_needs_valid_selection_and_explicit_current_series() {
        let mut view = ViewBuilder::new(6).build();
        view.targeting = Some(baylee_view::TargetingContext {
            source: token(20, 0, "source", 2, 2),
            text: None,
            whole_spell: false,
            second: false,
            batch_count: 50,
        });
        let mut i = Interaction::new(pending(), PlayerId::new(0));
        assert!(batch_answer(&i, &view).is_none());
        i.toggle_player(PlayerId::new(5));
        assert!(
            matches!(batch_answer(&i, &view), Some(PlayerAction::ChooseTargetBatch { players, count: 50, .. }) if players == vec![PlayerId::new(5)])
        );
        assert!(
            matches!(i.confirm(), Some(PlayerAction::ChooseTargets { .. })),
            "ordinary confirm stays a single answer"
        );
        view.targeting.as_mut().unwrap().batch_count = 1;
        assert!(batch_answer(&i, &view).is_none());
        view.targeting = None;
        assert!(batch_answer(&i, &view).is_none());
    }
}

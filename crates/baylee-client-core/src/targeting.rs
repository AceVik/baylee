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

/// Filter controls occupy a separate namespace from legal option indices.
#[must_use]
pub fn filter_index(player: Option<PlayerId>) -> usize {
    player.map_or(usize::MAX - 2, |p| usize::MAX - 3 - usize::from(p.get()))
}

/// Decode an all-seats or single-seat filter control.
#[must_use]
pub fn filter_at(index: usize) -> Option<Option<PlayerId>> {
    if index == filter_index(None) {
        Some(None)
    } else {
        (usize::MAX - 3)
            .checked_sub(index)
            .and_then(|p| u8::try_from(p).ok())
            .map(|p| Some(PlayerId::new(p)))
    }
}

/// Visible choices retain their original indices; filtering never changes
/// the engine selection or hides the selection summary.
#[must_use]
pub fn filtered(
    pending: &Pending,
    view: &PlayerView,
    player: Option<PlayerId>,
) -> Vec<(usize, Target)> {
    options(pending)
        .into_iter()
        .enumerate()
        .filter(|(_, t)| {
            player.is_none_or(|p| match t {
                Target::Player(target) => *target == p,
                Target::Object(id) => view.object(*id).is_some_and(|o| o.controller == p),
            })
        })
        .collect()
}

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
    fn seat_filter_preserves_original_indices_and_control_namespace() {
        let view = ViewBuilder::new(6)
            .with_battlefield(3, vec![token(10, 3, "Bear", 2, 2)])
            .build();
        assert_eq!(
            filtered(&pending(), &view, Some(PlayerId::new(3))),
            vec![
                (3, Target::Player(PlayerId::new(3))),
                (6, Target::Object(ObjectId::new(10, 0))),
            ]
        );
        assert_eq!(filtered(&pending(), &view, None).len(), 7);
        for player in (0..=u8::MAX).map(PlayerId::new) {
            assert_eq!(filter_at(filter_index(Some(player))), Some(Some(player)));
        }
        assert_eq!(filter_at(filter_index(None)), Some(None));
        for index in [0, 6, PAGE_SIZE, PREVIOUS, NEXT] {
            assert_eq!(filter_at(index), None);
        }
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

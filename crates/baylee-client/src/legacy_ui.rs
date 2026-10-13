//! The button of `bevy_ui`'s old pointer model, which this client still
//! stands on.
//!
//! Bevy 0.20 deprecates `bevy::ui::widget::Button` and `bevy::ui::Interaction`
//! in favour of `bevy::ui_widgets::Button` with `picking::hover::Hovered` and
//! `ui::Pressed`, and removes both in the release after. Its `Button` became
//! an alias of a crate-private type, so it can no longer be spawned at all;
//! `Interaction` is an alias of the public `DeprecatedInteraction`, which
//! `bevy_ui`'s `ui_focus_system` still keeps up to date on every node that has
//! one.
//!
//! The new button is not a drop-in: its observers stop the propagation of
//! every press, release and click on a button, and turn Enter or Space on a
//! focused button into an `Activate` whose key they also stop, which would
//! change who hears those here. So the upgrade keeps the old model — the
//! marker below requires what the old `Button` required — and the move to
//! the new one is its own change. Every use says `LegacyButton` or
//! `LegacyInteraction`, so that change is one `grep` wide.

use accesskit::{Node as AccessKitNode, Role};
use bevy::a11y::AccessibilityNode;
use bevy::prelude::*;
use bevy::ui::FocusPolicy;

/// `bevy_ui`'s per-node pointer state (`None`, `Hovered`, `Pressed`), under
/// the name that is not deprecated.
pub(crate) type LegacyInteraction = bevy::ui::DeprecatedInteraction;

/// A control the pointer presses: what `bevy_ui`'s `Button` was until 0.20.
///
/// It requires a [`Node`], [`FocusPolicy::Block`] (a press stops here and
/// does not reach the node behind) and a [`LegacyInteraction`], which
/// `bevy_ui` updates each frame. It also says "button" to accessibility, which
/// `bevy_ui`'s own accessibility system did for the old marker.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
#[require(
    Node,
    FocusPolicy::Block,
    LegacyInteraction,
    AccessibilityNode(AccessKitNode::new(Role::Button))
)]
pub(crate) struct LegacyButton;

#[cfg(test)]
mod tests {
    use super::*;

    /// Spawning the marker alone gives the entity what bevy 0.19's `Button`
    /// required: a node to lay out, a focus policy that blocks the press, an
    /// interaction state for `ui_focus_system` to keep, and the "button" role
    /// for accessibility. Lose one of them and a button either never
    /// updates its interaction, lets a press through to what is behind it,
    /// or is not announced as a button — none of which a compile notices.
    #[test]
    fn a_legacy_button_brings_what_the_old_button_required() {
        let mut world = World::new();
        let button = world.spawn(LegacyButton).id();
        let entity = world.entity(button);
        assert!(entity.contains::<Node>(), "a node to lay out");
        assert_eq!(entity.get::<FocusPolicy>(), Some(&FocusPolicy::Block));
        assert_eq!(
            entity.get::<LegacyInteraction>(),
            Some(&LegacyInteraction::None),
            "an interaction that starts untouched"
        );
        let accessible = entity
            .get::<AccessibilityNode>()
            .expect("an accessibility node");
        assert_eq!(accessible.role(), Role::Button);
    }
}

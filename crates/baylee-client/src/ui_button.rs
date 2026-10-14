//! Bevy's headless button with the client's UI and picking defaults.
//!
//! `ButtonPlugin` owns the transient `Pressed` state. Its `Activate` events
//! deliberately do not dispatch domain actions: the client's pointer and
//! keyboard paths already enforce modifiers, modal ownership and rebuilds.

use bevy::ecs::system::SystemParam;
use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui::{FocusPolicy, InteractionDisabled, Pressed};
use bevy::ui_widgets::Button;

/// A UI button, initially released. `Hovered` includes its descendants;
/// `Pressed` is inserted and removed by Bevy, never a required component.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
#[require(Button, Node, FocusPolicy::Block, Hovered)]
pub(crate) struct UiButton;

/// Keeps disabling a held control from leaving Bevy's transient state set.
pub(crate) struct UiButtonPlugin;

impl Plugin for UiButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ButtonClick>()
            .add_observer(pointer_click)
            .add_observer(
                |event: On<Add<InteractionDisabled>>,
                 held: Query<(), (With<UiButton>, With<Pressed>)>,
                 mut commands: Commands| {
                    if held.contains(event.entity) {
                        commands.entity(event.entity).remove::<Pressed>();
                    }
                },
            )
            // Also cover a queued press arriving after the disable in the same
            // dispatch. No frame system or repeated component writes are needed.
            .add_observer(
                |event: On<Add<Pressed>>,
                 disabled: Query<(), (With<UiButton>, With<InteractionDisabled>)>,
                 mut commands: Commands| {
                    if disabled.contains(event.entity) {
                        commands.entity(event.entity).remove::<Pressed>();
                    }
                },
            );
    }
}

/// A primary pointer click accepted while the native button was held.
/// This is not Bevy's source-less `Activate`: keyboard actions stay with
/// the existing keymap and focus handlers.
#[derive(Message)]
struct ButtonClick(Entity);

type HeldButton = (With<UiButton>, With<Pressed>, Without<InteractionDisabled>);

fn pointer_click(
    event: On<PointerClick>,
    buttons: Query<(), HeldButton>,
    mut clicks: MessageWriter<ButtonClick>,
) {
    if event.button == PointerButton::Primary && buttons.contains(event.entity) {
        clicks.write(ButtonClick(event.entity));
    }
}

pub(crate) fn install(app: &mut App) {
    if !app.is_plugin_added::<UiButtonPlugin>() {
        app.add_plugins(UiButtonPlugin);
    }
}

/// Completed primary clicks for one kind of UI action, consumed in Update.
/// Closing a panel here leaves it standing for the entire picking dispatch,
/// so its release cannot also hit a control underneath it.
#[derive(SystemParam)]
pub(crate) struct Clicks<'w, 's, T: Component> {
    events: MessageReader<'w, 's, ButtonClick>,
    actions: Query<'w, 's, &'static T, (With<UiButton>, Without<InteractionDisabled>)>,
}

impl<T: Component + Clone> Clicks<'_, '_, T> {
    /// Returns each action once per click. Bevy stops pointer propagation at
    /// the innermost button even when it has a different action type. Touch
    /// uses the same primary-click path without requiring mouse hover.
    pub(crate) fn read(&mut self) -> Vec<T> {
        self.events
            .read()
            .filter_map(|event| self.actions.get(event.0).ok().cloned())
            .collect()
    }
}

#[cfg(test)]
pub(crate) mod testing;
#[cfg(test)]
mod tests;

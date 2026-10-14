//! Bevy's headless button with the client's UI and picking defaults.
//!
//! `ButtonPlugin` supplies native button behavior; we reconcile transient
//! `Pressed` ownership after its dispatch for cancellation and multiple pointers. Its `Activate` events
//! deliberately do not dispatch domain actions: the client's pointer and
//! keyboard paths already enforce modifiers, modal ownership and rebuilds.

use bevy::ecs::system::SystemParam;
use bevy::picking::events::{PointerClick, PointerPress, PointerState, pointer_events};
use bevy::picking::hover::Hovered;
use bevy::picking::pointer::{PointerButton, PointerId};
use bevy::prelude::*;
use bevy::ui::{FocusPolicy, InteractionDisabled, Pressed};
use bevy::ui_widgets::Button;

/// A UI button, initially released. `Hovered` includes its descendants;
/// `Pressed` is transient native input state, reconciled per pointer and
/// never a required component.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
#[require(Button, Node, FocusPolicy::Block, Hovered)]
pub(crate) struct UiButton;

/// Completes native pointer ownership for cancellation, disabling and touch.
pub(crate) struct UiButtonPlugin;

impl Plugin for UiButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ButtonClick>()
            .init_resource::<Holds>()
            .init_resource::<PointerState>()
            .add_observer(pointer_press)
            .add_observer(pointer_click)
            .add_systems(
                PreUpdate,
                settle_holds
                    .after(pointer_events)
                    .after(bevy::picking::PickingSystems::Last),
            )
            .add_observer(
                |event: On<Add<InteractionDisabled>>,
                 buttons: Query<Has<Pressed>, With<UiButton>>,
                 mut holds: ResMut<Holds>,
                 mut commands: Commands| {
                    let Ok(pressed) = buttons.get(event.entity) else {
                        return;
                    };
                    holds.0.retain(|hold| hold.button != event.entity);
                    if pressed {
                        commands.entity(event.entity).remove::<Pressed>();
                    }
                },
            )
            // Also cover a queued press arriving after the disable in the same
            // dispatch. Settling below only writes markers whose state differs.
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

/// A primary pointer click accepted for the pointer that held this button.
/// This is not Bevy's source-less `Activate`: keyboard actions stay with
/// the existing keymap and focus handlers.
#[derive(Message)]
struct ButtonClick(Entity);

/// One native button can be held by more than one pointer. Bevy's single
/// `Pressed` marker does not record ownership, so a different pointer's
/// release must not consume the first one's click.
#[derive(Resource, Default)]
struct Holds(Vec<Hold>);

struct Hold {
    pointer: PointerId,
    source: PointerButton,
    button: Entity,
    hit: Entity,
}

type EnabledButton = (With<UiButton>, Without<InteractionDisabled>);

fn pointer_press(
    event: On<PointerPress>,
    buttons: Query<(), EnabledButton>,
    mut holds: ResMut<Holds>,
) {
    if !buttons.contains(event.entity) {
        return;
    }
    // Keep capacity between gestures; the idle path allocates nothing.
    holds
        .0
        .retain(|hold| hold.pointer != event.pointer.id || hold.source != event.button);
    holds.0.push(Hold {
        pointer: event.pointer.id,
        source: event.button,
        button: event.entity,
        hit: event.original_event_target(),
    });
}

fn pointer_click(
    event: On<PointerClick>,
    buttons: Query<(), EnabledButton>,
    holds: Res<Holds>,
    mut clicks: MessageWriter<ButtonClick>,
) {
    if event.button == PointerButton::Primary
        && buttons.contains(event.entity)
        && holds.0.iter().any(|hold| {
            hold.pointer == event.pointer.id
                && hold.source == event.button
                && hold.button == event.entity
        })
    {
        clicks.write(ButtonClick(event.entity));
    }
}

/// Runs after the whole native picking dispatch, including its deferred
/// observers. A cancelled pointer has already left the hover map and need
/// not emit `PointerCancel` at all; its authoritative press state still
/// clears. Reconcile only transient markers, never dispatch an action here.
fn settle_holds(
    state: Res<PointerState>,
    mut holds: ResMut<Holds>,
    enabled: Query<(), EnabledButton>,
    entities: Query<()>,
    pressed: Query<Entity, (With<UiButton>, With<Pressed>)>,
    mut commands: Commands,
) {
    if holds.0.is_empty() && pressed.is_empty() {
        return;
    }
    let valid = |hold: &Hold| {
        enabled.contains(hold.button)
            && entities.contains(hold.hit)
            && state
                .get(hold.pointer, hold.source)
                .is_some_and(|state| state.pressing.contains_key(&hold.hit))
    };
    if holds.0.iter().any(|hold| !valid(hold)) {
        holds.0.retain(valid);
    }
    for entity in &pressed {
        if !holds.0.iter().any(|hold| hold.button == entity) {
            commands.entity(entity).remove::<Pressed>();
        }
    }
    for (at, hold) in holds.0.iter().enumerate() {
        if !pressed.contains(hold.button)
            && !holds.0[..at]
                .iter()
                .any(|earlier| earlier.button == hold.button)
        {
            commands.entity(hold.button).insert(Pressed);
        }
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

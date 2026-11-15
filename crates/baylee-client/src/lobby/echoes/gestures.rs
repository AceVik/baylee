//! Native pointer gestures produce the same revision-bound commands as buttons.
use super::{Action, LobbyState, Slot};
use bevy::{
    picking::{
        events::{PointerClick, PointerDragDrop},
        pointer::PointerButton,
    },
    prelude::*,
};

#[derive(Clone, Copy, PartialEq, Eq)]
struct Binding(uuid::Uuid, u64);

impl Binding {
    fn of(run: &super::api::Run) -> Self {
        Self(run.id, run.revision)
    }
}

#[derive(Component)]
pub(super) struct Owned {
    id: u32,
    binding: Binding,
}
impl Owned {
    pub(super) fn new(run: &super::api::Run, id: u32) -> Self {
        Self {
            id,
            binding: Binding::of(run),
        }
    }
}
#[derive(Component)]
pub(super) struct Destination {
    slot: Slot,
    binding: Binding,
}
impl Destination {
    pub(super) fn new(run: &super::api::Run, slot: Slot) -> Self {
        Self {
            slot,
            binding: Binding::of(run),
        }
    }
}

pub(super) fn install(app: &mut App) {
    #[cfg(not(target_arch = "wasm32"))]
    app.add_systems(
        Update,
        gestures
            .before(super::ui)
            .run_if(in_state(super::DuelPhase::Closed)),
    );
    #[cfg(target_arch = "wasm32")]
    let _ = app;
}

#[cfg(not(target_arch = "wasm32"))]
fn ancestor<T: Component>(
    mut entity: Entity,
    query: &Query<&T>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    for _ in 0..32 {
        if query.contains(entity) {
            return Some(entity);
        }
        entity = parents.get(entity).ok()?.parent();
    }
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn gestures(
    mut clicks: MessageReader<PointerClick>,
    mut drops: MessageReader<PointerDragDrop>,
    owned: Query<&Owned>,
    destinations: Query<&Destination>,
    parents: Query<&ChildOf>,
    mut state: ResMut<LobbyState>,
) {
    if !state.echoes.opened || state.echoes.model.busy() || state.echoes.confirmation.is_some() {
        clicks.clear();
        drops.clear();
        return;
    }
    for drop in drops
        .read()
        .filter(|drop| drop.button == PointerButton::Primary)
    {
        let source =
            ancestor(drop.dropped, &owned, &parents).and_then(|entity| owned.get(entity).ok());
        let target = ancestor(drop.entity, &destinations, &parents)
            .and_then(|entity| destinations.get(entity).ok());
        if let (Some(source), Some(target)) = (source, target)
            && state.echoes.model.run().is_some_and(|run| {
                source.binding == Binding::of(run)
                    && target.binding == source.binding
                    && run
                        .economy
                        .inventory
                        .relics
                        .iter()
                        .any(|item| item.id == source.id && item.slot != target.slot)
            })
        {
            super::native::ask(&mut state, Action::Transfer(source.id, target.slot));
        }
    }
    for click in clicks
        .read()
        .filter(|click| click.button == PointerButton::Secondary)
    {
        if let Some(entity) = ancestor(click.entity, &owned, &parents)
            && let Ok(item) = owned.get(entity)
            && state
                .echoes
                .model
                .run()
                .is_some_and(|run| item.binding == Binding::of(run))
        {
            super::native::ask(&mut state, Action::QuickEquip(item.id));
        }
    }
}

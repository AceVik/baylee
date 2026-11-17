//! Native pointer gestures produce the same revision-bound commands as buttons.
use super::Slot;
#[cfg(not(target_arch = "wasm32"))]
use super::{Action, LobbyState};
#[cfg(not(target_arch = "wasm32"))]
use bevy::picking::{
    events::{PointerClick, PointerDrag, PointerDragDrop, PointerDragEnd},
    pointer::PointerButton,
};
use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
struct Binding(uuid::Uuid, u64);

#[derive(Component)]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(super) struct Sale {
    id: u32,
    binding: Binding,
}
impl Sale {
    pub(super) fn new(run: &super::api::Run, id: u32) -> Self {
        Self {
            id,
            binding: Binding::of(run),
        }
    }
}

impl Binding {
    fn of(run: &super::api::Run) -> Self {
        Self(run.id, run.revision)
    }
}

#[derive(Component)]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
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
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
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
        (gestures, drag_visual)
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
    sales: Query<&Sale>,
    destinations: Query<&Destination>,
    parents: Query<&ChildOf>,
    mut state: ResMut<LobbyState>,
) {
    if !state.echoes.opened || state.echoes.model.mutating() || state.echoes.confirmation.is_some()
    {
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
        if let Some(entity) = ancestor(click.entity, &sales, &parents)
            && let Ok(offer) = sales.get(entity)
            && state
                .echoes
                .model
                .run()
                .is_some_and(|run| offer.binding == Binding::of(run))
        {
            super::native::ask(&mut state, Action::PurchaseEquipped(offer.id));
            continue;
        }
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

#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)] // separate pointer streams and retained render state
fn drag_visual(
    mut commands: Commands,
    drags: Option<MessageReader<PointerDrag>>,
    ends: Option<MessageReader<PointerDragEnd>>,
    owned: Query<&Owned>,
    parents: Query<&ChildOf>,
    mut transforms: Query<&mut UiTransform>,
    scale: Option<Res<UiScale>>,
    state: Res<LobbyState>,
    mut ghost: Local<Option<(Entity, Binding, Entity)>>,
) {
    let (Some(mut drags), Some(mut ends)) = (drags, ends) else {
        return;
    };
    let ended = ends
        .read()
        .any(|event| event.button == PointerButton::Primary);
    let stale = ghost.as_ref().is_some_and(|(_, binding, source)| {
        !state.echoes.opened
            || !owned.contains(*source)
            || state
                .echoes
                .model
                .run()
                .is_none_or(|run| *binding != Binding::of(run))
    });
    if (ended || stale)
        && let Some((entity, _, _)) = ghost.take()
    {
        commands.entity(entity).try_despawn();
    }
    if ended
        || !state.echoes.opened
        || state.echoes.model.mutating()
        || state.echoes.confirmation.is_some()
    {
        drags.clear();
        return;
    }
    for drag in drags
        .read()
        .filter(|event| event.button == PointerButton::Primary)
    {
        let Some(source) = ancestor(drag.entity, &owned, &parents) else {
            continue;
        };
        let Ok(item) = owned.get(source) else {
            continue;
        };
        let Some(run) = state
            .echoes
            .model
            .run()
            .filter(|run| item.binding == Binding::of(run))
        else {
            continue;
        };
        let Some(item) = run
            .economy
            .inventory
            .relics
            .iter()
            .find(|entry| entry.id == item.id)
        else {
            continue;
        };
        let position = drag.pointer.position / scale.as_ref().map_or(1.0, |scale| scale.0);
        let side = 64.0;
        if let Some((entity, _, _)) = *ghost {
            if let Ok(mut transform) = transforms.get_mut(entity) {
                transform.translation =
                    Val2::new(px(position.x - side * 0.5), px(position.y - side * 0.5));
            }
        } else {
            let entity = crate::towerui::drag_ghost(
                &mut commands,
                super::domain::v2::shop::Goods::Relic(item.definition),
                position,
                side,
            );
            *ghost = Some((entity, Binding::of(run), source));
        }
    }
}

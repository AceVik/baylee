//! Fit and pan geometry is retained before rebuilt image nodes enter layout.
use super::{LargePhoto, PhotoViewport, photos};
use crate::lobby::LobbyState;
use bevy::prelude::*;
#[derive(Resource, Default)]
pub(in crate::lobby::memorial) struct PhotoPan {
    offset: Vec2,
    photo: usize,
    viewing: bool,
    room: Vec2,
}
pub(super) fn drag(
    event: On<PointerDrag>,
    state: Res<LobbyState>,
    scale: Option<Res<UiScale>>,
    mut pan: ResMut<PhotoPan>,
) {
    if state.memorial.viewing && state.memorial.zoom > 0 {
        pan.offset += event.delta / scale.as_deref().map_or(1.0, |s| s.0);
    }
}

pub(in crate::lobby::memorial) fn input(
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut state: ResMut<LobbyState>,
) {
    let amount: f32 = wheel.read().map(|w| w.y).sum();
    if state.memorial.open && state.memorial.viewing && amount.abs() > 0.01 {
        state.memorial.magnify(amount > 0.0);
    }
}

pub(in crate::lobby::memorial) fn layout(
    state: Res<LobbyState>,
    mut pan: ResMut<PhotoPan>,
    viewports: Query<&ComputedNode, With<PhotoViewport>>,
    mut photos: Query<
        (&mut Node, &mut UiTransform, &ComputedNode, &mut Visibility),
        With<LargePhoto>,
    >,
) {
    if !state.memorial.viewing {
        if pan.viewing {
            pan.viewing = false;
        }
        return;
    }
    if (!pan.viewing || pan.photo != state.memorial.photo || state.memorial.zoom == 0)
        && pan.offset != Vec2::ZERO
    {
        pan.offset = Vec2::ZERO;
    }
    if pan.photo != state.memorial.photo {
        pan.photo = state.memorial.photo;
    }
    if !pan.viewing {
        pan.viewing = true;
    }
    let Ok(viewport) = viewports.single() else {
        return;
    };
    let room = viewport.size() * viewport.inverse_scale_factor();
    if pan.room != room {
        pan.room = room;
    }
    let size = fit_size(room, photos::PHOTOS[state.memorial.photo].1);
    let zoom = 1.0 + f32::from(state.memorial.zoom) * 0.5;
    let bounds = ((size * zoom - room) * 0.5).max(Vec2::ZERO);
    let offset = pan.offset.clamp(-bounds, bounds);
    if pan.offset != offset {
        pan.offset = offset;
    }
    for (mut node, mut transform, computed, mut visible) in &mut photos {
        if (computed.size() * computed.inverse_scale_factor() - size)
            .abs()
            .max_element()
            < 1.0
            && size.min_element() > 0.0
        {
            visible.set_if_neq(Visibility::Inherited);
        }
        let wanted = (Val::Px(size.x), Val::Px(size.y));
        if (node.width, node.height) != wanted {
            node.width = wanted.0;
            node.height = wanted.1;
        }
        let next = UiTransform {
            scale: Vec2::splat(zoom),
            translation: Val2::px(pan.offset.x, pan.offset.y),
            ..default()
        };
        transform.set_if_neq(next);
    }
}

fn fit_size(room: Vec2, aspect: f32) -> Vec2 {
    let height = (room.y * 0.96).min(room.x * 0.96 / aspect).max(0.0);
    Vec2::new(height * aspect, height)
}

/// Seed dimensions before UI layout. Rebuilding for zoom must never paint a 1px photo.
pub(super) fn restore(c: &mut Commands, entity: Entity) {
    c.queue(move |world: &mut World| {
        let album = world.resource::<LobbyState>().memorial;
        let pan = world.resource::<PhotoPan>();
        let size = fit_size(pan.room, photos::PHOTOS[album.photo].1);
        let zoom = 1.0 + f32::from(album.zoom) * 0.5;
        let offset = if pan.viewing && pan.photo == album.photo && album.zoom > 0 {
            pan.offset
        } else {
            Vec2::ZERO
        };
        let bounds = ((size * zoom - pan.room) * 0.5).max(Vec2::ZERO);
        let offset = offset.clamp(-bounds, bounds);
        if let Ok(mut node) = world.get_entity_mut(entity) {
            node.insert((
                Node {
                    width: Val::Px(size.x),
                    height: Val::Px(size.y),
                    flex_shrink: 0.0,
                    ..default()
                },
                UiTransform {
                    scale: Vec2::splat(zoom),
                    translation: Val2::px(offset.x, offset.y),
                    ..default()
                },
            ));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rebuilding_zoom_seeds_fitted_dimensions_and_clamped_pan_before_layout() {
        let mut world = World::new();
        let mut state = LobbyState::default();
        state.memorial.viewing = true;
        state.memorial.zoom = 6;
        world.insert_resource(state);
        world.insert_resource(PhotoPan {
            room: Vec2::new(1000.0, 600.0),
            viewing: true,
            photo: 0,
            offset: Vec2::new(9000.0, 9000.0),
        });
        let entity = world
            .spawn((LargePhoto, Node::default(), Visibility::Hidden))
            .id();
        restore(&mut world.commands(), entity);
        world.flush();
        let node = world.get::<Node>(entity).unwrap();
        assert_eq!((node.width, node.height), (Val::Px(324.0), Val::Px(576.0)));
        let transform = world.get::<UiTransform>(entity).unwrap();
        assert_eq!(transform.scale, Vec2::splat(4.0));
        assert_eq!(transform.translation, Val2::px(148.0, 852.0));
        world.resource_mut::<LobbyState>().memorial.photo = 1;
        restore(&mut world.commands(), entity);
        world.flush();
        assert_eq!(
            world.get::<UiTransform>(entity).unwrap().translation,
            Val2::ZERO
        );
    }
}

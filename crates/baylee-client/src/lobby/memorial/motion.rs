//! Finite slides preserve their progress when the shell rebuilds for input.
use crate::{lobby::LobbyState, prefs::Prefs};
use bevy::prelude::*;

#[derive(Component)]
struct Slide {
    photo: bool,
}
#[derive(Resource, Default)]
struct Motion {
    open: bool,
    viewing: bool,
    photo: usize,
    surface: f32,
    preview: f32,
    direction: f32,
}
pub(super) struct MotionPlugin;
impl Plugin for MotionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Motion>().add_systems(
            Update,
            animate
                .after(crate::lobby::keyboard::keyboard)
                .after(crate::lobby::clicks::clicks)
                .after(crate::lobby::front::keys::activate_by_key)
                .after(crate::lobby::focusing::keys_press)
                .before(crate::lobby::shell::ui),
        );
    }
}
pub(super) fn attach(c: &mut Commands, entity: Entity, photo: bool) {
    c.queue(move |world: &mut World| {
        let transform = pose(world.resource::<Motion>(), photo);
        if let Ok(mut node) = world.get_entity_mut(entity) {
            node.insert((Slide { photo }, transform));
        }
    });
}
fn pose(motion: &Motion, photo: bool) -> UiTransform {
    let t = if photo {
        motion.preview
    } else {
        motion.surface
    };
    let remaining = (1.0 - t).powi(3);
    UiTransform::from_translation(if photo {
        Val2::px(12.0 * remaining * motion.direction, 0.0)
    } else {
        Val2::px(0.0, 14.0 * remaining)
    })
}
#[allow(clippy::float_cmp)] // Exact retained state prevents idle ECS writes.
fn animate(
    time: Res<Time>,
    state: Res<LobbyState>,
    prefs: Res<Prefs>,
    mut motion: ResMut<Motion>,
    mut nodes: Query<(&Slide, &mut UiTransform)>,
) {
    let album = state.memorial;
    if !album.open {
        if motion.open {
            motion.open = false;
        }
        return;
    }
    if !motion.open || motion.viewing != album.viewing {
        motion.surface = 0.0;
        motion.open = true;
        motion.viewing = album.viewing;
    }
    if motion.photo != album.photo {
        motion.direction = if album.photo_forward { 1.0 } else { -1.0 };
        motion.photo = album.photo;
        motion.preview = 0.0;
    }
    let step = time.delta_secs() / 0.3;
    let next = if prefs.all().reduce_motion {
        (1.0, 1.0)
    } else {
        (
            (motion.surface + step).min(1.0),
            (motion.preview + step).min(1.0),
        )
    };
    if (motion.surface, motion.preview) != next {
        motion.surface = next.0;
        motion.preview = next.1;
    }
    for (slide, mut transform) in &mut nodes {
        transform.set_if_neq(pose(&motion, slide.photo));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wrapping_keeps_the_requested_slide_direction() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<Prefs>()
            .init_resource::<LobbyState>()
            .insert_resource(Motion {
                open: true,
                photo: 9,
                ..default()
            })
            .add_systems(Update, animate);
        {
            let mut state = app.world_mut().resource_mut::<LobbyState>();
            state.memorial.open = true;
            state.memorial.photo = 9;
            state.memorial.turn(true);
        }
        app.update();
        assert!(app.world().resource::<Motion>().direction > 0.0);
        app.world_mut()
            .resource_mut::<LobbyState>()
            .memorial
            .turn(false);
        app.update();
        assert!(app.world().resource::<Motion>().direction < 0.0);
    }
}

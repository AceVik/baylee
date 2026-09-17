//! Short, bounded combat gestures. Geometry and sound share the same clock.
use crate::{
    Duel,
    table::{CardVisual, DuelStage, Lens, ShownRig, TABLE_Y},
};
use baylee_client_core::{combat::LineEnd, cue::Cue};
use bevy::prelude::*;

/// Added to the glide's goal, never accumulated into a card's resting pose.
#[derive(Component)]
pub struct Recoil {
    started: f32,
    direction: Vec3,
    duration: f32,
    earlier: Option<f32>,
}
impl Recoil {
    pub(crate) fn offset(&self, now: f32) -> Vec3 {
        let pulse_at = |start: f32, duration: f32| {
            let t = ((now - start) / duration).clamp(0.0, 1.0);
            (std::f32::consts::PI * t).sin().powi(2)
        };
        let pulse = pulse_at(self.started, self.duration)
            + self.earlier.map_or(0.0, |start| pulse_at(start, 0.30));
        self.direction * pulse * 0.30 + Vec3::Y * pulse * 0.075
    }
}
#[derive(Component)]
pub(crate) struct Impact {
    started: f32,
    duration: f32,
    first: bool,
    cue: bool,
    slash: bool,
}

/// Advance and retire effects; sound is emitted when the gesture begins.
#[allow(clippy::too_many_arguments)]
pub(crate) fn age(
    mut commands: Commands,
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut duel: ResMut<Duel>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    recoils: Query<(Entity, &Recoil)>,
    mut impacts: Query<
        (
            Entity,
            &mut Impact,
            &mut Transform,
            &MeshMaterial3d<StandardMaterial>,
        ),
        Without<CardVisual>,
    >,
) {
    let now = time.elapsed_secs();
    let still = prefs.all().reduce_motion;
    for (entity, recoil) in &recoils {
        if now >= recoil.started + recoil.duration || still {
            commands.entity(entity).remove::<Recoil>();
        }
    }
    for (entity, mut hit, mut at, material) in &mut impacts {
        let elapsed = now - hit.started;
        if elapsed < 0.0 {
            continue;
        }
        if hit.cue {
            duel.cues.push(if hit.first {
                Cue::FirstStrike
            } else {
                Cue::CombatStrike
            });
            hit.cue = false;
        }
        let t = elapsed / hit.duration;
        if t >= 1.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let alpha = (1.0 - t).powi(2) * 0.85;
        if let Some(mut mat) = materials.get_mut(&material.0) {
            mat.base_color.set_alpha(alpha);
        }
        let grow = if still { 1.0 } else { 0.4 + t * 1.2 };
        at.scale = if hit.slash {
            Vec3::new(grow * 1.25, 0.045 * (1.0 - t), 1.0)
        } else {
            Vec3::splat(grow)
        };
    }
}

/// At most 24 simultaneous combatants; excess strikes still contribute to
/// the game's numbers. Shared meshes and transient materials keep costs bounded.
#[allow(clippy::too_many_arguments)]
pub(crate) fn animate(
    mut commands: Commands,
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut duel: ResMut<Duel>,
    shown: Res<ShownRig>,
    windows: Query<&Window>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cards: Query<(Entity, &CardVisual, &Transform), Without<Impact>>,
    impacts: Query<Entity, With<Impact>>,
    mut geometry: Local<Option<(Handle<Mesh>, Handle<Mesh>)>>,
) {
    let now = time.elapsed_secs();
    let still = prefs.all().reduce_motion;
    let strikes = std::mem::take(&mut duel.strikes);
    if strikes.is_empty() {
        return;
    }
    let lens = shown
        .rig()
        .zip(windows.single().ok())
        .map(|(rig, w)| Lens::new(rig, Vec2::new(w.width(), w.height())));
    let (ring, slash) = geometry
        .get_or_insert_with(|| {
            (
                meshes.add(Annulus::new(0.30, 0.34)),
                meshes.add(Rectangle::new(1.0, 1.0)),
            )
        })
        .clone();
    let mut voiced = [false; 2];
    let first_sources: Vec<_> = strikes
        .iter()
        .filter(|s| s.first)
        .map(|s| s.source)
        .collect();
    let has_first = !first_sources.is_empty();
    let capacity = 48usize.saturating_sub(impacts.iter().count()) / 2;
    for strike in strikes.into_iter().take(capacity.min(24)) {
        let Some((entity, _, source)) = cards.iter().find(|(_, c, _)| c.object == strike.source)
        else {
            continue;
        };
        let target = match strike.target {
            LineEnd::Object(id) => cards
                .iter()
                .find(|(_, c, _)| c.object == id)
                .map(|(_, _, t)| t.translation),
            LineEnd::Seat(id) => duel
                .layout
                .as_ref()
                .and_then(|l| l.slot(id))
                .map(|s| crate::combatlines::player_end(s, lens.as_ref())),
        };
        let Some(target) = target else {
            continue;
        };
        let start = now
            + if !strike.first && has_first {
                0.32
            } else {
                0.0
            };
        let duration = if strike.first { 0.40 } else { 0.62 };
        let direction = Vec3::new(
            target.x - source.translation.x,
            0.0,
            target.z - source.translation.z,
        )
        .normalize_or_zero();
        if !still {
            commands.entity(entity).insert(Recoil {
                started: start,
                direction,
                duration,
                earlier: (!strike.first && first_sources.contains(&strike.source)).then_some(now),
            });
        }
        let index = usize::from(strike.first);
        for is_slash in [false, true] {
            let color = impact_color(strike.first);
            commands.spawn((
                DuelStage,
                Impact {
                    started: start,
                    duration,
                    first: strike.first,
                    cue: !voiced[index] && !is_slash,
                    slash: is_slash,
                },
                Mesh3d(if is_slash {
                    slash.clone()
                } else {
                    ring.clone()
                }),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: color.with_alpha(0.0),
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                })),
                Transform {
                    translation: Vec3::new(target.x, target.y.max(TABLE_Y) + 0.055, target.z),
                    rotation: Quat::from_rotation_y(if strike.first { -0.65 } else { 0.65 })
                        * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                    ..default()
                },
            ));
        }
        voiced[index] = true;
    }
}

fn impact_color(first: bool) -> Color {
    if first {
        Color::srgb(1.0, 0.84, 0.42)
    } else {
        Color::srgb(1.0, 0.40, 0.13)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_strike_returns_to_the_original_pose_without_drift() {
        let recoil = Recoil {
            started: 2.0,
            direction: Vec3::X,
            duration: 0.6,
            earlier: None,
        };
        assert!(recoil.offset(1.0).length() < 1e-5);
        assert!(recoil.offset(2.3).x > 0.29);
        assert!(recoil.offset(3.0).length() < 1e-5);
    }
    #[test]
    fn reduced_motion_keeps_cards_at_their_resting_pose() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<crate::prefs::Prefs>()
            .add_systems(Update, crate::table::glide);
        app.world_mut()
            .resource_mut::<crate::prefs::Prefs>()
            .edit()
            .reduce_motion = true;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.3));
        let target = Transform::from_xyz(2.0, 0.1, 4.0);
        let e = app
            .world_mut()
            .spawn((
                target,
                crate::table::Motion { target },
                Recoil {
                    started: 0.0,
                    direction: Vec3::X,
                    duration: 0.6,
                    earlier: None,
                },
            ))
            .id();
        app.update();
        assert_eq!(*app.world().get::<Transform>(e).unwrap(), target);
    }
}

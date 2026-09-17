//! A spring-driven compass: one detent per turn, independent of priority.
use bevy::audio::Volume;
use bevy::prelude::*;

/// Lives on the slab so a new game starts settled, without an arrival sound.
#[derive(Component, Default)]
pub struct Compass {
    seen: Option<(baylee_core::ids::PlayerId, u32)>,
    start: f32,
    angle: f32,
    target: f32,
    elapsed: f32,
}

/// Cached with the other synthesised cues at startup.
#[derive(Resource)]
pub(crate) struct CompassVoice(pub Handle<AudioSource>);

const DURATION: f32 = 1.15;

impl Compass {
    fn advance(
        &mut self,
        turn: (baylee_core::ids::PlayerId, u32),
        seats: usize,
        dt: f32,
        still: bool,
    ) -> bool {
        let changed = self.seen.is_some_and(|seen| seen != turn);
        self.seen = Some(turn);
        if changed {
            // Rebase each movement to avoid precision loss over long sessions.
            self.start = self.angle.rem_euclid(std::f32::consts::TAU);
            self.target = self.start + std::f32::consts::TAU / seats.max(2) as f32;
            self.elapsed = 0.0;
        }
        self.elapsed = if still {
            DURATION
        } else {
            (self.elapsed + dt).min(DURATION)
        };
        let u = (self.elapsed / DURATION).clamp(0.0, 1.0);
        // Accelerate the heavy ring, then catch it with a small spring detent.
        let smooth = u * u * (3.0 - 2.0 * u);
        let detent = (u * std::f32::consts::PI).sin().powi(2)
            * (u * std::f32::consts::TAU * 2.0).sin()
            * 0.022;
        self.angle = if still {
            self.target
        } else {
            self.start + (self.target - self.start) * (smooth + detent)
        };
        changed
    }
}

/// The shader clock animates the stones; only the ring's movement uploads a uniform.
#[allow(clippy::too_many_arguments)]
pub(crate) fn rotate(
    mut commands: Commands,
    duel: Res<crate::Duel>,
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut slabs: Query<(&mut Compass, &MeshMaterial3d<crate::feltmat::FeltMaterial>)>,
    mut materials: ResMut<Assets<crate::feltmat::FeltMaterial>>,
    voice: Option<Res<CompassVoice>>,
) {
    let Some(view) = &duel.view else {
        return;
    };
    let Ok((mut compass, handle)) = slabs.single_mut() else {
        return;
    };
    let changed = compass.advance(
        (view.active, view.turn),
        view.seats.len(),
        time.delta_secs(),
        prefs.all().reduce_motion,
    );
    if materials
        .get(&handle.0)
        .is_some_and(|material| (material.params.rotation - compass.angle).abs() > 1e-5)
        && let Some(mut material) = materials.get_mut(&handle.0)
    {
        material.params.rotation = compass.angle;
    }
    let gain = prefs.all().sound.gain();
    if changed
        && gain > 0.0
        && let Some(voice) = voice
    {
        commands.spawn((
            AudioPlayer::new(voice.0.clone()),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(gain)),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::PlayerId;

    #[test]
    fn a_turn_rotates_once_and_motion_off_lands_at_the_same_detent() {
        let mut ring = Compass::default();
        assert!(!ring.advance((PlayerId::new(0), 1), 2, 0.016, false));
        assert!(ring.advance((PlayerId::new(1), 2), 2, 0.016, false));
        assert!(ring.angle > 0.0 && ring.angle < 0.1);
        assert!(!ring.advance((PlayerId::new(1), 2), 2, DURATION, false));
        assert!((ring.angle - std::f32::consts::PI).abs() < 1e-5);
        assert!(ring.advance((PlayerId::new(0), 3), 2, 0.016, true));
        assert!((ring.angle - std::f32::consts::TAU).abs() < 1e-5);
    }
}

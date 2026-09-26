//! The successful sign-in's flight into the lobby. Authentication and data
//! loading finish normally underneath it; this owns only presentation.

use super::{LobbyState, Screen};
use bevy::prelude::*;

const SECONDS: f32 = 1.65;

#[derive(Resource, Default)]
pub(super) struct Entrance {
    saw_front: bool,
    progress: Option<f32>,
}

impl Entrance {
    pub(super) fn active(&self) -> bool {
        self.progress.is_some_and(|p| p < 1.0)
    }

    pub(super) fn progress(&self) -> f32 {
        self.progress.unwrap_or(0.0)
    }

    fn advance(&mut self, front: bool, authenticated: bool, still: bool, dt: f32) {
        if front || !authenticated || still {
            self.progress = None;
        } else if self.saw_front {
            self.progress = Some(0.0);
        } else if let Some(progress) = &mut self.progress {
            *progress = (*progress + dt.max(0.0) / SECONDS).min(1.0);
        }
        self.saw_front = front;
    }
}

pub(super) fn advance(
    time: Res<Time>,
    state: Res<LobbyState>,
    prefs: Res<crate::prefs::Prefs>,
    surfaces: Query<&crate::vista::Vista>,
    mut entrance: ResMut<Entrance>,
) {
    // An embedded/headless lobby without a scene has no visual flight to
    // wait for. A cold session restore has never shown the front door.
    let has_scene = surfaces
        .iter()
        .any(|kind| *kind == crate::vista::Vista::Front);
    entrance.advance(
        matches!(state.lobby.screen(), Screen::SignIn { .. }),
        state.lobby.token().is_some(),
        prefs.all().reduce_motion || !has_scene,
        time.delta_secs(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_successful_authentication_leaves_the_front_door() {
        let mut film = Entrance::default();
        film.advance(true, false, false, 0.0);
        // Waiting for HTTP and a rejected password both remain on the form.
        film.advance(true, false, false, 10.0);
        assert!(!film.active());
        film.advance(false, true, false, 0.1);
        assert!(film.active());
        film.advance(false, true, false, SECONDS * 0.5);
        assert!((film.progress() - 0.5).abs() < 1e-6);
        film.advance(false, true, false, SECONDS);
        assert!(!film.active());
        film.advance(false, true, false, 1.0);
        assert!(!film.active(), "lobby updates must not replay the flight");
    }

    #[test]
    fn restore_and_offline_have_no_flight() {
        let mut film = Entrance::default();
        film.advance(false, true, false, 0.1);
        assert!(!film.active());
        film.advance(true, false, false, 0.1);
        film.advance(false, false, false, 0.1);
        assert!(!film.active());
    }

    #[test]
    fn signout_and_reduced_motion_cancel_immediately() {
        for (front, signed_in, still) in [(true, false, false), (false, true, true)] {
            let mut film = Entrance::default();
            film.advance(true, false, false, 0.0);
            film.advance(false, true, false, 0.0);
            assert!(film.active());
            film.advance(front, signed_in, still, 0.1);
            assert!(!film.active());
        }
    }
}

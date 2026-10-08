//! The lighting machinery bevy runs for a scene with no light in it.
//!
//! Nothing this client draws is lit (`docs/client.md`; the table stage spawns
//! no light, and card art must never be tinted by one). Bevy's PBR plugin
//! does its light work whether or not a light exists, and two parts of it
//! were measurable on a table at rest (`docs/perf-baseline.md`, 08.10.2026):
//!
//! - **Light visibility.** `check_dir_light_mesh_visibility` and
//!   `check_point_light_mesh_visibility` build a fresh query state every
//!   frame to walk the lights, and with none they find nothing: about a
//!   fifth of the main thread's allocations. They run only while a light
//!   exists ([`install`]).
//! - **Clustering.** The table camera bins clusterable objects (lights,
//!   light probes, decals) into a view-space grid, on the GPU, four compute
//!   and raster passes per frame. There are none to bin, so the camera says
//!   so ([`CAMERA_CLUSTERS`]).
//!
//! Spawn a light one day and both come back by themselves: the visibility
//! sets by their run condition, and clustering by taking
//! [`CAMERA_CLUSTERS`] off the camera.

use bevy::light::cluster::{ClusterConfig, GlobalClusterSettings};
use bevy::light::{SimulationLightSystems, WithLight};
use bevy::prelude::*;

/// The table camera's clustering: none, since nothing is lit.
pub const CAMERA_CLUSTERS: ClusterConfig = ClusterConfig::None;

/// Runs bevy's light-visibility checks only while a light exists, and bins
/// clusterable objects on the CPU.
pub fn install(app: &mut App) {
    app.configure_sets(
        PostUpdate,
        SimulationLightSystems::CheckLightVisibility.run_if(any_light),
    )
    .add_systems(Startup, cluster_on_the_cpu);
}

/// Turns GPU clustering off, which `PbrPlugin::finish` turned on where the
/// device can do it. GPU clustering cannot take a camera that asks for no
/// clusters (a zero-wide "clustering dummy texture" fails validation and
/// quits the app, bevy 0.19.1), and the CPU path, which phones and the iOS
/// simulator always take, can: with [`CAMERA_CLUSTERS`] it bins nothing.
fn cluster_on_the_cpu(settings: Option<ResMut<GlobalClusterSettings>>) {
    if let Some(mut settings) = settings {
        settings.gpu_clustering = None;
    }
}

/// Whether any light stands in the world.
fn any_light(lights: Query<(), WithLight>) -> bool {
    !lights.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Resource, Default)]
    struct Ran(u32);

    fn count(mut ran: ResMut<Ran>) {
        ran.0 += 1;
    }

    /// The light-visibility set is skipped while no light exists and runs
    /// again the frame one is spawned.
    #[test]
    fn light_visibility_runs_only_with_a_light() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Ran>()
            .add_systems(
                PostUpdate,
                count.in_set(SimulationLightSystems::CheckLightVisibility),
            );
        install(&mut app);
        app.update();
        app.update();
        assert_eq!(app.world().resource::<Ran>().0, 0, "no light, no check");
        app.world_mut().spawn(bevy::light::PointLight::default());
        app.update();
        assert_eq!(app.world().resource::<Ran>().0, 1, "a light is checked");
    }
}

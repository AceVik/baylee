//! Standalone duel client.
//!
//! A thin wrapper: create a window, install a local host, open the duel. The
//! open-world client will do the same three things inside an app it already
//! owns, which is the point of keeping [`baylee_client::DuelPlugin`] free of
//! any window or schedule of its own.

use baylee_client::{DuelCommand, DuelConfig, DuelPlugin, InstalledHost, LocalHost};
use baylee_core::ids::PlayerId;
use bevy::prelude::*;

fn main() {
    let Some(preset) = acceptance_duel() else {
        eprintln!("could not find data/acceptance-decks.txt — run this from the repository root");
        return;
    };
    let Some(host) = LocalHost::new(&preset, PlayerId::new(0), &["You", "House AI"]) else {
        eprintln!("could not start the demo duel");
        return;
    };

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "baylee".to_string(),
                // Works as a desktop window and as a browser canvas; on mobile
                // the platform decides and this is ignored.
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(DuelPlugin {
            config: DuelConfig::default(),
        })
        .insert_resource(InstalledHost(Box::new(host)))
        .add_systems(Startup, open_duel)
        .run();
}

fn open_duel(mut commands: MessageWriter<DuelCommand>) {
    commands.write(DuelCommand::Open);
}

/// The demo duel from the acceptance decks.
///
/// The deck file is looked for next to the working directory first and then
/// beside the source tree, so `cargo run` from anywhere in the workspace works
/// and so does a binary run from the repository root.
fn acceptance_duel() -> Option<baylee_core::preset::GamePreset> {
    const CANDIDATES: [&str; 3] = [
        "data/acceptance-decks.txt",
        "../data/acceptance-decks.txt",
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/acceptance-decks.txt"
        ),
    ];
    let text = CANDIDATES
        .iter()
        .find_map(|path| std::fs::read_to_string(path).ok())?;
    baylee_client::host::demo_duel(&text, rand_seed())
}

/// A fresh shuffle per launch.
///
/// The engine is deterministic given a seed, which is what makes replays work;
/// that is a reason to *record* the seed, never a reason to reuse one.
fn rand_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0x5eed_1234, |d| d.as_nanos() as u64)
}

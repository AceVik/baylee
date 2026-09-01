//! Standalone duel client.
//!
//! A thin wrapper: create a window, install a host, open the duel. The
//! open-world client will do the same three things inside an app it already
//! owns, which is the point of keeping [`baylee_client::DuelPlugin`] free of
//! any window or schedule of its own.
//!
//! Which host depends on whether this launch was handed a seat: with a game id
//! and a seat token (from the environment natively, from the page's query
//! string in a browser) the duel is played against the gateway; without one it
//! is played solo against the house AI, in this process. Nothing above the
//! host can tell the difference — that is the whole point of the seam.

use baylee_client::host::DuelHost;
use baylee_client::{
    DuelCommand, DuelConfig, DuelPlugin, InstalledHost, LocalHost, NetworkHost, SeatTicket,
};
use baylee_core::ids::PlayerId;
use bevy::prelude::*;

fn main() {
    let Some(host) = duel_host() else {
        return;
    };

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some({
                        let mut window = Window {
                            title: "baylee".to_string(),
                            // A regular decorated window: the system close /
                            // minimize buttons stay available.
                            fit_canvas_to_parent: true,
                            ..default()
                        };
                        // Starts maximized (decorations kept).
                        window.set_maximized(true);
                        window
                    }),
                    ..default()
                })
                .set(bevy::asset::AssetPlugin {
                    // Natively the fonts live in the crate's assets dir (run
                    // from the repo root or anywhere else); trunk copies that
                    // dir to `dist/assets`, the browser's asset root.
                    file_path: asset_root().to_string(),
                    ..default()
                }),
        )
        .add_plugins(DuelPlugin {
            config: DuelConfig::default(),
        })
        .insert_resource(InstalledHost(host))
        .add_systems(Startup, open_duel)
        .run();
}

/// The host this launch plays through, or `None` after saying why it cannot.
///
/// A ticket that is present but unusable is a hard stop rather than a quiet
/// fall back to solo play: somebody is waiting at that table, and dropping
/// them into a game against the house instead would be the worst possible
/// answer to "the network is down".
fn duel_host() -> Option<Box<dyn DuelHost>> {
    if let Some(ticket) = SeatTicket::discover() {
        // Never the URL: it carries the seat token.
        let table = ticket.gateway.clone();
        return match NetworkHost::connect(ticket) {
            Ok(host) => Some(Box::new(host)),
            Err(reason) => {
                eprintln!("could not reach the table at {table}: {reason}");
                None
            }
        };
    }
    let Some(preset) = acceptance_duel() else {
        eprintln!("could not find data/acceptance-decks.txt — run this from the repository root");
        return None;
    };
    let Some(host) = LocalHost::new(&preset, PlayerId::new(0), &["You", "House AI"]) else {
        eprintln!("could not start the demo duel");
        return None;
    };
    Some(Box::new(host))
}

fn open_duel(mut commands: MessageWriter<DuelCommand>) {
    commands.write(DuelCommand::Open);
}

/// Where the asset server looks. Relative paths resolve against the
/// executable's directory (target/...), not the working directory — so
/// natively the crate's assets dir is baked in as an absolute path at
/// build time. Trunk copies the same dir to `dist/assets`, the browser's
/// asset root.
fn asset_root() -> &'static str {
    if cfg!(target_arch = "wasm32") {
        "assets"
    } else {
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets")
    }
}

/// The demo duel from the acceptance decks.
///
/// The deck file is looked for next to the working directory first and then
/// beside the source tree, so `cargo run` from anywhere in the workspace works
/// and so does a binary run from the repository root. The final fallback is
/// the copy embedded at build time — a browser has no filesystem at all.
fn acceptance_duel() -> Option<baylee_core::preset::GamePreset> {
    /// Embedded copy of the deck file (the only source in a browser build).
    const EMBEDDED: &str = include_str!("../../../data/acceptance-decks.txt");
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
        .find_map(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_else(|| EMBEDDED.to_string());
    baylee_client::host::demo_duel(&text, rand_seed())
}

/// A fresh shuffle per launch.
///
/// The engine is deterministic given a seed, which is what makes replays work;
/// that is a reason to *record* the seed, never a reason to reuse one. The
/// seed comes from the platform CSPRNG (Web Crypto in the browser) —
/// `std::time` panics on wasm32, so it is not an option here.
fn rand_seed() -> u64 {
    let mut bytes = [0u8; 8];
    match getrandom::fill(&mut bytes) {
        Ok(()) => u64::from_le_bytes(bytes),
        Err(_) => 0x5eed_1234,
    }
}

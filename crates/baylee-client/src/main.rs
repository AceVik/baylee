//! Standalone duel client.
//!
//! A thin wrapper: create a window, install a host, open the duel. The
//! open-world client will do the same three things inside an app it already
//! owns, which is the point of keeping [`baylee_client::DuelPlugin`] free of
//! any window or schedule of its own.
//!
//! Which front door depends on whether this launch was handed a seat: with a
//! game id and a seat token (from the environment natively, from the page's
//! query string in a browser) the duel opens straight against that table.
//! Without one the client shows its own lobby — sign in, pick a deck, take a
//! seat — and installs exactly the same [`NetworkHost`] when it gets one.
//! Nothing above the host can tell the two apart; that is the whole point of
//! the seam.

use baylee_client::host::DuelHost;
use baylee_client::{
    DuelCommand, DuelConfig, DuelPlugin, InstalledHost, LobbyPlugin, NetworkHost, SeatTicket,
};
use bevy::prelude::*;

fn main() {
    // A ticket means somebody is already waiting at a table; anything else
    // starts at the lobby.
    let seated = match seated_host() {
        Ok(host) => host,
        Err(reason) => {
            eprintln!("{reason}");
            return;
        }
    };

    let mut app = App::new();
    app.add_plugins(
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
    });
    // The dev-control harness, when this build has it and the environment
    // asks for it. Added before the front door so a lobby session can be
    // driven too, not only a seated duel.
    #[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
    if let Some(control) = baylee_client::devctl::DevControlPlugin::from_env() {
        app.add_plugins(control);
    }
    match seated {
        Some(host) => {
            app.insert_resource(InstalledHost(host))
                .add_systems(Startup, open_duel);
        }
        None => {
            app.add_plugins(LobbyPlugin);
        }
    }
    app.run();
}

/// The host for a launch that was handed a seat, if it was handed one.
///
/// A ticket that is present but unusable is a hard stop rather than a quiet
/// fall back to the lobby: somebody is waiting at that table, and offering to
/// sign in somewhere else would be the worst possible answer to "the network
/// is down".
fn seated_host() -> Result<Option<Box<dyn DuelHost>>, String> {
    let Some(ticket) = SeatTicket::discover() else {
        return Ok(None);
    };
    // Never the URL: it carries the seat token.
    let table = ticket.gateway.clone();
    match NetworkHost::connect(ticket) {
        Ok(host) => Ok(Some(Box::new(host))),
        Err(reason) => Err(format!("could not reach the table at {table}: {reason}")),
    }
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

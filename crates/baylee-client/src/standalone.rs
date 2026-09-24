//! The standalone duel client, as a function rather than a `main`.
//!
//! A thin wrapper: create a window, install a host, open the duel. The
//! open-world client will do the same three things inside an app it already
//! owns, which is the point of keeping [`DuelPlugin`](crate::DuelPlugin) free
//! of any window or schedule of its own.
//!
//! Which front door depends on whether this launch was handed a seat: with a
//! game id and a seat token (from the environment natively, from the page's
//! query string in a browser) the duel opens straight against that table.
//! Without one the client shows its own lobby — sign in, pick a deck, take a
//! seat — and installs exactly the same [`NetworkHost`](crate::NetworkHost)
//! when it gets one. Nothing above the host can tell the two apart; that is
//! the whole point of the seam.
//!
//! It lives in the library and not in `main.rs` because a phone does not
//! start a program by calling `main`. Android loads a shared object and calls
//! `android_main`, which is what `#[bevy_main]` writes in
//! `baylee-client-android`; that entry point needs the same three steps, and
//! two copies of them would drift the first time one of them changed.

use crate::host::DuelHost;
use crate::{
    DuelCommand, DuelConfig, DuelPlugin, InstalledHost, LobbyPlugin, NetworkHost, SeatTicket,
};
use bevy::prelude::*;

/// Builds the app and runs it. Returns when the window closes.
pub fn run() {
    // First, before anything reads a setting: this is a player's client and
    // may use their files. A test never comes through here, so a test never
    // touches them (`settings::store_is_open`).
    crate::settings::open_store();
    // Hot shader reload is watching from a root, and the wrong root reloads
    // nothing while looking exactly like the right one.
    #[cfg(all(feature = "dev-reload", not(target_arch = "wasm32")))]
    if let Err(reason) = watching_from_the_workspace_root() {
        eprintln!("{reason}");
        return;
    }
    // A ticket means somebody is already waiting at a table; anything else
    // starts at the lobby.
    let seated = match seated_host() {
        Ok(host) => host,
        Err(reason) => {
            eprintln!("{reason}");
            return;
        }
    };

    let plugins = DefaultPlugins
        .set(WindowPlugin {
            primary_window: Some({
                let mut window = Window {
                    title: "baylee".to_string(),
                    // A regular decorated window: the system close /
                    // minimize buttons stay available.
                    fit_canvas_to_parent: true,
                    ..default()
                };
                // Starts maximized (decorations kept). A phone has no
                // window manager to ask, and ignores it.
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
            // This repo ships no `.meta` file at all — `find assets -name
            // '*.meta'` is empty — so the default `Always` asks for a
            // sibling that never exists, and what happens next is not a 404.
            // A static host for a single-page app answers an unknown path
            // with `index.html` and a **200**, so bevy read a page of HTML
            // as a RON `AssetMetaMinimal`, failed, and treated the *asset*
            // as failed with it: measured in Chrome against `trunk serve`,
            // all eight fonts errored and the browser client drew no text
            // anywhere — no life totals, no prompt, no button labels — while
            // the table, the cards and their Scryfall art (which come over
            // HTTP and not through the asset server) rendered perfectly. It
            // is set for every platform and not behind a `cfg`, because
            // `Never` is what this repo means everywhere and a browser is
            // only where it was noticed.
            meta_check: bevy::asset::AssetMetaCheck::Never,
            ..default()
        });
    // What this build refuses to let a driver do. Empty everywhere but on a
    // phone, where one feature bit keeps bevy off a compute shader PowerVR's
    // compiler aborts on; `crate::gpu::disabled_features` has the whole
    // chain. Set unconditionally because `None` *is* the default — so on a
    // desktop this line builds exactly the `RenderPlugin` bevy would have.
    let plugins = plugins.set(bevy::render::RenderPlugin {
        render_creation: bevy::render::settings::WgpuSettings {
            disabled_features: crate::gpu::disabled_features(),
            ..default()
        }
        .into(),
        ..default()
    });

    let mut app = App::new();
    app.add_plugins(plugins).add_plugins(DuelPlugin {
        config: DuelConfig::default(),
    });
    // A phone's run loop is not a desktop's. `WinitSettings::default()` is
    // `game()`, which asks for `UpdateMode::Continuous` — and on iOS winit
    // hands the thread to `UIApplicationMain`, where nothing then wakes the
    // run loop: the table draws one frame and the process sits in
    // `CFRunLoopRun` at 3% CPU, alive and never asked for another. `mobile()`
    // asks for a frame on a 1/60 s timer instead, which is a `WaitUntil` the
    // run loop honours by itself, and which bevy's own `winit_config.rs`
    // names "default settings for mobile".
    #[cfg(any(target_os = "android", target_os = "ios"))]
    app.insert_resource(bevy::winit::WinitSettings::mobile());
    // The dev-control harness, when this build has it and the environment
    // asks for it. Added before the front door so a lobby session can be
    // driven too, not only a seated duel.
    #[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
    if let Some(control) = crate::devctl::DevControlPlugin::from_env() {
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
///
/// The two mobile arms are the same silent failure the browser one is: a
/// root that is one level off loads no font, draws no glyph, and reports
/// nothing.
fn asset_root() -> &'static str {
    if cfg!(target_arch = "wasm32") {
        "assets"
    } else if cfg!(target_os = "android") {
        // Android hands out no path at all. Bevy reads through the APK's
        // `AssetManager`, whose root *is* the `assets/` directory cargo-apk
        // packed, so the base has to be empty rather than "assets" — which
        // would look inside `assets/assets`.
        ""
    } else if cfg!(target_os = "ios") {
        // Beside the executable, inside the `.app` bundle: that is bevy's
        // own fallback when neither `BEVY_ASSET_ROOT` nor
        // `CARGO_MANIFEST_DIR` is set, and on a phone neither ever is.
        "assets"
    } else {
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets")
    }
}

/// Refuses to start when `BEVY_ASSET_ROOT` is not the workspace root.
///
/// `embedded_asset!` files a shader under the path `file!()` gives it, and
/// cargo writes that relative to the **workspace** root; bevy's watcher
/// strips its own base path off every changed file before looking it up, and
/// that base is `CARGO_MANIFEST_DIR` — this package, two directories deeper —
/// unless `BEVY_ASSET_ROOT` says otherwise. If the two are not the same
/// directory then every lookup misses: no error, no warning, and a client
/// that looks like it is watching. A hard stop is the only honest answer,
/// because the reload is the entire point of the feature.
#[cfg(all(feature = "dev-reload", not(target_arch = "wasm32")))]
fn watching_from_the_workspace_root() -> Result<(), String> {
    use std::path::Path;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("this crate sits two directories under the workspace root");
    let asked = std::env::var("BEVY_ASSET_ROOT").unwrap_or_default();
    // Through `canonicalize`, so a trailing slash or a symlinked checkout is
    // not read as a different directory.
    let same = match (std::fs::canonicalize(&asked), std::fs::canonicalize(root)) {
        (Ok(set), Ok(here)) => set == here,
        _ => false,
    };
    if same {
        return Ok(());
    }
    Err(format!(
        "dev-reload needs BEVY_ASSET_ROOT={} (it is {:?}); with any other root the \
         shader watcher looks every changed file up in the wrong place and reloads nothing",
        root.display(),
        asked
    ))
}

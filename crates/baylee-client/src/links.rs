//! `baylee:` links from the website (`docs/client.md` §"Links from the
//! website"): how they reach this client, and the window coming forward.
//! What the lobby makes of one is `lobby::links`.
//!
//! A link comes in one of three ways, and all three end in [`arrived`]:
//!
//! - **As an argument**, at a cold start on Windows and Linux: the system
//!   starts the launcher with the link, and the launcher passes its
//!   arguments to the client ([`start`]).
//! - **At the door** ([`baylee_update::link`]), while this client runs: a
//!   second launcher (or a second client started straight from a terminal)
//!   hands the link over and ends, so no second window opens.
//! - **As an Apple Event** on macOS, which sends it to the running app on a
//!   cold start as much as later (`links/macos.rs`).
//!
//! Every link is a stranger's words: [`intake`] reads it strictly
//! ([`deeplink::read`]), at most ten a minute, and keeps the latest for the
//! lobby. The lobby asks the player before anything a link asks for
//! happens; nothing here acts on one. The log names a link's kind, never its
//! text.

use baylee_client_core::deeplink::{self, Link, Origins, Pace};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::collections::VecDeque;
use std::sync::Mutex;

#[cfg(target_os = "macos")]
mod macos;

/// Links handed over and not yet read, oldest first. A static, because the
/// door's thread and macOS's Apple Event come from outside the world.
static INBOX: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

/// How many links wait to be read at most; more are dropped. A page firing
/// links in a loop is slowed again by [`Pace`].
const INBOX_CAP: usize = 16;

/// Which gateways a link may name: a `dev-control` build also takes a local
/// test gateway's numeric loopback address.
const ORIGINS: Origins = if cfg!(feature = "dev-control") {
    Origins::AlsoLoopback
} else {
    Origins::Public
};

/// A link handed to this client, from whichever way it came.
pub(crate) fn arrived(text: String) {
    if let Ok(mut inbox) = INBOX.lock()
        && inbox.len() < INBOX_CAP
    {
        inbox.push_back(text);
    }
}

/// The link read last and not yet followed, for the lobby to act on when it
/// can (`lobby::links`): never in a game, never over a question already
/// open. A newer link replaces one that waits.
#[derive(Resource, Debug, Default)]
pub struct Arrived(pub Option<Link>);

/// How links are read: the pace, by the clock it started on.
#[derive(Resource)]
struct Reading {
    pace: Pace,
    since: web_time::Instant,
}

impl Default for Reading {
    fn default() -> Self {
        Self {
            pace: Pace::default(),
            since: web_time::Instant::now(),
        }
    }
}

/// Reads the links that came in, keeps the last for the lobby, and brings
/// the window forward for each.
pub struct LinksPlugin;

impl Plugin for LinksPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Arrived>()
            .init_resource::<Reading>()
            .add_systems(Startup, say_how_links_come)
            .add_systems(PreUpdate, intake);
    }

    /// Before the event loop runs: macOS delivers a cold start's link while
    /// the app finishes launching.
    #[cfg(target_os = "macos")]
    fn finish(&self, _app: &mut App) {
        macos::listen();
    }
}

/// Reads what came in since the last frame.
fn intake(
    mut reading: ResMut<Reading>,
    mut arrived: ResMut<Arrived>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let texts: Vec<String> = match INBOX.lock() {
        Ok(mut inbox) if !inbox.is_empty() => inbox.drain(..).collect(),
        _ => return,
    };
    for text in texts {
        let Some(link) = deeplink::read(&text, ORIGINS) else {
            info!("links: not a link this client reads, dropped");
            continue;
        };
        let now = reading.since.elapsed();
        if !reading.pace.admit(now) {
            continue;
        }
        info!("links: {}", link.kind());
        arrived.0 = Some(link);
        for mut window in &mut windows {
            come_forward(&mut window);
        }
    }
}

/// The player clicked a link meant for this window: it comes forward,
/// unless it is not shown yet (a cold start shows it anyway) or it is a
/// watched agent run's, which never takes the focus.
fn come_forward(window: &mut Window) {
    #[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
    if crate::devctl::passive() {
        return;
    }
    if !window.visible {
        return;
    }
    window.set_minimized(false);
    if !window.focused {
        window.focused = true;
    }
}

/// What became of the door at [`start`], said once the log is up.
static DOOR: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();

fn say_how_links_come() {
    match DOOR.get() {
        Some(Ok(())) => info!("links: the door is open"),
        Some(Err(why)) => info!("links: no door ({why}); links reach only a starting client"),
        None => {}
    }
}

/// At start, before any window: takes a link among the arguments, or hands
/// it to the client already running here and answers `false`, which ends
/// this process before it opens anything. Otherwise opens this client's
/// door and answers `true`.
///
/// Only a client started without a launcher hands over: one the launcher
/// started is the only one its installation runs (the launcher knocked
/// itself if one ran).
#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
#[must_use]
pub fn start() -> bool {
    use baylee_update::link::{Door, Knock};
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    let link = baylee_update::link::among(&args);
    let launched = crate::update::native::launched_stage();
    let door = match launched {
        Some(stage) => Door::of_stage(stage),
        None => Door::direct(&direct_key()),
    };
    if let (Some(text), None, Ok(door)) = (&link, launched, &door) {
        let knock = Knock {
            link: text.clone(),
            token: None,
        };
        // Short: with nobody behind the door, this client is the one.
        if door
            .knock(&knock, std::time::Duration::from_millis(300))
            .unwrap_or(false)
        {
            return false;
        }
    }
    if let Some(text) = link {
        arrived(text);
    }
    let opened = door
        .and_then(|door| door.open(|knock| arrived(knock.link)))
        .map_err(|err| err.to_string());
    let _ = DOOR.set(opened);
    true
}

/// What tells a client started without a launcher apart: its program and
/// its settings, so two test clients with their own settings never take each
/// other's links.
#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
fn direct_key() -> Vec<u8> {
    let exe = std::env::current_exe().unwrap_or_default();
    let settings = crate::settings::store::folder("").unwrap_or_default();
    [
        exe.as_os_str().as_encoded_bytes(),
        b"\0",
        settings.as_os_str().as_encoded_bytes(),
    ]
    .concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Arrived>()
            .init_resource::<Reading>()
            .add_systems(Update, intake);
        app
    }

    /// One test, because the inbox is the process's: tests running beside
    /// each other would read each other's links.
    #[test]
    fn a_link_is_read_strictly_and_the_latest_waits_for_the_lobby() {
        let mut app = app();
        app.update();
        assert_eq!(app.world().resource::<Arrived>().0, None);
        arrived("baylee://gateway/add?origin=https%3A%2F%2Fa.example.org".into());
        arrived("baylee://gateway/add?origin=http%3A%2F%2Fnot-https.example".into());
        app.update();
        assert_eq!(
            app.world().resource::<Arrived>().0,
            Some(Link::AddGateway("https://a.example.org".into())),
            "the link that is one, and not the one that is not"
        );
        arrived("baylee://signin".into());
        app.update();
        assert_eq!(app.world().resource::<Arrived>().0, Some(Link::SignIn));
        // Ten a minute: the rest are dropped, not kept for later.
        for _ in 0..20 {
            arrived("baylee://open".into());
        }
        app.update();
        assert_eq!(app.world().resource::<Arrived>().0, Some(Link::Open));
        app.world_mut().resource_mut::<Arrived>().0 = None;
        arrived("baylee://signin".into());
        app.update();
        assert_eq!(
            app.world().resource::<Arrived>().0,
            None,
            "the eleventh link within the minute is not read"
        );
        assert!(INBOX.lock().unwrap().is_empty());
    }
}

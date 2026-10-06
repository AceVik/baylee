//! The updater behind the face, on a desktop build (#326).
//!
//! At start, before anything else reads the installation, an update a crash
//! interrupted is finished or undone, and one that finished last time is
//! announced once ("Updated to X"). Then a thread (`baylee_update::service`)
//! asks GitHub at start and every six hours, unless the player switched that
//! off, and downloads, verifies and unpacks a newer release beside the
//! installation. When the program ends (the `Drop` of its resource), a
//! staged update replaces it. `docs/client.md` §"Updating" is the whole story.
//!
//! A build that did not come out of the release workflow is a development
//! build: it checks and shows what it found, and never replaces itself,
//! because the folder it runs from is `target/release`, not an installation
//! (the notice says so). The release workflow marks its builds with
//! `BAYLEE_RELEASE_BUILD=1`.

use super::{Asked, Shown, UpdateNotice, UpdatePlugin, UpdatePrefs, UpdateRequest, Why};
use baylee_update::apply::{self, Install, Recovery, Unplaceable};
use baylee_update::check::{Context, GITHUB_RELEASES, Manual, Outcome};
use baylee_update::launch;
use baylee_update::plan::Os;
use baylee_update::service::{self, Command, Service, Settings};
use baylee_update::{VerifyingKey, Version, sign};
use bevy::prelude::*;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

/// Whether this build may replace itself: built by the release workflow,
/// with optimisations, from a clean commit.
#[must_use]
pub fn is_release_build() -> bool {
    option_env!("BAYLEE_RELEASE_BUILD") == Some("1")
        && !cfg!(debug_assertions)
        && !baylee_build::DIRTY
        && baylee_build::COMMIT != "unknown"
}

/// What this process is, as the updater sees it.
#[derive(Clone, Debug)]
pub struct Build {
    /// The version it reports.
    pub version: Version,
    /// Its target triple.
    pub target: String,
    /// Where it asks for releases.
    pub api: String,
    /// The keys an archive must be signed with.
    pub keys: Vec<VerifyingKey>,
    /// A development build: checks, never installs.
    pub dev: bool,
    /// Its installation, or why there is none to replace.
    pub install: Result<Install, Unplaceable>,
}

impl Build {
    /// This process. `None` for a target the updater knows no archive for.
    #[must_use]
    pub fn this() -> Option<Self> {
        let os = Os::of_target(baylee_build::TARGET)?;
        let exe = std::env::current_exe().ok()?;
        let mut build = Self {
            version: Version::parse(baylee_build::VERSION).ok()?,
            target: baylee_build::TARGET.to_owned(),
            api: GITHUB_RELEASES.to_owned(),
            keys: sign::trusted_keys(),
            dev: !is_release_build(),
            install: Install::around(&exe, os),
        };
        build.overrides();
        Some(build)
    }

    /// The dev-control harness may point the updater elsewhere, to test it
    /// against a stub: `BAYLEE_UPDATE_API` (the releases address),
    /// `BAYLEE_UPDATE_VERSION` (the version to pretend to be) and
    /// `BAYLEE_UPDATE_KEY` (one more key to trust). A shipped build never
    /// has the feature, and so reads none of these.
    #[cfg(feature = "dev-control")]
    fn overrides(&mut self) {
        if let Ok(api) = std::env::var("BAYLEE_UPDATE_API") {
            warn!("updates: asking {api} (BAYLEE_UPDATE_API)");
            self.api = api;
        }
        if let Some(version) = std::env::var("BAYLEE_UPDATE_VERSION")
            .ok()
            .and_then(|v| Version::parse(&v).ok())
        {
            warn!("updates: pretending to be {version} (BAYLEE_UPDATE_VERSION)");
            self.version = version;
        }
        if let Some(key) = std::env::var("BAYLEE_UPDATE_KEY")
            .ok()
            .and_then(|k| sign::public_key(&k).ok())
        {
            warn!("updates: also trusting a test key (BAYLEE_UPDATE_KEY)");
            self.keys.push(key);
        }
    }

    #[cfg(not(feature = "dev-control"))]
    #[allow(clippy::unused_self)] // the feature's twin takes it
    fn overrides(&mut self) {}
}

// Statics live until OS process teardown: clearing Bevy's world must not
// release an orphaned runtime's lifetime lease while other destructors run.
static CLIENT_LEASE: OnceLock<launch::ClientLease> = OnceLock::new();

/// The thread's door, and what the exit needs.
#[derive(Resource)]
struct Updater {
    service: Mutex<Option<Service>>,
    install: Option<Install>,
    version: String,
    dev: bool,
    /// "Update automatically", as the player last set it.
    allowed: AtomicBool,
}

/// The updater, on a desktop build.
pub struct NativeUpdatePlugin;

impl Plugin for NativeUpdatePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(UpdatePlugin);
        let Some(mut build) = Build::this() else {
            warn!("updates: no archive exists for {}", baylee_build::TARGET);
            return;
        };
        let lease = match launch::join() {
            Ok(Some((install, lease))) => {
                build.install = if lease.writable() {
                    Ok(install)
                } else {
                    Err(Unplaceable::ReadOnly)
                };
                Some(lease)
            }
            Ok(None) => {
                build.install = Err(Unplaceable::NotLaunched);
                None
            }
            Err(err) => {
                // A child of a killed launcher may have been superseded.
                // It must stop before touching installation state.
                baylee_client_core::say_err!("updates: cannot join the launcher session: {err}");
                std::process::exit(1);
            }
        };
        if let Some(lease) = lease {
            let _ = CLIENT_LEASE.set(lease);
        }
        let prefs = *app.world().resource::<UpdatePrefs>();
        let notice = on_start(&build);
        app.insert_resource(notice);
        let service = start(&build, prefs);
        app.insert_resource(Updater {
            service: Mutex::new(service),
            install: build.install.ok(),
            version: build.version.to_string(),
            dev: build.dev,
            allowed: AtomicBool::new(prefs.install),
        })
        .add_systems(Update, (forward, receive).chain());
    }
}

/// The start: finish or undo an interrupted update, announce a finished
/// one once, and say so if one is staged already.
#[must_use]
pub fn on_start(build: &Build) -> UpdateNotice {
    let mut notice = UpdateNotice::default();
    let Ok(install) = &build.install else {
        return notice;
    };
    match apply::recover(install) {
        Recovery::Deferred => {
            warn!("updates: legacy recovery deferred; keeping its journal and files");
            return notice;
        }
        Recovery::Nothing => {}
        Recovery::Finished(applied) => {
            info!("updates: finished the interrupted update to {}", applied.to);
        }
        Recovery::RolledBack => warn!("updates: undid an interrupted update"),
    }
    if let Ok(Some(version)) = launch::take_updated(install) {
        notice.shown = Some(Shown::Updated {
            page: Some(release_page(&version)),
            version,
        });
    }
    if let Some(applied) = apply::take_applied(install) {
        info!("updates: updated from {} to {}", applied.from, applied.to);
        notice.shown = Some(Shown::Updated {
            page: Some(release_page(&applied.to)),
            version: applied.to,
        });
    }
    if let Some(staged) = apply::staged(install)
        && Version::parse(&staged.version).is_ok_and(|v| v > build.version)
    {
        notice.shown = Some(if build.dev {
            Shown::Available {
                version: staged.version,
                page: staged.page,
                why: Why::DevBuild,
            }
        } else {
            Shown::Ready {
                version: staged.version,
                page: staged.page,
            }
        });
    }
    notice
}

/// A release's page on GitHub, for the "Updated to X" notice.
fn release_page(version: &str) -> String {
    format!("https://github.com/AceVik/baylee/releases/tag/v{version}")
}

fn start(build: &Build, prefs: UpdatePrefs) -> Option<Service> {
    let context = Context {
        current: build.version.clone(),
        target: build.target.clone(),
        install: build.install.clone(),
        keys: build.keys.clone(),
        installs: Ok(()),
    };
    match Service::start(
        build.api.clone(),
        context,
        settings(prefs),
        build.dev,
        service::EVERY,
    ) {
        Ok(service) => Some(service),
        Err(err) => {
            warn!("updates: no thread to check on: {err}");
            None
        }
    }
}

fn settings(prefs: UpdatePrefs) -> Settings {
    Settings {
        check: prefs.check,
        install: prefs.install,
    }
}

/// Hands the face's requests to the thread.
fn forward(mut requests: MessageReader<UpdateRequest>, updater: Res<Updater>) {
    let Ok(service) = updater.service.lock() else {
        return;
    };
    for request in requests.read() {
        // The exit obeys the switch even with no thread to tell.
        if let UpdateRequest::Prefs(prefs) = request {
            updater.allowed.store(prefs.install, Ordering::Relaxed);
        }
        let Some(service) = service.as_ref() else {
            continue;
        };
        service.send(match request {
            UpdateRequest::CheckNow => Command::CheckNow,
            UpdateRequest::Prefs(prefs) => Command::Settings(settings(*prefs)),
        });
    }
}

/// Reads what the thread found into the notice.
fn receive(updater: Res<Updater>, mut notice: ResMut<UpdateNotice>) {
    let Ok(service) = updater.service.lock() else {
        return;
    };
    let Some(service) = service.as_ref() else {
        return;
    };
    while let Ok(outcome) = service.outcomes.try_recv() {
        take(&mut notice, outcome);
    }
}

/// One outcome into the notice.
pub(crate) fn take(notice: &mut UpdateNotice, outcome: Outcome) {
    let asked = notice.asked == Some(Asked::Checking);
    let found = match outcome {
        Outcome::UpToDate => {
            if asked {
                notice.asked = Some(Asked::UpToDate);
            }
            return;
        }
        Outcome::Unknown(why) => {
            info!("updates: could not check: {why}");
            if asked {
                notice.asked = Some(Asked::Failed);
            }
            return;
        }
        Outcome::Staged { version, page } => {
            info!("updates: {version} is staged and installs on exit");
            Shown::Ready { version, page }
        }
        Outcome::Available { version, page, why } => {
            info!("updates: {version} is available, not installed: {why:?}");
            Shown::Available {
                version,
                page,
                why: why_of(&why),
            }
        }
    };
    if asked {
        notice.asked = Some(Asked::Found);
    }
    if notice.shown.as_ref() != Some(&found) {
        notice.shown = Some(found);
        notice.hidden = false;
    }
}

fn why_of(manual: &Manual) -> Why {
    match manual {
        Manual::Off => Why::Off,
        Manual::DevBuild => Why::DevBuild,
        Manual::NotWritable(_) | Manual::Unplaceable(Unplaceable::ReadOnly) => Why::Folder,
        Manual::Unplaceable(Unplaceable::Translocated) => Why::MoveApp,
        Manual::Unsigned | Manual::NotOurs(_) => Why::NotVerified,
        Manual::NoArchive
        | Manual::Unplaceable(_)
        | Manual::FailedBefore(_)
        | Manual::Download(_) => Why::Other,
    }
}

/// When the program ends: stops the thread, then installs a staged update
/// if this device lets it and this is not a development build.
///
/// A `Drop` and not a system reading [`AppExit`], because on macOS "Quit"
/// (⌘Q, the menu, the Dock) never sends one: macOS ends the process from
/// `applicationWillTerminate`, and all winit lets Bevy do first is clear its
/// world (`bevy_winit`'s `exiting`), which drops this resource. Closing the
/// window drops it the same way, once the event loop has ended. Seen in the
/// live check of 27.09.2026, where ⌘Q left the update staged. The window is
/// gone by then; nothing of the installation is read after this.
impl Drop for Updater {
    fn drop(&mut self) {
        if std::thread::panicking() {
            // A crash installs nothing; the next exit will.
            return;
        }
        if let Ok(mut service) = self.service.lock() {
            // Dropping the door ends the thread at its next wake-up; a
            // download it is in the middle of holds the stage's claim, and
            // `apply` then answers `Busy` and leaves it for the next exit.
            service.take();
        }
        let Some(install) = &self.install else {
            return;
        };
        let allowed = self.allowed.load(Ordering::Relaxed);
        if let Some(outcome) = exit_with(install, &self.version, allowed, self.dev) {
            info!("updates: {outcome}");
        }
    }
}

/// What the exit does, apart from Bevy: installs what is staged, if it may.
/// Returns what to log, `None` when nothing was staged.
pub(crate) fn exit_with(install: &Install, from: &str, allowed: bool, dev: bool) -> Option<String> {
    let staged = apply::staged(install)?;
    if dev {
        return Some(format!(
            "{} is staged; a development build never installs it",
            staged.version
        ));
    }
    if !allowed {
        return Some(format!(
            "{} is staged; automatic updates are off",
            staged.version
        ));
    }
    Some(match launch::activate(install, from) {
        Ok(version) => format!("installed {version}; it starts next time"),
        Err(err) => format!("did not install {}: {err}", staged.version),
    })
}

#[cfg(test)]
mod tests;

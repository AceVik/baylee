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

use super::{
    Asked, MoveTo, Shown, UpdateNotice, UpdatePlace, UpdatePlugin, UpdatePrefs, UpdateRequest, Why,
};
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_update::apply::{self, Install, Recovery, Unplaceable};
use baylee_update::check::{Context, GITHUB_RELEASES, Manual, Outcome};
use baylee_update::launch::{self, Blocked};
use baylee_update::plan::Os;
use baylee_update::relaunch;
use baylee_update::relocate::{self, Destination, Moved, SystemTrash, Trash};
use baylee_update::service::{self, Command, Service, Settings};
use baylee_update::{VerifyingKey, Version, sign};
use bevy::prelude::*;
use std::path::{Path, PathBuf};
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

/// The installation the launcher started this runtime for, and where its
/// original package is: what "Restart now" starts again. Unset for a
/// runtime started without a launcher.
static LAUNCHED: OnceLock<(Install, Option<PathBuf>)> = OnceLock::new();

/// The relaunch helper's pipe, for the same reason as the lease: it must
/// stay open until the process ends, after the update is installed on the
/// way out ([`Updater`]'s `Drop`), and a resource would close it as the
/// world is cleared, which may come first.
static RELAUNCH: OnceLock<relaunch::Handoff> = OnceLock::new();

/// When this process was started as the relaunch helper
/// (`baylee_update::relaunch::FLAG`), does that and nothing else, and says
/// so: `main` then returns before anything of the client is started — no
/// settings, no window, no launcher session.
#[must_use]
pub fn helper_if_asked() -> bool {
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let Some(again) = relaunch::asked(&args) else {
        return false;
    };
    if let Err(err) = relaunch::helper(std::io::stdin(), &again, relaunch::WAIT) {
        baylee_client_core::say_err!("relaunch: {err}");
    }
    true
}

/// "Restart now": leaves the helper behind with `handoff` on its pipe, to
/// start the client again once this one (and its launcher) has ended, with
/// [`baylee_client_core::resume::RESUME_ARG`] when `resume`. The caller then
/// quits; the staged update installs on the way out, as on any quit.
///
/// # Errors
/// A restart already under way, or the helper not starting.
pub(crate) fn restart(handoff: &[u8], resume: bool) -> std::io::Result<()> {
    if RELAUNCH.get().is_some() {
        return Err(std::io::Error::other("a restart is already under way"));
    }
    let args: Vec<std::ffi::OsString> = if resume {
        vec![baylee_client_core::resume::RESUME_ARG.into()]
    } else {
        Vec::new()
    };
    let exe = std::env::current_exe()?;
    let again = match LAUNCHED.get() {
        Some((install, original)) => relaunch::Again::launcher(install, original.as_deref(), args),
        None => relaunch::Again::direct(&exe, args),
    };
    let pipe = relaunch::leave_behind(std::process::Command::new(&exe), &again, handoff)?;
    info!("updates: restarting through {}", again.program.display());
    let _ = RELAUNCH.set(pipe);
    Ok(())
}

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
                } else if lease.blocked() == Some(&Blocked::Translocated) {
                    Err(Unplaceable::Translocated)
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
        let exe = std::env::current_exe().ok();
        let running = exe.as_deref().and_then(relocate::bundle_of);
        let seen = lease.as_ref().map(Seen::of);
        let mover = Mover {
            trash: Box::new(SystemTrash),
            source: running.clone(),
            original: lease
                .as_ref()
                .and_then(|l| l.original().map(Path::to_path_buf)),
            home: std::env::var_os("HOME").map(PathBuf::from),
        };
        let mut place = place_of(
            seen.as_ref(),
            running.is_some() && cfg!(target_os = "macos"),
            || relocate::can_write(&Destination::System.folder(Path::new("/"))),
        );
        place.moved = moved_here(mover.here().as_deref());
        app.insert_resource(place)
            .insert_resource(mover)
            .add_systems(Update, relocate_on_request);
        if let Some(lease) = lease {
            if let Ok(install) = &build.install {
                let original = lease.original().map(Path::to_path_buf);
                let _ = LAUNCHED.set((install.clone(), original));
            }
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
            // `relocate` answers these; the thread has no part in them.
            UpdateRequest::Move(_)
            | UpdateRequest::TrashOld
            | UpdateRequest::KeepOld
            | UpdateRequest::RestartNow => continue,
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

/// What the launcher said about the place, as `place_of` needs it.
pub(crate) struct Seen {
    pub writable: bool,
    pub blocked: Option<Blocked>,
    pub translocated: bool,
}

impl Seen {
    fn of(lease: &launch::ClientLease) -> Self {
        Self {
            writable: lease.writable(),
            blocked: lease.blocked().cloned(),
            translocated: lease.translocated(),
        }
    }
}

/// What the face says about where the app lies, and the moves it offers:
/// on macOS, from a bundle, when installing is off where it lies or macOS
/// translocates it. `/Applications` is offered too where this user may
/// write it (`system_writable`, asked only then). A runtime the launcher
/// did not start (`None`) has nothing to say.
pub(crate) fn place_of(
    seen: Option<&Seen>,
    movable: bool,
    system_writable: impl FnOnce() -> bool,
) -> UpdatePlace {
    let mut place = UpdatePlace::default();
    let Some(seen) = seen else {
        return place;
    };
    if let Some(Blocked::ReadOnly { folder, error }) = &seen.blocked {
        place.read_only = Some((folder.display().to_string(), error.clone()));
    }
    place.translocated = seen.translocated || seen.blocked == Some(Blocked::Translocated);
    if movable && (!seen.writable || place.translocated) {
        place.moves.push(MoveTo::Home);
        if system_writable() {
            place.moves.push(MoveTo::System);
        }
    }
    place
}

/// Where the move's marker is kept, beside the settings: the copy reads it
/// at its first start to offer the old one to the Trash.
const MOVED_FILE: &str = "moved.json";

/// What the move needs to know about this process.
#[derive(Resource)]
pub(crate) struct Mover {
    /// Where the old copy goes: the system's Trash, a fake in tests.
    pub trash: Box<dyn Trash + Send + Sync>,
    /// The bundle this runtime runs from: the original package (perhaps a
    /// translocation mount of it) or the update generation the launcher
    /// selected, which is a complete release bundle of the newest version.
    pub source: Option<PathBuf>,
    /// The original package, where it lies (never a mount), if known.
    pub original: Option<PathBuf>,
    /// The player's home.
    pub home: Option<PathBuf>,
}

impl Mover {
    /// The package this process belongs to, to compare with a marker.
    fn here(&self) -> Option<PathBuf> {
        self.original.clone().or_else(|| self.source.clone())
    }

    /// Copies the running bundle to `to`, under the original's name.
    fn copy(&self, to: MoveTo) -> Result<PathBuf, String> {
        let source = self
            .source
            .as_ref()
            .ok_or("Baylee is not running from an app bundle")?;
        let home = self.home.as_ref().ok_or("no home folder")?;
        let name = self
            .original
            .as_deref()
            .and_then(Path::file_name)
            .map_or_else(
                || "Baylee.app".to_owned(),
                |n| n.to_string_lossy().into_owned(),
            );
        let folder = destination(to).folder(home);
        relocate::copy_bundle(source, &folder, &name).map_err(|err| err.to_string())
    }
}

fn destination(to: MoveTo) -> Destination {
    match to {
        MoveTo::Home => Destination::Home,
        MoveTo::System => Destination::System,
    }
}

/// The old copy to offer to the Trash, if the marker names this package as
/// the copy and the old one is still there; a stale marker is removed.
fn moved_here(here: Option<&Path>) -> Option<(String, String)> {
    let text = crate::settings::store::read_named(MOVED_FILE)?;
    let moved: Moved = serde_json::from_str(&text).ok()?;
    let same = |a: &Path, b: &Path| {
        std::fs::canonicalize(a)
            .ok()
            .zip(std::fs::canonicalize(b).ok())
            .is_some_and(|(a, b)| a == b)
    };
    if !here.is_some_and(|here| same(here, &moved.to)) {
        // The old copy is running again, or something else: ask the copy.
        return None;
    }
    if std::fs::symlink_metadata(&moved.from).is_err() {
        crate::settings::store::remove_named(MOVED_FILE);
        return None;
    }
    Some((
        moved.to.display().to_string(),
        moved.from.display().to_string(),
    ))
}

/// Answers the move's buttons: copy, start the copy and quit; or put the
/// old copy in the Trash; or keep it. Synchronous: in the live check of
/// 06.10.2026 the whole move of the 142 MB bundle out of a translocation
/// mount (a real copy, not a clone), signature check included, took 0.28 s.
fn relocate_on_request(
    mut requests: MessageReader<UpdateRequest>,
    mut place: ResMut<UpdatePlace>,
    mover: Res<Mover>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    mut exit: MessageWriter<AppExit>,
) {
    let lang = settings.map_or(Lang::En, |s| Lang::of(&s.lang));
    for request in requests.read() {
        match request {
            UpdateRequest::Move(to) => {
                let copied = mover.copy(*to);
                let started = copied.and_then(|copy| {
                    if let Some(from) = &mover.original {
                        let moved = Moved {
                            from: from.clone(),
                            to: copy.clone(),
                        };
                        if let Ok(text) = serde_json::to_string_pretty(&moved) {
                            crate::settings::store::write_named(MOVED_FILE, &text);
                        }
                    }
                    relocate::relaunch(&copy).map_err(|err| {
                        format!("{} was made, but did not start ({err})", copy.display())
                    })
                });
                match started {
                    Ok(()) => {
                        info!("updates: moved; the copy is starting, this one quits");
                        crate::lobby::departure::restarting();
                        exit.write(AppExit::Success);
                    }
                    Err(why) => {
                        warn!("updates: move failed: {why}");
                        place.failed = Some(Phrase::UpdateMoveFailed.fill(lang, &[&why]));
                    }
                }
            }
            UpdateRequest::TrashOld => {
                let Some((_, old)) = place.moved.clone() else {
                    continue;
                };
                match mover.trash.trash(Path::new(&old)) {
                    Ok(at) => {
                        info!("updates: the old copy is in the Trash: {}", at.display());
                        crate::settings::store::remove_named(MOVED_FILE);
                        place.moved = None;
                        place.failed = None;
                    }
                    Err(err) => {
                        place.failed =
                            Some(Phrase::UpdateTrashFailed.fill(lang, &[&err.to_string()]));
                    }
                }
            }
            UpdateRequest::KeepOld => {
                crate::settings::store::remove_named(MOVED_FILE);
                place.moved = None;
                place.failed = None;
            }
            UpdateRequest::CheckNow | UpdateRequest::Prefs(_) | UpdateRequest::RestartNow => {}
        }
    }
}

#[cfg(test)]
mod tests;

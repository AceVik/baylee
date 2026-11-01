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
//! All of that starts as early as the client can ([`begin`]): before its
//! window, renderer and lobby are built, so the first check has left long
//! before the lobby draws its first frame. The thread reports as it goes
//! ([`Report`]), and [`take`] turns each report into the card.
//!
//! A build that did not come out of the release workflow is a development
//! build: it checks and shows what it found, and never replaces itself,
//! because the folder it runs from is `target/release`, not an installation
//! (the notice says so). The release workflow marks its builds with
//! `BAYLEE_RELEASE_BUILD=1`.

#[cfg(feature = "dev-control")]
mod dev;
#[cfg(feature = "dev-control")]
pub(crate) use dev::feed as dev_feed;

use super::{
    Asked, Fetch, MoveTo, Progress, Shown, UpdateNotice, UpdatePlace, UpdatePlugin, UpdatePrefs,
    UpdateRequest, Why,
};
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_update::apply::{self, Install, Recovery, Unplaceable};
use baylee_update::check::{Context, GITHUB_RELEASES, Manual, Outcome};
use baylee_update::launch::{self, Blocked};
use baylee_update::plan::Os;
use baylee_update::progress::Stage;
use baylee_update::relaunch;
use baylee_update::relocate::{self, Destination, Moved, SystemTrash, Trash};
use baylee_update::service::{self, Command, Report, Service, Settings};
use baylee_update::{VerifyingKey, Version, sign};
use bevy::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

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
    pub fn this(notes: &mut Notes) -> Option<Self> {
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
        build.overrides(notes);
        Some(build)
    }

    /// The dev-control harness may point the updater elsewhere, to test it
    /// against a stub: `BAYLEE_UPDATE_API` (the releases address),
    /// `BAYLEE_UPDATE_VERSION` (the version to pretend to be) and
    /// `BAYLEE_UPDATE_KEY` (one more key to trust). A shipped build never
    /// has the feature, and so reads none of these.
    #[cfg(feature = "dev-control")]
    fn overrides(&mut self, notes: &mut Notes) {
        if let Ok(api) = std::env::var("BAYLEE_UPDATE_API") {
            notes.warn(format!("updates: asking {api} (BAYLEE_UPDATE_API)"));
            self.api = api;
        }
        if let Some(version) = std::env::var("BAYLEE_UPDATE_VERSION")
            .ok()
            .and_then(|v| Version::parse(&v).ok())
        {
            notes.warn(format!(
                "updates: pretending to be {version} (BAYLEE_UPDATE_VERSION)"
            ));
            self.version = version;
        }
        if let Some(key) = std::env::var("BAYLEE_UPDATE_KEY")
            .ok()
            .and_then(|k| sign::public_key(&k).ok())
        {
            notes.warn("updates: also trusting a test key (BAYLEE_UPDATE_KEY)".into());
            self.keys.push(key);
        }
    }

    #[cfg(not(feature = "dev-control"))]
    #[allow(clippy::unused_self)] // the feature's twin takes it
    fn overrides(&mut self, _notes: &mut Notes) {}
}

// Statics live until OS process teardown: clearing Bevy's world must not
// release an orphaned runtime's lifetime lease while other destructors run.
static CLIENT_LEASE: OnceLock<launch::ClientLease> = OnceLock::new();

/// The installation the launcher started this runtime for, and where its
/// original package is: what "Restart now" starts again. Unset for a
/// runtime started without a launcher.
static LAUNCHED: OnceLock<(Install, Option<PathBuf>)> = OnceLock::new();

/// The state directory of the installation whose launcher started this
/// runtime, writable or not: its link door is named after it
/// (`crate::links`). Unset for a runtime started without a launcher.
static STAGE: OnceLock<PathBuf> = OnceLock::new();

/// The installation's state directory, when a launcher started this runtime.
pub(crate) fn launched_stage() -> Option<&'static Path> {
    STAGE.get().map(PathBuf::as_path)
}

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

/// What the updater says before Bevy's logger exists: [`begin`] runs before
/// `LogPlugin` is built, which drops every `info!` and `warn!` until then.
/// Kept, and said once the plugin is built.
#[derive(Default)]
pub struct Notes(Vec<(bool, String)>);

impl Notes {
    fn info(&mut self, line: String) {
        self.0.push((false, line));
    }

    fn warn(&mut self, line: String) {
        self.0.push((true, line));
    }

    /// Says what was kept.
    fn say(self) {
        for (warned, line) in self.0 {
            if warned {
                warn!("{line}");
            } else {
                info!("{line}");
            }
        }
    }
}

/// When this process started, as near as the client can tell: the first
/// thing `main` does is [`mark_start`]. What the first check's time is
/// measured from.
static STARTED: OnceLock<Instant> = OnceLock::new();

/// Marks the process's start. `main`'s first line; later calls change
/// nothing.
pub fn mark_start() {
    let _ = STARTED.set(Instant::now());
}

/// How long ago, in milliseconds, `at` was after the process's start.
fn after_start(at: Instant) -> Option<u128> {
    STARTED
        .get()
        .map(|started| at.saturating_duration_since(*started).as_millis())
}

/// Everything the updater sets up before the app exists, ready for the
/// plugin to hand to the app.
struct Running {
    place: UpdatePlace,
    mover: Mover,
    notice: UpdateNotice,
    updater: Updater,
}

/// The updater as [`begin`] left it.
struct Begun {
    notes: Notes,
    running: Option<Running>,
}

/// The updater, on a desktop build: what [`begin`] started, given to the
/// app. Added without [`begin`] (an app that embeds the client), it begins
/// when it is built.
#[derive(Default)]
pub struct NativeUpdatePlugin {
    begun: Mutex<Option<Begun>>,
}

/// Starts the updater as early as the client can: before the window, the
/// renderer and the lobby are built, right after the settings store is
/// open (the switches are read from it). Joins the launcher's session,
/// finishes or undoes an interrupted update, and starts the thread, whose
/// first check (unless checks are off) leaves at once.
#[must_use]
pub fn begin() -> NativeUpdatePlugin {
    NativeUpdatePlugin {
        begun: Mutex::new(Some(begun())),
    }
}

fn begun() -> Begun {
    let mut notes = Notes::default();
    let Some(mut build) = Build::this(&mut notes) else {
        notes.warn(format!(
            "updates: no archive exists for {}",
            baylee_build::TARGET
        ));
        return Begun {
            notes,
            running: None,
        };
    };
    let lease = match launch::join() {
        Ok(Some((install, lease))) => {
            let _ = STAGE.set(install.stage());
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
    if let Some(lease) = lease {
        if let Ok(install) = &build.install {
            let original = lease.original().map(Path::to_path_buf);
            let _ = LAUNCHED.set((install.clone(), original));
        }
        let _ = CLIENT_LEASE.set(lease);
    }
    let prefs = UpdatePrefs::load();
    // The interrupted update first: a check staging into a directory whose
    // recovery has not finished is refused.
    let notice = on_start(&build, &mut notes);
    let service = start(&build, prefs, &mut notes);
    if let Some(ms) = after_start(Instant::now()) {
        notes.info(format!("updates: begun {ms} ms after the process started"));
    }
    let updater = Updater {
        service: Mutex::new(service),
        install: build.install.ok(),
        version: build.version.to_string(),
        dev: build.dev,
        allowed: AtomicBool::new(prefs.install),
    };
    Begun {
        notes,
        running: Some(Running {
            place,
            mover,
            notice,
            updater,
        }),
    }
}

impl Plugin for NativeUpdatePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(UpdatePlugin);
        let begun = self
            .begun
            .lock()
            .ok()
            .and_then(|mut begun| begun.take())
            .unwrap_or_else(begun);
        begun.notes.say();
        if let Some(ms) = after_start(Instant::now()) {
            info!("updates: built into the app {ms} ms after the process started");
        }
        let Some(running) = begun.running else {
            return;
        };
        app.insert_resource(running.place)
            .insert_resource(running.mover)
            .insert_resource(running.notice)
            .insert_resource(running.updater)
            .add_systems(Update, relocate_on_request)
            .add_systems(Update, (forward, receive).chain());
        #[cfg(feature = "dev-control")]
        app.add_systems(Update, dev::apply_direct);
    }
}

/// The start: finish or undo an interrupted update, announce a finished
/// one once, and say so if one is staged already.
#[must_use]
pub fn on_start(build: &Build, notes: &mut Notes) -> UpdateNotice {
    let mut notice = UpdateNotice::default();
    let Ok(install) = &build.install else {
        return notice;
    };
    match apply::recover(install) {
        Recovery::Deferred => {
            notes.warn("updates: legacy recovery deferred; keeping its journal and files".into());
            return notice;
        }
        Recovery::Nothing => {}
        Recovery::Finished(applied) => {
            notes.info(format!(
                "updates: finished the interrupted update to {}",
                applied.to
            ));
        }
        Recovery::RolledBack => notes.warn("updates: undid an interrupted update".into()),
    }
    if let Ok(Some(version)) = launch::take_updated(install) {
        notice.shown = Some(Shown::Updated {
            page: Some(release_page(&version)),
            version,
        });
    }
    if let Some(applied) = apply::take_applied(install) {
        notes.info(format!(
            "updates: updated from {} to {}",
            applied.from, applied.to
        ));
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

fn start(build: &Build, prefs: UpdatePrefs, notes: &mut Notes) -> Option<Service> {
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
            notes.warn(format!("updates: no thread to check on: {err}"));
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

/// Reads what the thread says into the notice, and in a `dev-control` build
/// what the harness feeds it (`dev`), by the same path. The first check's
/// start is logged against the process's: how soon the client asked.
fn receive(updater: Res<Updater>, mut notice: ResMut<UpdateNotice>, mut timed: Local<bool>) {
    let mut reports: Vec<Report> = Vec::new();
    if let Ok(service) = updater.service.lock()
        && let Some(service) = service.as_ref()
    {
        reports.extend(service.reports.try_iter());
    }
    #[cfg(feature = "dev-control")]
    reports.extend(dev::fed());
    for report in reports {
        if let Report::Checking { at } = &report
            && !*timed
        {
            *timed = true;
            if let Some(ms) = after_start(*at) {
                info!("updates: first check asked {ms} ms after the process started");
            }
        }
        take(&mut notice, report);
    }
}

/// One report into the notice: a download's stages into
/// [`Shown::Fetching`] and its readings into [`UpdateNotice::progress`],
/// the check's end through [`finish`].
pub(crate) fn take(notice: &mut UpdateNotice, report: Report) {
    match report {
        Report::Checking { .. } => {}
        Report::Stage(Stage::Found {
            version,
            page,
            size,
        }) => {
            info!("updates: downloading {version} ({size} bytes)");
            notice.show(Shown::Fetching {
                version,
                page,
                stage: Fetch::Downloading,
            });
            notice.progress = Some(Progress {
                received: 0,
                total: size,
                rate: None,
                left: None,
            });
        }
        Report::Stage(Stage::Downloading(reading)) => {
            let reading = Progress {
                received: reading.received,
                total: reading.total,
                rate: reading.rate,
                left: reading.left,
            };
            if notice.progress != Some(reading) {
                notice.progress = Some(reading);
            }
        }
        Report::Stage(Stage::Verifying) => onto(notice, Fetch::Verifying),
        Report::Stage(Stage::Unpacking) => onto(notice, Fetch::Unpacking),
        Report::Done(outcome) => finish(notice, outcome),
    }
}

/// A download moves on to its next stage.
fn onto(notice: &mut UpdateNotice, stage: Fetch) {
    if let Some(Shown::Fetching { version, page, .. }) = notice.shown.clone() {
        notice.show(Shown::Fetching {
            version,
            page,
            stage,
        });
    }
}

/// A check's end into the notice: what it found, and how the check the
/// player asked for went. A failed download is said as one, and the next
/// check tries it again.
pub(crate) fn finish(notice: &mut UpdateNotice, outcome: Outcome) {
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
        Outcome::Available {
            version,
            page,
            why: Manual::Download(reason),
        } => {
            warn!("updates: {version} did not download: {reason}");
            Shown::Failed {
                version,
                page,
                reason,
            }
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
    notice.show(found);
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

//! The updater's face (#326): the notice, the table menu's line, and the
//! two per-device switches on the settings screen.
//!
//! What it shows is [`UpdateNotice`], and only [`native`] fills it: a
//! desktop build checks GitHub on a thread of its own
//! (`baylee_update::service`) and installs a staged update after the window
//! has closed. A browser or a phone build compiles no updater at all, never
//! gets the resource, and so draws none of this (`docs/client.md`
//! §"Updating").
//!
//! The notice is quiet on purpose: one small card in the lobby's corner,
//! which the player can hide for the session, and one line in the table's
//! menu. The card names the version, fills a slim bar while the update
//! downloads (so many of so many MB, the rate, the time left), says when it
//! verifies and unpacks it, and offers the restart once it is ready
//! ([`card`]). Nothing interrupts a game, and nothing is installed while one
//! is open: the swap happens when the program has ended.

// A browser or phone build adds no updater, and so none of the systems here.
#![cfg_attr(
    any(target_arch = "wasm32", target_os = "android", target_os = "ios"),
    allow(dead_code)
)]

mod card;
#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
pub mod native;
mod settings;
mod words;

pub(crate) use card::menu_line;
pub(crate) use settings::controls;

use baylee_client_core::i18n::{Lang, Phrase};
use bevy::picking::events::{PointerClick, PointerScroll};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// What this device lets the updater do. Kept in `update.json` beside the
/// settings, per device like the music, because it has to hold before
/// anybody signs in. Both on unless the player said otherwise.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdatePrefs {
    /// Ask GitHub at start and every six hours. Off: no request at all.
    pub check: bool,
    /// Download, verify and install a newer release by itself.
    pub install: bool,
}

impl Default for UpdatePrefs {
    fn default() -> Self {
        Self {
            check: true,
            install: true,
        }
    }
}

const PREFS_FILE: &str = "update.json";

impl UpdatePrefs {
    /// This device's choice, or both on. Reads nothing in a process that
    /// never opened the settings store (every test).
    #[must_use]
    pub fn load() -> Self {
        crate::settings::store::read_named(PREFS_FILE)
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Writes it back. Best effort, as every setting is.
    pub fn save(&self) {
        if let Ok(text) = serde_json::to_string_pretty(self) {
            crate::settings::store::write_named(PREFS_FILE, &text);
        }
    }
}

/// Why an update is a link and not an install, as the notice words it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// The player switched installing off.
    Off,
    /// A development build.
    DevBuild,
    /// The folder cannot be written.
    Folder,
    /// macOS runs the app from a read-only copy.
    MoveApp,
    /// The signature or checksum did not verify.
    NotVerified,
    /// Anything else.
    Other,
}

impl Why {
    fn phrase(self) -> Phrase {
        match self {
            Self::Off => Phrase::UpdateWhyOff,
            Self::DevBuild => Phrase::UpdateWhyDev,
            Self::Folder => Phrase::UpdateWhyFolder,
            Self::MoveApp => Phrase::UpdateWhyMoveApp,
            Self::NotVerified => Phrase::UpdateWhyNotVerified,
            Self::Other => Phrase::UpdateWhyOther,
        }
    }
}

/// Where a move can copy the app to (macOS).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveTo {
    /// `~/Applications`: always this user's, no password asked.
    Home,
    /// `/Applications`, offered only where this user may write it.
    System,
}

impl MoveTo {
    fn phrase(self) -> Phrase {
        match self {
            Self::Home => Phrase::UpdateMoveHome,
            Self::System => Phrase::UpdateMoveSystem,
        }
    }
}

/// Where the app runs, as far as updating cares, and what moving it can
/// do. Only `native` fills it, and only on macOS offers a move; elsewhere
/// it stays empty and draws nothing.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct UpdatePlace {
    /// Installing is off because this user may not write the installation's
    /// folder: the folder, and what the system answered.
    pub read_only: Option<(String, String)>,
    /// macOS runs the app from a read-only copy (App Translocation).
    pub translocated: bool,
    /// Where a move can go, in the order offered; empty, no move.
    pub moves: Vec<MoveTo>,
    /// A move or a trashing that failed, worded for the player.
    pub failed: Option<String>,
    /// The first start after a move: where it runs now, and the old copy,
    /// offered to the Trash until the player decides.
    pub moved: Option<(String, String)>,
}

impl UpdatePlace {
    /// Whether the settings screen has anything to say about the place.
    fn worth_saying(&self) -> bool {
        self.read_only.is_some()
            || self.translocated
            || !self.moves.is_empty()
            || self.failed.is_some()
    }
}

/// What the notice says, if anything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shown {
    /// Newer, and being downloaded, verified or unpacked: it installs
    /// itself once it is ready. How far the download is lives beside it, in
    /// [`UpdateNotice::progress`].
    Fetching {
        /// Its version.
        version: String,
        /// Its release page.
        page: String,
        /// Which part of the work.
        stage: Fetch,
    },
    /// Staged and verified; it installs on quit.
    Ready {
        /// Its version.
        version: String,
        /// Its release page.
        page: String,
    },
    /// The download (or the unpacking) failed; the next check tries again.
    Failed {
        /// Its version.
        version: String,
        /// Its release page.
        page: String,
        /// What went wrong, as the system said it.
        reason: String,
    },
    /// Newer, and only a link.
    Available {
        /// Its version.
        version: String,
        /// Its release page.
        page: String,
        /// Why.
        why: Why,
    },
    /// This start is the first after an update.
    Updated {
        /// The version now running.
        version: String,
        /// Its release page, when known.
        page: Option<String>,
    },
}

/// Which part of fetching an update is under way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fetch {
    /// The archive is arriving.
    Downloading,
    /// Its checksum and signature are being checked.
    Verifying,
    /// It is being unpacked beside the installation.
    Unpacking,
}

/// How far a download is: the face's copy of the updater's reading (a
/// browser or phone build has no updater crate to take it from). Whole
/// numbers, so a reading that changes nothing drawn compares equal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    /// Bytes received.
    pub received: u64,
    /// Bytes in all.
    pub total: u64,
    /// Bytes a second, smoothed; `None` until the updater can say.
    pub rate: Option<u64>,
    /// Seconds left at that rate.
    pub left: Option<u64>,
}

impl Shown {
    /// The notice in one line: the table menu's, the settings' status.
    #[must_use]
    pub fn headline(&self, lang: Lang) -> String {
        match self {
            Self::Fetching { version, stage, .. } => match stage {
                Fetch::Downloading => Phrase::UpdateDownloadingLine,
                Fetch::Verifying => Phrase::UpdateVerifyingLine,
                Fetch::Unpacking => Phrase::UpdateUnpackingLine,
            }
            .fill(lang, &[version]),
            Self::Ready { version, .. } => Phrase::UpdateReady.fill(lang, &[version]),
            Self::Failed { version, .. } => Phrase::UpdateFailedLine.fill(lang, &[version]),
            Self::Available { version, .. } => Phrase::UpdateAvailable.fill(lang, &[version]),
            Self::Updated { version, .. } => Phrase::UpdatedTo.fill(lang, &[version]),
        }
    }

    /// The line under it, when there is a reason to give.
    #[must_use]
    pub fn reason(&self, lang: Lang) -> Option<&'static str> {
        match self {
            Self::Available { why, .. } => Some(why.phrase().text(lang)),
            _ => None,
        }
    }

    /// The release page, where the notice links to.
    #[must_use]
    pub fn page(&self) -> Option<&str> {
        match self {
            Self::Fetching { page, .. }
            | Self::Ready { page, .. }
            | Self::Failed { page, .. }
            | Self::Available { page, .. } => Some(page),
            Self::Updated { page, .. } => page.as_deref(),
        }
    }

    /// The version it is about.
    #[must_use]
    pub fn version(&self) -> &str {
        match self {
            Self::Fetching { version, .. }
            | Self::Ready { version, .. }
            | Self::Failed { version, .. }
            | Self::Available { version, .. }
            | Self::Updated { version, .. } => version,
        }
    }
}

/// The result of a check the player asked for, for the settings screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asked {
    /// Still running.
    Checking,
    /// Nothing newer.
    UpToDate,
    /// It found an update (the notice says which).
    Found,
    /// It could not ask.
    Failed,
}

/// Everything the updater's face shows. Inserted only on a desktop build.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct UpdateNotice {
    /// The notice, if there is one.
    pub shown: Option<Shown>,
    /// Hidden by the player for this session.
    pub hidden: bool,
    /// The last check the player asked for.
    pub asked: Option<Asked>,
    /// The download's last reading while [`Shown::Fetching`] stands: kept
    /// apart from [`Self::shown`], so a reading redraws the bar and the line
    /// under it and never the card or the table's menu.
    pub progress: Option<Progress>,
}

impl UpdateNotice {
    /// Shows `next`. The card comes back after the player hid it only for
    /// news: another version, or one that has become ready to install. A
    /// download moving on to its next stage, or the six-hourly check saying
    /// the same thing again, is not news.
    pub fn show(&mut self, next: Shown) {
        let news = self.shown.as_ref().is_none_or(|now| {
            now.version() != next.version()
                || (matches!(next, Shown::Ready { .. }) && !matches!(now, Shown::Ready { .. }))
        });
        if !matches!(next, Shown::Fetching { .. }) {
            self.progress = None;
        }
        if self.shown.as_ref() != Some(&next) {
            self.shown = Some(next);
        }
        if news {
            self.hidden = false;
        }
    }
}

/// What the face asks of the updater's thread. `native` forwards it.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateRequest {
    /// "Check for updates" was pressed.
    CheckNow,
    /// A switch changed.
    Prefs(UpdatePrefs),
    /// "Move to … Applications" was pressed.
    Move(MoveTo),
    /// The old copy after a move goes to the Trash.
    TrashOld,
    /// The old copy after a move stays; stop asking.
    KeepOld,
    /// "Restart now": the ready update is installed as this client quits,
    /// and the client starts again where the player is
    /// (`docs/client.md` §"Restarting into an update").
    RestartNow,
}

/// A button of the updater's face, and what it does.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub enum UpdateButton {
    /// Opens a release page.
    Open(String),
    /// Hides the notice for this session.
    Hide,
    /// Flips automatic checking.
    ToggleCheck,
    /// Flips automatic installing.
    ToggleInstall,
    /// Checks now.
    CheckNow,
    /// Moves the app ([`UpdateRequest::Move`]).
    Move(MoveTo),
    /// [`UpdateRequest::TrashOld`].
    TrashOld,
    /// [`UpdateRequest::KeepOld`].
    KeepOld,
    /// [`UpdateRequest::RestartNow`].
    RestartNow,
    /// Not now: the restart offer goes away for the session.
    Later,
    /// A failed download's "Try again": checks now, as the settings'
    /// button does.
    TryAgain,
}

/// The face: the notice, its buttons, the settings' labels. `native`
/// installs this and the thread behind it; a test installs it alone and
/// sets [`UpdateNotice`] by hand.
pub struct UpdatePlugin;

impl Plugin for UpdatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UpdateNotice>()
            .init_resource::<UpdatePlace>()
            .insert_resource(UpdatePrefs::load())
            .add_message::<UpdateRequest>()
            .add_message::<PointerScroll>()
            .add_message::<crate::shellkit::focus::Activated>()
            .add_systems(
                Update,
                (
                    press,
                    card::scroll_toasts,
                    card::show_toast,
                    card::show_progress,
                    card::follow_the_menu_line,
                    settings::show_settings,
                    settings::show_place,
                )
                    .chain(),
            )
            .add_systems(
                PostUpdate,
                (card::fit_toasts, card::fit_toast_bodies)
                    .chain()
                    .after(bevy::ui::UiSystems::Layout),
            );
    }
}

/// Answers a click on any of the updater's buttons.
fn press(
    mut clicks: MessageReader<PointerClick>,
    mut keyed: MessageReader<crate::shellkit::focus::Activated>,
    buttons: Query<&UpdateButton>,
    mut notice: ResMut<UpdateNotice>,
    mut prefs: ResMut<UpdatePrefs>,
    mut requests: MessageWriter<UpdateRequest>,
) {
    // A click, or Enter / Space on a focused button (the settings screen's
    // Updates section walks with the keys, WP5).
    let pressed: Vec<Entity> = clicks
        .read()
        .map(|click| click.entity)
        .chain(keyed.read().filter(|hit| hit.by_key).map(|hit| hit.entity))
        .collect();
    for entity in pressed {
        let Ok(button) = buttons.get(entity) else {
            continue;
        };
        match button {
            UpdateButton::Open(page) => open(page),
            UpdateButton::Hide | UpdateButton::Later => notice.hidden = true,
            UpdateButton::RestartNow => {
                requests.write(UpdateRequest::RestartNow);
            }
            UpdateButton::ToggleCheck | UpdateButton::ToggleInstall => {
                if *button == UpdateButton::ToggleCheck {
                    prefs.check = !prefs.check;
                } else {
                    prefs.install = !prefs.install;
                }
                prefs.save();
                requests.write(UpdateRequest::Prefs(*prefs));
            }
            UpdateButton::CheckNow | UpdateButton::TryAgain => {
                notice.asked = Some(Asked::Checking);
                requests.write(UpdateRequest::CheckNow);
            }
            UpdateButton::Move(to) => {
                requests.write(UpdateRequest::Move(*to));
            }
            UpdateButton::TrashOld => {
                requests.write(UpdateRequest::TrashOld);
            }
            UpdateButton::KeepOld => {
                requests.write(UpdateRequest::KeepOld);
            }
        }
    }
}

/// Opens a release page in the player's browser: the page comes from the
/// releases' answer, so only an `https` address is opened, or a loopback
/// `http` one (a stub a development build was pointed at).
fn open(page: &str) {
    if !openable(page) {
        warn!("updates: not opening {page}");
        return;
    }
    #[cfg(test)]
    tests::opened().push(page.to_owned());
    #[cfg(not(test))]
    if let Err(err) = webbrowser::open(page) {
        warn!("could not open {page}: {err}");
    }
}

/// Whether [`open`] opens `page`.
fn openable(page: &str) -> bool {
    page.starts_with("https://")
        || [
            "http://127.0.0.1:",
            "http://127.0.0.1/",
            "http://localhost:",
            "http://localhost/",
        ]
        .iter()
        .any(|prefix| page.starts_with(prefix))
}

#[cfg(test)]
mod tests;

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
//! The notice is quiet on purpose: one small panel in the lobby's corner,
//! which the player can hide for the session, and one line in the table's
//! menu. Nothing interrupts a game, and nothing is installed while one is
//! open: the swap happens when the program has ended.

// A browser or phone build adds no updater, and so none of the systems here.
#![cfg_attr(
    any(target_arch = "wasm32", target_os = "android", target_os = "ios"),
    allow(dead_code)
)]

#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
pub mod native;

use crate::hud::{UiFonts, palette, tf};
use crate::lobby::Metrics;
use crate::settings::ClientSettings;
use crate::{DuelPhase, lobby};
use baylee_client_core::i18n::{Lang, Phrase};
use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;
use bevy::ui::{percent, px};
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
    /// Staged and verified; it installs on quit.
    Ready {
        /// Its version.
        version: String,
        /// Its release page.
        page: String,
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

impl Shown {
    /// The notice's first line.
    #[must_use]
    pub fn headline(&self, lang: Lang) -> String {
        match self {
            Self::Ready { version, .. } => Phrase::UpdateReady.fill(lang, &[version]),
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
            Self::Ready { page, .. } | Self::Available { page, .. } => Some(page),
            Self::Updated { page, .. } => page.as_deref(),
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
}

/// The settings screen's place for [`UpdatePlace`], filled by `show_place`.
#[derive(Component)]
struct PlaceRow;

/// The notice's panel in the lobby's corner.
#[derive(Component)]
pub struct UpdateToast;

/// A text on the settings screen that follows the updater's state.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Readout {
    Check,
    Install,
    Status,
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
            .add_systems(
                Update,
                (press, show_toast, show_settings, show_place).chain(),
            );
    }
}

/// Answers a click on any of the updater's buttons.
fn press(
    mut clicks: MessageReader<Pointer<Click>>,
    buttons: Query<&UpdateButton>,
    mut notice: ResMut<UpdateNotice>,
    mut prefs: ResMut<UpdatePrefs>,
    mut requests: MessageWriter<UpdateRequest>,
) {
    for click in clicks.read() {
        let Ok(button) = buttons.get(click.entity) else {
            continue;
        };
        match button {
            UpdateButton::Open(page) => open(page),
            UpdateButton::Hide => notice.hidden = true,
            UpdateButton::ToggleCheck | UpdateButton::ToggleInstall => {
                if *button == UpdateButton::ToggleCheck {
                    prefs.check = !prefs.check;
                } else {
                    prefs.install = !prefs.install;
                }
                prefs.save();
                requests.write(UpdateRequest::Prefs(*prefs));
            }
            UpdateButton::CheckNow => {
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

/// Keeps the corner panel in step with the notice: shown in the lobby while
/// there is something to say and it was not hidden, gone at a table (the
/// menu says it there) and once hidden.
#[allow(clippy::too_many_arguments)] // one panel, drawn from everything it depends on
fn show_toast(
    mut commands: Commands,
    notice: Res<UpdateNotice>,
    place: Res<UpdatePlace>,
    phase: Option<Res<State<DuelPhase>>>,
    fonts: Option<Res<UiFonts>>,
    settings: Option<Res<ClientSettings>>,
    windows: Query<&Window>,
    toasts: Query<Entity, With<UpdateToast>>,
    mut drawn: Local<Option<(Option<Shown>, UpdatePlace, Lang)>>,
) {
    let lang = settings.map_or(Lang::En, |s| Lang::of(&s.lang));
    let in_lobby = phase.is_none_or(|p| *p.get() == DuelPhase::Closed);
    let wanted = (in_lobby && !notice.hidden && (notice.shown.is_some() || place.moved.is_some()))
        .then(|| (notice.shown.clone(), place.clone(), lang));
    if *drawn == wanted && (wanted.is_none() || !toasts.is_empty()) {
        return;
    }
    for toast in &toasts {
        commands.entity(toast).despawn();
    }
    (*drawn).clone_from(&wanted);
    let (Some((shown, place, lang)), Some(fonts)) = (wanted, fonts) else {
        return;
    };
    let width = windows.iter().next().map_or(1280.0, Window::width);
    spawn_toast(
        &mut commands,
        &fonts,
        Metrics::of(width),
        lang,
        shown.as_ref(),
        &place,
    );
}

/// The line under the headline: the exact folder and error where the
/// installation's folder is read-only, the notice's own reason otherwise.
fn reason_of(shown: &Shown, place: &UpdatePlace, lang: Lang) -> Option<String> {
    match (shown, &place.read_only) {
        (
            Shown::Available {
                why: Why::Folder, ..
            },
            Some((folder, error)),
        ) => Some(Phrase::UpdateWhyReadOnly.fill(lang, &[folder, error])),
        _ => shown.reason(lang).map(str::to_owned),
    }
}

/// Whether the notice offers the move: it is only a link because of where
/// the app lies, and a move is possible.
fn offers_move(shown: &Shown, place: &UpdatePlace) -> bool {
    !place.moves.is_empty()
        && matches!(
            shown,
            Shown::Available {
                why: Why::Folder | Why::MoveApp,
                ..
            }
        )
}

/// One small line of text for the toast or the settings.
fn small_line(commands: &mut Commands, fonts: &UiFonts, metrics: Metrics, text: String) -> Entity {
    commands
        .spawn((
            Text::new(text),
            tf(fonts, metrics.small),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id()
}

/// A row of buttons that wraps.
fn button_row(commands: &mut Commands, buttons: &[Entity]) -> Entity {
    let row = commands
        .spawn((
            Node {
                column_gap: px(8),
                row_gap: px(6),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_children(buttons);
    row
}

/// The move's buttons, one per destination, then what moving does, and
/// the last failure if there was one.
fn move_part(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
    place: &UpdatePlace,
) -> Vec<Entity> {
    let buttons: Vec<Entity> = place
        .moves
        .iter()
        .map(|to| {
            our_button(
                commands,
                fonts,
                metrics,
                to.phrase().text(lang),
                UpdateButton::Move(*to),
            )
        })
        .collect();
    let mut parts = vec![
        button_row(commands, &buttons),
        small_line(
            commands,
            fonts,
            metrics,
            Phrase::UpdateMoveWhat.text(lang).to_owned(),
        ),
    ];
    if let Some(failed) = &place.failed {
        parts.push(small_line(commands, fonts, metrics, failed.clone()));
    }
    parts
}

fn spawn_toast(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
    shown: Option<&Shown>,
    place: &UpdatePlace,
) -> Entity {
    let toast = commands
        .spawn((
            UpdateToast,
            Node {
                position_type: PositionType::Absolute,
                left: px(16),
                bottom: px(16),
                max_width: px(380),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                padding: UiRect::all(px(12)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            BorderColor::all(palette::MUTED.with_alpha(0.35)),
            GlobalZIndex(40),
        ))
        .id();
    let headline = |commands: &mut Commands, text: String| {
        commands
            .spawn((
                Text::new(text),
                tf(fonts, metrics.text),
                TextColor(palette::INK),
                Pickable::IGNORE,
            ))
            .id()
    };
    // After a move, the old copy's question comes first: it is asked once.
    if let Some((now, old)) = &place.moved {
        let title = headline(commands, Phrase::UpdateMoved.fill(lang, &[now]));
        let line = small_line(
            commands,
            fonts,
            metrics,
            Phrase::UpdateOldCopy.fill(lang, &[old]),
        );
        let trash = our_button(
            commands,
            fonts,
            metrics,
            Phrase::UpdateTrashOld.text(lang),
            UpdateButton::TrashOld,
        );
        let keep = our_button(
            commands,
            fonts,
            metrics,
            Phrase::UpdateKeepOld.text(lang),
            UpdateButton::KeepOld,
        );
        let row = button_row(commands, &[trash, keep]);
        commands.entity(toast).add_children(&[title, line, row]);
        if let Some(failed) = &place.failed {
            let failed = small_line(commands, fonts, metrics, failed.clone());
            commands.entity(toast).add_child(failed);
        }
    }
    let mut buttons = Vec::new();
    if let Some(shown) = shown {
        let title = headline(commands, shown.headline(lang));
        commands.entity(toast).add_child(title);
        if let Some(reason) = reason_of(shown, place, lang) {
            let line = small_line(commands, fonts, metrics, reason);
            commands.entity(toast).add_child(line);
        }
        if offers_move(shown, place) {
            for part in move_part(commands, fonts, metrics, lang, place) {
                commands.entity(toast).add_child(part);
            }
        }
        if let Some(page) = shown.page() {
            buttons.push(our_button(
                commands,
                fonts,
                metrics,
                Phrase::ReleaseNotes.text(lang),
                UpdateButton::Open(page.to_owned()),
            ));
        }
    }
    buttons.push(our_button(
        commands,
        fonts,
        metrics,
        Phrase::UpdateHide.text(lang),
        UpdateButton::Hide,
    ));
    let row = button_row(commands, &buttons);
    commands.entity(toast).add_child(row);
    toast
}

/// A lobby button that answers to [`press`] rather than to the lobby.
fn our_button(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
    action: UpdateButton,
) -> Entity {
    let id = lobby::button(
        commands,
        fonts,
        metrics,
        label,
        lobby::Press::PickerNothing,
        palette::PANEL,
        true,
    );
    commands.entity(id).remove::<lobby::Press>().insert(action);
    id
}

/// The notice as one line of the table's menu, if there is one to show.
/// Pressing it opens the release page.
pub(crate) fn menu_line(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    notice: Option<&UpdateNotice>,
    size: f32,
) -> Option<Entity> {
    let shown = notice?.shown.as_ref()?;
    let line = commands
        .spawn((
            Button,
            Node {
                width: percent(100),
                ..default()
            },
            Text::new(shown.headline(lang)),
            tf(fonts, size),
            TextColor(palette::DIALOG_INK),
        ))
        .id();
    if let Some(page) = shown.page() {
        commands
            .entity(line)
            .insert(UpdateButton::Open(page.to_owned()));
    }
    Some(line)
}

/// The settings screen's part: the two switches and "Check for updates",
/// with a line saying how the last check went. `None` where no updater is
/// compiled in.
pub(crate) fn controls(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
) -> Option<Entity> {
    if cfg!(any(
        target_arch = "wasm32",
        target_os = "android",
        target_os = "ios"
    )) {
        return None;
    }
    let root = commands
        .spawn((
            Node {
                width: percent(100),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for (label, why, action, readout) in [
        (
            Phrase::UpdateAutoCheck,
            Phrase::UpdateAutoCheckWhy,
            UpdateButton::ToggleCheck,
            Readout::Check,
        ),
        (
            Phrase::UpdateAutoInstall,
            Phrase::UpdateAutoInstallWhy,
            UpdateButton::ToggleInstall,
            Readout::Install,
        ),
    ] {
        let line = lobby::row(commands, metrics, false);
        let words = commands
            .spawn((
                Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                Pickable::IGNORE,
                children![
                    (
                        Text::new(label.text(lang)),
                        tf(fonts, metrics.text),
                        TextColor(palette::INK),
                    ),
                    (
                        Text::new(why.text(lang)),
                        tf(fonts, metrics.small),
                        TextColor(palette::MUTED),
                    )
                ],
            ))
            .id();
        let switch = our_button(commands, fonts, metrics, "", action);
        commands.entity(switch).with_child((
            Text::new(Phrase::SwitchOn.text(lang)),
            tf(fonts, metrics.text),
            TextColor(palette::INK),
            readout,
            Pickable::IGNORE,
        ));
        commands.entity(line).add_children(&[words, switch]);
        commands.entity(root).add_child(line);
    }
    let line = lobby::row(commands, metrics, true);
    let now = our_button(
        commands,
        fonts,
        metrics,
        Phrase::UpdateCheckNow.text(lang),
        UpdateButton::CheckNow,
    );
    let status = commands
        .spawn((
            Text::new(""),
            tf(fonts, metrics.small),
            TextColor(palette::MUTED),
            Readout::Status,
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(line).add_children(&[now, status]);
    let place = commands
        .spawn((
            PlaceRow,
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_children(&[line, place]);
    Some(root)
}

/// Fills the settings' [`PlaceRow`] from [`UpdatePlace`]: why installing is
/// off where the app lies, or that macOS runs it from a read-only copy, and
/// the move. Rebuilt when the place changes or the row is new.
fn show_place(
    mut commands: Commands,
    place: Res<UpdatePlace>,
    fonts: Option<Res<UiFonts>>,
    settings: Option<Res<ClientSettings>>,
    windows: Query<&Window>,
    rows: Query<(Entity, Option<&Children>), With<PlaceRow>>,
    fresh: Query<(), Added<PlaceRow>>,
) {
    if !place.is_changed() && fresh.is_empty() {
        return;
    }
    let Some(fonts) = fonts else {
        return;
    };
    let lang = settings.map_or(Lang::En, |s| Lang::of(&s.lang));
    let metrics = Metrics::of(windows.iter().next().map_or(1280.0, Window::width));
    for (row, children) in &rows {
        for child in children.into_iter().flatten() {
            commands.entity(*child).despawn();
        }
        if !place.worth_saying() {
            continue;
        }
        let said = if let Some((folder, error)) = &place.read_only {
            Some(Phrase::UpdateWhyReadOnly.fill(lang, &[folder, error]))
        } else if place.translocated {
            Some(Phrase::UpdateTranslocated.text(lang).to_owned())
        } else {
            None
        };
        if let Some(said) = said {
            let line = small_line(&mut commands, &fonts, metrics, said);
            commands.entity(row).add_child(line);
        }
        if place.moves.is_empty() {
            if let Some(failed) = &place.failed {
                let line = small_line(&mut commands, &fonts, metrics, failed.clone());
                commands.entity(row).add_child(line);
            }
        } else {
            for part in move_part(&mut commands, &fonts, metrics, lang, &place) {
                commands.entity(row).add_child(part);
            }
        }
    }
}

/// Keeps the settings' switch labels and status line true.
fn show_settings(
    prefs: Res<UpdatePrefs>,
    notice: Res<UpdateNotice>,
    settings: Option<Res<ClientSettings>>,
    mut texts: Query<(&mut Text, &Readout)>,
) {
    let lang = settings.map_or(Lang::En, |s| Lang::of(&s.lang));
    let switch = |on: bool| {
        if on {
            Phrase::SwitchOn
        } else {
            Phrase::SwitchOff
        }
        .text(lang)
        .to_owned()
    };
    for (mut text, readout) in &mut texts {
        let value = match readout {
            Readout::Check => switch(prefs.check),
            Readout::Install => switch(prefs.install),
            Readout::Status => match notice.asked {
                None => String::new(),
                Some(Asked::Checking) => Phrase::UpdateChecking.text(lang).to_owned(),
                Some(Asked::UpToDate) => Phrase::UpdateUpToDate.text(lang).to_owned(),
                Some(Asked::Failed) => Phrase::UpdateCheckFailed.text(lang).to_owned(),
                Some(Asked::Found) => notice
                    .shown
                    .as_ref()
                    .map(|shown| shown.headline(lang))
                    .unwrap_or_default(),
            },
        };
        if **text != value {
            **text = value;
        }
    }
}

#[cfg(test)]
mod tests;

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
use crate::ui_button::UiButton;
use crate::{DuelPhase, lobby};
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::resume::RestartOffer;
use bevy::input::mouse::MouseScrollUnit;
use bevy::picking::events::{PointerClick, PointerScroll};
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
}

/// The settings screen's place for [`UpdatePlace`], filled by `show_place`.
#[derive(Component)]
struct PlaceRow;

/// The notice's panel in the lobby's corner.
#[derive(Component)]
pub struct UpdateToast;

/// Which edge the notice is anchored to; the table has no legal footer.
#[derive(Component)]
struct ToastPlacement(bool);

/// The text and actions scroll independently when a small window needs it.
#[derive(Component)]
struct ToastScroll;

#[derive(Component)]
struct ToastActions;

const TOAST_GAP: f32 = 16.0;
const TOAST_PADDING: f32 = 12.0;
const TOAST_BORDER: f32 = 1.0;

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
            .add_message::<PointerScroll>()
            .add_message::<crate::shellkit::focus::Activated>()
            .add_systems(
                Update,
                (press, scroll_toasts, show_toast, show_settings, show_place).chain(),
            )
            .add_systems(
                PostUpdate,
                (fit_toasts, fit_toast_bodies)
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

/// What the notice's panel was last drawn from: the notice, the place, the
/// language, and whether it stood over a table.
type Drawn = Option<(Option<Shown>, UpdatePlace, Lang, bool)>;

/// Keeps the notice's panel in step with the notice: shown in the lobby's
/// corner while there is something to say and it was not hidden, and once
/// hidden gone. At a table only a ready update's restart offer stands, at
/// the top, and never appears in the middle of a question of this player's
/// ([`RestartOffer`]): it waits for the answer and appears after it, and
/// then stays. The menu's line says the rest there.
#[allow(clippy::too_many_arguments)] // one panel, drawn from everything it depends on
fn show_toast(
    mut commands: Commands,
    notice: Res<UpdateNotice>,
    place: Res<UpdatePlace>,
    phase: Option<Res<State<DuelPhase>>>,
    duel: Option<Res<crate::Duel>>,
    fonts: Option<Res<UiFonts>>,
    settings: Option<Res<ClientSettings>>,
    windows: Query<&Window>,
    toasts: Query<Entity, With<UpdateToast>>,
    mut drawn: Local<Drawn>,
    mut offer: Local<RestartOffer>,
) {
    let lang = settings.map_or(Lang::En, |s| Lang::of(&s.lang));
    let in_lobby = phase.is_none_or(|p| *p.get() == DuelPhase::Closed);
    let ready = matches!(notice.shown, Some(Shown::Ready { .. }));
    let at_table = if in_lobby {
        *offer = RestartOffer::default();
        false
    } else {
        offer.step(ready, notice.hidden, duel.as_deref().and_then(asked))
    };
    let in_corner = in_lobby && !notice.hidden && (notice.shown.is_some() || place.moved.is_some());
    let wanted =
        (in_corner || at_table).then(|| (notice.shown.clone(), place.clone(), lang, at_table));
    if *drawn == wanted && (wanted.is_none() || !toasts.is_empty()) {
        return;
    }
    for toast in &toasts {
        commands.entity(toast).despawn();
    }
    (*drawn).clone_from(&wanted);
    let (Some((shown, place, lang, at_table)), Some(fonts)) = (wanted, fonts) else {
        return;
    };
    let width = windows.iter().next().map_or(1280.0, Window::width);
    let toast = spawn_toast(
        &mut commands,
        &fonts,
        Metrics::of(width),
        lang,
        shown.as_ref(),
        &place,
    );
    if at_table {
        commands.entity(toast).insert((
            ToastPlacement(true),
            UiTransform::from_translation(Val2::percent(-50.0, 0.0)),
        ));
    }
}

/// Fit the panel to the measured front-door page above its legal footer,
/// or to the table between the report corner and the hand/decision ledge.
/// Read UI units, including `UiScale`, and compare before writing: neither a
/// resize nor a taller footer requires rebuilding the notice or its buttons.
#[allow(clippy::type_complexity)] // disjoint root/actions nodes; measured lobby page
fn fit_toasts(
    windows: Query<&Window>,
    ui: Option<Res<UiScale>>,
    roots: Query<(), With<lobby::LobbyRoot>>,
    pages: Query<(
        &crate::shellkit::role::Role,
        &ChildOf,
        &ComputedNode,
        &bevy::ui::UiGlobalTransform,
    )>,
    mut toasts: Query<(&ToastPlacement, &mut Node), (With<UpdateToast>, Without<ToastActions>)>,
    mut actions: Query<(&ChildOf, &mut Node), (With<ToastActions>, Without<UpdateToast>)>,
) {
    if toasts.is_empty() {
        return;
    }
    let Some(window) = crate::hud::scale::window_space(&windows, ui.as_deref()) else {
        return;
    };
    // The front door's direct scroll child ends exactly where its fixed
    // colophon starts. This follows both footer forms and their text scale;
    // signed-in screens with no such page reserve only the window margin.
    let page_bottom = pages
        .iter()
        .filter(|(role, parent, node, _)| {
            **role == crate::shellkit::role::Role::Scroll
                && roots.contains(parent.parent())
                && node.size().y > 0.0
        })
        .map(|(_, _, node, at)| {
            (at.translation.y + node.size().y * 0.5) * node.inverse_scale_factor()
        })
        .fold(window.y, f32::min);
    for (placement, mut node) in &mut toasts {
        let top = if placement.0 {
            crate::hud::TOP_CLEAR
        } else {
            TOAST_GAP
        };
        let bottom = if placement.0 {
            crate::hud::HAND_ZONE_H + TOAST_GAP
        } else {
            (window.y - page_bottom).max(0.0) + TOAST_GAP
        };
        let next = Node {
            left: if placement.0 {
                percent(50)
            } else {
                px(TOAST_GAP)
            },
            top: if placement.0 { px(top) } else { Val::Auto },
            bottom: if placement.0 { Val::Auto } else { px(bottom) },
            max_width: px((window.x - 2.0 * TOAST_GAP).clamp(0.0, 380.0)),
            max_height: px((window.y - top - bottom).max(0.0)),
            padding: UiRect::all(px(if window.y - top - bottom < 160.0 {
                8.0
            } else {
                TOAST_PADDING
            })),
            ..node.clone()
        };
        node.set_if_neq(next);
    }
    for (parent, mut node) in &mut actions {
        let Ok((_, toast)) = toasts.get(parent.parent()) else {
            continue;
        };
        let Val::Px(height) = toast.max_height else {
            continue;
        };
        let Val::Px(padding) = toast.padding.top else {
            continue;
        };
        // Keep one line of the offer readable above the actions, even when
        // a short table needs to scroll a second row of German buttons.
        let max_height = px((height - 2.0 * (padding + TOAST_BORDER) - 6.0 - 24.0).max(0.0));
        if node.max_height != max_height {
            node.max_height = max_height;
        }
    }
}

/// A scrolling flex child has no intrinsic minimum height. Measure its
/// complete text instead: short notices show their whole explanation, while
/// long failures use the space left above the persistent actions.
#[allow(clippy::type_complexity)] // disjoint body and root Node access
fn fit_toast_bodies(
    roots: Query<&Node, (With<UpdateToast>, Without<ToastScroll>)>,
    actions: Query<(&ChildOf, &ComputedNode), With<ToastActions>>,
    measured: Query<&ComputedNode>,
    mut bodies: Query<
        (&ChildOf, &Children, &mut Node),
        (
            With<ToastScroll>,
            Without<ToastActions>,
            Without<UpdateToast>,
        ),
    >,
) {
    for (parent, children, mut body) in &mut bodies {
        let Ok(root) = roots.get(parent.parent()) else {
            continue;
        };
        let (Val::Px(limit), Val::Px(padding)) = (root.max_height, root.padding.top) else {
            continue;
        };
        let action_height = actions
            .iter()
            .find(|(owner, _)| owner.parent() == parent.parent())
            .map_or(0.0, |(_, node)| node.size().y * node.inverse_scale_factor());
        let mut natural = 0.0;
        for (at, child) in children.iter().enumerate() {
            if let Ok(node) = measured.get(child) {
                natural += node.size().y * node.inverse_scale_factor();
                if at > 0 {
                    natural += 6.0;
                }
            }
        }
        let available = (limit - 2.0 * (padding + TOAST_BORDER) - action_height - 6.0).max(0.0);
        let height = px(natural.min(available));
        if body.height != height {
            body.height = height;
        }
    }
}

/// Overflow is useful only with input: wheel messages climb from a button
/// or a text run to the notice's nearest scrolling body, in either context.
fn scroll_toasts(
    mut wheels: MessageReader<PointerScroll>,
    parents: Query<&ChildOf>,
    mut panels: Query<(&mut ScrollPosition, &ComputedNode), With<ToastScroll>>,
) {
    for wheel in wheels.read() {
        let by = -wheel.y
            * match wheel.unit {
                MouseScrollUnit::Line => lobby::WHEEL_LINE,
                MouseScrollUnit::Pixel => 1.0,
            };
        let mut current = Some(wheel.entity);
        while let Some(entity) = current {
            if let Ok((mut position, node)) = panels.get_mut(entity) {
                let next = crate::hud::scrolled(
                    position.y,
                    by,
                    node.size().y,
                    node.content_size().y,
                    node.inverse_scale_factor(),
                );
                if (position.y - next).abs() > f32::EPSILON {
                    position.y = next;
                }
                break;
            }
            current = parents.get(entity).ok().map(ChildOf::parent);
        }
    }
}

/// The question this player owes the table now, by its view's `seq`: the
/// view says it is awaited, or a question of its own is open. `None` while
/// nothing is asked of it.
fn asked(duel: &crate::Duel) -> Option<u64> {
    let seq = duel.view.as_ref().map_or(0, |view| view.seq);
    let awaited = duel
        .view
        .as_ref()
        .is_some_and(|view| view.awaiting == Some(view.seat))
        || duel
            .interaction
            .as_ref()
            .is_some_and(baylee_client_core::interaction::Interaction::is_mine);
    awaited.then_some(seq)
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
            TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
            Node {
                min_width: px(0),
                flex_shrink: 0.0,
                ..default()
            },
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
                width: percent(100),
                min_width: px(0),
                flex_shrink: 0.0,
                column_gap: px(8),
                row_gap: px(6),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for &button in buttons {
        commands
            .entity(button)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.min_width = px(0);
                node.max_width = percent(100);
            });
    }
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
            let id = our_button(
                commands,
                fonts,
                metrics,
                to.phrase().text(lang),
                UpdateButton::Move(*to),
            );
            commands
                .entity(id)
                .insert(update_stop(&UpdateButton::Move(*to)));
            id
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

/// A ready update's offer: restart now and come back here, or later, and
/// what went wrong with a restart asked for already.
fn restart_part(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
    shown: &Shown,
    place: &UpdatePlace,
) -> Vec<Entity> {
    let mut parts: Vec<Entity> = std::iter::once(Phrase::UpdateRestartOffer.text(lang).to_owned())
        .chain(place.failed.clone())
        .map(|text| small_line(commands, fonts, metrics, text))
        .collect();
    let mut row = vec![
        our_button(
            commands,
            fonts,
            metrics,
            Phrase::UpdateRestartNow.text(lang),
            UpdateButton::RestartNow,
        ),
        our_button(
            commands,
            fonts,
            metrics,
            Phrase::UpdateLater.text(lang),
            UpdateButton::Later,
        ),
    ];
    if let Some(page) = shown.page() {
        row.push(our_button(
            commands,
            fonts,
            metrics,
            Phrase::ReleaseNotes.text(lang),
            UpdateButton::Open(page.to_owned()),
        ));
    }
    parts.push(button_row(commands, &row));
    parts
}

#[allow(clippy::too_many_lines)] // one notice, its scrolling text and persistent action rows
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
            ToastPlacement(false),
            Node {
                position_type: PositionType::Absolute,
                left: px(TOAST_GAP),
                bottom: px(TOAST_GAP),
                max_width: px(380),
                min_width: px(0),
                min_height: px(0),
                overflow: Overflow::clip(),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                padding: UiRect::all(px(TOAST_PADDING)),
                border: UiRect::all(px(TOAST_BORDER)),
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            BorderColor::all(palette::MUTED.with_alpha(0.35)),
            GlobalZIndex(40),
        ))
        .id();
    let body = commands
        .spawn((
            ToastScroll,
            ScrollPosition::default(),
            Node {
                width: percent(100),
                min_width: px(0),
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    let actions = commands
        .spawn((
            ToastActions,
            ToastScroll,
            ScrollPosition::default(),
            Node {
                width: percent(100),
                min_width: px(0),
                min_height: px(0),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands.entity(toast).add_children(&[body, actions]);
    let headline = |commands: &mut Commands, text: String| {
        commands
            .spawn((
                Text::new(text),
                TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                Node {
                    min_width: px(0),
                    flex_shrink: 0.0,
                    ..default()
                },
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
        commands.entity(body).add_children(&[title, line]);
        commands.entity(actions).add_child(row);
        if let Some(failed) = &place.failed {
            let failed = small_line(commands, fonts, metrics, failed.clone());
            commands.entity(body).add_child(failed);
        }
    }
    let mut buttons = Vec::new();
    if let Some(shown) = shown {
        let title = headline(commands, shown.headline(lang));
        commands.entity(body).add_child(title);
        if let Some(reason) = reason_of(shown, place, lang) {
            let line = small_line(commands, fonts, metrics, reason);
            commands.entity(body).add_child(line);
        }
        if matches!(shown, Shown::Ready { .. }) {
            let mut parts = restart_part(commands, fonts, metrics, lang, shown, place);
            if let Some(row) = parts.pop() {
                commands.entity(actions).add_child(row);
            }
            commands.entity(body).add_children(&parts);
            return toast;
        }
        if offers_move(shown, place) {
            let mut parts = move_part(commands, fonts, metrics, lang, place).into_iter();
            if let Some(row) = parts.next() {
                commands.entity(actions).add_child(row);
            }
            for part in parts {
                commands.entity(body).add_child(part);
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
    commands.entity(actions).add_child(row);
    toast
}

/// A lobby button that answers to [`press`] rather than to the lobby.
/// An Updates control's place in the settings screen's tab order.
fn update_stop(action: &UpdateButton) -> crate::shellkit::focus::Stop {
    let item = match action {
        UpdateButton::ToggleCheck => 0,
        UpdateButton::ToggleInstall => 1,
        UpdateButton::CheckNow => 2,
        UpdateButton::Move(MoveTo::Home) => 3,
        UpdateButton::Move(MoveTo::System) => 4,
        _ => 9,
    };
    crate::shellkit::focus::Stop::item(crate::settingsui::keys::SETTINGS.name, "update", item)
}

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
        lobby::Press::Shared(lobby::SharedPress::PickerNothing),
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
            UiButton,
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
    kit: crate::shellkit::controls::Kit,
    metrics: Metrics,
    lang: Lang,
) -> Option<Entity> {
    let fonts = kit.fonts;
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
    for switch in [
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
        let line = switch_line(commands, kit, metrics, lang, switch);
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
    commands
        .entity(now)
        .insert(update_stop(&UpdateButton::CheckNow));
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

/// One of the two switches: its label and why, and the kit's toggle, as
/// every other settings section draws a switch (beta.6: it was a boxed
/// "on").
fn switch_line(
    commands: &mut Commands,
    kit: crate::shellkit::controls::Kit,
    metrics: Metrics,
    lang: Lang,
    (label, why, action, readout): (Phrase, Phrase, UpdateButton, Readout),
) -> Entity {
    let fonts = kit.fonts;
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
    let stop = update_stop(&action);
    let switch = crate::shellkit::controls::toggle(commands, kit, true, (action, stop, readout));
    commands.entity(line).add_children(&[words, switch]);
    line
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
    switches: Query<(&Readout, &Children), Without<Text>>,
    mut toggles: Query<&mut crate::shellkit::controls::ToggleMotion>,
) {
    for (readout, children) in &switches {
        let on = match readout {
            Readout::Check => prefs.check,
            Readout::Install => prefs.install,
            Readout::Status => continue,
        };
        for child in children {
            if let Ok(mut motion) = toggles.get_mut(*child)
                && motion.on != on
            {
                motion.on = on;
            }
        }
    }
    let lang = settings.map_or(Lang::En, |s| Lang::of(&s.lang));
    for (mut text, readout) in &mut texts {
        let value = match readout {
            Readout::Check | Readout::Install => continue,
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

//! The update card: in the lobby's corner, the restart offer at a table,
//! and the table menu's line (`docs/client.md` §"Updating").
//!
//! One card on the shell's kit for every state the updater can be in. It
//! names the version in its title, beside a mark in the accent, and says
//! the rest under it:
//!
//! - **downloading**: a slim bar, the share beside "Downloading…", and
//!   "45 of 150 MB · 3.2 MB/s · about 30 s left" under it;
//! - **verifying**, **unpacking**: the full bar, "Verifying…" over "150 MB
//!   downloaded", then "Unpacking…" over "Signature verified";
//! - **ready**: "Restart to update" (the screen's one primary), "Later"
//!   (it installs when the client quits) and the release notes;
//! - **failed**: why, that the next check tries again, and "Try again";
//! - **only a link** (installing off, a development build, a folder this
//!   user cannot write): why, the move where one helps, the release notes.
//!
//! Every card but the ready one has a close mark, which hides it for the
//! session ("Later" does that for the ready one); news brings it back
//! ([`UpdateNotice::show`]). Nothing here makes a sound, takes the keyboard
//! or covers the screen, and at a table only the ready card stands, at the
//! top, never in the middle of the player's own question ([`RestartOffer`]);
//! the menu's line says the rest there.
//!
//! The card is rebuilt only when what it says changes: its state, the
//! place, the language, the text step. A download's readings write the
//! bar's width and its two numbers in place ([`show_progress`]), four times
//! a second at most, so a download relays nothing else out.

mod draw;
mod look;

use super::{Asked, Fetch, Progress, Shown, UpdateButton, UpdateNotice, UpdatePlace, words};
use crate::hud::{UiFonts, palette, tf};
use crate::settings::ClientSettings;
use crate::shellkit::controls::Kit;
use crate::shellkit::role::Role;
use crate::shellkit::{InputClass, Platform, ShellMetrics, TextSize, Viewport, px_fixed};
use crate::ui_button::UiButton;
use crate::{DuelPhase, lobby};
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::resume::RestartOffer;
use bevy::input::mouse::MouseScrollUnit;
use bevy::picking::events::PointerScroll;
use bevy::prelude::*;
use bevy::ui::{percent, px};
use draw::spawn_card;
use look::Card;

/// The card's root.
#[derive(Component)]
pub struct UpdateToast;

/// Where the card stands: at a table (`true`, top centre, clear of the
/// hand), else in the lobby's bottom-left corner above the legal footer.
#[derive(Component)]
pub(super) struct ToastPlacement(pub(super) bool);

/// The card's width and padding at the text step it was drawn at: a small
/// window may only make it narrower ([`fit_toasts`]).
#[derive(Component, Clone, Copy, Debug)]
pub(super) struct CardSize {
    width: f32,
    pad: f32,
}

/// The text and actions scroll independently when a small window needs it.
#[derive(Component)]
pub(super) struct ToastScroll;

/// The card's row of actions, which stays in view while the text scrolls.
#[derive(Component)]
pub(super) struct ToastActions;

/// The bar's fill, as wide as the share that has arrived.
#[derive(Component)]
pub(super) struct ProgressFill;

/// The share that has arrived, beside the stage ("45%").
#[derive(Component)]
pub(super) struct ProgressPercent;

/// The line under the bar ("45 of 150 MB · 3.2 MB/s · about 30 s left").
#[derive(Component)]
pub(super) struct ProgressLine;

/// The table menu's line, which follows a download's share.
#[derive(Component)]
pub(super) struct MenuLine;

/// The air between the card and the window's edges.
pub(super) const TOAST_GAP: f32 = 16.0;
/// The card's hairline border.
pub(super) const TOAST_BORDER: f32 = 1.0;
/// The gap between the card's rows, and between the lines of its text.
pub(super) const ROW_GAP: f32 = 6.0;
/// Over a table the card stands above the HUD's own layers, as it always
/// did; in the lobby it takes the kit's toast band.
const TABLE_Z: i32 = 40;

/// What the card was last drawn from. Not the window's size: a resize
/// fits the drawn card ([`fit_toasts`]) and rebuilds nothing.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Drawn {
    shown: Option<Shown>,
    place: UpdatePlace,
    /// A failed card's retry, while one is under way or after it failed.
    retrying: Option<Asked>,
    lang: Lang,
    at_table: bool,
    /// The text step, the input and the big-screen step it was drawn at.
    size: (TextSize, InputClass, i32),
}

/// The kit's metrics for the card: the window in the UI's units, the
/// device's text step and input, and over a table the HUD's own scale taken
/// out (`ShellMetrics::under`), so the card is the same size in both places.
fn metrics(
    window: Option<&Window>,
    ui: Option<&UiScale>,
    settings: Option<&ClientSettings>,
    input: Option<&InputClass>,
) -> ShellMetrics {
    let (width, height) = window.map_or((1280.0, 800.0), |w| (w.width(), w.height()));
    ShellMetrics::under(
        Viewport {
            width,
            height,
            platform: Platform::current(),
            input: input.copied().unwrap_or_default(),
        },
        settings.map_or_else(TextSize::default, |s| s.text_size),
        ui.map_or(1.0, |ui| ui.0),
    )
}

/// Keeps the card in step with the notice: in the lobby's corner while there
/// is something to say and it was not hidden, and once hidden gone. At a
/// table only a ready update's restart offer stands, at the top, and never
/// appears in the middle of a question of this player's ([`RestartOffer`]):
/// it waits for the answer and appears after it, and then stays. The menu's
/// line says the rest there.
#[allow(clippy::too_many_arguments)] // one card, drawn from everything it depends on
pub(super) fn show_toast(
    mut commands: Commands,
    notice: Res<UpdateNotice>,
    place: Res<UpdatePlace>,
    phase: Option<Res<State<DuelPhase>>>,
    duel: Option<Res<crate::Duel>>,
    fonts: Option<Res<UiFonts>>,
    settings: Option<Res<ClientSettings>>,
    input: Option<Res<InputClass>>,
    ui: Option<Res<UiScale>>,
    windows: Query<&Window>,
    toasts: Query<Entity, With<UpdateToast>>,
    mut drawn: Local<Option<Drawn>>,
    mut offer: Local<RestartOffer>,
) {
    let lang = settings.as_ref().map_or(Lang::En, |s| Lang::of(&s.lang));
    let in_lobby = phase.is_none_or(|p| *p.get() == DuelPhase::Closed);
    let ready = matches!(notice.shown, Some(Shown::Ready { .. }));
    let at_table = if in_lobby {
        *offer = RestartOffer::default();
        false
    } else {
        offer.step(ready, notice.hidden, duel.as_deref().and_then(asked))
    };
    let in_corner = in_lobby && !notice.hidden && (notice.shown.is_some() || place.moved.is_some());
    let m = metrics(
        windows.iter().next(),
        ui.as_deref(),
        settings.as_deref(),
        input.as_deref(),
    );
    let wanted = (in_corner || at_table).then(|| Drawn {
        shown: notice.shown.clone(),
        place: place.clone(),
        retrying: notice
            .asked
            .filter(|_| matches!(notice.shown, Some(Shown::Failed { .. }))),
        lang,
        at_table,
        size: (m.step, m.input, (m.scale * 8.0).round() as i32),
    });
    if *drawn == wanted && (wanted.is_none() || !toasts.is_empty()) {
        return;
    }
    for toast in &toasts {
        commands.entity(toast).despawn();
    }
    (*drawn).clone_from(&wanted);
    let (Some(want), Some(fonts)) = (wanted, fonts) else {
        return;
    };
    let kit = Kit {
        fonts: &fonts,
        m,
        german: want.lang == Lang::De,
    };
    let card = Card {
        shown: want.shown.as_ref(),
        place: &want.place,
        retrying: want.retrying,
        progress: notice.progress.as_ref(),
    };
    let toast = spawn_card(&mut commands, kit, want.lang, &card);
    if want.at_table {
        commands.entity(toast).insert((
            ToastPlacement(true),
            UiTransform::from_translation(Val2::percent(-50.0, 0.0)),
            GlobalZIndex(TABLE_Z),
        ));
    }
}

/// What the bar and its two numbers say for `stage`: the bar's fill, the
/// share beside the stage, and the line under the bar.
pub(super) fn meter(
    stage: Fetch,
    progress: Option<&Progress>,
    lang: Lang,
) -> (f32, String, String) {
    match stage {
        Fetch::Downloading => progress.map_or((0.0, String::new(), String::new()), |p| {
            (
                words::fill(p),
                words::percent_text(lang, p),
                words::line(lang, p),
            )
        }),
        Fetch::Verifying => (
            1.0,
            String::new(),
            progress
                .map(|p| Phrase::UpdateDownloaded.fill(lang, &[&words::size(lang, p.total)]))
                .unwrap_or_default(),
        ),
        Fetch::Unpacking => (
            1.0,
            String::new(),
            Phrase::UpdateSignatureVerified.text(lang).to_owned(),
        ),
    }
}

/// The bar's fill as a width, in tenths of a percent: finer than a pixel
/// on any card, coarse enough that a reading which moves nothing writes
/// nothing.
fn fill_width(fill: f32) -> Val {
    percent((fill.clamp(0.0, 1.0) * 1000.0).round() / 10.0)
}

/// Writes a download's reading into the drawn card: the bar's width, the
/// share and the line under it, each only when it differs. Nothing runs
/// unless the notice changed or a card was just drawn.
#[allow(clippy::type_complexity)] // three disjoint parts of one card
pub(super) fn show_progress(
    notice: Res<UpdateNotice>,
    settings: Option<Res<ClientSettings>>,
    fresh: Query<(), Added<ProgressFill>>,
    mut fills: Query<&mut Node, With<ProgressFill>>,
    mut shares: Query<&mut Text, (With<ProgressPercent>, Without<ProgressLine>)>,
    mut lines: Query<&mut Text, (With<ProgressLine>, Without<ProgressPercent>)>,
) {
    if fills.is_empty() || (!notice.is_changed() && fresh.is_empty()) {
        return;
    }
    let Some(Shown::Fetching { stage, .. }) = &notice.shown else {
        return;
    };
    let lang = settings.map_or(Lang::En, |s| Lang::of(&s.lang));
    let (fill, share, line) = meter(*stage, notice.progress.as_ref(), lang);
    let width = fill_width(fill);
    for mut node in &mut fills {
        if node.width != width {
            node.width = width;
        }
    }
    for mut text in &mut shares {
        if text.0 != share {
            text.0.clone_from(&share);
        }
    }
    for mut text in &mut lines {
        if text.0 != line {
            text.0.clone_from(&line);
        }
    }
}

/// Fit the card to the measured front-door page above its legal footer, or
/// to the table between the report corner and the hand/decision ledge.
/// Read UI units, including `UiScale`, and compare before writing: neither a
/// resize nor a taller footer requires rebuilding the card or its buttons.
#[allow(clippy::type_complexity)] // disjoint root/actions nodes; measured lobby page
pub(super) fn fit_toasts(
    windows: Query<&Window>,
    ui: Option<Res<UiScale>>,
    roots: Query<(), With<lobby::LobbyRoot>>,
    hosts: Query<&ChildOf>,
    pages: Query<(&Role, &ChildOf, &ComputedNode, &bevy::ui::UiGlobalTransform)>,
    mut toasts: Query<
        (&ToastPlacement, &CardSize, &mut Node),
        (With<UpdateToast>, Without<ToastActions>),
    >,
    mut actions: Query<(&ChildOf, &mut Node), (With<ToastActions>, Without<UpdateToast>)>,
) {
    if toasts.is_empty() {
        return;
    }
    let Some(window) = crate::hud::scale::window_space(&windows, ui.as_deref()) else {
        return;
    };
    // The front door's scrolling page ends exactly where its fixed colophon
    // starts. This follows both footer forms and their text scale;
    // signed-in screens with no such page reserve only the window margin.
    // The page stands in the lobby's root, or in the host its scroll bar
    // shares with it (`lobby::scrollbars::attach`), which it fills.
    let in_root = |parent: Entity| {
        roots.contains(parent)
            || hosts
                .get(parent)
                .is_ok_and(|host| roots.contains(host.parent()))
    };
    let page_bottom = pages
        .iter()
        .filter(|(role, parent, node, _)| {
            **role == Role::Scroll && in_root(parent.parent()) && node.size().y > 0.0
        })
        .map(|(_, _, node, at)| {
            (at.translation.y + node.size().y * 0.5) * node.inverse_scale_factor()
        })
        .fold(window.y, f32::min);
    for (placement, size, mut node) in &mut toasts {
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
        let tight = window.y - top - bottom < 160.0;
        let next = Node {
            left: if placement.0 {
                percent(50)
            } else {
                px_fixed(TOAST_GAP)
            },
            top: if placement.0 { px(top) } else { Val::Auto },
            bottom: if placement.0 { Val::Auto } else { px(bottom) },
            width: px((window.x - 2.0 * TOAST_GAP).clamp(0.0, size.width)),
            max_height: px((window.y - top - bottom).max(0.0)),
            padding: UiRect::all(px(if tight { size.pad.min(8.0) } else { size.pad })),
            ..node.clone()
        };
        node.set_if_neq(next);
    }
    for (parent, mut node) in &mut actions {
        let Ok((_, _, toast)) = toasts.get(parent.parent()) else {
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
        let max_height = px((height - 2.0 * (padding + TOAST_BORDER) - ROW_GAP - 24.0).max(0.0));
        if node.max_height != max_height {
            node.max_height = max_height;
        }
    }
}

/// A scrolling flex child has no intrinsic minimum height. Measure its
/// complete text instead: short notices show their whole explanation, while
/// long failures use the space left above the persistent actions.
#[allow(clippy::type_complexity)] // disjoint body and root Node access
pub(super) fn fit_toast_bodies(
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
            .map_or(0.0, |(_, node)| {
                node.size().y * node.inverse_scale_factor() + ROW_GAP
            });
        let mut natural = 0.0;
        for (at, child) in children.iter().enumerate() {
            if let Ok(node) = measured.get(child) {
                natural += node.size().y * node.inverse_scale_factor();
                if at > 0 {
                    natural += ROW_GAP;
                }
            }
        }
        let available = (limit - 2.0 * (padding + TOAST_BORDER) - action_height).max(0.0);
        let height = px(natural.min(available));
        if body.height != height {
            body.height = height;
        }
    }
}

/// Overflow is useful only with input: wheel messages climb from a button
/// or a text run to the card's nearest scrolling part, in either context.
pub(super) fn scroll_toasts(
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

/// The notice in one line of the table's menu: the headline and, while it
/// downloads, how far.
pub(super) fn menu_text(notice: &UpdateNotice, lang: Lang) -> String {
    match (&notice.shown, &notice.progress) {
        (
            Some(
                shown @ Shown::Fetching {
                    stage: Fetch::Downloading,
                    ..
                },
            ),
            Some(progress),
        ) => format!(
            "{} · {}",
            shown.headline(lang),
            words::percent_text(lang, progress)
        ),
        (Some(shown), _) => shown.headline(lang),
        (None, _) => String::new(),
    }
}

/// The settings' status line while there is something under way or gone
/// wrong: the download's whole line, or the stage, or the failure.
pub(super) fn status_text(notice: &UpdateNotice, lang: Lang) -> Option<String> {
    match (&notice.shown, &notice.progress) {
        (
            Some(
                shown @ Shown::Fetching {
                    stage: Fetch::Downloading,
                    ..
                },
            ),
            Some(progress),
        ) => Some(format!(
            "{} · {}",
            shown.headline(lang),
            words::line(lang, progress)
        )),
        (Some(shown @ (Shown::Fetching { .. } | Shown::Failed { .. })), _) => {
            Some(shown.headline(lang))
        }
        _ => None,
    }
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
    let notice = notice?;
    let shown = notice.shown.as_ref()?;
    let line = commands
        .spawn((
            UiButton,
            MenuLine,
            Node {
                width: percent(100),
                ..default()
            },
            Text::new(menu_text(notice, lang)),
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

/// Keeps the menu's line in step with a download, while the menu stands.
/// A line is drawn from the notice as it is, so only a change moves it;
/// the menu takes a line away itself when there is nothing to say.
pub(super) fn follow_the_menu_line(
    notice: Res<UpdateNotice>,
    settings: Option<Res<ClientSettings>>,
    mut lines: Query<&mut Text, With<MenuLine>>,
) {
    if lines.is_empty() || !notice.is_changed() || notice.shown.is_none() {
        return;
    }
    let lang = settings.map_or(Lang::En, |s| Lang::of(&s.lang));
    let said = menu_text(&notice, lang);
    for mut text in &mut lines {
        if text.0 != said {
            text.0.clone_from(&said);
        }
    }
}

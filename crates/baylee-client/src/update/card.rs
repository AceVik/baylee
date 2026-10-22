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

use super::{Asked, Fetch, Progress, Shown, UpdateButton, UpdateNotice, UpdatePlace, Why, words};
use crate::hud::{UiFonts, icon_tf, palette, tf, tf_bold};
use crate::settings::ClientSettings;
use crate::shellkit::controls::{self, Kit, Live, Weight};
use crate::shellkit::role::Role;
use crate::shellkit::{InputClass, Platform, ShellMetrics, TextSize, Viewport, px_fixed, tokens};
use crate::ui_button::UiButton;
use crate::{DuelPhase, lobby};
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::resume::RestartOffer;
use bevy::input::mouse::MouseScrollUnit;
use bevy::picking::events::PointerScroll;
use bevy::prelude::*;
use bevy::ui::{percent, px};

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
/// The accent edge on the card's left, as the lobby's toasts wear it.
const ACCENT_EDGE: f32 = 3.0;
/// The card's width at the default text step: the download's line on one
/// row, and the ready card's three actions on one row in English.
const CARD_WIDTH: f32 = 380.0;
/// Over a table the card stands above the HUD's own layers, as it always
/// did; in the lobby it takes the kit's toast band.
const TABLE_Z: i32 = 40;

const DOWNLOAD: char = '\u{f019}';
const SHIELD: char = '\u{f3ed}';
const BOX: char = '\u{f49e}';
const DONE: char = '\u{f058}';
const WARNING: char = '\u{f071}';
const INFO: char = '\u{f05a}';
const FOLDER: char = '\u{f07c}';
const CLOSE: char = '\u{f00d}';

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
    // The front door's direct scroll child ends exactly where its fixed
    // colophon starts. This follows both footer forms and their text scale;
    // signed-in screens with no such page reserve only the window margin.
    let page_bottom = pages
        .iter()
        .filter(|(role, parent, node, _)| {
            **role == Role::Scroll && roots.contains(parent.parent()) && node.size().y > 0.0
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

/// What a card is drawn from.
struct Card<'a> {
    shown: Option<&'a Shown>,
    place: &'a UpdatePlace,
    retrying: Option<Asked>,
    progress: Option<&'a Progress>,
}

/// One action of a card.
struct Action {
    label: String,
    weight: Weight,
    does: UpdateButton,
}

impl Action {
    fn new(label: impl Into<String>, weight: Weight, does: UpdateButton) -> Self {
        Self {
            label: label.into(),
            weight,
            does,
        }
    }

    /// The release notes, as the least of a card's actions.
    fn notes(lang: Lang, page: &str, weight: Weight) -> Self {
        Self::new(
            Phrase::ReleaseNotes.text(lang),
            weight,
            UpdateButton::Open(page.to_owned()),
        )
    }
}

/// What a card says, before it is drawn.
struct Look {
    mark: char,
    accent: Color,
    title: String,
    /// While fetching: which part. The bar and its lines stand under the
    /// title.
    fetch: Option<Fetch>,
    lines: Vec<String>,
    actions: Vec<Action>,
    /// A close mark, which hides the card for the session.
    close: bool,
}

impl Look {
    fn bare(mark: char, title: String) -> Self {
        Self {
            mark,
            accent: tokens::ACCENT,
            title,
            fetch: None,
            lines: Vec::new(),
            actions: Vec::new(),
            close: true,
        }
    }

    #[allow(clippy::too_many_lines)] // one arm per state, each short
    fn of(card: &Card, lang: Lang) -> Self {
        // After a move, the old copy's question comes first: it is asked once.
        if let Some((now, old)) = &card.place.moved {
            let mut look = Self::bare(FOLDER, Phrase::UpdateMoved.fill(lang, &[now]));
            look.lines.push(Phrase::UpdateOldCopy.fill(lang, &[old]));
            look.lines.extend(card.place.failed.clone());
            look.actions = vec![
                Action::new(
                    Phrase::UpdateTrashOld.text(lang),
                    Weight::Secondary,
                    UpdateButton::TrashOld,
                ),
                Action::new(
                    Phrase::UpdateKeepOld.text(lang),
                    Weight::Ghost,
                    UpdateButton::KeepOld,
                ),
            ];
            return look;
        }
        let Some(shown) = card.shown else {
            return Self::bare(INFO, String::new());
        };
        match shown {
            Shown::Fetching { version, stage, .. } => {
                let mark = match stage {
                    Fetch::Downloading => DOWNLOAD,
                    Fetch::Verifying => SHIELD,
                    Fetch::Unpacking => BOX,
                };
                let mut look = Self::bare(mark, Phrase::UpdateCardTitle.fill(lang, &[version]));
                look.fetch = Some(*stage);
                look
            }
            Shown::Ready { version, .. } => {
                let mut look = Self::bare(DONE, Phrase::UpdateCardReady.fill(lang, &[version]));
                look.lines
                    .push(Phrase::UpdateRestartOffer.text(lang).to_owned());
                look.lines.extend(card.place.failed.clone());
                // Two actions and no third: at the card's width the release
                // notes would wrap onto a row of their own, and they are
                // what the "Updated to" card after the restart offers (and
                // the table menu's line, all along).
                look.actions = vec![
                    Action::new(
                        Phrase::UpdateRestartNow.text(lang),
                        Weight::Primary,
                        UpdateButton::RestartNow,
                    ),
                    Action::new(
                        Phrase::UpdateLater.text(lang),
                        Weight::Secondary,
                        UpdateButton::Later,
                    ),
                ];
                // "Later" is its way out.
                look.close = false;
                look
            }
            Shown::Failed {
                version,
                page,
                reason,
            } => {
                let mut look = Self::bare(WARNING, Phrase::UpdateCardTitle.fill(lang, &[version]));
                look.accent = tokens::DANGER;
                look.lines.push(match card.retrying {
                    Some(Asked::Checking) => Phrase::UpdateChecking.text(lang).to_owned(),
                    Some(Asked::Failed) => Phrase::UpdateCheckFailed.text(lang).to_owned(),
                    _ => Phrase::UpdateFailedWhy.fill(lang, &[reason]),
                });
                look.actions = vec![
                    Action::new(
                        Phrase::UpdateTryAgain.text(lang),
                        Weight::Secondary,
                        UpdateButton::TryAgain,
                    ),
                    Action::notes(lang, page, Weight::Ghost),
                ];
                look
            }
            Shown::Available { version, page, .. } => {
                let mut look = Self::bare(INFO, Phrase::UpdateCardAvailable.fill(lang, &[version]));
                look.lines.extend(reason_of(shown, card.place, lang));
                let moves = offers_move(shown, card.place);
                if moves {
                    look.actions = card
                        .place
                        .moves
                        .iter()
                        .map(|to| {
                            Action::new(
                                to.phrase().text(lang),
                                Weight::Secondary,
                                UpdateButton::Move(*to),
                            )
                        })
                        .collect();
                    look.lines
                        .push(Phrase::UpdateMoveWhat.text(lang).to_owned());
                    look.lines.extend(card.place.failed.clone());
                }
                let weight = if moves {
                    Weight::Ghost
                } else {
                    Weight::Secondary
                };
                look.actions.push(Action::notes(lang, page, weight));
                look
            }
            Shown::Updated { version, page } => {
                let mut look = Self::bare(DONE, Phrase::UpdatedTo.fill(lang, &[version]));
                if let Some(page) = page {
                    look.actions.push(Action::notes(lang, page, Weight::Ghost));
                }
                look
            }
        }
    }
}

/// The card's elevation over the painting: one soft shadow.
fn card_shadow() -> BoxShadow {
    BoxShadow(vec![ShadowStyle {
        color: Color::srgba(0.0, 0.0, 0.0, 0.45),
        x_offset: px_fixed(0.0),
        y_offset: px_fixed(6.0),
        spread_radius: px_fixed(0.0),
        blur_radius: px_fixed(18.0),
    }])
}

/// Figures that keep their width as they count: lining and tabular.
fn counting(fonts: &UiFonts, size: f32) -> TextFont {
    let mut font = tf(fonts, size);
    font.font_features = bevy::text::FontFeatures::builder()
        .enable(bevy::text::FontFeatureTag::LINING_FIGURES)
        .enable(bevy::text::FontFeatureTag::TABULAR_FIGURES)
        .build();
    font
}

/// A line of the card's text.
fn line(commands: &mut Commands, kit: Kit, text: String, ink: Color) -> Entity {
    commands
        .spawn((
            Text::new(text),
            TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
            Node {
                min_width: px_fixed(0.0),
                flex_shrink: 0.0,
                ..default()
            },
            tf(kit.fonts, kit.m.small),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id()
}

/// The title row: the mark in the accent and the title, which names the
/// version. Room is left on its right for the close mark.
fn title_row(commands: &mut Commands, kit: Kit, look: &Look, pad: f32) -> Entity {
    let row = commands
        .spawn((
            Node {
                width: percent(100),
                min_width: px_fixed(0.0),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: kit.m.px(8.0),
                padding: UiRect::right(px(if look.close {
                    (kit.m.hit - pad).max(0.0)
                } else {
                    0.0
                })),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let mark = commands
        .spawn((
            Text::new(look.mark.to_string()),
            icon_tf(kit.fonts, kit.m.text * 0.95),
            TextColor(look.accent),
            Pickable::IGNORE,
        ))
        .id();
    let title = commands
        .spawn((
            Text::new(look.title.clone()),
            TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
            // The rest of the row, whatever the text measures: a wrapping
            // text's own basis in a row is its narrowest, one character.
            Node {
                min_width: px_fixed(0.0),
                flex_basis: px_fixed(0.0),
                flex_grow: 1.0,
                flex_shrink: 1.0,
                ..default()
            },
            tf_bold(kit.fonts, kit.m.text),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_children(&[mark, title]);
    row
}

/// The stage with its share, the slim bar, and the line under it.
fn meter_part(
    commands: &mut Commands,
    kit: Kit,
    lang: Lang,
    stage: Fetch,
    progress: Option<&Progress>,
) -> Entity {
    let (fill, share, under) = meter(stage, progress, lang);
    let part = commands
        .spawn((
            Node {
                width: percent(100),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(5.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let head = commands
        .spawn((
            Node {
                width: percent(100),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Baseline,
                column_gap: kit.m.px(8.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let said = match stage {
        Fetch::Downloading => Phrase::UpdateDownloading,
        Fetch::Verifying => Phrase::UpdateVerifying,
        Fetch::Unpacking => Phrase::UpdateUnpacking,
    };
    let stage_words = line(commands, kit, said.text(lang).to_owned(), tokens::INK);
    let share = commands
        .spawn((
            ProgressPercent,
            Text::new(share),
            counting(kit.fonts, kit.m.small),
            TextColor(tokens::INK),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(head).add_children(&[stage_words, share]);
    let track = commands
        .spawn((
            Node {
                width: percent(100),
                height: kit.m.px(4.0),
                flex_shrink: 0.0,
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(tokens::TRACK_OFF),
            Pickable::IGNORE,
        ))
        .id();
    let filled = commands
        .spawn((
            ProgressFill,
            Node {
                width: fill_width(fill),
                height: percent(100),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(tokens::ACCENT),
            BoxShadow(vec![ShadowStyle {
                color: tokens::ACCENT.with_alpha(0.45),
                x_offset: px_fixed(0.0),
                y_offset: px_fixed(0.0),
                spread_radius: px_fixed(0.0),
                blur_radius: px_fixed(6.0),
            }]),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(track).add_child(filled);
    let under = commands
        .spawn((
            ProgressLine,
            Text::new(under),
            TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
            Node {
                min_width: px_fixed(0.0),
                ..default()
            },
            counting(kit.fonts, kit.m.small),
            TextColor(tokens::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(part).add_children(&[head, track, under]);
    part
}

/// The close mark at the card's top right: a hit area as large as the
/// kit's, laid over the padding, so it hides the card without taking a row.
fn close_mark(commands: &mut Commands, kit: Kit) -> Entity {
    let face = commands
        .spawn((
            Node {
                width: kit.m.px(24.0),
                height: kit.m.px(24.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            crate::ambience::Feel::rising_to(Color::NONE, tokens::HOVER),
            Pickable::IGNORE,
        ))
        .id();
    let glyph = commands
        .spawn((
            Text::new(CLOSE.to_string()),
            icon_tf(kit.fonts, kit.m.small * 0.9),
            TextColor(tokens::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(face).add_child(glyph);
    let hit = commands
        .spawn((
            Role::Hit,
            UpdateButton::Hide,
            Node {
                position_type: PositionType::Absolute,
                top: px_fixed(0.0),
                right: px_fixed(0.0),
                width: px(kit.m.hit),
                height: px(kit.m.hit),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .id();
    commands.entity(hit).add_child(face);
    hit
}

/// The card's actions in a row that wraps, the release notes (a ghost) to
/// the right of the rest.
fn action_row(commands: &mut Commands, kit: Kit, actions: Vec<Action>) -> Entity {
    let row = commands
        .spawn((
            Node {
                width: percent(100),
                min_width: px_fixed(0.0),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: kit.m.px(8.0),
                row_gap: kit.m.px(6.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for action in actions {
        let ghost = action.weight == Weight::Ghost;
        let button = controls::button(
            commands,
            kit,
            &action.label,
            action.weight,
            Live::Yes,
            None,
            action.does,
        );
        commands
            .entity(button)
            .entry::<Node>()
            .and_modify(move |mut node| {
                node.min_width = px_fixed(0.0);
                node.max_width = percent(100);
                if ghost {
                    node.margin.left = Val::Auto;
                }
            });
        commands.entity(row).add_child(button);
    }
    row
}

fn spawn_card(commands: &mut Commands, kit: Kit, lang: Lang, card: &Card) -> Entity {
    let look = Look::of(card, lang);
    let width = kit.m.scaled(CARD_WIDTH);
    let pad = kit.m.pad.min(kit.m.scaled(14.0));
    let base = tokens::OPAQUE.with_alpha(0.94);
    let edge = look.accent.with_alpha(0.35);
    let toast = commands
        .spawn((
            UpdateToast,
            ToastPlacement(false),
            CardSize { width, pad },
            Role::Toast,
            Node {
                position_type: PositionType::Absolute,
                left: px_fixed(TOAST_GAP),
                bottom: px_fixed(TOAST_GAP),
                width: px(width),
                min_width: px_fixed(0.0),
                min_height: px_fixed(0.0),
                overflow: Overflow::clip(),
                flex_direction: FlexDirection::Column,
                row_gap: px_fixed(ROW_GAP),
                padding: UiRect::all(px(pad)),
                border: UiRect::new(
                    px_fixed(ACCENT_EDGE),
                    px_fixed(TOAST_BORDER),
                    px_fixed(TOAST_BORDER),
                    px_fixed(TOAST_BORDER),
                ),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(base),
            BackgroundGradient::from(LinearGradient::to_bottom(vec![
                look.accent.with_alpha(0.16).into(),
                base.into(),
            ])),
            BorderColor {
                top: edge,
                right: edge,
                bottom: edge,
                left: look.accent.with_alpha(0.9),
            },
            card_shadow(),
            GlobalZIndex(tokens::z::TOAST),
        ))
        .id();
    let body = commands
        .spawn((
            ToastScroll,
            ScrollPosition::default(),
            Node {
                width: percent(100),
                min_width: px_fixed(0.0),
                min_height: px_fixed(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: px_fixed(ROW_GAP),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands.entity(toast).add_child(body);
    let title = title_row(commands, kit, &look, pad);
    commands.entity(body).add_child(title);
    if let Some(stage) = look.fetch {
        let part = meter_part(commands, kit, lang, stage, card.progress);
        commands.entity(body).add_child(part);
    }
    for text in &look.lines {
        let said = line(commands, kit, text.clone(), palette::MUTED);
        commands.entity(body).add_child(said);
    }
    let close = look.close;
    if !look.actions.is_empty() {
        let actions = commands
            .spawn((
                ToastActions,
                ToastScroll,
                ScrollPosition::default(),
                Node {
                    width: percent(100),
                    min_width: px_fixed(0.0),
                    min_height: px_fixed(0.0),
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
            ))
            .id();
        let row = action_row(commands, kit, look.actions);
        commands.entity(actions).add_child(row);
        commands.entity(toast).add_child(actions);
    }
    if close {
        let mark = close_mark(commands, kit);
        commands.entity(toast).add_child(mark);
    }
    toast
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

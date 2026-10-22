//! The notice's panel: in the lobby's corner, the restart offer at a
//! table, and the table menu's line (`docs/client.md` §"Updating").

use super::settings::update_stop;
use super::{Shown, UpdateButton, UpdateNotice, UpdatePlace, Why};
use crate::hud::{UiFonts, palette, tf};
use crate::lobby::Metrics;
use crate::settings::ClientSettings;
use crate::ui_button::UiButton;
use crate::{DuelPhase, lobby};
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::resume::RestartOffer;
use bevy::input::mouse::MouseScrollUnit;
use bevy::picking::events::PointerScroll;
use bevy::prelude::*;
use bevy::ui::{percent, px};

/// The notice's panel in the lobby's corner.
#[derive(Component)]
pub struct UpdateToast;

/// Which edge the notice is anchored to; the table has no legal footer.
#[derive(Component)]
pub(super) struct ToastPlacement(bool);

/// The text and actions scroll independently when a small window needs it.
#[derive(Component)]
pub(super) struct ToastScroll;

#[derive(Component)]
pub(super) struct ToastActions;

pub(super) const TOAST_GAP: f32 = 16.0;
pub(super) const TOAST_PADDING: f32 = 12.0;
pub(super) const TOAST_BORDER: f32 = 1.0;

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
pub(super) fn show_toast(
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
pub(super) fn fit_toasts(
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

/// One small line of text for the toast or the settings.
pub(super) fn small_line(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    text: String,
) -> Entity {
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
pub(super) fn move_part(
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

pub(super) fn our_button(
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

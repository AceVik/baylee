//! The settings screen's Updates section: the two per-device switches,
//! "Check for updates" and its status line, and where the app lies.

use super::card::status_text;
use super::{Asked, MoveTo, Shown, UpdateButton, UpdateNotice, UpdatePlace, UpdatePrefs};
use crate::hud::{UiFonts, palette, tf};
use crate::lobby::{self, Metrics};
use crate::settings::ClientSettings;
use baylee_client_core::i18n::{Lang, Phrase};
use bevy::prelude::*;
use bevy::ui::{percent, px};

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

/// A lobby button that answers to the updater's `press` rather than to
/// the lobby.
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

/// The settings screen's place for [`UpdatePlace`], filled by `show_place`.
#[derive(Component)]
pub(super) struct PlaceRow;

/// A text on the settings screen that follows the updater's state.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Readout {
    Check,
    Install,
    Status,
}

/// An Updates control's place in the settings screen's tab order.
pub(super) fn update_stop(action: &UpdateButton) -> crate::shellkit::focus::Stop {
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
pub(super) fn show_place(
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
pub(super) fn show_settings(
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
            // A download under way says how far, whoever asked for it; a
            // check the player asked for comes next, then a failed download.
            Readout::Status => match notice.asked {
                _ if matches!(notice.shown, Some(Shown::Fetching { .. })) => {
                    status_text(&notice, lang).unwrap_or_default()
                }
                Some(Asked::Checking) => Phrase::UpdateChecking.text(lang).to_owned(),
                _ if matches!(notice.shown, Some(Shown::Failed { .. })) => {
                    status_text(&notice, lang).unwrap_or_default()
                }
                None => String::new(),
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

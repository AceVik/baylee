//! The report form, drawn over whatever screen is up, and its keys.
//!
//! One retained tree under [`DeskRoot`], rebuilt whenever what it shows
//! changes (a keystroke, a box, an answer), the way the lobby draws its
//! modals. Every button is the lobby's button with a [`DeskPress`] in place
//! of its `Press`, so neither the lobby's click handler nor the table's ever
//! sees a press meant for the form.

use std::hash::{Hash, Hasher};

use baylee_client_core::bugreport::{Category, CrashConsent, Kind, MAX_TEXT_CHARS, Status};
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::textbuf::{Dir, Step};
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::ui::FocusPolicy;

use super::{Answers, ReportDesk};
use crate::hud::{UiFonts, palette, tf, tf_bold};
use crate::lobby::{LobbyState, Metrics};
use crate::settings::ClientSettings;

/// The form's root, over everything.
#[derive(Component)]
pub(super) struct DeskRoot;

/// The form's scrolling column.
#[derive(Component)]
pub(super) struct DeskScroll;

/// What a button on the form does.
#[derive(Component, Clone, Copy, Debug)]
pub(super) enum DeskPress {
    /// Pick a kind.
    Kind(Kind),
    /// Tick or clear a box.
    Toggle(Category),
    /// Tick or clear automatic crash reports.
    Crashes,
    /// Open or close the preview.
    Preview,
    /// Send.
    Send,
    /// Close the form.
    Close,
    /// The crash question: yes.
    CrashSend,
    /// The crash question: no.
    CrashNever,
}

/// The most of the preview drawn at once, in characters. The whole body is
/// what is sent; a view of a busy board runs to hundreds of kilobytes, and a
/// text node that size costs a frame to lay out on every keystroke.
const PREVIEW_CHARS: usize = 20_000;

/// Frames the form waits for its screenshot before drawing anyway.
const SHOT_PATIENCE: u32 = 30;

/// Hands this frame's keys to the form. `true` when `Esc` closed it.
pub(super) fn typing(
    desk: &mut ReportDesk,
    codes: &ButtonInput<KeyCode>,
    typed: &mut MessageReader<KeyboardInput>,
) -> bool {
    let shift = codes.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let word = codes.any_pressed([
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
    ]);
    let line = codes.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]);
    let command = line || codes.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let reach = if line {
        Step::Line
    } else if word {
        Step::Word
    } else {
        Step::Char
    };
    let mut closed = false;
    for key in typed.read() {
        if !key.state.is_pressed() {
            continue;
        }
        let text = &mut desk.form.text;
        match &key.logical_key {
            Key::Escape => closed = true,
            Key::Backspace => text.delete_back(),
            Key::Delete => text.delete_forward(),
            Key::ArrowLeft => text.move_caret(reach, Dir::Left, shift),
            Key::ArrowRight => text.move_caret(reach, Dir::Right, shift),
            Key::Home => text.move_caret(Step::Line, Dir::Left, shift),
            Key::End => text.move_caret(Step::Line, Dir::Right, shift),
            Key::Enter => text.replace_selection("\n"),
            Key::Character(c) if command => {
                if c.eq_ignore_ascii_case("a") {
                    text.select_all();
                }
                continue;
            }
            _ => {
                let Some(typed) = key.text.as_ref() else {
                    continue;
                };
                let printable: String = typed.chars().filter(|c| !c.is_control()).collect();
                if printable.is_empty() {
                    continue;
                }
                text.replace_selection(&printable);
            }
        }
        desk.form.edited();
    }
    closed
}

/// Scrolls the form's column under the wheel.
pub(super) fn scroll(
    desk: Res<ReportDesk>,
    mut wheel: MessageReader<MouseWheel>,
    mut column: Query<&mut ScrollPosition, With<DeskScroll>>,
) {
    let delta: f32 = wheel
        .read()
        .map(|w| match w.unit {
            MouseScrollUnit::Line => w.y * 40.0,
            MouseScrollUnit::Pixel => w.y,
        })
        .sum();
    if !desk.open || delta == 0.0 {
        return;
    }
    for mut position in &mut column {
        position.y = (position.y - delta).max(0.0);
    }
}

/// Everything the tree shows, folded into one number.
fn signature(desk: &ReportDesk, settings: &ClientSettings, signed_in: bool, width: f32) -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    desk.open.hash(&mut hash);
    desk.asking.hash(&mut hash);
    desk.shooting.hash(&mut hash);
    desk.gathered.screenshot.is_some().hash(&mut hash);
    for category in Category::ALL {
        desk.gathered.has(category).hash(&mut hash);
    }
    format!("{:?}", desk.form.kind).hash(&mut hash);
    desk.form.text.text().hash(&mut hash);
    desk.form.text.cursor().hash(&mut hash);
    format!("{:?}", desk.form.status).hash(&mut hash);
    desk.form.preview.hash(&mut hash);
    format!("{:?}{:?}", settings.reports, settings.lang).hash(&mut hash);
    signed_in.hash(&mut hash);
    (width as u32).hash(&mut hash);
    hash.finish()
}

/// Rebuilds the form when anything it shows has changed.
#[allow(clippy::too_many_arguments)] // one reader per thing the form shows
pub(super) fn draw(
    mut commands: Commands,
    mut desk: ResMut<ReportDesk>,
    settings: Res<ClientSettings>,
    lobby: Option<Res<LobbyState>>,
    fonts: Option<Res<UiFonts>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    roots: Query<Entity, With<DeskRoot>>,
    mut last: Local<Option<u64>>,
) {
    let Some(fonts) = fonts else {
        return;
    };
    // The picture is taken of the frame the form opened on; drawn on that
    // frame, the form would be in it. A few frames at most, and a capture
    // that never comes holds the form back no longer than half a second.
    if desk.open && desk.shooting && desk.waited < SHOT_PATIENCE {
        desk.waited += 1;
        return;
    }
    let signed_in = super::session(lobby.as_deref()).is_some();
    let width = windows.single().map_or(1280.0, Window::width);
    let now = signature(&desk, &settings, signed_in, width);
    if *last == Some(now) && (roots.iter().next().is_some() || !(desk.open || desk.asking)) {
        return;
    }
    *last = Some(now);
    for root in &roots {
        commands.entity(root).despawn();
    }
    let metrics = Metrics::of(width);
    let lang = Lang::of(&settings.lang);
    if desk.asking && !desk.open {
        crash_question(&mut commands, &fonts, metrics, lang);
    } else if desk.open {
        form(
            &mut commands,
            &desk,
            &settings,
            signed_in,
            &fonts,
            metrics,
            lang,
        );
    }
}

/// The shade over everything, and the panel on it.
fn shade(commands: &mut Commands, metrics: Metrics, max_width: f32) -> (Entity, Entity) {
    let shade = commands
        .spawn((
            DeskRoot,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::all(px(metrics.pad)),
                ..default()
            },
            BackgroundColor(Color::BLACK.with_alpha(0.8)),
            FocusPolicy::Block,
            GlobalZIndex(1000),
        ))
        .id();
    let panel = commands
        .spawn((
            DeskScroll,
            Node {
                width: percent(100),
                max_width: px(max_width),
                max_height: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(metrics.gap),
                padding: UiRect::all(px(metrics.pad)),
                overflow: Overflow::scroll_y(),
                border_radius: BorderRadius::all(px(10)),
                ..default()
            },
            ScrollPosition::default(),
            BackgroundColor(palette::PANEL.with_alpha(0.98)),
        ))
        .id();
    commands.entity(shade).add_child(panel);
    (shade, panel)
}

/// A line of text.
fn words(
    commands: &mut Commands,
    font: TextFont,
    text: impl Into<String>,
    colour: Color,
) -> Entity {
    commands
        .spawn((
            Text::new(text),
            font,
            TextColor(colour),
            Pickable::IGNORE,
            Node {
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .id()
}

/// One of the lobby's buttons, doing `press`.
fn button(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
    press: DeskPress,
    lit: bool,
    enabled: bool,
) -> Entity {
    let tone = if lit {
        palette::PANEL_HOT
    } else {
        palette::PANEL
    };
    let id = crate::lobby::button(
        commands,
        fonts,
        metrics,
        label,
        crate::lobby::Press::PickerNothing,
        tone,
        enabled,
    );
    commands.entity(id).remove::<crate::lobby::Press>();
    if enabled {
        commands.entity(id).insert((Button, press)).observe(pressed);
    }
    id
}

/// A row of controls.
fn row(commands: &mut Commands, metrics: Metrics, end: bool) -> Entity {
    commands
        .spawn((
            Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(metrics.gap),
                row_gap: px(metrics.gap / 2.0),
                justify_content: if end {
                    JustifyContent::FlexEnd
                } else {
                    JustifyContent::FlexStart
                },
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// The form itself.
#[allow(clippy::too_many_lines)] // the form, top to bottom
fn form(
    commands: &mut Commands,
    desk: &ReportDesk,
    settings: &ClientSettings,
    signed_in: bool,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
) {
    let (_, panel) = shade(commands, metrics, 760.0);
    let mut parts = Vec::new();
    parts.push(words(
        commands,
        tf_bold(fonts, metrics.head),
        Phrase::ReportButton.text(lang),
        palette::INK,
    ));

    let kinds = row(commands, metrics, false);
    for kind in Kind::OFFERED {
        let chip = button(
            commands,
            fonts,
            metrics,
            kind.phrase().text(lang),
            DeskPress::Kind(kind),
            desk.form.kind == kind,
            true,
        );
        commands.entity(kinds).add_child(chip);
    }
    parts.push(kinds);

    // The text box: the buffer with its caret, or the prompt when empty.
    let buffer = &desk.form.text;
    let (shown, ink) = if buffer.is_empty() {
        (
            format!("▏{}", Phrase::ReportTextHint.text(lang)),
            palette::MUTED,
        )
    } else {
        let text = buffer.text();
        let at = buffer.cursor().min(text.len());
        (format!("{}▏{}", &text[..at], &text[at..]), palette::INK)
    };
    let field = commands
        .spawn((
            Node {
                width: percent(100),
                min_height: px(metrics.tap * 3.0),
                padding: UiRect::all(px(metrics.gap)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(6)),
                flex_shrink: 0.0,
                ..default()
            },
            BorderColor::all(palette::ACCENT),
            BackgroundColor(palette::PANEL_LIT),
            Pickable::IGNORE,
        ))
        .id();
    let inside = words(commands, tf(fonts, metrics.text), shown, ink);
    commands.entity(field).add_child(inside);
    parts.push(field);
    let count = Phrase::ReportChars.fill(
        lang,
        &[&desk.form.chars().to_string(), &MAX_TEXT_CHARS.to_string()],
    );
    parts.push(words(
        commands,
        tf(fonts, metrics.small),
        count,
        if desk.form.over_limit() {
            palette::DANGER
        } else {
            palette::MUTED
        },
    ));
    parts.push(words(
        commands,
        tf(fonts, metrics.small),
        Phrase::ReportAlways.text(lang),
        palette::MUTED,
    ));
    parts.push(words(
        commands,
        tf_bold(fonts, metrics.text),
        Phrase::ReportIncludeHeading.text(lang),
        palette::INK,
    ));

    for category in Category::ALL {
        let there = desk.gathered.has(category);
        let ticked = settings.reports.allows(category);
        let mark = if ticked { "☑" } else { "☐" };
        let mut label = format!("{mark}  {}", category.phrase().text(lang));
        if category == Category::Screenshot
            && let Some(shot) = &desk.gathered.screenshot
        {
            label.push_str("  ·  ");
            label.push_str(&Phrase::ReportShotSize.fill(
                lang,
                &[
                    &shot.width.to_string(),
                    &shot.height.to_string(),
                    &shot.kilobytes().to_string(),
                ],
            ));
        }
        let coming = category == Category::Screenshot && desk.shooting;
        if !there && !coming {
            label.push_str("  ");
            label.push_str(Phrase::ReportCatNothing.text(lang));
        }
        let entry = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(2),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let tick = button(
            commands,
            fonts,
            metrics,
            &label,
            DeskPress::Toggle(category),
            false,
            true,
        );
        let hint = words(
            commands,
            tf(fonts, metrics.small),
            category.hint().text(lang),
            palette::MUTED,
        );
        commands.entity(entry).add_children(&[tick, hint]);
        parts.push(entry);
    }
    let crashes = settings.reports.crashes == CrashConsent::Send;
    parts.push(button(
        commands,
        fonts,
        metrics,
        &format!(
            "{}  {}",
            if crashes { "☑" } else { "☐" },
            Phrase::ReportCrashesBox.text(lang)
        ),
        DeskPress::Crashes,
        false,
        true,
    ));

    let preview_label = if desk.form.preview {
        Phrase::ReportPreviewHide
    } else {
        Phrase::ReportPreviewShow
    };
    let preview_row = row(commands, metrics, false);
    let toggle = button(
        commands,
        fonts,
        metrics,
        preview_label.text(lang),
        DeskPress::Preview,
        desk.form.preview,
        true,
    );
    commands.entity(preview_row).add_child(toggle);
    parts.push(preview_row);
    if desk.form.preview {
        let full = desk.form.preview_text(&desk.gathered, &settings.reports);
        let total = full.chars().count();
        let mut shown: String = full.chars().take(PREVIEW_CHARS).collect();
        if total > PREVIEW_CHARS {
            shown.push_str("\n… (+");
            shown.push_str(&(total - PREVIEW_CHARS).to_string());
            shown.push(')');
        }
        let pane = commands
            .spawn((
                Node {
                    width: percent(100),
                    padding: UiRect::all(px(metrics.gap)),
                    border_radius: BorderRadius::all(px(6)),
                    flex_shrink: 0.0,
                    ..default()
                },
                BackgroundColor(Color::BLACK.with_alpha(0.45)),
                Pickable::IGNORE,
            ))
            .id();
        let text = words(commands, tf(fonts, metrics.small), shown, palette::INK);
        commands.entity(pane).add_child(text);
        parts.push(pane);
    }

    if !signed_in {
        parts.push(words(
            commands,
            tf(fonts, metrics.small),
            Phrase::ReportNeedsSession.text(lang),
            palette::DANGER,
        ));
    }
    if let Some(said) = desk.form.status.text(lang) {
        let colour = match desk.form.status {
            Status::Sent(_) => palette::HEAL,
            Status::Sending | Status::Editing => palette::MUTED,
            Status::Failed(_) | Status::Blocked(_) => palette::DANGER,
        };
        parts.push(words(commands, tf(fonts, metrics.text), said, colour));
    }
    if matches!(desk.form.status, Status::Sending | Status::Sent(_))
        && (desk.form.trimmed.screenshot || desk.form.trimmed.log_lines.is_some())
    {
        parts.push(words(
            commands,
            tf(fonts, metrics.small),
            Phrase::ReportTrimmed.text(lang),
            palette::MUTED,
        ));
    }

    let actions = row(commands, metrics, true);
    let close = button(
        commands,
        fonts,
        metrics,
        Phrase::ReportClose.text(lang),
        DeskPress::Close,
        false,
        true,
    );
    let send = button(
        commands,
        fonts,
        metrics,
        Phrase::ReportSend.text(lang),
        DeskPress::Send,
        true,
        desk.form.can_send(signed_in),
    );
    commands.entity(actions).add_children(&[close, send]);
    parts.push(actions);
    commands.entity(panel).add_children(&parts);
}

/// The one-time question after a crash.
fn crash_question(commands: &mut Commands, fonts: &UiFonts, metrics: Metrics, lang: Lang) {
    let (_, panel) = shade(commands, metrics, 560.0);
    let title = words(
        commands,
        tf_bold(fonts, metrics.head),
        Phrase::CrashAskTitle.text(lang),
        palette::INK,
    );
    let body = words(
        commands,
        tf(fonts, metrics.text),
        Phrase::CrashAskBody.text(lang),
        palette::INK,
    );
    let actions = row(commands, metrics, true);
    let never = button(
        commands,
        fonts,
        metrics,
        Phrase::CrashAskNever.text(lang),
        DeskPress::CrashNever,
        false,
        true,
    );
    let send = button(
        commands,
        fonts,
        metrics,
        Phrase::CrashAskSend.text(lang),
        DeskPress::CrashSend,
        true,
        true,
    );
    commands.entity(actions).add_children(&[never, send]);
    commands.entity(panel).add_children(&[title, body, actions]);
}

/// A button on the form was pressed.
fn pressed(
    mut click: On<Pointer<Click>>,
    presses: Query<&DeskPress>,
    mut desk: ResMut<ReportDesk>,
    mut settings: ResMut<ClientSettings>,
    lobby: Option<Res<LobbyState>>,
    answers: Res<Answers>,
) {
    let Ok(press) = presses.get(click.entity) else {
        return;
    };
    click.propagate(false);
    match *press {
        DeskPress::Kind(kind) => {
            desk.form.kind = kind;
            desk.form.edited();
        }
        DeskPress::Toggle(category) => {
            super::consent_mut(&mut settings).toggle(category);
            settings.save();
            desk.form.edited();
        }
        DeskPress::Crashes => {
            let consent = super::consent_mut(&mut settings);
            consent.crashes = if consent.crashes == CrashConsent::Send {
                CrashConsent::Never
            } else {
                CrashConsent::Send
            };
            settings.save();
        }
        DeskPress::Preview => desk.form.preview = !desk.form.preview,
        DeskPress::Send => {
            let desk = desk.as_mut();
            super::send(desk, lobby.as_deref(), &settings, &answers);
        }
        DeskPress::Close => {
            desk.open = false;
            desk.swallow = true;
        }
        DeskPress::CrashSend | DeskPress::CrashNever => {
            let send = matches!(press, DeskPress::CrashSend);
            super::answer_the_crash_question(&mut desk, &mut settings, send);
        }
    }
}

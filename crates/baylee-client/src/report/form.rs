//! The report form, drawn over whatever screen is up, and its keys.
//!
//! One retained tree under [`DeskRoot`], rebuilt whenever what it shows
//! changes (a keystroke, a box, an answer), the way the lobby draws its
//! modals. Every button is the lobby's button with a [`DeskPress`] in place
//! of its `Press`, so neither the lobby's click handler nor the table's ever
//! sees a press meant for the form.

use std::hash::{Hash, Hasher};

use baylee_client_core::bugreport::{
    Category, CrashConsent, Kind, MAX_TEXT_CHARS, Part, RecordConsent, Route, Status, Via,
};
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
pub(crate) struct DeskRoot;

/// The form's scrolling column.
#[derive(Component)]
pub(crate) struct DeskScroll;

/// The report's text box: it scrolls on its own, and keeps the caret in view
/// ([`place_the_caret`]).
#[derive(Component)]
pub(crate) struct DeskBox;

/// The paragraph inside [`DeskBox`]: the text as three spans — before the
/// selection, the selection, after it — which is what the caret is placed
/// between.
#[derive(Component)]
pub(crate) struct DeskText;

/// The caret in [`DeskBox`]: a bar of its own, stood where the paragraph was
/// laid out ([`place_the_caret`]) and blinked by [`blink`].
///
/// It was the glyph `▏` inside the text until #320, which neither of the
/// faces this client ships has, so nothing was drawn where the caret was.
#[derive(Component, Default)]
pub(crate) struct DeskCaret {
    /// Whether it has been stood where the text is. Until then it is not
    /// drawn: a bar at the box's corner over a line of text is a caret in
    /// the wrong place for a frame.
    pub(crate) placed: bool,
}

/// The spans of [`DeskText`], as the renderer numbers its sections: the
/// empty root is 0.
const HEAD: usize = 1;
const SELECTED: usize = 2;
const TAIL: usize = 3;

/// The text box's height in lines of text before it scrolls, and the height
/// it keeps when empty.
const BOX_LINES_MIN: f32 = 4.0;
const BOX_LINES_MAX: f32 = 10.0;

/// The caret's width, in logical pixels.
const CARET_WIDTH: f32 = 1.5;

/// What a button on the form does.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) enum DeskPress {
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
    /// Tick or clear the local game's record, for this report.
    Record,
    /// Never offer the record, or offer it again.
    RecordNever,
    /// The confirmation: send.
    ConfirmSend,
    /// The confirmation: back to the form.
    ConfirmBack,
}

/// What the preview names the device id by before this device has one:
/// it is made the moment a direct report is first sent.
const DEVICE_TO_BE_MADE: &str = "(made for the first direct report)";

/// The most of the preview drawn at once, in characters. The whole body is
/// what is sent; a view of a busy board runs to hundreds of kilobytes, and a
/// text node that size costs a frame to lay out on every keystroke.
const PREVIEW_CHARS: usize = 20_000;

/// Frames the form waits for its screenshot before drawing anyway.
const SHOT_PATIENCE: u32 = 30;

/// Hands this frame's keys to the form. `true` when `Esc` closed it.
///
/// `Ctrl`/`Cmd`+`V` asks `clipboard` for its text; natively the answer is
/// there at once, in a browser it comes a frame or more later, so the read
/// waits on the desk and [`take_the_paste`] lands it whenever it arrives.
/// Without a clipboard (a headless test) the chord does nothing, and in
/// particular does not type a `v`.
pub(super) fn typing(
    desk: &mut ReportDesk,
    codes: &ButtonInput<KeyCode>,
    typed: &mut MessageReader<KeyboardInput>,
    mut clipboard: Option<&mut bevy::clipboard::Clipboard>,
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
    if desk.form.confirming {
        // The words are what is being confirmed: they do not change under
        // the confirmation. `Esc` goes back to them.
        for key in typed.read() {
            if key.state.is_pressed() && key.logical_key == Key::Escape {
                desk.form.confirming = false;
            }
        }
        return false;
    }
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
                } else if c.eq_ignore_ascii_case("v")
                    && let Some(clipboard) = clipboard.as_deref_mut()
                {
                    desk.paste = Some(clipboard.fetch_text());
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

/// Lands a paste the clipboard has answered, cut to the room the limit
/// leaves ([`baylee_client_core::bugreport::ReportForm::paste`]). A read
/// that failed (an empty or non-text clipboard) is dropped without a word:
/// nothing was pasted, and the box shows that.
pub(super) fn take_the_paste(desk: &mut ReportDesk) {
    let Some(result) = desk
        .paste
        .as_mut()
        .and_then(bevy::clipboard::ClipboardRead::poll_result)
    else {
        return;
    };
    desk.paste = None;
    if let Ok(text) = result
        && desk.open
    {
        desk.form.paste(&text);
    }
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
fn signature(desk: &ReportDesk, settings: &ClientSettings, route: &Route, width: f32) -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    desk.form.send_record.hash(&mut hash);
    desk.form.confirming.hash(&mut hash);
    desk.gathered.local_record.is_some().hash(&mut hash);
    format!("{route:?}{:?}", settings.report_device).hash(&mut hash);
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
    desk.form.text.selection().hash(&mut hash);
    format!("{:?}", desk.form.status).hash(&mut hash);
    desk.form.preview.hash(&mut hash);
    format!("{:?}{:?}", settings.reports, settings.lang).hash(&mut hash);
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
    column: Query<&ScrollPosition, With<DeskScroll>>,
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
    // Closed, asking nothing and with nothing left up to take down: the
    // frame has no work here. This runs every frame of every screen, and
    // the route and the signature below allocate, so most frames stop here.
    if !(desk.open || desk.asking) && roots.is_empty() {
        *last = None;
        return;
    }
    let route = super::route(lobby.as_deref(), &settings);
    let width = windows.single().map_or(1280.0, Window::width);
    let now = signature(&desk, &settings, &route, width);
    if *last == Some(now) && (roots.iter().next().is_some() || !(desk.open || desk.asking)) {
        return;
    }
    *last = Some(now);
    if desk.drawn_form
        && let Ok(scrolled) = column.single()
    {
        desk.panel_scroll = scrolled.y;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    desk.drawn_form = desk.open && !desk.form.confirming;
    let metrics = Metrics::of(width);
    let lang = Lang::of(&settings.lang);
    if desk.asking && !desk.open {
        crash_question(&mut commands, &fonts, metrics, lang);
    } else if desk.open && desk.form.confirming {
        confirmation(
            &mut commands,
            &desk,
            &settings,
            &route,
            &fonts,
            metrics,
            lang,
        );
    } else if desk.open {
        form(
            &mut commands,
            &desk,
            &settings,
            &route,
            &fonts,
            metrics,
            lang,
        );
    }
}

/// The shade over everything, and the panel on it.
fn shade(
    commands: &mut Commands,
    metrics: Metrics,
    max_width: f32,
    scrolled: f32,
) -> (Entity, Entity) {
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
            ScrollPosition(Vec2::new(0.0, scrolled)),
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
    if lit && enabled {
        // As the lobby marks its chosen language: the colour the button
        // rests at, and the one it warms from.
        commands.entity(id).insert((
            BackgroundColor(palette::PANEL_HOT),
            crate::ambience::Feel::new(palette::PANEL_HOT),
        ));
    }
    if enabled {
        commands.entity(id).insert((Button, press)).observe(pressed);
    }
    id
}

/// A box: the lobby's button, with a square in front that is ticked or not.
///
/// The mark is a glyph of the icon font; the interface's own font has no
/// ballot boxes.
fn checkbox(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
    press: DeskPress,
    ticked: bool,
) -> Entity {
    let id = button(commands, fonts, metrics, label, press, ticked, true);
    let (glyph, ink) = if ticked {
        ('\u{f14a}', palette::ACCENT)
    } else {
        ('\u{f0c8}', palette::MUTED.with_alpha(0.5))
    };
    let mark = commands
        .spawn((
            Text::new(glyph.to_string()),
            crate::hud::icon_tf(fonts, metrics.text),
            TextColor(ink),
            Pickable::IGNORE,
            Node {
                margin: UiRect::right(px(metrics.gap)),
                ..default()
            },
        ))
        .id();
    commands.entity(id).insert_children(0, &[mark]);
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
    route: &Route,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
) {
    let (_, panel) = shade(commands, metrics, 760.0, desk.panel_scroll);
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

    parts.push(text_box(commands, desk, fonts, metrics, lang));
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
        if desk.gathered.game_id.is_some() {
            Phrase::ReportAlways.text(lang)
        } else {
            Phrase::ReportAlwaysLocal.text(lang)
        },
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
        let mut label = category.phrase().text(lang).to_string();
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
            label.push_str(
                category
                    .nothing_where(super::shot::TAKES_PICTURES)
                    .text(lang),
            );
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
        let tick = checkbox(
            commands,
            fonts,
            metrics,
            &label,
            DeskPress::Toggle(category),
            ticked,
        );
        let hint = words(
            commands,
            tf(fonts, metrics.small),
            category.hint_where(super::shot::TAKES_PICTURES).text(lang),
            palette::MUTED,
        );
        commands.entity(entry).add_children(&[tick, hint]);
        parts.push(entry);
    }
    let crashes = settings.reports.crashes == CrashConsent::Send;
    parts.push(checkbox(
        commands,
        fonts,
        metrics,
        Phrase::ReportCrashesBox.text(lang),
        DeskPress::Crashes,
        crashes,
    ));
    parts.extend(record_boxes(commands, desk, settings, fonts, metrics, lang));

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
        let device = settings
            .report_device
            .as_deref()
            .unwrap_or(DEVICE_TO_BE_MADE);
        let full = desk
            .form
            .preview_text(&desk.gathered, &settings.reports, via(route, device));
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

    match route {
        Route::Gateway(_) => {}
        Route::Direct(url) => parts.push(words(
            commands,
            tf(fonts, metrics.small),
            Phrase::ReportGoesDirect.fill(lang, &[&service_of(url)]),
            palette::MUTED,
        )),
        Route::Nowhere => parts.push(words(
            commands,
            tf(fonts, metrics.small),
            Phrase::ReportNeedsSession.text(lang),
            palette::DANGER,
        )),
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
    if matches!(desk.form.status, Status::Sending | Status::Sent(_)) && desk.form.trimmed.record {
        parts.push(words(
            commands,
            tf(fonts, metrics.small),
            Phrase::ReportRecordLeftOut.text(lang),
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
        desk.form.can_send(route.sends()),
    );
    commands.entity(actions).add_children(&[close, send]);
    parts.push(actions);
    commands.entity(panel).add_children(&parts);
}

/// How a report goes by `route`, for the form's own calls.
fn via<'a>(route: &Route, device: &'a str) -> Via<'a> {
    if route.is_direct() {
        Via::Direct { device }
    } else {
        Via::Gateway
    }
}

/// A direct route's service, as the form names it: its address without
/// the path every service takes reports at.
fn service_of(url: &str) -> String {
    url.strip_suffix(baylee_client_core::bugreport::DIRECT_PATH)
        .unwrap_or(url)
        .to_string()
}

/// The local game's record: its box, ticked for this report alone, the
/// sentence saying what it shows, and the standing "never". With "never"
/// set, only that box, ticked, to take it back; without a record to offer,
/// nothing.
fn record_boxes(
    commands: &mut Commands,
    desk: &ReportDesk,
    settings: &ClientSettings,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
) -> Vec<Entity> {
    let never = settings.reports.record == RecordConsent::Never;
    if never {
        return vec![checkbox(
            commands,
            fonts,
            metrics,
            Phrase::ReportRecordNever.text(lang),
            DeskPress::RecordNever,
            true,
        )];
    }
    let Some(record) = desk
        .gathered
        .local_record
        .as_ref()
        .filter(|_| desk.gathered.offers_record(&settings.reports))
    else {
        return Vec::new();
    };
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
    let tick = checkbox(
        commands,
        fonts,
        metrics,
        Phrase::ReportRecordBox.text(lang),
        DeskPress::Record,
        desk.form.send_record,
    );
    let hint = words(
        commands,
        tf(fonts, metrics.small),
        Phrase::ReportRecordHint.fill(lang, &[&record.kilobytes().to_string()]),
        palette::MUTED,
    );
    commands.entity(entry).add_children(&[tick, hint]);
    let never = checkbox(
        commands,
        fonts,
        metrics,
        Phrase::ReportRecordNever.text(lang),
        DeskPress::RecordNever,
        false,
    );
    vec![entry, never]
}

/// The page before a report goes straight to the service or carries a
/// game's record: where it goes, and every part it carries, one line each.
fn confirmation(
    commands: &mut Commands,
    desk: &ReportDesk,
    settings: &ClientSettings,
    route: &Route,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
) {
    let (_, panel) = shade(commands, metrics, 620.0, 0.0);
    let mut lines = vec![words(
        commands,
        tf_bold(fonts, metrics.head),
        Phrase::ReportConfirmTitle.text(lang),
        palette::INK,
    )];
    let to = match route {
        Route::Gateway(gateway) => gateway.clone(),
        Route::Direct(url) => service_of(url),
        Route::Nowhere => String::new(),
    };
    lines.push(words(
        commands,
        tf_bold(fonts, metrics.text),
        Phrase::ReportConfirmTo.fill(lang, &[&to]),
        palette::INK,
    ));
    let device = settings.report_device.as_deref().unwrap_or_default();
    for part in desk
        .form
        .parts(&desk.gathered, &settings.reports, via(route, device))
    {
        let (said, colour) = match part {
            Part::Text(chars) => (
                Phrase::ReportConfirmText.fill(lang, &[&chars.to_string()]),
                palette::INK,
            ),
            Part::Category(category) => (
                Phrase::ReportConfirmPart.fill(lang, &[category.phrase().text(lang)]),
                palette::INK,
            ),
            Part::Record { kilobytes, .. } => (
                Phrase::ReportConfirmRecord.fill(lang, &[&kilobytes.to_string()]),
                palette::DANGER,
            ),
            Part::Device => (
                Phrase::ReportConfirmDevice.text(lang).to_string(),
                palette::MUTED,
            ),
        };
        lines.push(words(commands, tf(fonts, metrics.text), said, colour));
    }
    let actions = row(commands, metrics, true);
    let back = button(
        commands,
        fonts,
        metrics,
        Phrase::ReportConfirmBack.text(lang),
        DeskPress::ConfirmBack,
        false,
        true,
    );
    let send = button(
        commands,
        fonts,
        metrics,
        Phrase::ReportConfirmSend.text(lang),
        DeskPress::ConfirmSend,
        true,
        desk.form.can_send(route.sends()),
    );
    commands.entity(actions).add_children(&[back, send]);
    lines.push(actions);
    commands.entity(panel).add_children(&lines);
}

/// The text box: a paragraph that wraps inside it, a caret, and a scroll of
/// its own once the text is longer than the box.
///
/// The paragraph is three spans with the caret between two of them
/// ([`DeskText`], [`DeskCaret`]); the empty box shows the prompt as the text
/// after the caret, muted, the way a placeholder stands behind a caret.
///
/// Until #320 this was one text node holding the buffer with `▏` spliced in
/// at the caret: the glyph is in neither face this client ships, so no
/// caret was drawn, and the node would not shrink below its widest line, so
/// the text ran out of the box to the right instead of wrapping.
fn text_box(
    commands: &mut Commands,
    desk: &ReportDesk,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
) -> Entity {
    let font = tf(fonts, metrics.text);
    let line = line_height(&font);
    let field = commands
        .spawn((
            DeskBox,
            Node {
                width: percent(100),
                min_height: px(line * BOX_LINES_MIN + metrics.gap * 2.0),
                max_height: px(line * BOX_LINES_MAX + metrics.gap * 2.0),
                padding: UiRect::all(px(metrics.gap)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(6)),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                ..default()
            },
            // Where the box was scrolled to before this rebuild, so a
            // keystroke does not throw a long text back to its first line.
            ScrollPosition(Vec2::new(0.0, desk.box_scroll)),
            BorderColor::all(palette::ACCENT),
            BackgroundColor(palette::PANEL_LIT),
            Pickable::IGNORE,
        ))
        .id();
    let page = commands
        .spawn((
            Node {
                width: percent(100),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let buffer = &desk.form.text;
    let seg = buffer.segments();
    let (head, selected, tail, ink) = if buffer.is_empty() {
        (
            String::new(),
            String::new(),
            Phrase::ReportTextHint.text(lang).to_string(),
            palette::MUTED,
        )
    } else {
        // One space the player never sees ends the tail, so the text after
        // the caret is never empty and its first run is where the caret
        // stands, even at the very end after a line break
        // (`baylee_client_core::caretspot`).
        (
            seg.head.to_string(),
            seg.selected.to_string(),
            format!("{} ", seg.tail),
            palette::INK,
        )
    };
    let text = commands
        .spawn((
            DeskText,
            Text::new(""),
            font.clone(),
            TextColor(palette::INK),
            TextLayout::linebreak(bevy::text::LineBreak::WordOrCharacter),
            Node {
                width: percent(100),
                ..default()
            },
            Pickable::IGNORE,
            children![
                (TextSpan::new(head), font.clone(), TextColor(palette::INK)),
                (
                    TextSpan::new(selected),
                    font.clone(),
                    TextColor(palette::INK),
                    bevy::text::TextBackgroundColor(palette::SELECTION),
                ),
                (TextSpan::new(tail), font, TextColor(ink)),
            ],
        ))
        .id();
    let caret = commands
        .spawn((
            DeskCaret::default(),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: px(CARET_WIDTH),
                height: px(line),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(page).add_children(&[text, caret]);
    commands.entity(field).add_child(page);
    field
}

/// How tall a line of `font` is laid out, in logical pixels: bevy's default
/// line height is 1.2 times the size.
fn line_height(font: &TextFont) -> f32 {
    match font.font_size {
        bevy::text::FontSize::Px(size) => size * 1.2,
        _ => 20.0,
    }
}

/// Stands the caret where the paragraph was laid out, and scrolls the box so
/// the caret is in it.
///
/// After the text is laid out (`UiSystems::PostLayout`): the runs it reads
/// are this frame's, and what it writes is laid out on the next.
pub(super) fn place_the_caret(
    mut desk: ResMut<ReportDesk>,
    texts: Query<&bevy::text::TextLayoutInfo, With<DeskText>>,
    mut carets: Query<(&mut Node, &mut DeskCaret)>,
    mut boxes: Query<(&mut ScrollPosition, &ComputedNode), With<DeskBox>>,
) {
    let (Ok(layout), Ok((mut node, mut caret))) = (texts.single(), carets.single_mut()) else {
        return;
    };
    if layout.run_geometry.is_empty() && !desk.form.text.is_empty() {
        // Not laid out yet: no font, or the first frame of the tree.
        return;
    }
    let scale = layout.scale_factor.max(f32::EPSILON);
    let runs: Vec<baylee_client_core::caretspot::Run> = layout
        .run_geometry
        .iter()
        .map(|run| baylee_client_core::caretspot::Run {
            section: run.section_index,
            left: run.bounds.min.x / scale,
            top: run.bounds.min.y / scale,
            right: run.bounds.max.x / scale,
            bottom: run.bounds.max.y / scale,
        })
        .collect();
    let seg = desk.form.text.segments();
    let (before, after, seam): (&[usize], &[usize], _) = if seg.caret_after_selection {
        let before = format!("{}{}", seg.head, seg.selected);
        (
            &[HEAD, SELECTED],
            &[TAIL],
            baylee_client_core::caretspot::Seam::between(&before, seg.tail),
        )
    } else {
        let after = format!("{}{}", seg.selected, seg.tail);
        (
            &[HEAD],
            &[SELECTED, TAIL],
            baylee_client_core::caretspot::Seam::between(seg.head, &after),
        )
    };
    let line = match &node.height {
        Val::Px(height) => *height,
        _ => 20.0,
    };
    let spot = baylee_client_core::caretspot::spot(&runs, before, after, seam, line);
    node.left = px(spot.x - CARET_WIDTH / 2.0);
    node.top = px(spot.top);
    node.height = px(spot.height);
    caret.placed = true;
    if let Ok((mut scroll, computed)) = boxes.single_mut() {
        let inverse = computed.inverse_scale_factor();
        let inset = computed.padding().min_inset.y
            + computed.padding().max_inset.y
            + computed.border().min_inset.y
            + computed.border().max_inset.y;
        let viewport = (computed.size().y - inset) * inverse;
        if viewport <= 0.0 {
            // Not measured yet: nothing to keep the caret inside.
            return;
        }
        let followed = baylee_client_core::caretspot::follow(scroll.y, viewport, spot);
        if (followed - scroll.y).abs() > f32::EPSILON {
            scroll.y = followed;
        }
        desk.box_scroll = followed;
    }
}

/// What the caret last stood at: cursor, selection, text length. A change
/// in any of them is a caret that moved, lit again from the start.
type Drawn = (usize, Option<std::ops::Range<usize>>, usize);

/// Blinks the report's caret: lit while it moves, then on and off at the
/// rate every other text box here blinks at, and still under
/// `reduce_motion` (`lobby::caret_lit`).
pub(super) fn blink(
    time: Res<Time>,
    desk: Res<ReportDesk>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut carets: Query<(&DeskCaret, &mut BackgroundColor)>,
    mut since: Local<f32>,
    mut drawn: Local<Option<Drawn>>,
) {
    let Ok((caret, mut colour)) = carets.single_mut() else {
        *drawn = None;
        return;
    };
    let text = &desk.form.text;
    let now = Some((text.cursor(), text.selection(), text.text().len()));
    *since = if *drawn == now {
        *since + time.delta_secs()
    } else {
        0.0
    };
    *drawn = now;
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let want = if caret.placed && crate::lobby::caret_lit(*since, still) {
        palette::INK
    } else {
        Color::NONE
    };
    if colour.0 != want {
        colour.0 = want;
    }
}

/// The one-time question after a crash.
fn crash_question(commands: &mut Commands, fonts: &UiFonts, metrics: Metrics, lang: Lang) {
    let (_, panel) = shade(commands, metrics, 560.0, 0.0);
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
    holders: super::Holders,
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
            super::ask_to_send(desk, &holders, &mut settings, &answers);
        }
        DeskPress::ConfirmSend => {
            let desk = desk.as_mut();
            super::send(desk, &holders, &mut settings, &answers);
        }
        DeskPress::ConfirmBack => desk.form.confirming = false,
        DeskPress::Record => {
            desk.form.send_record = !desk.form.send_record;
            desk.form.edited();
        }
        DeskPress::RecordNever => {
            let consent = super::consent_mut(&mut settings);
            consent.record = if consent.record == RecordConsent::Never {
                RecordConsent::Ask
            } else {
                RecordConsent::Never
            };
            settings.save();
            desk.form.send_record = false;
            desk.form.edited();
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

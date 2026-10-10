//! The report sheet, drawn over whatever screen is up, and its keys (window
//! B, `windows-b6/DESIGN.md` §B.3).
//!
//! One retained tree under [`DeskRoot`], rebuilt whenever what it shows
//! changes (a keystroke, an answer, the suggestions), the way the lobby draws
//! its modals; a tick or a kind is redrawn in place ([`retick`]). The tree is
//! the kit's: a medium sheet (`shellkit::surfaces::sheet_box_with`) with the
//! kind as a segmented control, the text field, one small line under it, the
//! route, the Attachments disclosure and a sticky footer. Its focus order is
//! [`REPORT`] (and [`REPORT_CONFIRM`] on the confirmation page).
//!
//! The one kit sheet sized by the lobby's fixed step rather than the text
//! step (Q-B5): it stands over the table too, whose own interface follows no
//! text step, so its metrics are `ShellMetrics` at [`TextSize::L`] (the
//! step the lobby's `Metrics::of` values are) whatever the setting says.
//!
//! The scrim keeps `GlobalZIndex(1000)` over everything, the table's end
//! screen and the corner button included; a press on the form is a
//! [`DeskPress`] answered by `keys::pressed`, an observer on every entity, so
//! neither the lobby's click handler nor the table's ever sees one.

use std::hash::{Hash, Hasher};

use baylee_client_core::bugreport::refs::{CardRef, RefZone};
use baylee_client_core::bugreport::{
    Category, CrashConsent, Kind, MAX_TEXT_CHARS, Part, RecordConsent, Route, Status, Suggestion,
    Via,
};
use baylee_client_core::i18n::{Lang, Phrase};
use bevy::prelude::*;
use bevy::ui::FocusPolicy;

use super::ReportDesk;
use super::field::{chip_row, text_box};
use crate::hud::{UiFonts, icon_tf, tf, tf_bold};
use crate::lobby::LobbyState;
use crate::settings::ClientSettings;
use crate::shellkit::controls::{self, Kit, Live, Weight};
use crate::shellkit::focus::{Current, Stop, TabOrder};
use crate::shellkit::metrics::px_fixed;
use crate::shellkit::surfaces::{self, SheetWidth};
use crate::shellkit::tokens::{self, RADIUS_CONTROL};
use crate::shellkit::{Frame, InputClass, Platform, Role, ShellMetrics, TextSize, Viewport};

/// The sheet's focus order (`KEYBOARD.md` §7.11, "Report sheet"): the kind,
/// the text (focused first: the form's one purpose), the disclosure and its
/// rows while open, the footer. `copy` is drawn only where a report can go
/// nowhere.
pub(crate) const REPORT: TabOrder = TabOrder {
    name: "report",
    stops: &[
        "kind",
        "text",
        "attachments",
        "system",
        "game",
        "log",
        "settings",
        "screenshot",
        "crashes",
        "record",
        "never",
        "what-is-sent",
        "copy",
        "close",
        "send",
    ],
    modal: true,
};

/// The confirmation page's order: Back, then Send now.
pub(crate) const REPORT_CONFIRM: TabOrder = TabOrder {
    name: "report-confirm",
    stops: &["back", "send-now"],
    modal: true,
};

/// The form's root, over everything.
#[derive(Component)]
pub(crate) struct DeskRoot;

/// The sheet's scrolling body.
#[derive(Component)]
pub(crate) struct DeskScroll;

/// The nodes the tours anchor to (`TOURS.md` §3.1: `report_form`,
/// `report_attachments`, `report_record_row`, `report_route`), until the
/// tour's own `TourAnchor` exists to stand in their place.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ReportAnchor {
    /// The sheet.
    Form,
    /// The Attachments disclosure.
    Attachments,
    /// The local game's record row.
    RecordRow,
    /// The line saying where the report goes.
    Route,
}

impl ReportAnchor {
    /// The tour's id for it.
    #[cfg(any(test, all(feature = "dev-control", not(target_arch = "wasm32"))))]
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::Form => "report_form",
            Self::Attachments => "report_attachments",
            Self::RecordRow => "report_record_row",
            Self::Route => "report_route",
        }
    }
}

/// What a control on the form does.
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
    /// Open or close the attachments.
    Attachments,
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
    /// Take the suggestion in this row.
    Take(usize),
    /// Copy the text and what it would carry, where nothing can be sent.
    CopyText,
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

/// The [`REPORT`] stop of a box.
const fn category_stop(category: Category) -> &'static str {
    match category {
        Category::System => "system",
        Category::Game => "game",
        Category::Log => "log",
        Category::Settings => "settings",
        Category::Screenshot => "screenshot",
    }
}

/// The kit the sheet is drawn with: the window's size class at the fixed
/// step (Q-B5).
///
/// `ui` is the UI scale the sheet is drawn under (the table's, `hud::scale`),
/// taken back out of the big-screen step.
pub(super) fn kit_for(
    fonts: &UiFonts,
    window: Vec2,
    input: InputClass,
    lang: Lang,
    ui: f32,
) -> Kit<'_> {
    Kit {
        fonts,
        m: ShellMetrics::under(
            Viewport {
                width: window.x,
                height: window.y,
                platform: Platform::current(),
                input,
            },
            TextSize::L,
            ui,
        ),
        german: lang == Lang::De,
    }
}

/// The count beside the Attachments disclosure.
#[derive(Component)]
pub(crate) struct AttachedCount;

/// How many attachments are ticked of those this report has, as said.
fn attached_words(desk: &ReportDesk, settings: &ClientSettings, lang: Lang) -> String {
    let (ticked, there) = attached(desk, settings);
    Phrase::ReportAttachmentsCount.fill(lang, &[&ticked.to_string(), &there.to_string()])
}

/// How many attachments are ticked of those this report has.
fn attached(desk: &ReportDesk, settings: &ClientSettings) -> (usize, usize) {
    let there: Vec<Category> = Category::ALL
        .into_iter()
        .filter(|c| desk.gathered.has(*c))
        .collect();
    let ticked = there
        .iter()
        .filter(|c| settings.reports.allows(**c))
        .count();
    (ticked, there.len())
}

/// Everything the tree shows, folded into one number.
fn signature(
    desk: &ReportDesk,
    settings: &ClientSettings,
    route: &Route,
    window: Vec2,
    input: InputClass,
    ui: f32,
) -> u64 {
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
    // The kind and the ticks are redrawn in place by [`retick`] (§10 #9 of
    // the shell design) — except under an open preview, whose text they
    // change. The record's consent decides whether its box is drawn at all.
    if desk.form.preview {
        format!("{:?}{:?}", desk.form.kind, settings.reports).hash(&mut hash);
    }
    format!("{:?}", settings.reports.record).hash(&mut hash);
    desk.form.text.text().hash(&mut hash);
    desk.form.text.cursor().hash(&mut hash);
    desk.form.text.selection().hash(&mut hash);
    format!("{:?}", desk.form.status).hash(&mut hash);
    desk.form.preview.hash(&mut hash);
    // The suggestions: which reference is being typed, what it offers (the
    // pool arrives on the first `#`), and the row the keys are on.
    desk.form.dismissed.hash(&mut hash);
    desk.form.chosen.hash(&mut hash);
    desk.gathered.refs.cards.len().hash(&mut hash);
    desk.form.picked.len().hash(&mut hash);
    desk.copied.hash(&mut hash);
    // The disclosure (its count is redrawn in place by [`retick`]).
    settings.report_attachments_closed.hash(&mut hash);
    settings.lang.hash(&mut hash);
    (window.x as u32, window.y as u32).hash(&mut hash);
    ui.to_bits().hash(&mut hash);
    (input == InputClass::Touch).hash(&mut hash);
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
    input: Option<Res<InputClass>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    ui: Option<Res<UiScale>>,
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
    let window = windows.single().map_or(Vec2::new(1280.0, 800.0), |w| {
        Vec2::new(w.width(), w.height())
    });
    let input = input.as_deref().copied().unwrap_or(InputClass::Pointer);
    let now = signature(
        &desk,
        &settings,
        &route,
        window,
        input,
        ui.as_deref().map_or(1.0, |ui| ui.0),
    );
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
    desk.redraws += 1;
    desk.drawn_form = desk.open && !desk.form.confirming;
    let lang = Lang::of(&settings.lang);
    let kit = kit_for(
        &fonts,
        window,
        input,
        lang,
        ui.as_deref().map_or(1.0, |ui| ui.0),
    );
    let surface = if desk.asking && !desk.open {
        Some(crash_question(&mut commands, kit, lang))
    } else if desk.open && desk.form.confirming {
        Some(confirmation(
            &mut commands,
            &desk,
            &settings,
            &route,
            kit,
            lang,
        ))
    } else if desk.open {
        Some(form(&mut commands, &desk, &settings, &route, kit, lang))
    } else {
        None
    };
    if let Some(surface) = surface {
        scrim(&mut commands, kit, surface);
    }
}

/// The shade over everything, and the sheet on it: centred, or on a phone
/// the whole screen between the side insets and above the gesture inset.
fn scrim(commands: &mut Commands, kit: Kit, surface: Entity) -> Entity {
    let phone = kit.m.frame == Frame::Phone;
    let padding = if phone {
        UiRect {
            left: px_fixed(48.0),
            right: px_fixed(48.0),
            top: px_fixed(0.0),
            bottom: px_fixed(22.0),
        }
    } else {
        UiRect::all(px_fixed(kit.m.pad))
    };
    let shade = commands
        .spawn((
            DeskRoot,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: if phone {
                    AlignItems::Stretch
                } else {
                    AlignItems::Center
                },
                justify_content: JustifyContent::Center,
                padding,
                ..default()
            },
            BackgroundColor(Color::BLACK.with_alpha(0.8)),
            FocusPolicy::Block,
            GlobalZIndex(1000),
        ))
        .id();
    commands.entity(surface).insert(ReportAnchor::Form);
    commands.entity(shade).add_child(surface);
    shade
}

/// A line of text that wraps.
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

/// Redraws the kind and the boxes in place when one is pressed.
///
/// [`signature`] leaves the kind and the ticks out while the preview is
/// closed, so a toggle costs the one control it changed rather than the
/// whole form (§10 #9 of the shell design). After [`draw`], so a rebuild
/// this frame is already right and this writes nothing.
#[allow(clippy::type_complexity)] // Bevy queries: the controls and their inks
pub(super) fn retick(
    desk: Res<ReportDesk>,
    settings: Res<ClientSettings>,
    mut controls: Query<(&DeskPress, &Children, Option<&mut Current>)>,
    mut grounds: Query<(&mut BackgroundColor, Option<&Children>)>,
    mut marks: Query<(&mut Text, &mut TextColor), Without<AttachedCount>>,
    mut counts: Query<&mut Text, With<AttachedCount>>,
) {
    if !desk.open {
        return;
    }
    let said = attached_words(&desk, &settings, Lang::of(&settings.lang));
    for mut count in &mut counts {
        if count.0 != said {
            count.0.clone_from(&said);
        }
    }
    for (press, children, current) in &mut controls {
        let Some(&first) = children.first() else {
            continue;
        };
        match *press {
            // A segment: the face first, its words inside it; and which of
            // them holds the kind's place for the walker.
            DeskPress::Kind(kind) => {
                let on = desk.form.kind == kind;
                if let Some(mut current) = current
                    && current.0 != on
                {
                    current.0 = on;
                }
                let Ok((mut ground, inner)) = grounds.get_mut(first) else {
                    continue;
                };
                let want = if on { tokens::SELECTED } else { Color::NONE };
                if ground.0 != want {
                    ground.0 = want;
                }
                if let Some(&label) = inner.and_then(|c| c.first())
                    && let Ok((_, mut ink)) = marks.get_mut(label)
                {
                    let want = if on { tokens::INK } else { tokens::MUTED };
                    if ink.0 != want {
                        ink.0 = want;
                    }
                }
            }
            DeskPress::Toggle(_) | DeskPress::Crashes => {
                let lit = match *press {
                    DeskPress::Toggle(category) => settings.reports.allows(category),
                    _ => settings.reports.crashes == CrashConsent::Send,
                };
                // A box's first child is its mark.
                if let Ok((mut glyph, mut ink)) = marks.get_mut(first) {
                    let (want, colour) = tick_mark(lit);
                    if glyph.0.chars().ne(std::iter::once(want)) {
                        glyph.0 = want.to_string();
                    }
                    if ink.0 != colour {
                        ink.0 = colour;
                    }
                }
            }
            _ => {}
        }
    }
}

/// The glyph and ink of a box's mark, ticked or not.
fn tick_mark(ticked: bool) -> (char, Color) {
    if ticked {
        ('\u{f14a}', tokens::ACCENT)
    } else {
        ('\u{f0c8}', tokens::MUTED.with_alpha(0.5))
    }
}

/// A consent box: a row whose first child is its mark, then its label over
/// its hint, the whole row the target.
#[allow(clippy::too_many_arguments)] // a row's parts
fn checkbox(
    commands: &mut Commands,
    kit: Kit,
    label: &str,
    hint: Option<&str>,
    press: DeskPress,
    stop: &'static str,
    ticked: bool,
) -> Entity {
    let row = commands
        .spawn((
            Role::Hit,
            Node {
                min_height: px_fixed(kit.m.hit),
                width: percent(100),
                column_gap: px_fixed(kit.m.gap),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                padding: UiRect::axes(px_fixed(0.0), kit.m.px(4.0)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_CONTROL)),
                ..default()
            },
            press,
            Stop::new(REPORT.name, stop),
        ))
        .id();
    let (glyph, ink) = tick_mark(ticked);
    let mark = commands
        .spawn((
            Text::new(glyph.to_string()),
            icon_tf(kit.fonts, kit.m.text),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    let column = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_width: px_fixed(0.0),
                row_gap: kit.m.px(2.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let name = words(commands, tf(kit.fonts, kit.m.text), label, tokens::INK);
    commands.entity(column).add_child(name);
    if let Some(hint) = hint {
        let hint = words(commands, tf(kit.fonts, kit.m.small), hint, tokens::MUTED);
        commands.entity(column).add_child(hint);
    }
    commands.entity(row).add_children(&[mark, column]);
    row
}

/// The label a suggestion carries after its name: where the card is (and
/// whose, for a hand, the battlefield and a graveyard), or outside a game
/// its type line.
pub(super) fn meta_of(card: &CardRef, desk: &ReportDesk, lang: Lang) -> String {
    let Some(zone) = card.zone else {
        // The pool's type line is English; the player reads it in the
        // language the rest of the sheet is in.
        return card.kind.as_deref().map_or_else(String::new, |line| {
            baylee_client_core::deckbuilder::translated_type_line(line, lang)
        });
    };
    let (phrase, whose) = match zone {
        RefZone::Hand => (Phrase::ReportRefHand, true),
        RefZone::Battlefield => (Phrase::ReportRefBattlefield, true),
        RefZone::Stack => (Phrase::ReportRefStack, false),
        RefZone::Graveyard => (Phrase::ReportRefGraveyard, true),
        RefZone::Exile => (Phrase::ReportRefExile, false),
        RefZone::Command => (Phrase::ReportRefCommand, false),
        RefZone::Shown => (Phrase::ReportRefShown, false),
        RefZone::LibraryTop => (Phrase::ReportRefLibraryTop, false),
    };
    let zone = phrase.text(lang);
    let owner = card.owner.filter(|_| whose).map(|owner| {
        if desk.me == Some(owner) {
            Phrase::ReportRefYours.text(lang).to_string()
        } else {
            desk.seat_names
                .get(&owner.get())
                .cloned()
                .unwrap_or_else(|| format!("{}", owner.get() + 1))
        }
    });
    match owner {
        Some(owner) => format!("{zone} \u{b7} {owner}"),
        None => zone.to_string(),
    }
}

/// A suggestion's name and second column.
pub(super) fn suggestion_words(
    desk: &ReportDesk,
    suggestion: Suggestion,
    lang: Lang,
) -> (String, String) {
    match suggestion {
        Suggestion::Card(i) => {
            let card = &desk.gathered.refs.cards[i];
            (card.name.clone(), meta_of(card, desk, lang))
        }
        Suggestion::Player(i) => (desk.gathered.refs.players[i].name.clone(), String::new()),
    }
}

/// The form itself, in its sheet.
fn form(
    commands: &mut Commands,
    desk: &ReportDesk,
    settings: &ClientSettings,
    route: &Route,
    kit: Kit,
    lang: Lang,
) -> Entity {
    let mut body = Vec::new();
    let selected = Kind::OFFERED
        .iter()
        .position(|k| *k == desk.form.kind)
        .unwrap_or(0);
    let labels: Vec<&str> = Kind::OFFERED
        .iter()
        .map(|k| k.phrase().text(lang))
        .collect();
    body.push(controls::segmented(commands, kit, &labels, selected, |i| {
        (
            DeskPress::Kind(Kind::OFFERED[i]),
            Stop::item(REPORT.name, "kind", u8::try_from(i).unwrap_or(0)),
            Current(i == selected),
        )
    }));
    let suggestions = desk.form.suggestions(&desk.gathered.refs);
    body.push(text_box(commands, desk, kit, lang, suggestions.as_ref()));
    if kit.m.touch()
        && let Some(list) = &suggestions
    {
        body.push(chip_row(commands, desk, kit, lang, list));
    }
    let small = Phrase::ReportSmallLine.fill(
        lang,
        &[&desk.form.chars().to_string(), &MAX_TEXT_CHARS.to_string()],
    );
    body.push(words(
        commands,
        tf(kit.fonts, kit.m.small),
        small,
        if desk.form.over_limit() {
            tokens::DANGER
        } else {
            tokens::MUTED
        },
    ));
    let (said, ink) = match route {
        Route::Gateway(at) => (
            Phrase::ReportGoesGateway.fill(lang, &[&service_of(at)]),
            tokens::MUTED,
        ),
        Route::Direct(url) => (
            Phrase::ReportGoesDirect.fill(lang, &[&service_of(url)]),
            tokens::MUTED,
        ),
        Route::Nowhere => (
            Phrase::ReportNeedsSession.text(lang).to_string(),
            tokens::DANGER,
        ),
    };
    let route_line = words(commands, tf(kit.fonts, kit.m.small), said, ink);
    commands.entity(route_line).insert(ReportAnchor::Route);
    body.push(route_line);
    body.push(attachments(commands, desk, settings, kit, lang));
    body.extend(status_lines(commands, desk, kit, lang));
    if desk.form.preview {
        body.push(preview_pane(commands, desk, settings, route, kit, lang));
    }
    let footer = form_foot(commands, desk, route, kit, lang);
    surfaces::sheet_box_with(
        commands,
        kit,
        SheetWidth::Medium,
        Phrase::ReportButton.text(lang),
        &body,
        &footer,
        (
            DeskScroll,
            Role::Scroll,
            ScrollPosition(Vec2::new(0.0, desk.panel_scroll)),
        ),
    )
}

/// The Attachments disclosure (remembered per device): its head counts
/// what rides of what this report has, and its rows stand only while open.
fn attachments(
    commands: &mut Commands,
    desk: &ReportDesk,
    settings: &ClientSettings,
    kit: Kit,
    lang: Lang,
) -> Entity {
    let open = !settings.report_attachments_closed;
    let column = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(4.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
            ReportAnchor::Attachments,
        ))
        .id();
    let head = commands
        .spawn((
            Role::Row,
            Node {
                min_height: px_fixed(kit.m.hit),
                column_gap: kit.m.px(8.0),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
            DeskPress::Attachments,
            Stop::new(REPORT.name, "attachments"),
        ))
        .id();
    let chevron = commands
        .spawn((
            Text::new(if open { "\u{f078}" } else { "\u{f054}" }),
            icon_tf(kit.fonts, kit.m.small * 0.8),
            TextColor(tokens::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    let name = controls::label(
        commands,
        kit,
        &format!("{} \u{b7}", Phrase::ReportAttachments.text(lang)),
        kit.m.text,
        tokens::INK,
    );
    let count = controls::label(
        commands,
        kit,
        &attached_words(desk, settings, lang),
        kit.m.text,
        tokens::INK,
    );
    commands.entity(count).insert(AttachedCount);
    commands.entity(head).add_children(&[chevron, name, count]);
    commands.entity(column).add_child(head);
    if open {
        let rows = consent_rows(commands, desk, settings, kit, lang);
        commands.entity(column).add_children(&rows);
    }
    column
}

/// The disclosure's rows: each kind with its size or why there is none,
/// automatic crash reports, and the local game's record.
fn consent_rows(
    commands: &mut Commands,
    desk: &ReportDesk,
    settings: &ClientSettings,
    kit: Kit,
    lang: Lang,
) -> Vec<Entity> {
    let mut rows = Vec::new();
    for category in Category::ALL {
        let there = desk.gathered.has(category);
        let mut label = category.phrase().text(lang).to_string();
        if category == Category::Screenshot
            && let Some(shot) = &desk.gathered.screenshot
        {
            label.push_str("  \u{b7}  ");
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
        let row = checkbox(
            commands,
            kit,
            &label,
            Some(category.hint_where(super::shot::TAKES_PICTURES).text(lang)),
            DeskPress::Toggle(category),
            category_stop(category),
            settings.reports.allows(category),
        );
        rows.push(row);
    }
    rows.push(checkbox(
        commands,
        kit,
        Phrase::ReportCrashesBox.text(lang),
        None,
        DeskPress::Crashes,
        "crashes",
        settings.reports.crashes == CrashConsent::Send,
    ));
    rows.extend(record_boxes(commands, desk, settings, kit, lang));
    rows
}

/// How sending went, and what a send had to leave out.
fn status_lines(commands: &mut Commands, desk: &ReportDesk, kit: Kit, lang: Lang) -> Vec<Entity> {
    let mut lines = Vec::new();
    if let Some(said) = desk.form.status.text(lang) {
        let colour = match desk.form.status {
            Status::Sent(_) => crate::hud::palette::HEAL,
            Status::Sending | Status::Editing => tokens::MUTED,
            Status::Failed(_) | Status::Blocked(_) => tokens::DANGER,
        };
        lines.push(words(commands, tf(kit.fonts, kit.m.text), said, colour));
    }
    let gone = matches!(desk.form.status, Status::Sending | Status::Sent(_));
    if gone && (desk.form.trimmed.screenshot || desk.form.trimmed.log_lines.is_some()) {
        lines.push(words(
            commands,
            tf(kit.fonts, kit.m.small),
            Phrase::ReportTrimmed.text(lang),
            tokens::MUTED,
        ));
    }
    if gone && desk.form.trimmed.record {
        lines.push(words(
            commands,
            tf(kit.fonts, kit.m.small),
            Phrase::ReportRecordLeftOut.text(lang),
            tokens::MUTED,
        ));
    }
    if let Some(copied) = desk.copied {
        lines.push(words(
            commands,
            tf(kit.fonts, kit.m.small),
            if copied {
                Phrase::ExportCopied.text(lang)
            } else {
                Phrase::ReportFailed.text(lang)
            },
            tokens::MUTED,
        ));
    }
    lines
}

/// The footer: What is sent (a ghost, left), Copy as text where a report
/// can go nowhere, Close and Send.
fn form_foot(
    commands: &mut Commands,
    desk: &ReportDesk,
    route: &Route,
    kit: Kit,
    lang: Lang,
) -> Vec<Entity> {
    let what = controls::button(
        commands,
        kit,
        Phrase::ReportWhatIsSent.text(lang),
        Weight::Ghost,
        Live::Yes,
        None,
        (DeskPress::Preview, Stop::new(REPORT.name, "what-is-sent")),
    );
    // Left, in a stretch that takes the row's slack, the rest at the right.
    // (An auto margin on the button itself let a wrapping row run out of
    // the sheet.)
    let left = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                justify_content: JustifyContent::FlexStart,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(left).add_child(what);
    let mut footer = vec![left];
    if matches!(route, Route::Nowhere) {
        footer.push(controls::button(
            commands,
            kit,
            Phrase::CopyAsText.text(lang),
            Weight::Secondary,
            Live::Yes,
            None,
            (DeskPress::CopyText, Stop::new(REPORT.name, "copy")),
        ));
    }
    footer.push(controls::button(
        commands,
        kit,
        Phrase::ReportClose.text(lang),
        Weight::Secondary,
        Live::Yes,
        Some("Esc"),
        (DeskPress::Close, Stop::new(REPORT.name, "close")),
    ));
    let reason = match route {
        // The route line says the whole of it; under a finger this short
        // reason stands under the button.
        Route::Nowhere => Some(Phrase::ReportNowhere.text(lang).to_string()),
        _ if desk.form.status == Status::Sending => {
            Some(Phrase::ReportSending.text(lang).to_string())
        }
        _ if desk.form.over_limit() => {
            Some(Phrase::ReportTextTooLong.fill(lang, &[&MAX_TEXT_CHARS.to_string()]))
        }
        _ if desk.form.text.text().trim().is_empty() => {
            Some(Phrase::ReportTextHint.text(lang).to_string())
        }
        _ => None,
    };
    let send_keys = if crate::shellkit::keys::mac() {
        "\u{2318}\u{21b5}"
    } else {
        "Ctrl \u{21b5}"
    };
    footer.push(controls::button(
        commands,
        kit,
        Phrase::ReportSend.text(lang),
        Weight::Primary,
        reason.as_deref().map_or(Live::Yes, Live::No),
        Some(send_keys),
        (DeskPress::Send, Stop::new(REPORT.name, "send")),
    ));
    footer
}

/// Exactly what is sent, as the preview shows it: what always goes, in a
/// sentence, then the body itself.
fn preview_pane(
    commands: &mut Commands,
    desk: &ReportDesk,
    settings: &ClientSettings,
    route: &Route,
    kit: Kit,
    lang: Lang,
) -> Entity {
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
        shown.push_str("\n\u{2026} (+");
        shown.push_str(&(total - PREVIEW_CHARS).to_string());
        shown.push(')');
    }
    let pane = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(8.0),
                padding: UiRect::all(px_fixed(kit.m.gap)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_CONTROL)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::BLACK.with_alpha(0.45)),
            Pickable::IGNORE,
        ))
        .id();
    let always = words(
        commands,
        tf(kit.fonts, kit.m.small),
        if desk.gathered.game_id.is_some() {
            Phrase::ReportAlways.text(lang)
        } else {
            Phrase::ReportAlwaysLocal.text(lang)
        },
        tokens::MUTED,
    );
    let text = words(commands, tf(kit.fonts, kit.m.small), shown, tokens::INK);
    commands.entity(pane).add_children(&[always, text]);
    pane
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
    kit: Kit,
    lang: Lang,
) -> Vec<Entity> {
    let never = settings.reports.record == RecordConsent::Never;
    if never {
        return vec![checkbox(
            commands,
            kit,
            Phrase::ReportRecordNever.text(lang),
            None,
            DeskPress::RecordNever,
            "never",
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
    let tick = checkbox(
        commands,
        kit,
        Phrase::ReportRecordBox.text(lang),
        Some(&Phrase::ReportRecordHint.fill(lang, &[&record.kilobytes().to_string()])),
        DeskPress::Record,
        "record",
        desk.form.send_record,
    );
    commands.entity(tick).insert(ReportAnchor::RecordRow);
    let never = checkbox(
        commands,
        kit,
        Phrase::ReportRecordNever.text(lang),
        None,
        DeskPress::RecordNever,
        "never",
        false,
    );
    vec![tick, never]
}

/// The lines a confirmation, or a copy as text, lists: every part the
/// report carries.
pub(super) fn part_lines(
    desk: &ReportDesk,
    settings: &ClientSettings,
    route: &Route,
    lang: Lang,
) -> Vec<(String, Color)> {
    let device = settings.report_device.as_deref().unwrap_or_default();
    desk.form
        .parts(&desk.gathered, &settings.reports, via(route, device))
        .into_iter()
        .map(|part| match part {
            Part::Text(chars) => (
                Phrase::ReportConfirmText.fill(lang, &[&chars.to_string()]),
                tokens::INK,
            ),
            Part::Refs { cards, players } => {
                let counted = |n: usize, one: Phrase, many: Phrase| {
                    (if n == 1 { one } else { many }).fill(lang, &[&n.to_string()])
                };
                let mut said = Vec::new();
                if cards > 0 {
                    said.push(counted(
                        cards,
                        Phrase::ReportConfirmCardRef,
                        Phrase::ReportConfirmCardRefs,
                    ));
                }
                if players > 0 {
                    said.push(counted(
                        players,
                        Phrase::ReportConfirmPlayerName,
                        Phrase::ReportConfirmPlayerNames,
                    ));
                }
                (said.join(" \u{b7} "), tokens::INK)
            }
            Part::Category(category) => (
                Phrase::ReportConfirmPart.fill(lang, &[category.phrase().text(lang)]),
                tokens::INK,
            ),
            Part::Record { kilobytes, .. } => (
                Phrase::ReportConfirmRecord.fill(lang, &[&kilobytes.to_string()]),
                tokens::DANGER,
            ),
            Part::Device => (
                Phrase::ReportConfirmDevice.text(lang).to_string(),
                tokens::MUTED,
            ),
        })
        .collect()
}

/// The page before a report goes straight to the service or carries a
/// game's record, in the same sheet: where it goes, and every part it
/// carries, one line each.
fn confirmation(
    commands: &mut Commands,
    desk: &ReportDesk,
    settings: &ClientSettings,
    route: &Route,
    kit: Kit,
    lang: Lang,
) -> Entity {
    let to = match route {
        Route::Gateway(gateway) => gateway.clone(),
        Route::Direct(url) => service_of(url),
        Route::Nowhere => String::new(),
    };
    let mut lines = vec![words(
        commands,
        tf_bold(kit.fonts, kit.m.text),
        Phrase::ReportConfirmTo.fill(lang, &[&to]),
        tokens::INK,
    )];
    for (said, colour) in part_lines(desk, settings, route, lang) {
        lines.push(words(commands, tf(kit.fonts, kit.m.text), said, colour));
    }
    let back = controls::button(
        commands,
        kit,
        Phrase::ReportConfirmBack.text(lang),
        Weight::Secondary,
        Live::Yes,
        Some("Esc"),
        (
            DeskPress::ConfirmBack,
            Stop::new(REPORT_CONFIRM.name, "back"),
        ),
    );
    let send = controls::button(
        commands,
        kit,
        Phrase::ReportConfirmSend.text(lang),
        Weight::Primary,
        if desk.form.can_send(route.sends()) {
            Live::Yes
        } else {
            Live::No(Phrase::ReportNeedsSession.text(lang))
        },
        Some("\u{21b5}"),
        (
            DeskPress::ConfirmSend,
            Stop::new(REPORT_CONFIRM.name, "send-now"),
        ),
    );
    surfaces::sheet_box(
        commands,
        kit,
        SheetWidth::Medium,
        Phrase::ReportConfirmTitle.text(lang),
        &lines,
        &[back, send],
    )
}

/// The one-time question after a crash.
fn crash_question(commands: &mut Commands, kit: Kit, lang: Lang) -> Entity {
    let body = surfaces::prose(commands, kit, Phrase::CrashAskBody.text(lang), false);
    let never = controls::button(
        commands,
        kit,
        Phrase::CrashAskNever.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        DeskPress::CrashNever,
    );
    let send = controls::button(
        commands,
        kit,
        Phrase::CrashAskSend.text(lang),
        Weight::Primary,
        Live::Yes,
        None,
        DeskPress::CrashSend,
    );
    surfaces::sheet_box(
        commands,
        kit,
        SheetWidth::Small,
        Phrase::CrashAskTitle.text(lang),
        &[body],
        &[never, send],
    )
}

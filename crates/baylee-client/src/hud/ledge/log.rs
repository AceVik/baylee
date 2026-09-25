//! The game log (#262): the panel the tray's scroll button opens.
//!
//! The owner asked for a Logs button beside the zone button, a chat-like
//! panel with a scrollbar, and the whole log on the game-over screen. This is
//! the panel. [`super::tray`] draws the button, and `hud::finish` the log at
//! the end.
//!
//! # Where it stands
//!
//! Hard against the right margin, like the game menu, but its bottom is the
//! **strip's top** and not the strip's bottom. The menu may cover the zones
//! button because the next press dismisses it (the owner's gloss beside
//! [`Z_TRAY`]). This panel stays up while the game goes on under it, and a
//! standing panel over the two doors on the strip would bury the zones and
//! its own way back. So it stands on the strip and covers none of it.
//!
//! It is the shelf's rung ([`Z_LOG`]): a zone dialog answering a question
//! stands over it, and so does the menu.
//!
//! # What it redraws
//!
//! The book writes a line each time it is read, because the language and the
//! card text can change under it. The panel reads every line when it opens,
//! when the language changes and when card text arrives, and otherwise reads
//! only the lines that arrived since it last drew ([`LogRevision::drawn`]):
//! a line the host has sent never changes, so a new line is a row appended
//! and nothing above it is touched. A long game has thousands of lines, and
//! rebuilding them for each one would be the whole log written per action.
//!
//! # Following the newest line
//!
//! The list stays at its end while the player leaves it there, and stops
//! following the moment they scroll up ([`LogFollow`]). Lines that arrive
//! while it is scrolled up light a pill at the bottom that takes it back.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use baylee_client_core::gamelog::{CardTextLookup, LogLine, Wording};
use baylee_core::ids::CardIndex;
use bevy::picking::events::{Click, Pointer};
use bevy::ui::ScrollPosition;
use bevy::ui_widgets::{ControlOrientation, Scrollbar, ScrollbarThumb};

/// How wide the panel is, at most.
///
/// A column of sentences, and wide enough that a line naming two cards
/// mostly fits on one row. A narrow window gets less: see [`log_size`].
const LOG_W: f32 = 360.0;

/// How tall the panel is, at most.
const LOG_H: f32 = 420.0;

/// The air inside the panel's border, and round the list.
const LOG_PAD: f32 = 10.0;

/// The head's height: the zone dialog's title bar.
const LOG_HEAD_H: f32 = 24.0;

/// The cross in the head, square and a pixel inside the head each way.
const LOG_CLOSE: f32 = LOG_HEAD_H - 2.0;

/// The cross's glyph.
const LOG_CLOSE_PT: f32 = 10.0;

/// A line's size: the shelf's label size, which is what the rest of this
/// strip of the screen is read at.
const LOG_LINE_PT: f32 = LABEL_PT;

/// A turn's heading, which is quieter than a line and not smaller than the
/// shelf's captions.
const LOG_TURN_PT: f32 = CAP_PT;

/// Between two lines.
pub(in crate::hud) const LOG_ROW_GAP: f32 = 3.0;

/// Above a turn's heading, so a turn reads as a paragraph.
const LOG_TURN_GAP: f32 = 8.0;

/// The seat swatch at a line's left, and the gap after it.
///
/// Drawn empty today: it is filled from the seat a line is about once the
/// book says which seat that is (`LogLine::subject`, from the log's owner).
/// It already takes its width, so the lines do not move when it is filled.
const SWATCH_W: f32 = 3.0;

/// Between the swatch and the sentence.
const SWATCH_GAP: f32 = 6.0;

/// The scrollbar's track.
pub(in crate::hud) const TRACK_W: f32 = 6.0;

/// The shortest the thumb gets, so a long log's thumb can still be caught.
pub(in crate::hud) const THUMB_MIN: f32 = 20.0;

/// How near its end the list has to be to count as following.
///
/// A few logical pixels, not zero: a wheel stops wherever its last notch
/// put it, and a player who scrolled back down meant the bottom.
const FOLLOW_SLACK: f32 = 4.0;

/// The retained panel.
#[derive(Component)]
pub struct LogPanel;

/// The scrolling column of lines.
#[derive(Component)]
pub struct LogList;

/// The pill that says lines arrived under a list scrolled up.
#[derive(Component)]
pub struct LogNewBelow;

/// Whether the list is following its newest line.
#[derive(Component, Clone, Copy, Debug)]
pub struct LogFollow {
    /// At the end, and kept there as lines arrive.
    stuck: bool,
    /// Where this left the list last. A position that is not this one was
    /// moved by the player, with the wheel or the bar.
    set: f32,
    /// Lines arrived while the list was not following.
    unseen: bool,
}

impl Default for LogFollow {
    fn default() -> Self {
        Self {
            stuck: true,
            set: 0.0,
            unseen: false,
        }
    }
}

/// How far the panel's arrival or departure has run.
///
/// [`super::menu::MenuZoom`]'s twin, for the reason that one gives about the
/// pool's: two panels shown by two systems.
#[derive(Component)]
pub struct LogZoom {
    t: f32,
    closing: bool,
}

impl Default for LogZoom {
    fn default() -> Self {
        Self {
            t: 1.0,
            closing: true,
        }
    }
}

/// What the panel was last drawn from.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct LogRevision {
    /// The panel it was drawn into. The overlay can spawn a new one.
    panel: Option<Entity>,
    /// Whether it is open.
    open: bool,
    /// The language every line is written in.
    lang: Option<Lang>,
    /// The card text's generation: card names are the catalog's.
    texts: u64,
    /// The seat that is "you".
    seat: Option<PlayerId>,
    /// How often the book was rewritten, which moves lines that were drawn.
    rewrites: u32,
    /// How many of the book's lines are drawn.
    drawn: usize,
}

impl LogRevision {
    /// Whether what is drawn can be kept and only added to.
    fn extends(&self, next: &Self, len: usize) -> bool {
        self.panel == next.panel
            && self.open
            && next.open
            && self.lang == next.lang
            && self.texts == next.texts
            && self.seat == next.seat
            && self.rewrites == next.rewrites
            // An empty book drew its empty line, which the first line
            // replaces rather than follows.
            && self.drawn > 0
            && len >= self.drawn
    }
}

/// Where the panel stands.
///
/// On the strip's top edge rather than the strip's bottom: see the module
/// doc. Its own function so a test can read it without an app.
pub(in crate::hud) fn root_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        bottom: px(hand::HAND_ZONE_H - STRIP_LIP + STRIP_H),
        right: px(EDGE),
        width: px(LOG_W),
        height: px(LOG_H),
        flex_direction: FlexDirection::Column,
        border: UiRect::all(px(1)),
        border_radius: BorderRadius::top(px(STRIP_R)),
        ..default()
    }
}

/// How big the panel is in a window of this band.
///
/// [`LOG_W`] by [`LOG_H`] where they fit, and otherwise the room there is:
/// the window less its margins across, and the band above the strip up.
pub(in crate::hud) fn log_size(band: (f32, f32)) -> (f32, f32) {
    let width = LOG_W.min(band.0 - 2.0 * EDGE).max(0.0);
    let height = LOG_H.min(band.1 - (STRIP_H - STRIP_LIP)).max(0.0);
    (width, height)
}

/// Spawns the panel, once, beside the shelf.
///
/// Pickable, for the menu's reason: it floats over the table, and a press on
/// its padding must not reach a card under it.
pub(in crate::hud) fn spawn_log_panel(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            LogPanel,
            LogZoom::default(),
            root_node(),
            BackgroundColor(palette::DIALOG),
            BorderColor::all(palette::DIALOG_LINE),
            ZIndex(Z_LOG),
            Visibility::Hidden,
        ))
        .id()
}

/// Fills the panel, adds the lines that arrived, and sizes it to the window.
///
/// Nothing here shows or hides it: [`grow_the_log`] does both, at the ends
/// of the movement.
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
pub fn sync_log(
    mut commands: Commands,
    duel: Res<Duel>,
    fonts: Res<UiFonts>,
    settings: Res<crate::settings::ClientSettings>,
    texts: Option<Res<crate::cardtext::CardTexts>>,
    windows: Query<&Window>,
    mut revision: ResMut<LogRevision>,
    mut panel: Query<(Entity, Option<&Children>, &mut LogZoom, &mut Node), With<LogPanel>>,
    mut lists: Query<(Entity, &mut LogFollow), With<LogList>>,
) {
    let Ok((panel, standing, mut fold, mut node)) = panel.single_mut() else {
        return;
    };
    let (width, height) = log_size(band_of(&windows));
    if node.width != px(width) {
        node.width = px(width);
    }
    if node.height != px(height) {
        node.height = px(height);
    }
    let lang = Lang::of(&settings.lang);
    // A finished game closes it, for the menu's reason: `DuelSet::Input`
    // does not run in `Finished`, so its cross would answer nothing. The end
    // screen shows the whole log instead.
    let next = LogRevision {
        panel: Some(panel),
        open: duel.log_open && duel.ending().is_none(),
        lang: Some(lang),
        texts: texts.as_ref().map_or(0, |t| t.generation()),
        seat: duel.view.as_ref().map(|v| v.seat),
        rewrites: duel.log.rewrites(),
        drawn: revision.drawn,
    };
    if next.open == fold.closing {
        fold.closing = !next.open;
        fold.t = 0.0;
    }
    if !next.open {
        // Closing keeps what it shows, so the fold takes the lines with it.
        // The next opening writes them all again.
        revision.open = false;
        return;
    }
    let len = duel.log.len();
    let drawn = standing.is_some_and(|kids| !kids.is_empty());
    if revision.extends(&next, len) && drawn {
        if len == revision.drawn {
            return;
        }
        let Some((list, mut follow)) = lists.iter_mut().next() else {
            return;
        };
        let lookup = |card: CardIndex, face: u8| texts.as_ref().and_then(|t| t.face(card, face));
        let wording = wording(&duel, lang, &lookup);
        for line in duel.log.lines_since(revision.drawn, &wording) {
            let swatch = subject_ink(&wording, line.subject);
            let row = spawn_line(
                &mut commands,
                &fonts,
                lang,
                &line,
                &PANEL_INKS,
                Some(swatch),
            );
            commands.entity(list).add_child(row);
        }
        if !follow.stuck {
            follow.unseen = true;
        }
        revision.drawn = len;
        return;
    }

    // Everything again: the panel opened, or what every line is written
    // from changed under it.
    if let Some(kids) = standing {
        for kid in kids {
            commands.entity(*kid).despawn();
        }
    }
    let lookup = |card: CardIndex, face: u8| texts.as_ref().and_then(|t| t.face(card, face));
    let wording = wording(&duel, lang, &lookup);
    let lines = duel.log.lines(&wording);
    let parts = draw_all(&mut commands, &fonts, &wording, &lines);
    commands.entity(panel).add_children(&parts);
    *revision = LogRevision { drawn: len, ..next };
}

/// The whole panel's inside: the head, and the list with its scrollbar and
/// its pill, holding `lines` or, for none, the sentence that says so.
fn draw_all(
    commands: &mut Commands,
    fonts: &UiFonts,
    wording: &Wording,
    lines: &[LogLine],
) -> [Entity; 2] {
    let lang = wording.lang;
    let head = head(commands, fonts, lang);
    let body = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                flex_grow: 1.0,
                min_height: px(0),
                padding: UiRect::all(px(LOG_PAD)),
                column_gap: px(4),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let list = commands
        .spawn((
            LogList,
            LogFollow::default(),
            Scrolls,
            ScrollPosition::default(),
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                min_width: px(0),
                row_gap: px(LOG_ROW_GAP),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    if lines.is_empty() {
        let empty = commands
            .spawn((
                Text::new(Phrase::GameLogEmpty.text(lang)),
                tf(fonts, LOG_LINE_PT),
                TextColor(palette::DIALOG_SOFT),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(list).add_child(empty);
    }
    for line in lines {
        let swatch = subject_ink(wording, line.subject);
        let row = spawn_line(commands, fonts, lang, line, &PANEL_INKS, Some(swatch));
        commands.entity(list).add_child(row);
    }
    let track = scrollbar(
        commands,
        list,
        (
            palette::DOCK_EDGE.with_alpha(0.35),
            palette::CANDLE.with_alpha(0.8),
        ),
        at_rest,
    );
    let pill = new_lines_pill(commands, fonts, lang);
    commands.entity(body).add_children(&[list, track, pill]);
    [head, body]
}

/// Who the lines are written for.
pub(in crate::hud) fn wording<'a>(
    duel: &'a Duel,
    lang: Lang,
    texts: &'a dyn CardTextLookup,
) -> Wording<'a> {
    Wording {
        lang,
        seat: duel.view.as_ref().map_or(PlayerId::new(0), |v| v.seat),
        statics: duel.statics.as_ref(),
        texts,
    }
}

/// The swatch of a line about `subject`: the seat's own colour, the one its
/// bar wears ([`seat_colour`]), and none for a line about the table.
pub(in crate::hud) fn subject_ink(wording: &Wording, subject: Option<PlayerId>) -> Color {
    subject.map_or(Color::NONE, |player| {
        seat_colour(wording.seat, wording.statics, player)
    })
}

/// The head: the title, and the cross that closes it.
fn head(commands: &mut Commands, fonts: &UiFonts, lang: Lang) -> Entity {
    let title = commands
        .spawn((
            Text::new(Phrase::GameLogTitle.text(lang)),
            tf(fonts, LABEL_PT),
            TextColor(palette::DIALOG_INK),
            Pickable::IGNORE,
        ))
        .id();
    let close = commands
        .spawn((
            Button,
            MenuButton {
                action: MenuAction::ToggleLog,
            },
            Node {
                width: px(LOG_CLOSE),
                height: px(LOG_CLOSE),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(palette::DIALOG),
            BorderColor::all(palette::DIALOG_LINE),
            Feel::new(palette::DIALOG),
            children![(
                Text::new(glyph::CLOSE.to_string()),
                icon_tf(fonts, LOG_CLOSE_PT),
                TextColor(palette::DIALOG_SOFT),
                Pickable::IGNORE,
            )],
        ))
        .id();
    commands
        .spawn((
            Node {
                height: px(LOG_HEAD_H),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::new(px(LOG_PAD), px(1), px(0), px(0)),
                border: UiRect::bottom(px(1)),
                border_radius: BorderRadius::top(px(STRIP_R)),
                ..default()
            },
            BackgroundColor(palette::DIALOG_LIT),
            BorderColor::all(palette::DIALOG_LINE),
            Pickable::IGNORE,
        ))
        .add_children(&[title, close])
        .id()
}

/// The list's scrollbar: Bevy's own, so the thumb can be dragged. The
/// track and the thumb are `track` and `thumb`, put on by `paint`.
pub(in crate::hud) fn scrollbar(
    commands: &mut Commands,
    list: Entity,
    (track, thumb): (Color, Color),
    paint: fn(&mut EntityCommands, Paint),
) -> Entity {
    let mut grip = commands.spawn(ScrollbarThumb {
        border_radius: BorderRadius::all(px(TRACK_W / 2.0)),
        border: UiRect::ZERO,
    });
    paint(&mut grip, Paint::Fill(thumb));
    let grip = grip.id();
    let mut bar = commands.spawn((
        Node {
            width: px(TRACK_W),
            flex_shrink: 0.0,
            border_radius: BorderRadius::all(px(TRACK_W / 2.0)),
            ..default()
        },
        Scrollbar::new(list, ControlOrientation::Vertical, THUMB_MIN),
    ));
    paint(&mut bar, Paint::Fill(track));
    bar.add_child(grip).id()
}

/// The pill at the list's bottom, hidden until lines arrive under a list the
/// player has scrolled up. Pressing it takes the list back to its end.
fn new_lines_pill(commands: &mut Commands, fonts: &UiFonts, lang: Lang) -> Entity {
    commands
        .spawn((
            LogNewBelow,
            Button,
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                bottom: px(LOG_PAD),
                align_self: AlignSelf::Center,
                column_gap: px(6),
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(10), px(3)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(10)),
                ..default()
            },
            BackgroundColor(palette::DIALOG_LIT),
            BorderColor::all(palette::CANDLE),
            Feel::new(palette::DIALOG_LIT),
            children![
                (
                    Text::new(glyph::CARET_DOWN.to_string()),
                    icon_tf(fonts, LOG_TURN_PT),
                    TextColor(palette::CANDLE),
                    Pickable::IGNORE,
                ),
                (
                    Text::new(Phrase::GameLogNewBelow.text(lang)),
                    tf(fonts, LOG_TURN_PT),
                    TextColor(palette::DIALOG_INK),
                    Pickable::IGNORE,
                ),
            ],
        ))
        .observe(
            |mut click: On<Pointer<Click>>, mut lists: Query<&mut LogFollow, With<LogList>>| {
                click.propagate(false);
                for mut follow in &mut lists {
                    follow.stuck = true;
                }
            },
        )
        .id()
}

/// What a line is drawn in, and how its colours go on.
///
/// The panel's lines and the end sheet's are one builder in two inks. The
/// panel is dark and stands at once; the sheet is parchment and settles as
/// the veil rises, so its colours go on clear and are remembered
/// (`finish::Settling`). Everything else about a line, its pieces, its
/// weights and its count, is the same line in both.
pub(in crate::hud) struct LineInks {
    /// A line.
    pub line_pt: f32,
    /// A turn's heading.
    pub turn_pt: f32,
    /// The sentence and the names in it.
    pub ink: Color,
    /// A turn's heading, and "(×N)".
    pub soft: Color,
    /// The rule over a turn's heading.
    pub rule: Color,
    /// Puts one of those colours on a node.
    pub paint: fn(&mut EntityCommands, Paint),
}

/// One colour a line puts on one node.
#[derive(Clone, Copy, Debug)]
pub(in crate::hud) enum Paint {
    /// A text's or a span's.
    Ink(Color),
    /// A rule's.
    Rule(Color),
    /// A surface's: the scrollbar's track and thumb.
    Fill(Color),
}

/// The panel's inks.
pub(in crate::hud) const PANEL_INKS: LineInks = LineInks {
    line_pt: LOG_LINE_PT,
    turn_pt: LOG_TURN_PT,
    ink: palette::DIALOG_INK,
    soft: palette::DIALOG_SOFT,
    rule: palette::DIALOG_LINE,
    paint: at_rest,
};

/// A colour as it stands from the first frame.
fn at_rest(node: &mut EntityCommands, paint: Paint) {
    match paint {
        Paint::Ink(colour) => node.insert(TextColor(colour)),
        Paint::Rule(colour) => node.insert(BorderColor::all(colour)),
        Paint::Fill(colour) => node.insert(BackgroundColor(colour)),
    };
}

/// What a line's sentence tells the list about its height: its own box, and
/// nothing past it.
///
/// Bevy reports the sentence's content as the text set at its narrowest, one
/// word to a line, and taffy carries a child's content up into its parent's
/// scroll range wherever the child's overflow is visible. Measured on the
/// panel: a row 19 pixels high reported 75, a two-line row 37.5 reported 206,
/// and the list's range came to 776 against 620 of rows and gaps, so the
/// list scrolled past its last line into blank space. A clip on the text's
/// own node stops that at the row and clips nothing it draws: a node's
/// overflow clips its children, and a text's glyphs are its own.
///
/// A turn's heading carries none, because it reports its height truly,
/// wrapped or not (`the_list_ends_at_its_last_line`). The sentence grows into
/// its row from nothing (`min_width: 0`); the heading is set at its own
/// width.
const TEXT_OWN_HEIGHT: Overflow = Overflow::clip_y();

/// One line of the log, as a row.
///
/// A turn's heading is a quieter line under a rule. Any other line is the
/// seat swatch, when `swatch` gives it a colour ([`subject_ink`]), and the
/// sentence, with each name the sentence gives an object set one weight up,
/// and "(×N)" after a line that happened more than once.
pub(in crate::hud) fn spawn_line(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    line: &LogLine,
    inks: &LineInks,
    swatch: Option<Color>,
) -> Entity {
    if line.header {
        let mut heading = commands.spawn((
            Text::new(line.text.clone()),
            tf(fonts, inks.turn_pt),
            Pickable::IGNORE,
        ));
        (inks.paint)(&mut heading, Paint::Ink(inks.soft));
        let heading = heading.id();
        let mut rule = commands.spawn((
            Node {
                margin: UiRect::top(px(LOG_TURN_GAP)),
                padding: UiRect::top(px(3)),
                border: UiRect::top(px(1)),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ));
        (inks.paint)(&mut rule, Paint::Rule(inks.rule));
        return rule.add_child(heading).id();
    }
    let mut sentence = commands.spawn((
        Text::default(),
        tf(fonts, inks.line_pt),
        Node {
            flex_grow: 1.0,
            min_width: px(0),
            overflow: TEXT_OWN_HEIGHT,
            ..default()
        },
        Pickable::IGNORE,
    ));
    (inks.paint)(&mut sentence, Paint::Ink(inks.ink));
    let sentence = sentence.id();
    for (text, name) in pieces(line) {
        let font = if name {
            TextFont {
                font: bevy::text::FontSource::Handle(fonts.medium.clone()),
                ..tf(fonts, inks.line_pt)
            }
        } else {
            tf(fonts, inks.line_pt)
        };
        let mut span = commands.spawn((TextSpan::new(text), font));
        (inks.paint)(&mut span, Paint::Ink(inks.ink));
        let span = span.id();
        commands.entity(sentence).add_child(span);
    }
    if let Some(times) = times(line, lang) {
        let mut span = commands.spawn((TextSpan::new(times), tf(fonts, inks.line_pt)));
        (inks.paint)(&mut span, Paint::Ink(inks.soft));
        let span = span.id();
        commands.entity(sentence).add_child(span);
    }
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(SWATCH_GAP),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    if let Some(colour) = swatch {
        let swatch = commands
            .spawn((
                Node {
                    width: px(SWATCH_W),
                    flex_shrink: 0.0,
                    border_radius: BorderRadius::all(px(SWATCH_W / 2.0)),
                    ..default()
                },
                BackgroundColor(colour),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(row).add_child(swatch);
    }
    commands.entity(row).add_child(sentence);
    row
}

/// A line's sentence cut at its names: each piece, and whether it is a name.
///
/// A range the sentence cannot be cut at is read as no name at all rather
/// than as a panic: the sentence is still true, only set in one weight.
pub(in crate::hud) fn pieces(line: &LogLine) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let mut at = 0;
    for name in &line.names {
        let (Some(before), Some(named)) = (
            line.text.get(at..name.range.start),
            line.text.get(name.range.clone()),
        ) else {
            return vec![(line.text.clone(), false)];
        };
        if !before.is_empty() {
            out.push((before.to_string(), false));
        }
        out.push((named.to_string(), true));
        at = name.range.end;
    }
    match line.text.get(at..) {
        Some(rest) if !rest.is_empty() => out.push((rest.to_string(), false)),
        Some(_) => {}
        None => return vec![(line.text.clone(), false)],
    }
    out
}

/// What follows a line that happened more than once: the part of
/// [`Phrase::LogRepeated`] after the sentence, with the count in it.
///
/// Not [`LogLine::plain`], which returns one string: the names are drawn as
/// their own spans, so the count has to be one too.
pub(in crate::hud) fn times(line: &LogLine, lang: Lang) -> Option<String> {
    if line.times <= 1 {
        return None;
    }
    let phrase = Phrase::LogRepeated.text(lang);
    let after = phrase.split_once("{0}").map_or(phrase, |(_, after)| after);
    Some(after.replace("{1}", &line.times.to_string()))
}

/// Keeps the list at its end while it is following, lets it go when the
/// player scrolls up, and shows the pill when lines arrive under it.
///
/// Every frame, because the list's size is only known after layout: the rows
/// a frame appends are measured by the next one, and the list moves to its
/// new end then.
pub fn follow_the_log(
    mut lists: Query<(&mut ScrollPosition, &ComputedNode, &mut LogFollow), With<LogList>>,
    mut pills: Query<&mut Node, With<LogNewBelow>>,
) {
    for (mut at, computed, mut follow) in &mut lists {
        let scale = computed.inverse_scale_factor();
        let end = (computed.content_size().y - computed.size().y).max(0.0) * scale;
        // The player moved it, with the wheel or the bar: it follows again
        // only if they left it at the end.
        if (at.y - follow.set).abs() > f32::EPSILON {
            follow.stuck = at.y >= end - FOLLOW_SLACK;
            follow.set = at.y;
        }
        if follow.stuck {
            if (at.y - end).abs() > f32::EPSILON {
                at.y = end;
                follow.set = end;
            }
            if follow.unseen {
                follow.unseen = false;
            }
        }
        let display = if follow.unseen {
            Display::Flex
        } else {
            Display::None
        };
        for mut pill in &mut pills {
            if pill.display != display {
                pill.display = display;
            }
        }
    }
}

/// Opens the panel and shuts it: [`super::menu::grow_the_menu`] for this
/// panel, and the same three claims.
pub fn grow_the_log(
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut panels: Query<(&mut LogZoom, &mut UiTransform, &mut Visibility), With<LogPanel>>,
) {
    let still = prefs.all().reduce_motion;
    for (mut fold, mut transform, mut seen) in &mut panels {
        if fold.t >= 1.0 {
            continue;
        }
        let span = if fold.closing {
            motion::ZOOM_OUT
        } else {
            motion::ZOOM_IN
        };
        fold.t = motion::step(fold.t, span, time.delta_secs(), still);
        if fold.closing {
            if fold.t >= 1.0 {
                *seen = Visibility::Hidden;
                continue;
            }
        } else if *seen != Visibility::Inherited {
            *seen = Visibility::Inherited;
        }
        let scale = if fold.closing {
            motion::shutting(fold.t)
        } else {
            motion::opening(fold.t)
        };
        transform.scale = Vec2::splat(scale);
        transform.translation = motion::from_bottom_right(scale);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::gamelog::NameSpan;
    use baylee_core::ids::ObjectId;
    use baylee_view::{DayNight, GameStatic, LogEntry, LogEvent, LogTail, SeatIdentity};

    fn fonts() -> UiFonts {
        UiFonts {
            text: Handle::default(),
            medium: Handle::default(),
            bold: Handle::default(),
            italic: Handle::default(),
            medium_italic: Handle::default(),
            serif: Handle::default(),
            serif_italic: Handle::default(),
            icons: Handle::default(),
            mana: Handle::default(),
        }
    }

    /// The panel stands on the strip and covers none of it: the zones and
    /// its own door stay reachable while it is up, because it stays up.
    ///
    /// Read out of the two `root_node`s, as the menu's test reads its own:
    /// the claim is about where two nodes are drawn.
    #[test]
    fn the_panel_stands_on_the_strip_and_covers_none_of_it() {
        let strip = super::super::tray::root_node();
        let panel = root_node();
        let (Val::Px(strip_bottom), Val::Px(strip_h), Val::Px(bottom)) =
            (strip.bottom, strip.height, panel.bottom)
        else {
            panic!("the strip and the panel are placed in pixels");
        };
        assert!(
            (bottom - (strip_bottom + strip_h)).abs() < 0.01,
            "the panel's bottom is {bottom}, the strip's top {}",
            strip_bottom + strip_h
        );
        assert_eq!(panel.right, strip.right, "and against the same margin");
    }

    /// A wide window gets the panel's own size, and a narrow one the room
    /// it has: bounded both ways, so a panel that ignored the window and one
    /// that always shrank both fail.
    #[test]
    fn a_narrow_window_gets_the_room_there_is() {
        assert_eq!(log_size((1600.0, 1000.0)), (LOG_W, LOG_H));
        let (w, h) = log_size((300.0, 200.0));
        assert!(
            (w - (300.0 - 2.0 * EDGE)).abs() < 0.01,
            "a 300 wide window's panel is {w} wide"
        );
        assert!(
            (h - (200.0 - (STRIP_H - STRIP_LIP))).abs() < 0.01,
            "a 200 high band's panel is {h} high"
        );
    }

    fn line(text: &str, names: &[(usize, usize)], times: u32) -> LogLine {
        LogLine {
            index: 0,
            turn: 1,
            times,
            header: false,
            text: text.to_string(),
            names: names
                .iter()
                .map(|&(start, end)| NameSpan {
                    range: start..end,
                    id: ObjectId::new(5, 0),
                    card: None,
                })
                .collect(),
            ability: None,
            subject: None,
        }
    }

    /// Each name is a piece of its own, the words round it are kept, and a
    /// range the sentence cannot be cut at costs the weight and not the line.
    #[test]
    fn a_name_is_its_own_piece() {
        let bolt = line("You cast Lightning Bolt at Ana", &[(9, 23), (27, 30)], 1);
        assert_eq!(
            pieces(&bolt),
            [
                ("You cast ".to_string(), false),
                ("Lightning Bolt".to_string(), true),
                (" at ".to_string(), false),
                ("Ana".to_string(), true),
            ]
        );
        let first = line("Ana drew a card", &[(0, 3)], 1);
        assert_eq!(
            pieces(&first),
            [
                ("Ana".to_string(), true),
                (" drew a card".to_string(), false)
            ]
        );
        // Inside the `ö`, which is bytes 2 and 3: not a place to cut.
        let broken = line("Björn drew", &[(0, 3)], 1);
        assert_eq!(pieces(&broken), [("Björn drew".to_string(), false)]);
    }

    /// A line that happened more than once says how often, after the
    /// sentence, and a line that happened once says nothing.
    #[test]
    fn a_repeat_says_how_often() {
        assert_eq!(
            times(&line("You drew", &[], 3), Lang::En),
            Some(" (×3)".into())
        );
        assert_eq!(
            times(&line("Du ziehst", &[], 2), Lang::De),
            Some(" (×2)".into())
        );
        assert_eq!(times(&line("You drew", &[], 1), Lang::En), None);
    }

    fn tail(from: u32, count: u32) -> LogTail {
        LogTail {
            from,
            entries: (from..from + count)
                .map(|i| LogEntry {
                    turn: 1,
                    repeat: 1,
                    event: LogEvent::Mulliganed {
                        player: PlayerId::new(u8::try_from(i % 2).unwrap_or(0)),
                    },
                })
                .collect(),
        }
    }

    /// A panel over a book, with `sync_log` running.
    fn panel_over(lines: u32) -> (App, Entity) {
        panel_with(&tail(0, lines), None)
    }

    /// A panel over a book of `entries`, read by seat 0 with `statics` as
    /// its roster.
    fn panel_with(entries: &LogTail, statics: Option<GameStatic>) -> (App, Entity) {
        let mut app = App::new();
        let mut duel = Duel::default();
        let view = baylee_client_core::test_support::ViewBuilder::new(2).build();
        duel.log.append(entries, &view);
        duel.view = Some(view);
        duel.statics = statics;
        duel.log_open = true;
        app.insert_resource(duel)
            .insert_resource(fonts())
            .insert_resource(crate::settings::ClientSettings::default())
            .init_resource::<LogRevision>()
            .add_systems(Update, sync_log);
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let panel = {
            let mut commands = Commands::new(&mut queue, app.world());
            spawn_log_panel(&mut commands)
        };
        queue.apply(app.world_mut());
        app.update();
        (app, panel)
    }

    /// A book of `events` from line `from` on, all in turn 1.
    fn book(from: u32, events: Vec<LogEvent>) -> LogTail {
        LogTail {
            from,
            entries: events
                .into_iter()
                .map(|event| LogEntry {
                    turn: 1,
                    repeat: 1,
                    event,
                })
                .collect(),
        }
    }

    fn mulligan(seat: u8) -> LogEvent {
        LogEvent::Mulliganed {
            player: PlayerId::new(seat),
        }
    }

    /// A line about the table: nobody's.
    fn dawn() -> LogEvent {
        LogEvent::DayNight { now: DayNight::Day }
    }

    /// Two seats on two teams.
    fn two_teams() -> GameStatic {
        GameStatic {
            decision_secs: None,
            reconnect_secs: None,
            view_version: baylee_view::VIEW_VERSION,
            game_id: "g".into(),
            your_seat: PlayerId::new(0),
            seats: (0..2)
                .map(|seat| SeatIdentity {
                    player: PlayerId::new(seat),
                    display_name: format!("seat {seat}"),
                    is_ai: false,
                    away: false,
                    team: Some(seat),
                })
                .collect(),
            prints: vec![],
        }
    }

    /// Each row's swatch colour, in order.
    fn swatches(app: &mut App) -> Vec<Color> {
        let rows = rows(app);
        let world = app.world();
        rows.iter()
            .map(|row| {
                let swatch = world.get::<Children>(*row).expect("a row's parts")[0];
                world.get::<BackgroundColor>(swatch).expect("a swatch").0
            })
            .collect()
    }

    /// A line wears the colour of the seat it is about, the one that seat's
    /// bar wears: the reader's own, another seat's team's, and none for a line
    /// about the table. Both ways a line is drawn: with the panel as it
    /// opens, and arriving after it.
    ///
    /// The reader's team is not the reader's colour, so a swatch that asked
    /// only for teams would fail here too.
    #[test]
    fn a_line_wears_the_colour_of_the_seat_it_is_about() {
        let opened = book(0, vec![mulligan(0), mulligan(1), dawn()]);
        let (mut app, _) = panel_with(&opened, Some(two_teams()));
        let (own, theirs) = (palette::ACTIVE, team_color(Some(1)));
        assert_ne!(own, team_color(Some(0)), "the test cannot tell them apart");
        assert_eq!(
            swatches(&mut app),
            [own, theirs, Color::NONE],
            "the lines drawn as the panel opened"
        );

        {
            let mut duel = app.world_mut().resource_mut::<Duel>();
            let view = duel.view.clone().expect("a view");
            duel.log
                .append(&book(3, vec![dawn(), mulligan(1), mulligan(0)]), &view);
        }
        app.update();
        assert_eq!(
            swatches(&mut app)[3..],
            [Color::NONE, theirs, own],
            "the lines that arrived after it"
        );
    }

    fn rows(app: &mut App) -> Vec<Entity> {
        let mut lists = app.world_mut().query_filtered::<&Children, With<LogList>>();
        lists
            .iter(app.world())
            .flat_map(|kids| kids.iter().collect::<Vec<_>>())
            .collect()
    }

    /// Lines that arrive are added under the ones drawn, and nothing drawn
    /// is written again; a change of language writes everything again.
    ///
    /// The row entities are the measurement: a panel that redrew the whole
    /// log for each new line would show the same rows and hold new entities.
    #[test]
    fn new_lines_are_added_and_nothing_drawn_is_drawn_again() {
        let (mut app, _) = panel_over(3);
        let first = rows(&mut app);
        assert_eq!(first.len(), 3, "three lines, three rows");

        {
            let mut duel = app.world_mut().resource_mut::<Duel>();
            let view = duel.view.clone().expect("a view");
            duel.log.append(&tail(3, 2), &view);
        }
        app.update();
        let second = rows(&mut app);
        assert_eq!(second.len(), 5, "the two new lines were not added");
        assert_eq!(
            second[..3],
            first[..],
            "the three drawn rows were drawn again for two new lines"
        );

        app.world_mut()
            .resource_mut::<crate::settings::ClientSettings>()
            .lang = "de".to_string();
        app.update();
        let third = rows(&mut app);
        assert_eq!(third.len(), 5);
        assert!(
            !third.contains(&first[0]),
            "a new language left the lines written in the old one"
        );
    }

    /// An empty book says so, and its first line takes that sentence's place
    /// rather than standing under it.
    #[test]
    fn an_empty_log_says_so_until_its_first_line() {
        let (mut app, _) = panel_over(0);
        assert_eq!(rows(&mut app).len(), 1, "the empty log's one sentence");
        {
            let mut duel = app.world_mut().resource_mut::<Duel>();
            let view = duel.view.clone().expect("a view");
            duel.log.append(&tail(0, 2), &view);
        }
        app.update();
        assert_eq!(
            rows(&mut app).len(),
            2,
            "the empty log's sentence stayed under the lines that ended it"
        );
    }

    /// The list's scroll range ends at its last line, measured by Bevy's own
    /// layout with the shipped fonts: lines that wrap, lines that do not, a
    /// turn's heading, in a list narrower than most of them.
    ///
    /// Bounded both ways, so a range that stops short of the last line fails
    /// as well as one that runs past it into blank space.
    #[test]
    fn the_list_ends_at_its_last_line() {
        let (mut app, fonts) = crate::face::tests::layout_app();
        // A turn's heading last, as a turn that has only just begun leaves
        // it: the one row whose own reach nothing under it covers. Its seat
        // has a long name, so that it wraps and its wrapped height is the one
        // held to account.
        let said = [
            "Lightning Bolt deals 3 damage to Ana",
            "You draw a card",
            "steady 1 puts an ability of Charming Prince on the stack",
            "You are at 37 life (was 40)",
            "Esper Sentinel moves from the battlefield into exile, and then from exile back onto the battlefield under its owner's control",
            "Turn 14 · Maximiliane von Hohenzollern-Sigmaringen's turn",
        ];
        let list = {
            let mut commands = app.world_mut().commands();
            let list = commands
                .spawn(Node {
                    width: px(260),
                    height: px(120),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(LOG_ROW_GAP),
                    overflow: Overflow::scroll_y(),
                    ..default()
                })
                .id();
            for (at, text) in said.iter().enumerate() {
                let mut line = line(text, &[], 1);
                line.header = at == said.len() - 1;
                let row = spawn_line(
                    &mut commands,
                    &fonts,
                    Lang::En,
                    &line,
                    &PANEL_INKS,
                    Some(Color::NONE),
                );
                commands.entity(list).add_child(row);
            }
            list
        };
        app.world_mut().flush();
        app.update();
        app.update();
        let world = app.world();
        let kids: Vec<Entity> = world.get::<Children>(list).expect("rows").iter().collect();
        let rows: f32 = kids
            .iter()
            .map(|row| world.get::<ComputedNode>(*row).expect("laid out").size().y)
            .sum();
        // The heading's margin is part of the column too.
        let spaced = rows + LOG_ROW_GAP * (kids.len() - 1) as f32 + LOG_TURN_GAP;
        let range = world
            .get::<ComputedNode>(list)
            .expect("laid out")
            .content_size()
            .y;
        assert!(
            rows > 6.0 * LOG_LINE_PT,
            "the rows were never laid out, so the range proves nothing: {rows}"
        );
        assert!(
            (range - spaced).abs() < 1.0,
            "the list scrolls to {range}, and its lines end at {spaced}"
        );
    }

    /// A list and its pill, laid out as `size` inside `content`.
    fn laid_out(app: &mut App, content: f32) -> (Entity, Entity) {
        let list = app
            .world_mut()
            .spawn((
                LogList,
                LogFollow::default(),
                ScrollPosition::default(),
                ComputedNode {
                    size: Vec2::new(100.0, 200.0),
                    content_size: Vec2::new(100.0, content),
                    inverse_scale_factor: 1.0,
                    ..default()
                },
            ))
            .id();
        let pill = app
            .world_mut()
            .spawn((
                LogNewBelow,
                Node {
                    display: Display::None,
                    ..default()
                },
            ))
            .id();
        (list, pill)
    }

    /// The list keeps to its end while the player leaves it there, lets go
    /// when they scroll up, lights the pill when lines arrive under it, and
    /// goes back to the end when the pill is pressed.
    #[test]
    fn the_list_follows_its_newest_line_until_the_player_scrolls_up() {
        let mut app = App::new();
        app.add_systems(Update, follow_the_log);
        let (list, pill) = laid_out(&mut app, 1000.0);
        let at = |app: &App| app.world().get::<ScrollPosition>(list).expect("a list").y;
        let shown =
            |app: &App| app.world().get::<Node>(pill).expect("a pill").display != Display::None;

        app.update();
        assert!((at(&app) - 800.0).abs() < 0.01, "it did not go to its end");

        // The wheel.
        app.world_mut()
            .get_mut::<ScrollPosition>(list)
            .expect("a list")
            .y = 300.0;
        app.update();
        assert!(
            (at(&app) - 300.0).abs() < 0.01,
            "it took the list back from the player"
        );

        // Lines arrive: the sync marks them unseen, and the list grows.
        app.world_mut()
            .get_mut::<LogFollow>(list)
            .expect("a list")
            .unseen = true;
        app.world_mut()
            .get_mut::<ComputedNode>(list)
            .expect("a list")
            .content_size
            .y = 1200.0;
        app.update();
        assert!(
            (at(&app) - 300.0).abs() < 0.01,
            "new lines took the list away"
        );
        assert!(shown(&app), "lines arrived under it and nothing said so");

        // The pill.
        app.world_mut()
            .get_mut::<LogFollow>(list)
            .expect("a list")
            .stuck = true;
        app.update();
        assert!(
            (at(&app) - 1000.0).abs() < 0.01,
            "the pill did not take it to its end"
        );
        assert!(!shown(&app), "the pill stayed after the lines were seen");

        // And a wheel back to the end follows again: to within the slack
        // of it, where a wheel's last notch leaves it.
        app.world_mut()
            .get_mut::<ScrollPosition>(list)
            .expect("a list")
            .y = 500.0;
        app.update();
        app.world_mut()
            .get_mut::<ScrollPosition>(list)
            .expect("a list")
            .y = 998.0;
        app.update();
        app.world_mut()
            .get_mut::<ComputedNode>(list)
            .expect("a list")
            .content_size
            .y = 1400.0;
        app.update();
        assert!(
            (at(&app) - 1200.0).abs() < 0.01,
            "scrolled back to the end, it did not follow the next line"
        );
    }
}

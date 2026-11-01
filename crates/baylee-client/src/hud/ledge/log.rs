//! The game log (#262): the panel the scroll button in the bar opens.
//!
//! The owner asked for a Logs button beside the zone button, a chat-like
//! panel with a scrollbar, and the whole log on the game-over screen. This is
//! the panel. [`super::tray`] draws the button, and `hud::finish` the log at
//! the end.
//!
//! # Where it stands
//!
//! Hard against the right margin, like the game menu, but its bottom is the
//! **right strip's top** and not the strip's bottom. The strip was the tray
//! when this was written: the menu may cover the zones button because the
//! next press dismisses it (the owner's gloss beside [`Z_TRAY`]), but this
//! panel stays up while the game goes on under it, and a standing panel over
//! the two doors would have buried the zones and its own way back. Since #264
//! the doors are in the bar and the strip is the mana pool's, and the rule
//! holds for the pool on its own terms: the mana a player is holding is read
//! while they spend it, and a log left open must not stand over it.
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
//! # Names, links and times (#300)
//!
//! A line reads like a chat: its time on the device's own clock in a column
//! of its own ([`local_clock`]), every player it names in bold, "you"
//! included, and every card or token it shows the seat as a link in the
//! manner of a game's item links, "[Lightning Bolt]" in the link's ink.
//! Hovering a link opens the table's own preview of that card, drawn from the
//! printing the line named ([`LogLink`]) and never from the object as it is
//! now: a card shuffled away since is still the card the line showed, and a
//! card the line did not show has no link to hover. A hidden card names no
//! span at all, and a face-down one a span with nothing to preview, which is
//! set a weight up and not linked ([`Piece::Name`]).
//!
//! # Turns fold (feedback of 24.10.2026)
//!
//! Every turn but the newest stands as its heading alone, a chevron and how
//! many lines it holds; a press on a heading opens or folds it
//! ([`LogFolds`], [`spawn_turn`]). The player's choice holds for that turn
//! for the rest of the game, in the panel, the chat window's Log tab and the
//! end screen alike; a new turn folds the one before unless it was opened.
//!
//! # Following the newest line
//!
//! The list stays at its end while the player leaves it there, and stops
//! following the moment they scroll up ([`LogFollow`]). Lines that arrive
//! while it is scrolled up light a pill at the bottom that takes it back.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use crate::ui_button::UiButton;
use baylee_client_core::card_face::CardText;
use baylee_client_core::gamelog::{CardTextLookup, LogLine, NameSpan, Wording};
use baylee_client_core::images::{ArtSize, ImageKey};
use baylee_core::ids::CardIndex;
use baylee_view::{CardIdentity, Phase, RulesFace};
use bevy::picking::events::{PointerClick, PointerOut, PointerOver};
use bevy::ui::ScrollPosition;
use bevy::ui_widgets::{ControlOrientation, Scrollbar, ScrollbarThumb};
use std::collections::BTreeMap;

pub(crate) mod virtual_list;

/// How wide the panel is at least, where the window has the room.
///
/// A column of sentences, and wide enough that a line naming two cards
/// mostly fits on one row. A narrow window gets less: see [`log_size`].
pub(crate) const LOG_W: f32 = 360.0;

/// How wide the panel grows at most (the owner, 12.10.2026: "etwas
/// breiter"): a line naming a spell, its mode and two targets reads on two
/// rows rather than five.
pub(crate) const LOG_W_MAX: f32 = 540.0;

/// The share of the window's width the panel takes between the two.
///
/// About a quarter, so at 1920 × 1080 (and at every larger window, which
/// the HUD's `UiScale` draws as that one, `hud::scale`) the panel is near
/// its widest and three quarters of the table stay in sight beside it.
const LOG_W_SHARE: f32 = 0.27;

/// How tall the panel is at least, where the window has the room.
pub(super) const LOG_H: f32 = 420.0;

/// How tall it grows at most.
const LOG_H_MAX: f32 = 640.0;

/// The share of the band above the hand zone the panel takes between the
/// two: the top of the right side stays the stack's and the report
/// corner's.
const LOG_H_SHARE: f32 = 0.72;

/// The air inside the panel's border, and round the list.
pub(crate) const LOG_PAD: f32 = 10.0;

/// The head's height: the zone dialog's title bar.
pub(crate) const LOG_HEAD_H: f32 = 24.0;

/// The cross in the head, square and a pixel inside the head each way.
const LOG_CLOSE: f32 = LOG_HEAD_H - 2.0;

/// The cross's glyph.
const LOG_CLOSE_PT: f32 = 10.0;

/// A line's size: the shelf's label size, which is what the rest of this
/// strip of the screen is read at.
pub(crate) const LOG_LINE_PT: f32 = LABEL_PT;

/// A turn's heading, which is quieter than a line and not smaller than the
/// shelf's captions.
pub(crate) const LOG_TURN_PT: f32 = CAP_PT;

/// Between two lines.
pub(crate) const LOG_ROW_GAP: f32 = 3.0;

/// Above a turn's heading, so a turn reads as a paragraph.
const LOG_TURN_GAP: f32 = 8.0;

/// The seat swatch at a line's left, and the gap after it.
///
/// Drawn empty today: it is filled from the seat a line is about once the
/// book says which seat that is (`LogLine::subject`, from the log's owner).
/// It already takes its width, so the lines do not move when it is filled.
pub(crate) const SWATCH_W: f32 = 3.0;

/// Between the swatch and the sentence.
pub(crate) const SWATCH_GAP: f32 = 6.0;

/// The scrollbar's track.
pub(crate) const TRACK_W: f32 = 6.0;

/// The shortest the thumb gets, so a long log's thumb can still be caught.
pub(crate) const THUMB_MIN: f32 = 20.0;

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
    pub(crate) stuck: bool,
    /// Where this left the list last. A position that is not this one was
    /// moved by the player, with the wheel or the bar.
    pub(crate) set: f32,
    /// Lines arrived while the list was not following.
    pub(crate) unseen: bool,
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
#[derive(Resource, Default, Debug)]
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
    /// Where the last drawn line left the phase headings.
    run: PhaseRun,
    /// The bounded window of mounted rows.
    rows: virtual_list::VirtualLog,
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
            // replaces rather than follows. Two empty frames still extend.
            && (self.drawn > 0 || len == 0)
            && len >= self.drawn
    }
}

/// Where the panel stands.
///
/// On the strip's top edge rather than the strip's bottom: see the module
/// doc. Its own function so a test can read it without an app.
pub(crate) fn root_node() -> Node {
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

/// The band the panel grows up into in a window this big: the whole width,
/// and from the hand zone up to [`TOP_CLEAR`], the report button's corner
/// (#309), which it stands under.
///
/// Not the zone dialog's [`band_of`]: that one is floored at the dialog's
/// least height, and a short window (a phone on its side) would grow the
/// panel past the top of the window and under the button, which is over
/// everything.
pub(crate) fn log_band(window: Vec2) -> (f32, f32) {
    (window.x, window.y - TOP_CLEAR - hand::HAND_ZONE_H)
}

/// How big the panel is in a window of this band.
///
/// A share of the band ([`LOG_W_SHARE`], [`LOG_H_SHARE`]) held between
/// [`LOG_W`] by [`LOG_H`] and [`LOG_W_MAX`] by [`LOG_H_MAX`], and never more
/// than the room there is: the window less its margins across, and the band
/// above the strip up.
pub(crate) fn log_size(band: (f32, f32)) -> (f32, f32) {
    let width = (band.0 * LOG_W_SHARE)
        .clamp(LOG_W, LOG_W_MAX)
        .min(band.0 - 2.0 * EDGE)
        .max(0.0);
    let height = (band.1 * LOG_H_SHARE)
        .clamp(LOG_H, LOG_H_MAX)
        .min(band.1 - (STRIP_H - STRIP_LIP))
        .max(0.0);
    (width, height)
}

/// Spawns the panel, once, beside the shelf.
///
/// Pickable, for the menu's reason: it floats over the table, and a press on
/// its padding must not reach a card under it.
pub(crate) fn spawn_log_panel(commands: &mut Commands) -> Entity {
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
#[allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::too_many_lines
)]
// a Bevy system: every one is an injection
pub fn sync_log(
    mut commands: Commands,
    duel: Res<Duel>,
    fonts: Res<UiFonts>,
    settings: Res<crate::settings::ClientSettings>,
    texts: Option<Res<crate::cardtext::CardTexts>>,
    windows: Query<&Window>,
    ui: Option<Res<UiScale>>,
    mut revision: ResMut<LogRevision>,
    mut panel: Query<
        (Entity, Option<&Children>, &mut LogZoom, &mut Node),
        (With<LogPanel>, Without<virtual_list::LogSpacer>),
    >,
    mut lists: Query<(Entity, &mut LogFollow, &mut ScrollPosition), With<LogList>>,
    nodes: Query<&ComputedNode>,
    mut spacers: Query<&mut Node, With<virtual_list::LogSpacer>>,
    chat: Option<Res<crate::social::SocialHost>>,
    folds: Option<Res<LogFolds>>,
) {
    let Ok((panel, standing, mut fold, mut node)) = panel.single_mut() else {
        return;
    };
    let window = crate::hud::scale::window_space(&windows, ui.as_deref())
        .unwrap_or(Vec2::new(1280.0, 720.0));
    let (width, height) = log_size(log_band(window));
    if node.width != px(width) {
        node.width = px(width);
    }
    if node.height != px(height) {
        node.height = px(height);
    }
    let lang = Lang::of(&settings.lang);
    // With a social session the panel is the chat window, the log one of
    // its tabs (the beta.8 chat window design §1): `social::window` fills
    // it. What is drawn here is forgotten, so the log comes back whole if
    // the session ends mid-game.
    if chat.as_ref().is_some_and(|c| c.active()) {
        let open = duel.log_open && duel.ending().is_none();
        if open == fold.closing {
            fold.closing = !open;
            fold.t = 0.0;
        }
        revision.panel = None;
        revision.rows.reset();
        return;
    }
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
        run: revision.run,
        rows: virtual_list::VirtualLog::default(),
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
        let Some((list, mut follow, mut scroll)) = lists.iter_mut().next() else {
            return;
        };
        let lookup = LogTexts(texts.as_deref());
        let wording = wording(&duel, lang, &lookup);
        if len > revision.drawn {
            let from = revision.drawn;
            let mut run = revision.run;
            let lines = duel.log.lines_since(from, &wording);
            revision
                .rows
                .append(virtual_list::append_entries(&lines, from, &mut run));
            if !follow.stuck {
                follow.unseen = true;
            }
            revision.drawn = len;
            revision.run = run;
        }
        if let Some(folds) = &folds {
            revision.rows.fold_with(folds, scroll.y);
        }
        if len > 0 {
            revision.rows.sync(
                &mut commands,
                list,
                &fonts,
                lang,
                Some(&wording),
                &PANEL_INKS,
                &mut follow,
                &mut scroll,
                &nodes,
                &mut spacers,
                &mut |_, _, _, _| {},
            );
        }
        return;
    }

    // Everything again: the panel opened, or what every line is written
    // from changed under it. The root removal owns the old row entities.
    revision.rows.reset();
    if let Some(kids) = standing {
        for kid in kids {
            commands.entity(*kid).despawn();
        }
    }
    let lookup = LogTexts(texts.as_deref());
    let wording = wording(&duel, lang, &lookup);
    let lines = duel.log.lines(&wording);
    let (entries, run) = virtual_list::entries(&lines);
    let (parts, list) = draw_all(&mut commands, &fonts, &wording, &lines);
    commands.entity(panel).add_children(&parts);
    let mut rows = std::mem::take(&mut revision.rows);
    rows.replace(entries);
    if let Some(folds) = &folds {
        rows.fold_with(folds, 0.0);
    }
    if len > 0 {
        let mut follow = LogFollow::default();
        let mut scroll = ScrollPosition::default();
        rows.sync(
            &mut commands,
            list,
            &fonts,
            lang,
            Some(&wording),
            &PANEL_INKS,
            &mut follow,
            &mut scroll,
            &nodes,
            &mut spacers,
            &mut |_, _, _, _| {},
        );
        commands.entity(list).insert((follow, scroll));
    }
    *revision = LogRevision {
        drawn: len,
        run,
        ..next
    };
    revision.rows = rows;
}

/// The whole panel's inside: the head, and the list with its scrollbar and
/// its pill, holding `lines` or, for none, the sentence that says so.
fn draw_all(
    commands: &mut Commands,
    fonts: &UiFonts,
    wording: &Wording,
    lines: &[LogLine],
) -> ([Entity; 2], Entity) {
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
    ([head, body], list)
}

/// The catalog's text as the log reads it: card names, and the words of a
/// modal card's modes, through the cast chooser's own door
/// ([`crate::choices::mode_words`]).
pub(crate) struct LogTexts<'a>(pub Option<&'a crate::cardtext::CardTexts>);

impl CardTextLookup for LogTexts<'_> {
    fn text(&self, card: CardIndex, face: u8) -> Option<CardText> {
        self.0.and_then(|texts| texts.face(card, face))
    }

    fn mode(&self, rules: RulesFace, mode: usize) -> Option<String> {
        crate::choices::mode_words(self.0, rules.card, mode)
    }
}

/// Where a turn's lines change phase, for a quiet heading between them.
///
/// The turn's own heading stands for its beginning, so the beginning phase
/// gets none of its own; every later phase gets one before its first line,
/// and a phase in which nothing happened gets none at all.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(crate) struct PhaseRun {
    /// The turn and phase of the last line read.
    last: Option<(u32, Phase)>,
}

impl PhaseRun {
    /// The phase whose heading stands before `line`, if one does, and the
    /// run moved on past it.
    pub(crate) fn heading(&mut self, line: &LogLine) -> Option<Phase> {
        if line.header {
            self.last = Some((line.turn, Phase::Beginning));
            return None;
        }
        let phase = line.phase?;
        let new = self.last != Some((line.turn, phase));
        self.last = Some((line.turn, phase));
        (new && phase != Phase::Beginning).then_some(phase)
    }
}

/// Which turns of this game's log are open: the player's choices, over the
/// rule that only the newest is (feedback of 24.10.2026). Every list that
/// folds reads it ([`virtual_list::VirtualLog::fold_with`]); a press on a
/// turn's heading writes it ([`spawn_turn`]), and nothing else does.
#[derive(Resource, Default, Debug)]
pub struct LogFolds {
    /// The turns the player opened (`true`) or folded (`false`).
    chosen: BTreeMap<u32, bool>,
    /// The game these are about: its first line's time and how often its
    /// book was rewritten, so the next game, or a replay's jump, starts over.
    game: Option<(u64, u32)>,
    /// Counts every change, so a list refolds only when one was made.
    generation: u64,
}

impl LogFolds {
    /// Whether `turn` is shown open; `newest` is whether it is the newest.
    pub(crate) fn open(&self, turn: u32, newest: bool) -> bool {
        self.chosen.get(&turn).copied().unwrap_or(newest)
    }

    /// Opens or folds `turn`.
    pub(crate) fn set(&mut self, turn: u32, open: bool) {
        if self.chosen.insert(turn, open) != Some(open) {
            self.generation += 1;
        }
    }

    /// How many changes have been made.
    pub(crate) const fn generation(&self) -> u64 {
        self.generation
    }

    /// The choices are about `game` from now on; another game's are
    /// forgotten.
    fn about(&mut self, game: (u64, u32)) {
        self.game = Some(game);
        if !self.chosen.is_empty() {
            self.chosen.clear();
            self.generation += 1;
        }
    }
}

/// Forgets the turns opened in another game. Writes only when the game
/// changed, never at rest.
pub fn keep_the_folds(duel: Option<Res<Duel>>, folds: Option<ResMut<LogFolds>>) {
    let (Some(duel), Some(mut folds)) = (duel, folds) else {
        return;
    };
    let game = (
        duel.log.entries().first().map_or(0, |entry| entry.at),
        duel.log.rewrites(),
    );
    if folds.game != Some(game) {
        folds.about(game);
    }
}

/// A turn's heading in a list that folds: the rule, a chevron, the heading's
/// words and, folded, how many lines the turn holds. A press opens or folds
/// it ([`LogFolds::set`]).
#[allow(clippy::too_many_arguments)] // a row of the log, and its fold
pub(crate) fn spawn_turn(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    line: &LogLine,
    inks: &LineInks,
    open: bool,
    lines: usize,
) -> Entity {
    let mut chevron = commands.spawn((
        Text::new(if open { "\u{f078}" } else { "\u{f054}" }),
        icon_tf(fonts, inks.turn_pt * 0.7),
        Node {
            width: px(inks.turn_pt * 1.1),
            flex_shrink: 0.0,
            ..default()
        },
        Pickable::IGNORE,
    ));
    (inks.paint)(&mut chevron, Paint::Ink(inks.soft));
    let chevron = chevron.id();
    let words = if open || lines == 0 {
        line.text.clone()
    } else {
        format!(
            "{} \u{b7} {}",
            line.text,
            Phrase::GameLogTurnLines.fill(lang, &[&lines.to_string()])
        )
    };
    // Grows into the row as a sentence does: left to its content, Bevy
    // sets a text in a row at its narrowest, a word to a line.
    let mut heading = commands.spawn((
        Text::new(words),
        tf(fonts, inks.turn_pt),
        Node {
            flex_grow: 1.0,
            min_width: px(0),
            ..default()
        },
        Pickable::IGNORE,
    ));
    (inks.paint)(&mut heading, Paint::Ink(inks.soft));
    let heading = heading.id();
    let mut rule = commands.spawn((
        LogFoldToggle {
            turn: line.turn,
            open,
        },
        Node {
            margin: UiRect::top(px(LOG_TURN_GAP)),
            padding: UiRect::top(px(3)),
            border: UiRect::top(px(1)),
            align_items: AlignItems::Center,
            flex_shrink: 0.0,
            ..default()
        },
        Pickable {
            should_block_lower: false,
            is_hoverable: true,
        },
    ));
    (inks.paint)(&mut rule, Paint::Rule(inks.rule));
    rule.add_children(&[chevron, heading])
        .observe(
            |mut click: On<PointerClick>,
             toggles: Query<&LogFoldToggle>,
             parents: Query<&ChildOf>,
             mut lists: Query<&mut LogFollow>,
             folds: Option<ResMut<LogFolds>>| {
                let (Ok(toggle), Some(mut folds)) = (toggles.get(click.entity), folds) else {
                    return;
                };
                click.propagate(false);
                folds.set(toggle.turn, !toggle.open);
                // The heading stays under the pointer and the turn opens
                // below it, rather than the list following its end away
                // from what was pressed.
                if let Ok(parent) = parents.get(click.entity)
                    && let Ok(mut follow) = lists.get_mut(parent.parent())
                {
                    follow.stuck = false;
                }
            },
        )
        .id()
}

/// A turn's heading that opens or folds the turn when pressed.
#[derive(Component, Clone, Copy, Debug)]
pub struct LogFoldToggle {
    /// The turn.
    pub turn: u32,
    /// Whether it is drawn open.
    pub open: bool,
}

/// What a phase's heading says.
pub(crate) const fn phase_words(phase: Phase) -> Phrase {
    match phase {
        Phase::Beginning => Phrase::LogPhaseBeginning,
        Phase::FirstMain => Phrase::LogPhaseFirstMain,
        Phase::Combat => Phrase::LogPhaseCombat,
        Phase::SecondMain => Phrase::LogPhaseSecondMain,
        Phase::Ending => Phrase::LogPhaseEnding,
    }
}

/// A phase's heading: a small caption in the quieter ink, set in from the
/// lines under it.
pub(crate) fn spawn_phase(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    phase: Phase,
    inks: &LineInks,
) -> Entity {
    let mut caption = commands.spawn((
        Text::new(phase_words(phase).text(lang).to_uppercase()),
        TextFont {
            font: bevy::text::FontSource::Handle(fonts.medium.clone()),
            ..tf(fonts, inks.turn_pt * PHASE_SCALE)
        },
        Node {
            margin: UiRect::new(px(PHASE_INDENT), px(0), px(PHASE_GAP), px(0)),
            flex_shrink: 0.0,
            ..default()
        },
        Pickable::IGNORE,
    ));
    (inks.paint)(&mut caption, Paint::Ink(inks.soft));
    caption.id()
}

/// A phase heading's size, against a turn's.
const PHASE_SCALE: f32 = 0.85;

/// Above a phase's heading.
const PHASE_GAP: f32 = 4.0;

/// How far a phase's heading stands in: past the time column, over the
/// sentences it heads.
const PHASE_INDENT: f32 = CLOCK_W + CLOCK_AIR + SWATCH_GAP;

/// Who the lines are written for.
pub(crate) fn wording<'a>(
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
pub(crate) fn subject_ink(wording: &Wording, subject: Option<PlayerId>) -> Color {
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
            UiButton,
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
pub(crate) fn scrollbar(
    commands: &mut Commands,
    list: Entity,
    colours: (Color, Color),
    paint: fn(&mut EntityCommands, Paint),
) -> Entity {
    scrollbar_parts(commands, list, colours, paint).0
}

/// [`scrollbar`], answering with the track and its thumb, for a caller that
/// puts more on the thumb (a stack entry's text box fades both in with its
/// row).
pub(crate) fn scrollbar_parts(
    commands: &mut Commands,
    list: Entity,
    (track, thumb): (Color, Color),
    paint: fn(&mut EntityCommands, Paint),
) -> (Entity, Entity) {
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
    (bar.add_child(grip).id(), grip)
}

/// The pill at the list's bottom, hidden until lines arrive under a list the
/// player has scrolled up. Pressing it takes the list back to its end.
pub(crate) fn new_lines_pill(commands: &mut Commands, fonts: &UiFonts, lang: Lang) -> Entity {
    commands
        .spawn((
            LogNewBelow,
            UiButton,
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
                    LogPillWords { unseen: true },
                    Text::new(Phrase::GameLogNewBelow.text(lang)),
                    tf(fonts, LOG_TURN_PT),
                    TextColor(palette::DIALOG_INK),
                    Pickable::IGNORE,
                ),
                (
                    LogPillWords { unseen: false },
                    Text::new(Phrase::GameLogLatest.text(lang)),
                    tf(fonts, LOG_TURN_PT),
                    TextColor(palette::DIALOG_INK),
                    Node {
                        display: Display::None,
                        ..default()
                    },
                    Pickable::IGNORE,
                ),
            ],
        ))
        .observe(
            |mut click: On<PointerClick>, mut lists: Query<&mut LogFollow, With<LogList>>| {
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
pub(crate) struct LineInks {
    /// A line.
    pub line_pt: f32,
    /// A turn's heading.
    pub turn_pt: f32,
    /// The sentence and the names in it.
    pub ink: Color,
    /// A turn's heading, "(×N)" and a line's time.
    pub soft: Color,
    /// A card a line names that the seat was shown: a link.
    pub link: Color,
    /// The rule over a turn's heading.
    pub rule: Color,
    /// Puts one of those colours on a node.
    pub paint: fn(&mut EntityCommands, Paint),
}

/// One colour a line puts on one node.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Paint {
    /// A text's or a span's.
    Ink(Color),
    /// A rule's.
    Rule(Color),
    /// A surface's: the scrollbar's track and thumb.
    Fill(Color),
}

/// The panel's inks.
pub(crate) const PANEL_INKS: LineInks = LineInks {
    line_pt: LOG_LINE_PT,
    turn_pt: LOG_TURN_PT,
    ink: palette::DIALOG_INK,
    soft: palette::DIALOG_SOFT,
    link: palette::CANDLE,
    rule: palette::DIALOG_LINE,
    paint: at_rest,
};

/// A colour as it stands from the first frame.
pub(crate) fn at_rest(node: &mut EntityCommands, paint: Paint) {
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
pub(crate) const TEXT_OWN_HEIGHT: Overflow = Overflow::clip_y();

/// One line of the log, as a row.
///
/// A turn's heading is a quieter line under a rule. Any other line is its
/// time ([`local_clock`]), the seat swatch, when `swatch` gives it a colour
/// ([`subject_ink`]), and the sentence, cut into its [`Piece`]s: players in
/// bold, a card the seat was shown as a link, a card it was not shown a
/// weight up, and "(×N)" after a line that happened more than once.
pub(crate) fn spawn_line(
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
    let sentence = sentence(commands, fonts, lang, line, inks);
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
    // The time first, in a column as wide as any time is, so the sentences
    // start in one place down the list. A line the host never dated has
    // none, and neither then has any other line of that game.
    if let Some(clock) = local_clock(line) {
        // A line that carries on the one above it stands under that line's
        // sentence, with no time of its own: it is the same moment.
        let clock = if line.follows { String::new() } else { clock };
        let mut time = commands.spawn(clock_bundle(fonts, inks.line_pt, inks.soft, clock));
        let soft = inks.soft;
        (inks.paint)(
            &mut time,
            Paint::Ink(soft.with_alpha(soft.alpha() * CLOCK_FADE)),
        );
        let time = time.id();
        commands.entity(row).add_child(time);
    }
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
    if line.crowned {
        // The monarch's crown, as their plate and chip wear it.
        let mut crown = commands.spawn((
            Text::new(glyph::CROWN.to_string()),
            icon_tf(fonts, inks.line_pt * 0.8),
            Node {
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ));
        (inks.paint)(&mut crown, Paint::Ink(palette::CANDLE));
        let crown = crown.id();
        commands.entity(row).add_child(crown);
    }
    commands.entity(row).add_child(sentence);
    row
}

/// A line's sentence, cut into its [`Piece`]s and set piece by piece.
fn sentence(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    line: &LogLine,
    inks: &LineInks,
) -> Entity {
    let pieces = pieces(line);
    // Hoverable only where there is something to hover, and never in the
    // way of what is under it: the list scrolls under the pointer whatever
    // line it is on.
    let linked = pieces
        .iter()
        .any(|(_, piece)| matches!(piece, Piece::Link(_)));
    let mut sentence = commands.spawn((
        Text::default(),
        tf(fonts, inks.line_pt),
        Node {
            flex_grow: 1.0,
            min_width: px(0),
            overflow: TEXT_OWN_HEIGHT,
            ..default()
        },
        if linked {
            Pickable {
                should_block_lower: false,
                is_hoverable: true,
            }
        } else {
            Pickable::IGNORE
        },
    ));
    (inks.paint)(&mut sentence, Paint::Ink(inks.ink));
    let sentence = sentence.id();
    if line.follows {
        let mut mark = commands.spawn((TextSpan::new(FOLLOWS_MARK), tf(fonts, inks.line_pt)));
        (inks.paint)(&mut mark, Paint::Ink(inks.soft));
        let mark = mark.id();
        commands.entity(sentence).add_child(mark);
    }
    for (text, piece) in pieces {
        let face = match piece {
            Piece::Words => None,
            Piece::Player => Some(&fonts.bold),
            Piece::Name | Piece::Link(_) => Some(&fonts.medium),
        };
        let font = face.map_or_else(
            || tf(fonts, inks.line_pt),
            |face| TextFont {
                font: bevy::text::FontSource::Handle(face.clone()),
                ..tf(fonts, inks.line_pt)
            },
        );
        let mut span = commands.spawn((TextSpan::new(text), font));
        if let Piece::Link(link) = piece {
            span.insert(link);
            (inks.paint)(&mut span, Paint::Ink(inks.link));
        } else {
            (inks.paint)(&mut span, Paint::Ink(inks.ink));
        }
        let span = span.id();
        commands.entity(sentence).add_child(span);
    }
    if let Some(times) = times(line, lang) {
        let mut span = commands.spawn((TextSpan::new(times), tf(fonts, inks.line_pt)));
        (inks.paint)(&mut span, Paint::Ink(inks.soft));
        let span = span.id();
        commands.entity(sentence).add_child(span);
    }
    sentence
}

/// What opens a line that carries on the line above it (`LogLine::follows`).
const FOLLOWS_MARK: &str = "\u{2192} ";

/// How wide a line's time is: "00:00" at [`CLOCK_SCALE`] of the line's
/// size, in figures of one width, with a little to spare.
pub(crate) const CLOCK_W: f32 = 30.0;

/// A line's time against its sentence: a step smaller, so it reads as the
/// margin's and not the sentence's (feedback of 24.10.2026: "the time should
/// stand apart, but look quieter").
pub(crate) const CLOCK_SCALE: f32 = 0.84;

/// The air between a line's time and the rest of it, on top of the row's
/// own gap (the same feedback: "some room between the time and the line").
pub(crate) const CLOCK_AIR: f32 = 6.0;

/// How much of the quieter ink a line's time takes.
pub(crate) const CLOCK_FADE: f32 = 0.7;

/// How far the smaller time stands down, so its figures sit on the
/// sentence's first line rather than above it: the difference of the two
/// sizes' ascents, near enough, at the default text step.
pub(crate) fn clock_drop_at(line_pt: f32) -> f32 {
    (1.0 - CLOCK_SCALE) * line_pt * crate::hud::UI_SCALE * 0.78
}

/// A line's time: [`CLOCK_SCALE`] of `line_pt`, its figures lining and of
/// one width so the column's times stand flush.
pub(crate) fn clock_font(fonts: &UiFonts, line_pt: f32) -> TextFont {
    TextFont {
        font_features: bevy::text::FontFeatures::builder()
            .enable(bevy::text::FontFeatureTag::LINING_FIGURES)
            .enable(bevy::text::FontFeatureTag::TABULAR_FIGURES)
            .build(),
        ..tf(fonts, line_pt * CLOCK_SCALE)
    }
}

/// A line's time: the column's node, its words and its ink, as both the log
/// and the chat window draw it.
pub(crate) fn clock_bundle(
    fonts: &UiFonts,
    line_pt: f32,
    soft: Color,
    clock: String,
) -> impl Bundle {
    (
        Text::new(clock),
        clock_font(fonts, line_pt),
        TextLayout::no_wrap(),
        TextColor(soft.with_alpha(soft.alpha() * CLOCK_FADE)),
        Node {
            width: px(CLOCK_W),
            margin: UiRect {
                top: px(clock_drop_at(line_pt)),
                right: px(CLOCK_AIR),
                ..default()
            },
            flex_shrink: 0.0,
            ..default()
        },
        Pickable::IGNORE,
    )
}

/// A line's time on the device's own clock, "14:05" (#300).
///
/// The offset is asked of the platform for the moment the line was written,
/// not for now, so a line from before the clocks changed keeps its hour.
/// Where the platform will not say, the time is UTC, which is what the host
/// stamped.
pub(crate) fn local_clock(line: &LogLine) -> Option<String> {
    let offset = i64::try_from(line.at / 1000)
        .ok()
        .and_then(|secs| time::OffsetDateTime::from_unix_timestamp(secs).ok())
        .and_then(|utc| time::UtcOffset::local_offset_at(utc).ok())
        .map_or(0, time::UtcOffset::whole_seconds);
    line.clock(offset)
}

/// A card or token a line names, as the line showed it to this seat: what its
/// link previews (#300).
///
/// Taken from the line and not from the table, so the preview is the printing
/// and finish the line named, whatever has become of the card since.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct LogLink {
    /// The card, its printing and its face.
    pub card: Option<CardIdentity>,
    /// The registry token, for a token.
    pub token: Option<u16>,
}

impl LogLink {
    /// The link a name makes, when the line showed the seat what it names.
    /// A face-down card's name shows neither, and makes none.
    fn of(name: &NameSpan) -> Option<Self> {
        (name.card.is_some() || name.token.is_some()).then_some(Self {
            card: name.card,
            token: name.token,
        })
    }

    /// The picture its preview shows, at `size`.
    pub(crate) fn art(self, size: ArtSize) -> Option<ImageKey> {
        match (self.card, self.token) {
            (Some(card), _) => Some(ImageKey::of(card, size)),
            (None, Some(token)) => Some(ImageKey::token(token, size)),
            (None, None) => None,
        }
    }
}

/// What a piece of a line's sentence is, which is how it is set.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Piece {
    /// The sentence's own words.
    Words,
    /// A player, "you" included, in bold: a line is found by who it is about.
    Player,
    /// An object the seat was not shown, a face-down card: a weight up, and
    /// nothing to hover.
    Name,
    /// A card or token the seat was shown: in brackets and the link's ink, and
    /// the table's preview of it under the pointer.
    Link(LogLink),
}

/// A line's sentence cut at its names and players: each piece, and what it
/// is. A link's piece carries its brackets.
///
/// A range the sentence cannot be cut at, or two that overlap, is read as no
/// mark at all rather than as a panic: the sentence is still true, only set
/// in one weight.
pub(crate) fn pieces(line: &LogLine) -> Vec<(String, Piece)> {
    let whole = || vec![(line.text.clone(), Piece::Words)];
    let mut marks: Vec<(std::ops::Range<usize>, Piece)> = line
        .names
        .iter()
        .map(|name| {
            let piece = LogLink::of(name).map_or(Piece::Name, Piece::Link);
            (name.range.clone(), piece)
        })
        .chain(
            line.players
                .iter()
                .map(|player| (player.range.clone(), Piece::Player)),
        )
        .collect();
    marks.sort_by_key(|(range, _)| range.start);
    let mut out = Vec::new();
    let mut at = 0;
    for (range, piece) in marks {
        let (Some(before), Some(marked)) =
            (line.text.get(at..range.start), line.text.get(range.clone()))
        else {
            return whole();
        };
        if !before.is_empty() {
            out.push((before.to_string(), Piece::Words));
        }
        let text = match piece {
            Piece::Link(_) => format!("[{marked}]"),
            _ => marked.to_string(),
        };
        out.push((text, piece));
        at = range.end;
    }
    match line.text.get(at..) {
        Some(rest) if !rest.is_empty() => out.push((rest.to_string(), Piece::Words)),
        Some(_) => {}
        None => return whole(),
    }
    out
}

/// Where the pointer is on a link in the log, for the preview (#300).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct LogHover {
    /// The link's span, which the pointer leaves by.
    pub span: Entity,
    /// What it previews.
    pub link: LogLink,
    /// Where the pointer found it, in logical pixels: the preview stands
    /// beside it, as it does beside a row in the zone browser.
    pub at: Vec2,
    /// The card the link lies over, in logical window pixels, when it is a
    /// card's picture rather than a word in a line (a revealed card,
    /// [`crate::hud::revealed`]): the preview then opens at that card's edge,
    /// as a permanent's does, wherever on it the pointer came in.
    pub card: Option<Rect>,
}

/// A link that lies over a whole card's picture ([`LogHover::card`]).
#[derive(Component)]
pub struct CardLens;

/// Follows the pointer onto the log's links and off them again
/// ([`Duel::hovered_log`]). Written only when it changes, so the overlay's
/// redraw gate sees a hover and not a frame.
#[allow(clippy::too_many_arguments)] // the pointer, the links, a lens's box and a span's visibility
pub fn hover_log_links(
    mut overs: MessageReader<PointerOver>,
    mut outs: MessageReader<PointerOut>,
    links: Query<&LogLink>,
    lenses: Query<(&ComputedNode, &UiGlobalTransform), With<CardLens>>,
    ui: Option<Res<UiScale>>,
    shown: Query<&InheritedVisibility>,
    parents: Query<&ChildOf>,
    mut duel: ResMut<Duel>,
) {
    for over in overs.read() {
        if let Ok(link) = links.get(over.entity) {
            // A node's box is in physical pixels, centred; the preview's
            // anchors are logical window pixels (`PreviewAt::in_ui` divides
            // the UI scale back out).
            let card = lenses.get(over.entity).ok().map(|(node, at)| {
                let to_logical = node.inverse_scale_factor() * ui.as_deref().map_or(1.0, |ui| ui.0);
                Rect::from_center_size(at.translation * to_logical, node.size() * to_logical)
            });
            duel.hovered_log = Some(LogHover {
                span: over.entity,
                link: *link,
                at: over.pointer.position,
                card,
            });
        }
    }
    for out in outs.read() {
        if duel
            .hovered_log
            .is_some_and(|hover| hover.span == out.entity)
        {
            duel.hovered_log = None;
        }
    }
    // A link that is gone with its panel, or with its game, shows nothing:
    // a despawned span is never left by the pointer. Nor is one whose panel
    // was shut under it, which keeps its lines while it is hidden: the
    // preview of a line hovered as `L` shut the panel stayed on the table.
    // A span has no visibility of its own: its sentence's node has.
    let hidden = |span: Entity| {
        shown
            .get(span)
            .ok()
            .or_else(|| {
                parents
                    .get(span)
                    .ok()
                    .and_then(|up| shown.get(up.parent()).ok())
            })
            .is_some_and(|seen| !seen.get())
    };
    if duel
        .hovered_log
        .is_some_and(|hover| !links.contains(hover.span) || hidden(hover.span))
    {
        duel.hovered_log = None;
    }
}

/// What follows a line that happened more than once: the part of
/// [`Phrase::LogRepeated`] after the sentence, with the count in it.
///
/// Not [`LogLine::plain`], which returns one string: the names are drawn as
/// their own spans, so the count has to be one too.
pub(crate) fn times(line: &LogLine, lang: Lang) -> Option<String> {
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
#[allow(clippy::type_complexity)] // the pill's words are nodes beside the pill's own
pub fn follow_the_log(
    mut lists: Query<(&mut ScrollPosition, &ComputedNode, &mut LogFollow), With<LogList>>,
    mut pills: Query<&mut Node, With<LogNewBelow>>,
    mut words: Query<(&mut Node, &LogPillWords), Without<LogNewBelow>>,
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
        // Scrolled more than half a list up, the pill offers the way back
        // even with nothing new under it.
        let away = !follow.stuck && end - at.y > computed.size().y * scale * PILL_AWAY;
        let display = if follow.unseen || away {
            Display::Flex
        } else {
            Display::None
        };
        for mut pill in &mut pills {
            if pill.display != display {
                pill.display = display;
            }
        }
        for (mut node, said) in &mut words {
            let shown = if said.unseen == follow.unseen {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != shown {
                node.display = shown;
            }
        }
    }
}

/// How far up, in lists, a list is scrolled before its pill offers the way
/// back to the newest line with nothing new under it.
const PILL_AWAY: f32 = 0.5;

/// One of the pill's two sentences: new lines below, or back to the latest.
#[derive(Component)]
pub struct LogPillWords {
    /// Said when lines arrived under the list.
    pub unseen: bool,
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
mod tests;

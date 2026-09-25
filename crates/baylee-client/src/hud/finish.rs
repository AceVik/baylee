//! The end of a game, on a sheet over a darkened table.
//!
//! Before this the client said a game was over the way it says everything
//! else: one line in the prompt bar at thirteen pixels, while the board went
//! on being drawn underneath and two buttons floated over it sixty-four
//! pixels from the top of the window. A result is not a prompt — there is
//! nothing to answer — so it does not belong in the bar that holds questions,
//! and the bar goes quiet while this stands (`overlay::sync_overlay`).
//!
//! # Parchment, and one hue for all three endings
//!
//! A result is *read*, so it is a sheet and not a dialog, and the prompt slip
//! already settled that a read sheet may carry its own answers — this is that
//! slip grown up, in the same paper with the same grain, shadow and radius.
//!
//! Won, lost and drawn are **the same sheet**; only the sentence differs.
//! Every hue in this palette already carries a claim — `INK_DANGER` is
//! damage, `HEAL` is life gained, `BRASS` is a thing already taken, `CANDLE`
//! is an offer — so a red "you lost" would read as damage and a gold "you
//! won" as something taken. `docs/redesign-proposal.md` §1 retires hues; it
//! does not hand out new ones. Size carries the feeling instead: forty
//! pixels, against the twenty this overlay never used to go above.
//!
//! # Why it is spawned once and not synchronised
//!
//! Everything else here is a retained tree with a revision, because
//! everything else changes: a hover, a card drawn, a life total. A result does
//! not. `Pending::GameOver` is the last thing the engine ever says at this
//! table, so the screen is built on `OnEnter(DuelPhase::Finished)`, taken down
//! on the way out, and never touched in between.
//!
//! # Two plugins, one composition
//!
//! The verdict is the *duel's* to say and the way out is the *shell's*:
//! `DuelPlugin` is meant to be embeddable in an application that already has a
//! front door, and it cannot know whether there is a lobby behind it. So this
//! draws the sheet and leaves one row in it marked [`FinishExits`], and
//! `lobby::ui::spawn_leave_button` puts its buttons there when a lobby is what
//! is behind this duel. The row has a **fixed height** so that the sheet does
//! not reflow when those buttons land a frame later. A launch handed a
//! `SeatTicket` adds no `LobbyPlugin` at all (`main.rs`), and then the row
//! stays empty — which is honest, because that client has nowhere to go
//! either.
//!
//! The **keyboard** way out is the shell's for the same reason and lives
//! beside those buttons, in `lobby::systems::leave_keys`: `DuelSet::Input`
//! runs only in `DuelPhase::Playing`, so every binding is off while this
//! sheet is up, and for a while that made it the one screen a keyboard could
//! reach and not leave. It reads the row rather than a list of its own, so
//! the two cases above stay one rule.
//!
//! # The whole game, under its end
//!
//! Under the losses the sheet carries the game's log, every line of it
//! (#262), in the panel's own rows ([`super::ledge::log::spawn_line`]) set in
//! the sheet's inks. It is the one thing here that scrolls, so it is the one
//! thing a short window takes its room out of. The wheel reaches it because
//! `hud::scrolls` is the one input system that runs past `Playing`.

use super::ledge::log::{LOG_ROW_GAP, LineInks, Paint, scrollbar, spawn_line, wording};
#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use baylee_client_core::gamelog::LogLine;
use baylee_client_core::interaction::{ending_reason, table_losses, verdict};
use baylee_core::ids::CardIndex;
use bevy::text::LineHeight;
use bevy::ui::ScrollPosition;

/// How wide the sheet is drawn, at a window with room for it.
///
/// Wider than the ability sheet's 300, and it should be: that one stands
/// beside a card and has to leave the board readable, and this is the only
/// thing on the screen.
///
/// The number is the longest verdict, **measured out of the shipped font**
/// rather than guessed at. `Das Spiel endet unentschieden` sets 585 px in
/// `Faustina.ttf` at [`VERDICT_PT`] times [`super::SERIF_SCALE`], weight 600;
/// every other verdict is shorter, the nearest being `Team 2 gewinnt — deins`
/// at 466. (In `Inter.ttf`, which this replaced, the pair was 561 and 443 at
/// [`VERDICT_PT`] itself. The serif is the **wider** face here, by 4.4% — it
/// is set 10% larger to match Inter's x-height and does not give all of that
/// back in the advances, which is a cost of the change and not a benefit of
/// it.) Against it this sheet gives
/// 638 — 720 less two forty-pixel margins and its two one-pixel borders — so
/// the worst line has fifty pixels of air. If a verdict ever does wrap, widen
/// the sheet rather than shrink the type: a headline that changes size with
/// the sentence is a headline that says the sentence matters less.
const SHEET_W: f32 = 720.0;

/// The air the sheet leaves at each side of a window too narrow for
/// [`SHEET_W`]. Below about 816 logical pixels the sheet is the window less
/// this, and the verdict wraps to at most two lines rather than scaling down.
///
/// And the air above and below it in a window too short for all it says:
/// the sheet is then the window less this, and the room comes out of the
/// game log, which scrolls, and out of nothing else.
const SHEET_AIR: f32 = 96.0;

/// The verdict, in logical pixels.
///
/// Nothing else on this overlay is drawn above twenty, which is the point:
/// the largest thing the client ever has to say is the sentence that ends the
/// game.
const VERDICT_PT: f32 = 40.0;

/// Its line height, as a multiple of the size. A headline set at the body
/// text's leading looks like body text that was enlarged by accident.
const VERDICT_LEADING: f32 = 1.15;

/// The line under it.
const REASON_PT: f32 = 18.0;

/// The air between the verdict and the reason.
const REASON_GAP: f32 = 10.0;

/// Why each seat that is out lost, one line apiece under the reason (#83).
///
/// Below the reason and smaller than it, upright where it is italic: the
/// reason is the table's one sentence about how the game was decided, and
/// these are facts about single seats, several of them at a table of four.
const LOSS_PT: f32 = 15.0;

/// The air above each loss line.
const LOSS_GAP: f32 = 4.0;

/// The air between the sheet's prose and the way out of it.
const EXITS_GAP: f32 = 28.0;

/// How tall the exits row is held, whether or not anything has arrived in it.
///
/// Derived from the prompt slip's own answer: a 13 px label, `answer_node`'s
/// seven pixels of padding each way and its one-pixel border. Fixed rather
/// than grown from the content, because the buttons are another plugin's and
/// land a frame later, and a sheet that jumped as they arrived would be a
/// sheet that had not finished settling.
const EXITS_H: f32 = 32.0;

/// The game log's lines: the size the losses are read at, because on this
/// sheet the log is read, not glanced at.
const LOG_PT: f32 = LOSS_PT;

/// A turn's heading in it, quieter than a line: the size the shelf's
/// labels are set at.
const LOG_TURN_PT: f32 = 13.0;

/// How tall the log's box gets, at most. A few turns, and the rest a scroll
/// away: the verdict stays the thing this sheet is about.
const LOG_BOX_H: f32 = 320.0;

/// Between the log's caption and its box.
const LOG_CAPTION_GAP: f32 = 6.0;

/// The air inside the box.
const LOG_PAD: f32 = 8.0;

/// The log's lines in the sheet's inks, settling with it.
///
/// No seat swatch: the sheet is one reader's account of a game that is over,
/// and a column of seat colours down its left is the panel's, where the table
/// is still being played beside it.
const SHEET_INKS: LineInks = LineInks {
    line_pt: LOG_PT,
    turn_pt: LOG_TURN_PT,
    ink: palette::PARCHMENT_INK,
    soft: palette::SLIP_SOFT,
    rule: palette::PARCHMENT_EDGE,
    swatch: false,
    paint: settle,
};

/// How far above its resting place the sheet starts.
///
/// It settles; it does not slide. Twelve pixels is a sheet coming to rest on
/// a table, and anything more is a panel flying in.
const SHEET_LIFT: f32 = 12.0;

/// The end screen's root. One per finished game, taken down on the way out.
#[derive(Component)]
pub(crate) struct FinishRoot;

/// The sheet itself, which is the one node that moves.
#[derive(Component)]
pub(crate) struct FinishSheet;

/// The row an embedding shell puts its exits in.
///
/// A marker rather than a callback because the two plugins do not know each
/// other and must not have to: the duel says *where* a way out belongs in this
/// composition, and whoever owns the screen after this duel says what the ways
/// out are.
#[derive(Component)]
pub(crate) struct FinishExits;

/// The game log's scrolling list on the sheet.
#[derive(Component)]
pub(crate) struct FinishLog;

/// A node of this sheet, and the colour it rests at.
///
/// The resting colour has to be *remembered* rather than read back off the
/// node: one frame into the fade the node is carrying the faded value, and
/// taking that as the base is how a thing fades to nothing and stays there.
/// A node may carry two of these at once — the sheet has a fill and a border
/// — so all four channels are one component.
#[derive(Component, Clone, Copy)]
pub(crate) struct Settling {
    /// [`BackgroundColor`] at rest.
    fill: Option<Color>,
    /// [`BorderColor`] at rest.
    border: Option<Color>,
    /// [`TextColor`] at rest.
    ink: Option<Color>,
    /// [`ImageNode`]'s tint at rest — the parchment grain.
    grain: Option<Color>,
}

impl Settling {
    const NONE: Self = Self {
        fill: None,
        border: None,
        ink: None,
        grain: None,
    };

    /// A surface with an edge.
    const fn panel(fill: Color, border: Color) -> Self {
        Self {
            fill: Some(fill),
            border: Some(border),
            ..Self::NONE
        }
    }

    /// A line of text.
    const fn ink(ink: Color) -> Self {
        Self {
            ink: Some(ink),
            ..Self::NONE
        }
    }

    /// A rule, which is a border alone.
    const fn edge(border: Color) -> Self {
        Self {
            border: Some(border),
            ..Self::NONE
        }
    }

    /// A surface with no edge: the log's scrollbar.
    const fn fill(fill: Color) -> Self {
        Self {
            fill: Some(fill),
            ..Self::NONE
        }
    }

    /// The grain laid over a sheet, which is an image and is tinted white.
    const fn grain() -> Self {
        Self {
            grain: Some(Color::WHITE),
            ..Self::NONE
        }
    }
}

/// A colour on this sheet: clear at first, and remembered for the veil to
/// raise ([`settle_the_sheet`]). What the log's rows are painted with here.
fn settle(node: &mut EntityCommands, paint: Paint) {
    match paint {
        Paint::Ink(colour) => {
            node.insert((TextColor(colour.with_alpha(0.0)), Settling::ink(colour)))
        }
        Paint::Rule(colour) => node.insert((
            BorderColor::all(colour.with_alpha(0.0)),
            Settling::edge(colour),
        )),
        Paint::Fill(colour) => node.insert((
            BackgroundColor(colour.with_alpha(0.0)),
            Settling::fill(colour),
        )),
    };
}

/// Builds the screen, if there is a result to build it from.
///
/// `fonts` and `sheets` are optional for the reason every other drawing system
/// here takes them so: a headless test app runs `Startup` without a renderer,
/// and a screen that panicked rather than drawing nothing would make every
/// such test a test about the GPU.
pub(crate) fn spawn_finish(
    mut commands: Commands,
    duel: Res<crate::Duel>,
    settings: Res<crate::settings::ClientSettings>,
    fonts: Option<Res<UiFonts>>,
    sheets: Option<Res<UiSheets>>,
    texts: Option<Res<crate::cardtext::CardTexts>>,
    windows: Query<&Window>,
) {
    let (Some(result), Some(fonts)) = (duel.ending(), fonts) else {
        return;
    };
    // The seat and its team, the same way the prompt bar used to find them: a
    // `Victor::Team` is answered by one comparison, and the seat's own team is
    // in the roster, which no view carries.
    let Some(statics) = duel.statics.as_ref() else {
        // No roster, so no way to say whose win this is. That is a game that
        // never really started, and a screen telling a player who never sat
        // down that they lost would be worse than no screen.
        return;
    };
    let seat = statics.your_seat;
    let team = duel.my_team();
    let lang = Lang::of(&settings.lang);
    let (width, height) = windows.iter().next().map_or((1280.0, 720.0), |w| {
        (w.resolution.width(), w.resolution.height())
    });

    let root = commands
        .spawn((
            FinishRoot,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            // A **root**, so this is ordered against other roots and not
            // against `HudRoot`'s children: `ui_stack_system` sorts roots by
            // `(GlobalZIndex, ZIndex)` and then walks each subtree. Every
            // other root in this client sits at `GlobalZIndex(0)` — the seat
            // bars at -1 — so one is enough to stand over all of them, and it
            // says what it means instead of continuing the ladder of `ZIndex`
            // numbers that only ever order siblings.
            GlobalZIndex(1),
            Pickable::IGNORE,
        ))
        .id();

    // The same veil a zone dialog draws, from the same constructor and
    // painted by the same system: `tray::dim_the_table` now has a finished
    // game as its second reason to darken a table. The board stays visible
    // through it, because the board is the evidence for what the sheet says.
    let veil = super::tray::spawn_veil(&mut commands);
    commands.entity(root).add_child(veil);

    let sheet = commands
        .spawn((
            FinishSheet,
            Node {
                width: px(SHEET_W.min(width - SHEET_AIR)),
                max_height: px(height - SHEET_AIR),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect {
                    left: px(40),
                    right: px(40),
                    top: px(40),
                    bottom: px(32),
                },
                border: UiRect::all(px(1)),
                border_radius: sheet_radius(),
                ..default()
            },
            BackgroundColor(palette::PARCHMENT.with_alpha(0.0)),
            BorderColor::all(palette::PARCHMENT_EDGE.with_alpha(0.0)),
            sheet_shadow(),
            Settling::panel(palette::PARCHMENT, palette::PARCHMENT_EDGE),
            UiTransform::from_translation(Val2::px(0.0, SHEET_LIFT)),
            // Above the veil, which is its **sibling** under this root and
            // would otherwise be painted over it: `ui_stack_system` orders
            // siblings by `ZIndex`, and a node with none is a zero against
            // the veil's three. The ladder in `hud` says what that costs —
            // "the veil is the hinge: what is below it goes dark, what is
            // above it stays lit" — and this sheet was below it, drawn at
            // thirty per cent of the paper it is written on.
            ZIndex(Z_SHEET),
            Pickable::IGNORE,
        ))
        .id();
    if let Some(sheets) = sheets.as_deref() {
        commands
            .entity(sheet)
            .with_child((sheet_surface(sheets), Settling::grain()));
    }
    commands.entity(root).add_child(sheet);

    write_the_verdict(
        &mut commands,
        &fonts,
        sheet,
        Said {
            lang,
            result,
            seat,
            team,
            losses: duel
                .view
                .as_ref()
                .map(|view| table_losses(lang, view, Some(statics), seat))
                .unwrap_or_default(),
        },
    );

    // The whole game, under how it ended (#262).
    let lookup = |card: CardIndex, face: u8| texts.as_ref().and_then(|t| t.face(card, face));
    let lines = duel.log.lines(&wording(&duel, lang, &lookup));
    write_the_log(&mut commands, &fonts, sheet, lang, &lines);

    let exits = commands
        .spawn((
            FinishExits,
            Node {
                width: percent(100),
                height: px(EXITS_H),
                // Its own height in a window of any height: a short one
                // takes its room out of the log above.
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                column_gap: px(BUTTON_GAP),
                justify_content: JustifyContent::Center,
                margin: UiRect::top(px(EXITS_GAP)),
                ..default()
            },
            // The row itself answers nothing; a button put in it brings its
            // own `Pickable`.
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(sheet).add_child(exits);
}

/// Everything the sheet has to say, and who is reading it.
///
/// One argument rather than four, because `spawn_finish` already passes a
/// `Commands`, a font set and the sheet to hang them on, and seven is where
/// this workspace stops counting parameters as readable.
struct Said<'a> {
    lang: Lang,
    result: &'a baylee_engine::win::GameResult,
    seat: baylee_core::ids::PlayerId,
    team: Option<u8>,
    /// [`table_losses`]: the reader's own loss first, then each other
    /// seat's that is out.
    losses: Vec<String>,
}

/// The verdict, and the line under it when there is one.
fn write_the_verdict(commands: &mut Commands, fonts: &UiFonts, sheet: Entity, said: Said) {
    let Said {
        lang,
        result,
        seat,
        team,
        losses,
    } = said;
    let headline = commands
        .spawn((
            Text::new(verdict(lang, result, seat, team)),
            // The one place this client asks a face for a weight, and Faustina
            // is the one it can ask: Alegreya Sans ships as static cuts,
            // where `weight` reaches nothing at all.
            super::tf_serif(fonts, VERDICT_PT, 600),
            LineHeight::RelativeToFont(VERDICT_LEADING),
            // Opaque, unlike the slip's ink: `SLIP_INK` lets the grain through
            // at thirteen pixels, where it is a stroke with paper in it, and
            // at forty it would be a headline with holes in it.
            TextColor(palette::PARCHMENT_INK.with_alpha(0.0)),
            Settling::ink(palette::PARCHMENT_INK),
            TextLayout::justify(Justify::Center),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(sheet).add_child(headline);

    // A draw gets no second line at all, gap included — `ending_reason`
    // refuses it, because the only thing it could say is the verdict again.
    if let Some(reason) = ending_reason(lang, result) {
        let line = commands
            .spawn((
                Text::new(reason),
                tf_italic(fonts, REASON_PT),
                TextColor(palette::SLIP_SOFT.with_alpha(0.0)),
                Settling::ink(palette::SLIP_SOFT),
                TextLayout::justify(Justify::Center),
                Node {
                    margin: UiRect::top(px(REASON_GAP)),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(sheet).add_child(line);
    }

    // Why each seat went out, and who answered its last decision (#83). The
    // first gap is the reason's, so a draw, which has no reason line, still
    // sets its losses apart from the verdict.
    for (at, loss) in losses.into_iter().enumerate() {
        let line = commands
            .spawn((
                FinishLoss,
                Text::new(loss),
                tf(fonts, LOSS_PT),
                TextColor(palette::SLIP_SOFT.with_alpha(0.0)),
                Settling::ink(palette::SLIP_SOFT),
                TextLayout::justify(Justify::Center),
                Node {
                    margin: UiRect::top(px(if at == 0 { REASON_GAP } else { LOSS_GAP })),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(sheet).add_child(line);
    }
}

/// The whole log, between the losses and the way out (#262).
///
/// Nothing at all for a game that logged nothing: an empty box on a sheet
/// that is only read would be a question with no answer. Built once, with
/// the rest of the sheet: the host sends every line before the game's last
/// question (`Session::pump`), so the book is whole when this is drawn.
///
/// It starts at the top and does not follow its end, unlike the panel's
/// list: this is read from the start of the game, and nothing is arriving.
fn write_the_log(
    commands: &mut Commands,
    fonts: &UiFonts,
    sheet: Entity,
    lang: Lang,
    lines: &[LogLine],
) {
    if lines.is_empty() {
        return;
    }
    let mut caption = commands.spawn((
        Text::new(Phrase::GameLogTitle.text(lang)),
        tf(fonts, LOG_TURN_PT),
        Node {
            align_self: AlignSelf::FlexStart,
            margin: UiRect::top(px(EXITS_GAP)),
            ..default()
        },
        Pickable::IGNORE,
    ));
    settle(&mut caption, Paint::Ink(palette::SLIP_SOFT));
    let caption = caption.id();
    // Pickable, unlike everything else on the sheet: the wheel is aimed at
    // what is under the pointer, and the rows under it pass it up to here.
    let list = commands
        .spawn((
            FinishLog,
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
    for line in lines {
        let row = spawn_line(commands, fonts, lang, line, &SHEET_INKS);
        commands.entity(list).add_child(row);
    }
    let bar = scrollbar(
        commands,
        list,
        (
            palette::PARCHMENT_EDGE.with_alpha(0.45),
            palette::SLIP_SOFT.with_alpha(0.7),
        ),
        settle,
    );
    let mut frame = commands.spawn((
        Node {
            width: percent(100),
            max_height: px(LOG_BOX_H),
            // Overrides the least a flex item may be, which would otherwise
            // be its content: the box is what gives way in a short window.
            min_height: px(0),
            flex_direction: FlexDirection::Row,
            column_gap: px(4),
            margin: UiRect::top(px(LOG_CAPTION_GAP)),
            padding: UiRect::all(px(LOG_PAD)),
            border: UiRect::all(px(1)),
            border_radius: btn_radius(),
            ..default()
        },
        Pickable::IGNORE,
    ));
    settle(&mut frame, Paint::Rule(palette::PARCHMENT_EDGE));
    let frame = frame.add_children(&[list, bar]).id();
    commands.entity(sheet).add_children(&[caption, frame]);
}

/// One line of [`table_losses`] on the sheet.
#[derive(Component)]
pub(crate) struct FinishLoss;

/// Every colour channel a settling node might carry.
///
/// A named shape rather than the tuple written out at the query, which clippy
/// is right to refuse: five `Option`s in a row say nothing about what they are
/// for.
type Painted = (
    &'static Settling,
    Option<&'static mut BackgroundColor>,
    Option<&'static mut BorderColor>,
    Option<&'static mut TextColor>,
    Option<&'static mut ImageNode>,
);

/// Settles the sheet as the veil rises.
///
/// It reads [`Veil::lit`] rather than easing anything of its own, which is
/// what makes the two one movement instead of two that could disagree: the
/// veil's rate, its resting point and its `reduce_motion` shortcut are the
/// sheet's as well, and nothing had to be told a transition is happening.
/// The same shape `sky::table_light` uses for the felt.
///
/// A colour is written only when it differs, which is every frame of the
/// fade and none after it: the log puts thousands of spans on this sheet in
/// a long game, and a write to each on every frame would mark every one of
/// them changed for as long as the sheet stands.
///
/// The exits are deliberately **not** settled. They belong to another plugin
/// and carry `ambience::Feel`, which owns their `BackgroundColor` from their
/// first frame; two systems writing that component would be two answers to
/// what colour a button is. Inside the fade or arriving after it, they simply
/// appear.
pub(crate) fn settle_the_sheet(
    veil: Res<Veil>,
    mut nodes: Query<Painted>,
    mut sheets: Query<&mut UiTransform, With<FinishSheet>>,
) {
    let lit = veil.lit.clamp(0.0, 1.0);
    let at = |colour: Color| colour.with_alpha(colour.alpha() * lit);
    for (settling, fill, border, ink, image) in &mut nodes {
        if let (Some(colour), Some(mut node)) = (settling.fill, fill) {
            node.set_if_neq(BackgroundColor(at(colour)));
        }
        if let (Some(colour), Some(mut node)) = (settling.border, border) {
            node.set_if_neq(BorderColor::all(at(colour)));
        }
        if let (Some(colour), Some(mut node)) = (settling.ink, ink) {
            node.set_if_neq(TextColor(at(colour)));
        }
        if let (Some(colour), Some(mut node)) = (settling.grain, image)
            && node.color != at(colour)
        {
            node.color = at(colour);
        }
    }
    for mut transform in &mut sheets {
        transform.translation = Val2::px(0.0, SHEET_LIFT * (1.0 - lit));
    }
}

/// Takes it down again — on the way back to the lobby, and on the way to
/// anything else.
pub(crate) fn despawn_finish(mut commands: Commands, roots: Query<Entity, With<FinishRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::PlayerId;
    use baylee_engine::win::{EndReason, GameResult, Victor};

    /// Fonts with no asset server behind them.
    ///
    /// A `Handle::default()` names no asset and draws no glyph, which is
    /// exactly right here: what is under test is the tree — which nodes exist,
    /// what they say and how they fade — and none of that is the GPU's.
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

    fn statics(team: Option<u8>) -> baylee_view::GameStatic {
        baylee_view::GameStatic {
            decision_secs: None,
            reconnect_secs: None,
            view_version: baylee_view::VIEW_VERSION,
            game_id: "g".into(),
            your_seat: PlayerId::new(0),
            seats: vec![baylee_view::SeatIdentity {
                player: PlayerId::new(0),
                display_name: "me".into(),
                is_ai: false,
                away: false,
                team,
            }],
            prints: vec![],
        }
    }

    /// A duel that has just ended the given way, with the roster it needs.
    fn ended(result: GameResult, team: Option<u8>) -> crate::Duel {
        crate::Duel {
            statics: Some(statics(team)),
            interaction: Some(over(result)),
            ..crate::Duel::default()
        }
    }

    /// The engine's last word, as `Interaction` holds it.
    fn over(result: GameResult) -> baylee_client_core::Interaction {
        baylee_client_core::Interaction::new(
            baylee_engine::choice::Pending::GameOver(result),
            PlayerId::new(0),
        )
    }

    fn app_at(duel: crate::Duel) -> App {
        let mut app = App::new();
        app.insert_resource(duel)
            .insert_resource(crate::settings::ClientSettings::default())
            .insert_resource(fonts())
            .init_resource::<Veil>()
            .add_systems(Update, (spawn_finish, settle_the_sheet).chain());
        app.update();
        app
    }

    fn lines(app: &mut App) -> Vec<String> {
        let mut found = app.world_mut().query::<&Text>();
        found.iter(app.world()).map(|t| t.0.clone()).collect()
    }

    #[test]
    fn a_won_game_puts_its_verdict_and_its_reason_on_one_sheet() {
        let mut app = app_at(ended(
            GameResult {
                winner: Some(Victor::Player(PlayerId::new(0))),
                reason: EndReason::LastPlayerStanding,
            },
            None,
        ));
        let said = lines(&mut app);
        assert!(
            said.contains(&Phrase::YouWon.text(Lang::En).to_string()),
            "{said:?}"
        );
        assert!(
            said.contains(&Phrase::EndedLastPlayer.text(Lang::En).to_string()),
            "{said:?}"
        );
        let mut roots = app.world_mut().query_filtered::<Entity, With<FinishRoot>>();
        assert_eq!(roots.iter(app.world()).count(), 1);
        let mut exits = app
            .world_mut()
            .query_filtered::<Entity, With<FinishExits>>();
        assert_eq!(
            exits.iter(app.world()).count(),
            1,
            "the shell has nowhere to put a way out"
        );
    }

    /// The one ending whose reason would only say the verdict again.
    #[test]
    fn a_draw_is_one_line_and_not_two() {
        let mut app = app_at(ended(
            GameResult {
                winner: None,
                reason: EndReason::Draw,
            },
            None,
        ));
        let said = lines(&mut app);
        assert_eq!(
            said,
            vec![Phrase::TheGameIsADraw.text(Lang::En).to_string()],
            "a draw gets a verdict and nothing under it"
        );
    }

    /// A duel lost to the clock says so, under the table's reason and in
    /// the sheet's own order: the verdict, how the game was decided, why
    /// this seat went out, and that the house answered its last decision.
    /// The winner's seat, still in, gets no line.
    #[test]
    fn a_duel_lost_to_the_clock_says_so_under_the_verdict() {
        let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
        view.seats[0].loss = Some(baylee_view::LossCause::Life);
        view.seats[0].house_answered = Some(baylee_view::HouseAnswer::Clock);
        view.seats[1].house_answered = Some(baylee_view::HouseAnswer::Clock);
        let mut app = app_at(crate::Duel {
            view: Some(view),
            ..ended(
                GameResult {
                    winner: Some(Victor::Player(PlayerId::new(1))),
                    reason: EndReason::LastPlayerStanding,
                },
                None,
            )
        });
        let mut sheets = app
            .world_mut()
            .query_filtered::<&Children, With<FinishSheet>>();
        let children: Vec<Entity> = sheets
            .single(app.world())
            .expect("one sheet")
            .iter()
            .collect();
        let said: Vec<String> = children
            .iter()
            .filter_map(|child| app.world().get::<Text>(*child).map(|t| t.0.clone()))
            .collect();
        assert_eq!(
            said,
            vec![
                Phrase::YouLost.text(Lang::En).to_string(),
                Phrase::EndedLastPlayer.text(Lang::En).to_string(),
                Phrase::LostLifeYou.text(Lang::En).to_string(),
                Phrase::HouseClockYou.text(Lang::En).to_string(),
            ]
        );
        let mut losses = app.world_mut().query_filtered::<Entity, With<FinishLoss>>();
        assert_eq!(losses.iter(app.world()).count(), 2);
    }

    /// A game with no roster cannot say whose win it is, so it says nothing.
    #[test]
    fn a_game_with_no_roster_draws_no_screen() {
        let mut app = app_at(crate::Duel {
            interaction: Some(over(GameResult {
                winner: Some(Victor::Player(PlayerId::new(0))),
                reason: EndReason::EffectWin,
            })),
            ..crate::Duel::default()
        });
        let mut roots = app.world_mut().query_filtered::<Entity, With<FinishRoot>>();
        assert_eq!(roots.iter(app.world()).count(), 0);
    }

    /// The sheet is spawned invisible and is painted by the veil's own number.
    ///
    /// Both halves matter. A sheet that arrived opaque would need no
    /// `Settling` at all, and a sheet painted from a fade of its own could
    /// drift from the veil it is supposed to arrive with — so the test moves
    /// `Veil::lit` by hand and reads the ink back out.
    #[test]
    fn the_sheet_settles_on_the_veils_own_number() {
        let mut app = app_at(ended(
            GameResult {
                winner: Some(Victor::Player(PlayerId::new(0))),
                reason: EndReason::LastPlayerStanding,
            },
            None,
        ));
        let ink = |app: &mut App| {
            let mut found = app.world_mut().query::<(&Settling, &TextColor)>();
            found
                .iter(app.world())
                .filter(|(s, _)| s.ink == Some(palette::PARCHMENT_INK))
                .map(|(_, c)| c.0.alpha())
                .next()
                .expect("a verdict")
        };
        let lift = |app: &mut App| {
            let mut found = app
                .world_mut()
                .query_filtered::<&UiTransform, With<FinishSheet>>();
            found.iter(app.world()).next().expect("a sheet").translation
        };
        assert!(ink(&mut app).abs() < 1e-6, "it is spawned clear");
        assert_eq!(lift(&mut app), Val2::px(0.0, SHEET_LIFT));

        app.world_mut().resource_mut::<Veil>().lit = 0.5;
        app.update();
        let half = ink(&mut app);
        assert!(
            (half - palette::PARCHMENT_INK.alpha() * 0.5).abs() < 1e-6,
            "half way up the veil the ink is half way up too: {half}"
        );
        assert_eq!(lift(&mut app), Val2::px(0.0, SHEET_LIFT * 0.5));

        app.world_mut().resource_mut::<Veil>().lit = 1.0;
        app.update();
        assert!((ink(&mut app) - palette::PARCHMENT_INK.alpha()).abs() < 1e-6);
        assert_eq!(lift(&mut app), Val2::px(0.0, 0.0), "it comes to rest");
    }

    /// A game that has just ended with `lines` lines in its log: a turn's
    /// heading, then mulligans, turn after turn.
    fn logged(lines: u32) -> crate::Duel {
        use baylee_view::{LogEntry, LogEvent, LogTail};
        let view = baylee_client_core::test_support::ViewBuilder::new(2).build();
        let mut duel = crate::Duel {
            view: Some(view.clone()),
            ..ended(
                GameResult {
                    winner: Some(Victor::Player(PlayerId::new(0))),
                    reason: EndReason::LastPlayerStanding,
                },
                None,
            )
        };
        let tail = LogTail {
            from: 0,
            entries: (0..lines)
                .map(|i| LogEntry {
                    turn: i / 3,
                    repeat: 1,
                    event: if i % 3 == 0 {
                        LogEvent::TurnStarted {
                            active: PlayerId::new(0),
                        }
                    } else {
                        LogEvent::Mulliganed {
                            player: PlayerId::new(1),
                        }
                    },
                })
                .collect(),
        };
        duel.log.append(&tail, &view);
        duel
    }

    /// Everything under `root`, itself included.
    fn under(app: &App, root: Entity) -> Vec<Entity> {
        let mut out = vec![root];
        let mut at = 0;
        while let Some(&entity) = out.get(at) {
            if let Some(kids) = app.world().get::<Children>(entity) {
                out.extend(kids.iter());
            }
            at += 1;
        }
        out
    }

    fn the_log(app: &mut App) -> Option<Entity> {
        let mut found = app.world_mut().query_filtered::<Entity, With<FinishLog>>();
        found.iter(app.world()).next()
    }

    /// The whole log is on the sheet, a row for each line, between why the
    /// seats went out and the way out of the screen.
    #[test]
    fn the_whole_log_stands_between_the_losses_and_the_way_out() {
        let duel = logged(7);
        let expected = duel.log.len();
        let mut app = app_at(duel);
        let list = the_log(&mut app).expect("the log is on the sheet");
        let rows = app.world().get::<Children>(list).map_or(0, Children::len);
        assert_eq!(rows, expected, "a row for each of the {expected} lines");

        let mut sheets = app
            .world_mut()
            .query_filtered::<&Children, With<FinishSheet>>();
        let order: Vec<Entity> = sheets
            .single(app.world())
            .expect("one sheet")
            .iter()
            .collect();
        let frame = app.world().get::<ChildOf>(list).expect("a box").parent();
        let exits = order
            .iter()
            .position(|e| app.world().get::<FinishExits>(*e).is_some())
            .expect("the exits");
        let boxed = order.iter().position(|e| *e == frame).expect("the box");
        assert_eq!(boxed + 1, exits, "the log stands right above the way out");
        let caption = app
            .world()
            .get::<Text>(order[boxed - 1])
            .map(|t| t.0.clone());
        assert_eq!(
            caption.as_deref(),
            Some(Phrase::GameLogTitle.text(Lang::En)),
            "and says what it is"
        );
    }

    /// A game that logged nothing gets no box and no caption over one.
    #[test]
    fn a_game_that_logged_nothing_puts_no_log_on_the_sheet() {
        let mut app = app_at(logged(0));
        assert!(the_log(&mut app).is_none(), "an empty log was boxed");
        assert!(
            !lines(&mut app).contains(&Phrase::GameLogTitle.text(Lang::En).to_string()),
            "a caption over nothing"
        );
    }

    /// Every colour in the log goes on clear and rises with the veil, the
    /// rest of the sheet's way: a log painted at rest would stand fully
    /// inked over a sheet that has not arrived yet.
    #[test]
    fn the_log_settles_with_the_sheet() {
        let mut app = app_at(logged(4));
        let list = the_log(&mut app).expect("the log is on the sheet");
        let frame = app.world().get::<ChildOf>(list).expect("a box").parent();
        let loudest = |app: &App| {
            under(app, frame)
                .into_iter()
                .flat_map(|e| {
                    let world = app.world();
                    [
                        world.get::<TextColor>(e).map(|c| c.0.alpha()),
                        world.get::<BackgroundColor>(e).map(|c| c.0.alpha()),
                        world.get::<BorderColor>(e).map(|c| c.top.alpha()),
                    ]
                })
                .flatten()
                .fold(0.0_f32, f32::max)
        };
        assert!(
            loudest(&app) < 1e-6,
            "the log stood inked before the veil rose: {}",
            loudest(&app)
        );

        app.world_mut().resource_mut::<Veil>().lit = 1.0;
        app.update();
        let inks: Vec<f32> = under(&app, frame)
            .into_iter()
            .filter_map(|e| app.world().get::<TextColor>(e).map(|c| c.0.alpha()))
            .collect();
        assert!(!inks.is_empty(), "the log has no text to settle");
        assert!(
            inks.iter().all(|alpha| *alpha > 0.5),
            "a line stayed clear after the veil rose: {inks:?}"
        );
        let edge = app.world().get::<BorderColor>(frame).expect("a box").top;
        assert!(edge.alpha() > 0.5, "the box's edge stayed clear");
    }

    /// The wheel reaches the log once the game is over, and only a game in
    /// hand or just ended: the schedule is the client's own
    /// (`add_input_systems`), whose every other system stops at `Playing`.
    #[test]
    fn a_wheel_scrolls_the_log_after_the_game() {
        use bevy::input::mouse::MouseScrollUnit;
        use bevy::picking::events::{Pointer, Scroll};
        use bevy::picking::pointer::{Location, PointerId};

        use bevy::ecs::system::RunSystemOnce;

        let moved_in = |phase: crate::DuelPhase| {
            let mut app = App::new();
            app.add_plugins(bevy::state::app::StatesPlugin)
                .init_state::<crate::DuelPhase>()
                .add_message::<Pointer<Scroll>>()
                .insert_resource(logged(7))
                .insert_resource(crate::settings::ClientSettings::default())
                .insert_resource(fonts())
                .init_resource::<PreviewScroll>();
            app.world_mut()
                .run_system_once(spawn_finish)
                .expect("the sheet is built");
            crate::add_input_systems(&mut app);
            app.world_mut()
                .resource_mut::<NextState<crate::DuelPhase>>()
                .set(phase);
            app.update();
            let list = the_log(&mut app).expect("the log is on the sheet");
            // Layout never runs here, so the list is told how much of it
            // there is: three windows of it.
            app.world_mut().entity_mut(list).insert(ComputedNode {
                size: Vec2::new(300.0, 300.0),
                content_size: Vec2::new(300.0, 900.0),
                inverse_scale_factor: 1.0,
                ..default()
            });
            // Aimed at a line's sentence, which is what the pointer is over:
            // the row under it and the list under that pass it up.
            let row = app.world().get::<Children>(list).expect("rows")[1];
            let sentence = app.world().get::<Children>(row).expect("a sentence")[0];
            let wheel = Pointer::new(
                PointerId::Mouse,
                Location {
                    target: bevy::camera::NormalizedRenderTarget::Window(
                        bevy::window::WindowRef::Primary
                            .normalize(Some(Entity::PLACEHOLDER))
                            .expect("a window reference"),
                    ),
                    position: Vec2::ZERO,
                },
                Scroll {
                    unit: MouseScrollUnit::Line,
                    x: 0.0,
                    y: -1.0,
                    hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                    phase: bevy::input::touch::TouchPhase::Moved,
                },
                sentence,
            );
            app.world_mut()
                .resource_mut::<Messages<Pointer<Scroll>>>()
                .write(wheel);
            app.update();
            app.world().get::<ScrollPosition>(list).expect("a list").y > 0.0
        };
        assert!(
            moved_in(crate::DuelPhase::Finished),
            "the end sheet's log did not take the wheel"
        );
        assert!(
            !moved_in(crate::DuelPhase::Opening),
            "the wheel ran before the game did, so the test reads no schedule"
        );
    }

    /// The sheet stands over the veil it brought with it.
    ///
    /// It shipped under it. Both are children of the same root, the veil
    /// carries `Z_VEIL` and the sheet carried none, so the veil was painted
    /// over the whole screen — parchment `(224, 212, 176)` reached the window
    /// as `(125, 116, 94)` and brass `(201, 162, 39)` as `(117, 93, 24)`,
    /// each within a unit of what seventy per cent of `TABLE_VEIL` predicts.
    /// It read as a *dimmer* sheet rather than as a bug, which is exactly why
    /// the ladder is asserted and not merely written down.
    #[test]
    fn the_sheet_stands_over_the_veil_it_brought() {
        let mut app = app_at(ended(
            GameResult {
                winner: Some(Victor::Player(PlayerId::new(0))),
                reason: EndReason::LastPlayerStanding,
            },
            None,
        ));
        let mut sheets = app
            .world_mut()
            .query_filtered::<(&ZIndex, &ChildOf), With<FinishSheet>>();
        let (sheet_z, sheet_parent) = sheets.iter(app.world()).next().expect("a sheet");
        let (sheet_z, sheet_parent) = (sheet_z.0, sheet_parent.parent());
        let mut veils = app
            .world_mut()
            .query_filtered::<(&ZIndex, &ChildOf), With<TableVeil>>();
        let (veil_z, veil_parent) = veils.iter(app.world()).next().expect("a veil");
        let (veil_z, veil_parent) = (veil_z.0, veil_parent.parent());
        assert_eq!(
            sheet_parent, veil_parent,
            "the two are ordered against each other only as siblings"
        );
        assert!(
            sheet_z > veil_z,
            "the veil ({veil_z}) is painted over the sheet ({sheet_z})"
        );
    }
}

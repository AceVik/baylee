//! The drawer: what a question needs when it is more than one line.
//!
//! The shelf answers every question that fits on a line — the sentence, the
//! answers, the armed deed, a refusal. This is the other half of AX §5: a
//! panel that grows *upward* out of the shelf for the five things that are
//! more than a line, and that is closed the rest of the time.
//!
//! - a **pick hint**, for a choice that is answered by clicking the table;
//! - combat's **aim** and the **threat** coming the other way;
//! - the **number stepper**;
//! - the **creature-type filter**;
//! - the **indexed chooser** — a colour, a seat, a card, a target.
//!
//! Three things about its shape were measured rather than designed, and each
//! of them is a trap the first draft walked into.
//!
//! **It is not a child of the hand zone.** §5 said it was, and the zone
//! stands on `Overflow::clip()` ([`crate::hud::hand::spawn_hand_zone`]): a
//! child whose bottom edge sits a pixel under the zone's *top* edge and grows
//! upward is cut to that one pixel. Nothing about it would have looked
//! misconfigured — the drawer would simply have been absent. So it is an
//! absolute node of its own in the HUD root, a sibling of the shelf, exactly
//! as the slip row it replaces was.
//!
//! **It does not know its own width, and does not have to.** The centre it
//! stands on is the shelf's middle column, which `arrange` slides off the
//! window's centre when the outer columns need the room. A node the full
//! width of the window, centring its one child, with the slide paid out of
//! one side's padding puts the drawer over the question at any arrangement —
//! [`super::mid_padding`] is the same arithmetic the middle column itself
//! uses, so the two cannot drift.
//!
//! **It has a revision of its own.** [`super::sync_ledge`] despawns every
//! child of the shelf on each rebuild, and [`super::LedgeRevision`] sees none
//! of what this panel draws: the filter text, the picked row, the stepper's
//! value and the combat focus all change with no new snapshot behind them.
//! A drawer governed by the shelf's counter would be a panel that redraws
//! when nothing in it moved and stands still when everything in it did.

use super::*;

use baylee_client_core::Prompt;

/// The narrowest the panel is drawn, so a two-word answer is still a panel.
const MIN_W: f32 = 320.0;

/// The widest, which is the slip's old ceiling: past this a line of prose
/// stops being one line and starts being a paragraph.
const MAX_W: f32 = 620.0;

/// Side padding.
const PAD_X: f32 = 16.0;

/// Top and bottom padding. Smaller than the sides because the bottom edge is
/// not really there — the panel runs into the shelf.
const PAD_Y: f32 = 10.0;

/// Between one row and the next.
const ROW_GAP: f32 = 7.0;

/// The widest a row inside the panel may be: the panel's ceiling less its
/// padding and its two borders. A row that does not bound itself is a row
/// that makes the panel wider than the panel is allowed to be.
const INNER_W: f32 = MAX_W - 2.0 * (PAD_X + 1.0);

/// A written line that is about the choice rather than being one of its
/// answers: the pick hint.
const HINT_PT: f32 = 12.0;

/// Combat's two lines, which are a little louder than the hint because one of
/// them is about damage arriving.
const LINE_PT: f32 = 13.0;

/// The number being chosen, which is the one numeral on this panel and is
/// read rather than pressed.
const NUMBER_PT: f32 = 20.0;

/// A step button is square.
const STEP: f32 = 30.0;

/// Between the two arrows and the number between them.
const STEP_GAP: f32 = 10.0;

/// The mana symbol a chooser row leads with. Large, because for the colour
/// chooser it is the whole answer — there is no word beside it.
const PIP_PT: f32 = 20.0;

/// A cost drawn after a row's words, which is read and not chosen.
const COST_PT: f32 = 15.0;

/// Between a row's parts, and between two rows.
const ROW_INNER_GAP: f32 = 5.0;

/// How much of the candle a picked row is washed with.
///
/// The shelf's own answer to "this one is taken", and deliberately not brass:
/// brass is a light on a *card*, and one register per claim is the rule this
/// whole shelf is built on.
///
/// Measured, because §5 named the strength and nothing had held it to
/// anything: **the wash does not say it.** Laid over [`palette::DIALOG`] it
/// comes out 1.08 : 1 against the fill an unpicked row already has, which is
/// below anything an eye reads as a difference. The border is what says it —
/// [`palette::CANDLE`] against [`palette::DIALOG_LINE`] is 5.52 : 1 — and what
/// the wash is for is that it leaves the words alone: [`palette::DIALOG_INK`]
/// on the washed ground is 11.65 : 1 against 13.74 : 1 on the bare panel.
/// `a_picked_row_is_said_by_its_border` holds both ends of that, so a later
/// hand that strengthens the wash to make it carry the claim on its own is
/// told what it is spending.
pub(super) const PICKED_WASH: f32 = 0.10;

/// Fill, border and ink for a button that stands on **this panel**.
///
/// Deliberately not [`Weight::Secondary`], which every button here used to
/// borrow. That weight is the *dock's* inset key, and when the dock was
/// re-dressed it moved to the dock's own cold ground and champagne edge — so
/// the panel was left carrying two registers, which is the thing `arrow`'s
/// note says §3 exists to stop, and `filter_field` two functions down had
/// gone on naming the panel's own pair directly all along.
///
/// It cost a measured claim, not only a look. A candle border against a
/// champagne one is **2.12 : 1** and does not say which row is taken; against
/// [`palette::DIALOG_LINE`] it is 5.52 : 1. The wash lost its end of it too —
/// 1.26 : 1 against the dock's ground, where the whole point of
/// [`PICKED_WASH`] is that it stays under what an eye reads as a difference.
/// `a_picked_row_is_said_by_its_border` found the first and could not see the
/// second: it was still reading the colour the row no longer had.
pub(super) const PANEL_KEY: (Color, Color, Color) = (
    palette::DIALOG_LIT,
    palette::DIALOG_LINE,
    palette::DIALOG_INK,
);

/// The drawer's positioning node, which outlives every rebuild.
///
/// A marker on the node rather than on the panel, because the panel is what
/// comes and goes: this node is spawned once with the shelf, is exempted from
/// [`super::super::overlay::sync_overlay`]'s sweep by the same name, and is
/// empty whenever there is nothing to draw.
#[derive(Component)]
pub struct DrawerRoot;

/// Where the panel is in its own opening, and which way it is going.
///
/// Its own state and not the sheet's, for the reason `hud/motion.rs` gives:
/// the two are despawned by different systems and a shared component would
/// have to be told which. `t` is reset rather than reversed when the
/// direction changes, because the two spans and the two curves are different
/// — [`motion::ZOOM_OUT`] is shorter than [`motion::ZOOM_IN`] on purpose.
#[derive(Component, Default)]
pub struct DrawerZoom {
    /// 0 at the start of the movement, 1 at its end.
    t: f32,
    /// Whether this is the way out.
    closing: bool,
}

/// One written line above the rows.
#[derive(Clone, PartialEq, Debug)]
struct Line {
    text: String,
    size: f32,
    /// The only thing that varies: an attack that reaches this seat is
    /// written in [`palette::DANGER`] and everything else is quiet.
    ink: Color,
}

/// What the drawer is drawing, so it is not drawn again for nothing.
///
/// Every field is *what came out*, not what went in — the rows as they are
/// written, the value as it stands, the centre as it was placed. That is the
/// stricter comparison of the two: a new snapshot of the game that changes
/// none of them changes nothing on this panel, and rebuilding for it would
/// take a list out from under a pointer that was about to press it.
///
/// There is no `seq` here for exactly that reason, and no `lang`: a language
/// is already in every string below.
#[derive(Resource, Default, Clone, PartialEq, Debug)]
pub struct DrawerRevision {
    /// The hint and combat's two lines, in the order they are drawn.
    lines: Vec<Line>,
    /// The value of the number being chosen, when one is.
    number: Option<u32>,
    /// What has been typed into the creature-type filter, when it is open.
    /// `Some("")` is an open and empty box, which is not the same as no box.
    ///
    /// This is the one field that keeps the drawer open on its own, and
    /// [`DrawerRevision::empty`] leans on it: a creature type is chosen from
    /// three hundred and fifty, so the list is cut to twelve and a filter that
    /// matches nothing leaves **no rows at all**. The box has to stay drawn
    /// there — it is where the typing goes, and a drawer that shut on the
    /// letter that narrowed the list to nothing would take the way out with
    /// it.
    filter: Option<String>,
    /// Every row of the indexed chooser, exactly as [`crate::choices`] wrote
    /// them.
    rows: Vec<crate::choices::ChoiceOption>,
    /// Which row has already been taken.
    picked: Option<usize>,
    /// The slide that puts the panel over the question, in logical pixels of
    /// padding on one side.
    shift: f32,
}

impl DrawerRevision {
    /// Whether there is anything at all to draw.
    fn empty(&self) -> bool {
        self.lines.is_empty() && self.number.is_none() && self.filter.is_none()
            // Rows are drawn even when the list is empty *if* a filter is
            // open — see the filter's own comment — but an empty list with no
            // filter is a chooser with nothing in it, which is no chooser.
            && self.rows.is_empty()
    }
}

/// Where the drawer stands: across the window, over the shelf's top edge.
///
/// Its own function so a test can read it without an app, which is what the
/// slip row it replaces had for the same reason.
pub(in crate::hud) fn root_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        // One pixel of overlap with the shelf's lip, so the panel's own
        // bottom edge and the lip are one line and not two.
        bottom: px(hand::HAND_ZONE_H - 1.0),
        left: px(0),
        right: px(0),
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::Center,
        ..default()
    }
}

/// Spawns the node the panel hangs from, once, beside the shelf.
pub(in crate::hud) fn spawn_drawer_root(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            DrawerRoot,
            root_node(),
            ZIndex(Z_LEDGE),
            // The node is the width of the window and is empty most of the
            // time; a pickable one would take every click aimed at the table
            // under it. The panel inside it is pickable in its own right.
            Pickable::IGNORE,
        ))
        .id()
}

/// Fills the drawer, or empties it.
///
/// Runs after [`super::sync_ledge`], because the centre it stands on is what
/// that system worked out.
#[allow(clippy::too_many_arguments)] // one retained-UI rebuild, like the shelf's
pub fn sync_drawer(
    mut commands: Commands,
    duel: Res<Duel>,
    mut revision: ResMut<DrawerRevision>,
    layout: Res<super::LedgeLayout>,
    root: Query<(Entity, Option<&Children>), With<DrawerRoot>>,
    mut node: Query<&mut Node, With<DrawerRoot>>,
    mut zooms: Query<&mut DrawerZoom>,
    fonts: Res<UiFonts>,
    settings: Res<crate::settings::ClientSettings>,
    texts: Res<crate::cardtext::CardTexts>,
) {
    let Ok((root, standing)) = root.single() else {
        return;
    };
    let lang = Lang::of(&settings.lang);
    let next = reading(&duel, lang, &texts, &layout);
    let shut = next.empty();
    // At most one, and it is the panel. `Option` rather than a loop because
    // everything below asks what state that one panel is in.
    let hanging = standing
        .into_iter()
        .flatten()
        .copied()
        .next()
        .map(|panel| (panel, zooms.get(panel).is_ok_and(|z| z.closing)));
    // Whether the tree is showing an open drawer *right now*. A panel on its
    // way out is not one: the reading it belonged to is already gone, and it
    // is on screen only because §7 gives the drawer a way out as well as a
    // way in.
    let open = hanging.is_some_and(|(_, closing)| !closing);
    // The second half is the same guard the shelf carries and is here for the
    // same reason turned the other way up: a node spawned afresh with the
    // revision still describing the panel that used to hang off it. The tree
    // agrees with the reading when an open panel is exactly what a non-empty
    // reading asked for.
    if *revision == next && open != shut {
        return;
    }
    *revision = next;

    if let Ok(mut node) = node.single_mut() {
        node.padding = super::mid_padding(layout.mid_x, layout.window_w);
    }
    if shut {
        // Sent away rather than despawned. `zoom_the_drawer` is what takes it
        // off the tree, at the end of the movement — and until then its rows
        // are still pickable, which is safe because `input::pick_choice`
        // re-resolves every click against the *current* interaction and
        // answers nothing when there is none.
        if let Some((panel, _)) = hanging
            && let Ok(mut zoom) = zooms.get_mut(panel)
        {
            zoom.closing = true;
            zoom.t = 0.0;
        }
        return;
    }

    // An open panel is kept and refilled. Only its contents changed — the
    // drawer did not open again, and a question that gained a line must not
    // make the whole thing pop a second time.
    let panel = match hanging {
        Some((panel, false)) => {
            commands.entity(panel).despawn_related::<Children>();
            panel
        }
        // Nothing there, or something leaving. A panel that was on its way
        // out cannot be caught and turned round: its `t` runs on the closing
        // span and against the closing curve, so it goes and a fresh one
        // opens in its place on the same frame.
        was => {
            if let Some((leaving, _)) = was {
                commands.entity(leaving).despawn();
            }
            spawn_panel(&mut commands, root)
        }
    };

    for line in &revision.lines {
        let written = sentence(&mut commands, &fonts, &line.text, line.size, line.ink);
        commands.entity(panel).add_child(written);
    }

    if let Some(value) = revision.number {
        let row = stepper(&mut commands, &fonts, value);
        commands.entity(panel).add_child(row);
    }

    if let Some(typed) = revision.filter.clone() {
        let field = filter_field(&mut commands, &fonts, &typed);
        commands.entity(panel).add_child(field);
    }

    if !revision.rows.is_empty() {
        let rows = chooser(&mut commands, &fonts, &revision.rows, revision.picked);
        commands.entity(panel).add_child(rows);
    }
}

/// The panel itself: the drawer's one child, and the thing that moves.
///
/// Spawned already small, the way the sheet is — a panel that appeared at
/// full size for the frame before [`zoom_the_drawer`] first ran would be a
/// flash, and the movement is supposed to be the whole of its arrival.
fn spawn_panel(commands: &mut Commands, root: Entity) -> Entity {
    let panel = commands
        .spawn((
            DrawerZoom::default(),
            UiTransform {
                scale: Vec2::splat(motion::ZOOM_FROM),
                translation: motion::from_bottom(motion::ZOOM_FROM),
                ..default()
            },
            Node {
                min_width: px(MIN_W),
                max_width: px(MAX_W),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(ROW_GAP),
                padding: UiRect::axes(px(PAD_X), px(PAD_Y)),
                // Open at the bottom: the panel grows out of the shelf, and a
                // line across the join would say it is a thing standing on
                // the shelf rather than part of it.
                border: UiRect {
                    left: px(1),
                    right: px(1),
                    top: px(1),
                    bottom: px(0),
                },
                border_radius: BorderRadius {
                    top_left: px(BTN_R),
                    top_right: px(BTN_R),
                    bottom_left: px(0),
                    bottom_right: px(0),
                },
                ..default()
            },
            BackgroundColor(palette::DIALOG),
            BorderColor::all(palette::DIALOG_LINE),
            // Upward, onto the table. The shelf's own shadow falls the other
            // way, onto the cards, and the two together are what make the
            // pair read as one piece of furniture with a drawer out of it.
            BoxShadow(vec![ShadowStyle {
                color: palette::SHADOW,
                x_offset: px(0.0),
                y_offset: px(-6.0),
                spread_radius: px(0.0),
                blur_radius: px(18.0),
            }]),
        ))
        .id();
    commands.entity(root).add_child(panel);
    panel
}

/// Opens and closes the drawer, and takes it off the tree at the end of a
/// close.
///
/// The one movement §7 gives the shelf, and the only one on it that
/// overshoots: the drawer *appears* (like a sheet) where the answers on the
/// shelf merely *change* (like a sentence). What makes it a drawer and not a
/// second sheet is the origin — [`motion::from_bottom`] pins the bottom edge,
/// so it grows out of the lip instead of swelling around its own middle.
///
/// The despawn is here and not in [`sync_drawer`] for the reason the sheet's
/// is in `zoom_the_sheet`: a closing panel has outlived the question it was
/// about, and `sync_drawer` has already written the reading that says so.
pub fn zoom_the_drawer(
    mut commands: Commands,
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut panels: Query<(Entity, &mut DrawerZoom, &mut UiTransform)>,
) {
    let still = prefs.all().reduce_motion;
    for (panel, mut zoom, mut transform) in &mut panels {
        let span = if zoom.closing {
            motion::ZOOM_OUT
        } else {
            motion::ZOOM_IN
        };
        zoom.t = motion::step(zoom.t, span, time.delta_secs(), still);
        if zoom.closing && zoom.t >= 1.0 {
            commands.entity(panel).despawn();
            continue;
        }
        let scale = if zoom.closing {
            motion::shutting(zoom.t)
        } else {
            motion::opening(zoom.t)
        };
        transform.scale = Vec2::splat(scale);
        transform.translation = motion::from_bottom(scale);
    }
}

/// What the drawer would draw right now.
///
/// Separate from the drawing for the reason every revision in this client is:
/// it is run on every frame and the drawing is not, so what it costs is what
/// the comparison costs.
fn reading(
    duel: &Duel,
    lang: Lang,
    texts: &crate::cardtext::CardTexts,
    layout: &super::LedgeLayout,
) -> DrawerRevision {
    let shift = super::mid_shift(layout.mid_x, layout.window_w);
    let over = duel.ending().is_some();
    if over {
        // A finished game asks nothing, and the panel that was open when the
        // last spell resolved must not be left standing over the end screen.
        return DrawerRevision {
            shift,
            ..DrawerRevision::default()
        };
    }
    let waiting = !duel.is_my_turn_to_act();
    // Whether the zone browser's dialog is holding this question. The drawer
    // then draws nothing about *how* to answer: the dialog's own rows are the
    // answer, and "click a card on the board" points at a board behind a veil
    // with nothing on it to click. It is also §5's flat rule — over a dialog
    // stands nothing but the dialog's own sheet.
    let elsewhere = duel.browser.answers_here(duel.interaction.as_ref());
    let mut lines = Vec::new();

    // A choice that is answered by clicking has to say so. The bar used to
    // draw "Discard 1 card(s)" and stop: no button, because nothing is
    // submittable until something is picked, and no hint, because none
    // existed. A player who did not already know to click their hand had no
    // way to find out.
    if let Some(hint) = duel
        .interaction
        .as_ref()
        .filter(|i| !waiting && !elsewhere && i.selected().next().is_none())
        .and_then(|i| pick_hint(&i.prompt()))
    {
        lines.push(Line {
            text: hint.text(lang).to_string(),
            size: HINT_PT,
            ink: palette::DIALOG_SOFT,
        });
    }

    combat_lines(duel, lang, texts, waiting, &mut lines);

    // The one choice with nothing on the table to click. The headline on the
    // shelf says the range; what was missing was the value itself and any way
    // at all to change it with a pointer.
    let number = duel
        .interaction
        .as_ref()
        .filter(|_| !waiting)
        .and_then(|i| matches!(i.prompt(), Prompt::ChooseNumber { .. }).then(|| i.number()));

    // Drawn whether or not anything matches: a filter with no rows under it
    // is exactly when a player needs to see what they typed.
    let filter = duel
        .interaction
        .as_ref()
        .filter(|i| !waiting && matches!(i.prompt(), Prompt::ChooseSubtype { .. }))
        .map(|_| duel.subtype_filter.clone());

    // The client's own cast chooser is asked first and read by the same
    // function: it *is* a `Prompt::CastMode`, built one step before the engine
    // would have built it.
    let rows = duel
        .cast_menu
        .as_ref()
        .map(crate::CastMenu::prompt)
        .or_else(|| {
            duel.interaction
                .as_ref()
                .filter(|_| !waiting)
                .map(baylee_client_core::Interaction::prompt)
        })
        .and_then(|p| {
            crate::choices::options(
                &p,
                lang,
                duel.statics.as_ref(),
                &duel.subtype_filter,
                crate::choices::FaceNames {
                    view: duel.view.as_ref(),
                    texts: Some(texts),
                },
            )
        })
        .unwrap_or_default();
    let picked = duel.cast_menu.as_ref().map_or_else(
        || {
            duel.interaction
                .as_ref()
                .and_then(baylee_client_core::Interaction::chosen_index)
        },
        |m| Some(m.pick),
    );

    DrawerRevision {
        lines,
        number,
        filter,
        rows,
        picked,
        shift,
    }
}

/// Combat's two lines: where this seat is aiming, and what is coming back.
fn combat_lines(
    duel: &Duel,
    lang: Lang,
    texts: &crate::cardtext::CardTexts,
    waiting: bool,
    lines: &mut Vec<Line>,
) {
    let Some(view) = duel.view.as_ref() else {
        return;
    };
    // Combat is the one choice where clicking a creature is not enough: the
    // engine asks *which* defender (CR 508.1b), and a player who cannot see
    // the answer is guessing. This line says where the aim points and how many
    // declarations stand, and it is the same aim the keyboard cycles.
    if let Some(aim) = duel
        .interaction
        .as_ref()
        .filter(|i| i.is_combat() && !waiting)
        .and_then(|i| crate::hud::rail::combat_line(i, view, duel.statics.as_ref(), texts, lang))
    {
        lines.push(Line {
            text: aim,
            size: LINE_PT,
            ink: palette::DIALOG_SOFT,
        });
    }
    // Unlike the aim, this is not about a declaration this seat is making, so
    // it is not filtered on `waiting`: an attack aimed at you while the other
    // side is still choosing blockers is exactly the thing you need to be able
    // to read.
    if let Some((text, threatened)) = crate::hud::rail::incoming_line(
        view,
        duel.interaction.as_ref(),
        duel.statics.as_ref(),
        texts,
        lang,
    ) {
        lines.push(Line {
            text,
            size: LINE_PT,
            ink: if threatened {
                palette::DANGER
            } else {
                palette::DIALOG_INK
            },
        });
    }
}

/// The number, with an arrow either side of it.
fn stepper(commands: &mut Commands, fonts: &UiFonts, value: u32) -> Entity {
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(STEP_GAP),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let minus = arrow(commands, fonts, -1, "\u{2212}");
    let shown = commands
        .spawn((
            Text::new(value.to_string()),
            tf(fonts, NUMBER_PT),
            TextColor(palette::DIALOG_INK),
            Pickable::IGNORE,
        ))
        .id();
    let plus = arrow(commands, fonts, 1, "+");
    commands.entity(row).add_children(&[minus, shown, plus]);
    row
}

/// One of the stepper's two arrows, in the shelf's own register.
///
/// It was brass on parchment, which is the slip's register and the wrong one
/// here: a button on this panel is a button on the shelf, and two registers
/// on one piece of furniture is what §3 is written to stop.
fn arrow(commands: &mut Commands, fonts: &UiFonts, delta: i32, glyph: &str) -> Entity {
    let (fill, edge, ink) = PANEL_KEY;
    commands
        .spawn((
            PromptButton {
                action: PromptAction::Step(delta),
            },
            Node {
                width: px(STEP),
                height: px(STEP),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(edge),
            Feel::new(fill),
            children![(
                Text::new(glyph.to_string()),
                tf_bold(fonts, 17.0),
                TextColor(ink),
                // A label is a node, and a node under the pointer is what the
                // pointer is over.
                Pickable::IGNORE,
            )],
        ))
        .id()
}

/// The type-to-filter box, for the one choice whose list is too long to look
/// at.
fn filter_field(commands: &mut Commands, fonts: &UiFonts, typed: &str) -> Entity {
    let field = commands
        .spawn((
            Node {
                padding: UiRect::axes(px(BUTTON_PAD_X), px(5)),
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(palette::DIALOG_LIT),
            BorderColor::all(palette::DIALOG_LINE),
            Pickable::IGNORE,
        ))
        .id();
    // A caret with nothing before it, so an empty box still reads as somewhere
    // to type rather than as a blank panel.
    let text = commands
        .spawn((
            Text::new(format!("{typed}_")),
            tf(fonts, SENTENCE_PT),
            TextColor(palette::DIALOG_INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(field).add_child(text);
    field
}

/// The indexed chooser: a colour, a seat, a card, a way to cast.
///
/// Its own row and not one of the shelf's answers, because these are neither
/// "OK" nor things to do while holding priority — they are *the* answer, and
/// picking one sends it. Until this existed a tapped dual land drew "Choose a
/// colour" with nothing under it and the game stopped there.
fn chooser(
    commands: &mut Commands,
    fonts: &UiFonts,
    rows: &[crate::choices::ChoiceOption],
    picked: Option<usize>,
) -> Entity {
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(baylee_client_core::ledge::BUTTON_GAP),
                row_gap: px(ROW_GAP),
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for option in rows {
        let on = picked == Some(option.index);
        let (fill, edge, ink) = PANEL_KEY;
        // Taken, rather than louder: a picked row is still one answer among
        // several, so it is washed with the candle instead of being drawn as
        // one. `Weight::Candle` here would put two default answers on one
        // piece of furniture — the row and the shelf's own OK.
        let fill = if on {
            palette::CANDLE.with_alpha(PICKED_WASH)
        } else {
            fill
        };
        let edge = if on { palette::CANDLE } else { edge };
        let button = commands
            .spawn((
                ChoiceButton {
                    index: option.index,
                },
                Node {
                    padding: UiRect::axes(px(BUTTON_PAD_X), px(5)),
                    border: UiRect::all(px(1)),
                    border_radius: btn_radius(),
                    column_gap: px(ROW_INNER_GAP),
                    align_items: AlignItems::Center,
                    // An answer stays on its own panel. The cap is the
                    // drawer's content box and the floor is zero, and both are
                    // needed: without the floor a flex item's automatic
                    // minimum size is its content, so it refuses to shrink and
                    // the cap only moves the overflow; without the cap nothing
                    // bounds a row whose parent is itself sized to fit.
                    //
                    // `Wrap` is about the row's own parts and not about the
                    // sentence: an answer carrying cost marks as well as words
                    // drops the marks onto a second line instead of squeezing
                    // the words into a column beside them. Nothing in the pool
                    // prints both today, so it says what happens when one does.
                    max_width: px(INNER_W),
                    min_width: px(0),
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                },
                BackgroundColor(fill),
                BorderColor::all(edge),
                Feel::new(fill),
            ))
            .id();
        if let Some(pip) = option.pip {
            let mark = crate::manaui::spawn_pip(commands, fonts, pip, PIP_PT);
            commands.entity(button).add_child(mark);
        }
        if !option.label.is_empty() {
            // Rich, not plain: a label carries printed symbols in braces and
            // drawing them as letters is what made "Tap for WUBRG" a sentence
            // nobody could read.
            let text = crate::manaui::spawn_rich(commands, fonts, &option.label, LABEL_PT, ink);
            commands.entity(button).add_child(text);
        }
        if let Some(cost) = option.cost {
            for pip in baylee_client_core::manapip::cost(&cost) {
                let mark = crate::manaui::spawn_pip(commands, fonts, pip, COST_PT);
                commands.entity(button).add_child(mark);
            }
        }
        commands.entity(row).add_child(button);
    }
    row
}

/// The line that says how a choice is answered, when it is answered by
/// clicking something rather than by pressing a button.
///
/// `None` for every choice that draws its own answers, so a hint never appears
/// next to a row of buttons that already says what to do. A creature type is
/// the exception, because there the hint is not about where to click — it is
/// about the box, and a list cut to twelve of three hundred and fifty says
/// nothing about typing on its own.
const fn pick_hint(prompt: &Prompt) -> Option<Phrase> {
    match prompt {
        // The seat's own hand, which the engine does not enumerate because it
        // is already private — `Interaction::selectable` is empty for both.
        Prompt::Discard { .. } | Prompt::BottomCards { .. } => Some(Phrase::HintClickHand),
        Prompt::ChooseCards { .. } | Prompt::ChooseTargets { .. } | Prompt::LegendRule => {
            Some(Phrase::HintClickBoard)
        }
        Prompt::ChooseSubtype { .. } => Some(Phrase::HintTypeToFilter),
        _ => None,
    }
}

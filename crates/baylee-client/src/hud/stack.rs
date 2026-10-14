//! The stack, drawn as cards.
//!
//! Each entry is the spell's own picture — or, for an ability, the picture
//! of the permanent it came from — followed by a row of everything it
//! targets, each drawn as its own smaller card.
//!
//! The next entry is expanded; queued entries use fixed-height compact rows.
//! A virtual scrolling window keeps even trigger storms bounded in UI nodes.
//! Selection and scroll offsets survive retained HUD rebuilds.
//!
//! # Arriving, and resolving
//!
//! A row eases in: it lifts into place, grows the last few percent, and its
//! ink and its fills come up from nothing. A **promotion** is the same
//! movement run the other way — up out of the slot the row was queued in,
//! from a smaller scale, with its rail landing bright and cooling into the
//! accent on a slower ramp. That is the whole of the resolution animation,
//! and it is drawn as a movement rather than as a departure on purpose: the
//! object that resolved is gone from the view, and a ghost of it would be a
//! claim the view no longer makes.
//!
//! What is *not* here: a stagger when several rows land in one frame. Not
//! because the depth is out of reach — [`spawn_stack_panel`] walks the stack
//! in depth order and could bake a delay into the row on the way past, and
//! because the progress lives in [`StackMotion`] rather than in the row,
//! re-baking that delay onto a row already at rest would change nothing.
//! It is the *fade* that makes it expensive: a delay has to reach every node
//! the arrival touches, and [`Arriving`] is built at nineteen places in this
//! file — or [`StackMotion`] holds seconds instead of progress and every one
//! of those nodes remaps it. The panel's own fade and slide already cover the
//! case the owner asked about, so this is "not worth it today" and not
//! "impossible".
//!
//! The progress cannot live in the row, because the HUD is a retained tree
//! rebuilt whenever [`HudRevision`]
//! changes and *hover* is part of that gate — a pointer twitch during the
//! quarter second an arrival takes would despawn the row and spawn it again,
//! and a fade that restarts under the pointer reads as a flicker. It lives in
//! [`StackMotion`] instead, keyed by what the row draws, so a rebuild
//! re-attaches to the arrival already in progress.
//!
//! [`HudRevision`]: super::HudRevision

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use crate::ui_button::UiButton;
use baylee_client_core::card_face::TextBlock;
use baylee_client_core::manapip;

mod cards;
mod entry;
mod motion;
mod sentence;

use cards::{spawn_stack_card, spawn_stack_sentence, spawn_stack_targets, stack_face};
use entry::{bar_colours, fill_bar, spawn_controls, spawn_stack_entry};
pub use motion::ease_the_stack_in;
pub(super) use sentence::*;

/// Remember the user's choice across retained HUD rebuilds.
#[derive(Resource)]
pub struct StackFold {
    collapsed: bool,
    open: f32,
}
impl Default for StackFold {
    fn default() -> Self {
        Self {
            collapsed: false,
            open: 1.0,
        }
    }
}
#[derive(Component)]
pub struct StackBody {
    top: Option<ObjectId>,
    rows: usize,
    /// The least a top row is: [`STACK_FULL_HEIGHT`], or on a phone
    /// [`STACK_PHONE_FULL_HEIGHT`].
    floor: f32,
}
impl StackBody {
    /// Recover the measured full-row height, including when it is a spacer.
    pub(super) fn full_height(&self, node: &ComputedNode, top: Option<ObjectId>) -> f32 {
        if self.top != top {
            return self.floor;
        }
        (node.content_size().y * node.inverse_scale_factor()
            - self.rows.saturating_sub(1) as f32 * STACK_ROW_HEIGHT)
            .max(self.floor)
    }
}
#[derive(Component)]
pub struct StackViewport;
#[derive(Component)]
pub struct StackPanel;
#[derive(Component)]
pub struct StackToggle;
/// What a stack entry points at: the row of its targets' thumbnails.
#[derive(Component)]
pub struct StackTargets;
/// On a phone, the standing answers under the list: shown only while they
/// fit whole beside a top row ([`fold_the_stack`]), never cut in half.
#[derive(Component)]
pub struct StackControls;

/// The box a full row's sentence stands in (the owner, 08.10.2026: *"the
/// effect text area should then be scrollable, including a (visible)
/// scrollbar"*): [`STACK_SENTENCE_LINES`] tall at most, the sentence whole
/// inside it, and what does not fit scrolled rather than cut. The row's name
/// and its targets stand outside it, so neither scrolls away.
///
/// Not pickable, and that is deliberate: a node under the pointer inside the
/// row would take the row's hover, and the row's hover is what lights it,
/// previews it and answers a click on its sentence. A wheel finds the box
/// through the row instead ([`StackTextRow`]).
#[derive(Component, Clone, Copy, Debug)]
pub struct StackTextBox {
    /// The stack object whose sentence it holds.
    pub object: ObjectId,
}

/// The box's scrollbar: Bevy's own, so the thumb drags, shown only while the
/// sentence runs over ([`stack_text`]). Hidden, never taken out of the
/// layout, so the text never reflows when it comes and goes.
#[derive(Component, Clone, Copy, Debug)]
pub struct StackTextBar {
    /// The box it scrolls.
    pub text_box: Entity,
}

/// On a full row: the box a wheel over the row scrolls while its sentence
/// runs over ([`super::scroll::scrolls`]).
#[derive(Component, Clone, Copy, Debug)]
pub struct StackTextRow {
    /// The row's text box.
    pub text_box: Entity,
}

/// How far the top entry's sentence has been scrolled, and whether it runs
/// over at all.
///
/// The overlay is a retained tree rebuilt whenever [`super::HudRevision`]
/// changes — a hover, a log line — and a rebuilt box would start at its top
/// with its bar hidden until the next layout. So the offset lives here,
/// keyed on the object, as the preview's does in
/// [`super::scroll::PreviewScroll`], and a rebuilt box and bar are stood
/// back where they were.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq)]
pub struct StackTextScroll {
    /// The stack object whose box it is.
    pub object: Option<ObjectId>,
    /// How far its box has scrolled, in logical pixels.
    pub offset: f32,
    /// Whether its sentence runs over the box (its bar stands).
    pub runs_over: bool,
    /// A step the keyboard asked for and [`stack_text`] has not taken yet.
    pub nudge: Option<TextNudge>,
}

/// One keyboard step through a stack entry's sentence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextNudge {
    /// A line back towards the start (`↑`).
    LineUp,
    /// A line on (`↓`).
    LineDown,
    /// A box back, less a line (`PgUp`).
    PageUp,
    /// A box on, less a line (`PgDn`).
    PageDown,
}

impl TextNudge {
    /// How far it moves a box `view` pixels tall, in logical pixels: a line
    /// is the sentence's line, and a page keeps one line of the last for the
    /// eye to land on.
    fn travel(self, view: f32) -> f32 {
        let page = (view - STACK_SENTENCE_LINE).max(STACK_SENTENCE_LINE);
        match self {
            Self::LineUp => -STACK_SENTENCE_LINE,
            Self::LineDown => STACK_SENTENCE_LINE,
            Self::PageUp => -page,
            Self::PageDown => page,
        }
    }
}

/// The keys of the top entry's sentence: `↑`/`↓` (the stepper's
/// `NumberUp`/`NumberDown`, so `→`/`←` with them, as the arrangement menu
/// reads them) a line, `PgUp`/`PgDn` a box. Only while the entry has the
/// focus — the pointer or the card cursor on it, which are one `hovered` on
/// this table (TABLE-KEYBOARD §2) — and its sentence runs over; never while
/// a number is being chosen, whose keys the arrows are first.
///
/// Answers whether it took the keys. It only records the step: the box is
/// an entity, and [`stack_text`] moves it.
pub fn stack_text_keys(
    fired: crate::keys::Fired,
    duel: &Duel,
    scroll: Option<&mut StackTextScroll>,
) -> bool {
    use baylee_client_core::prefs::Action;
    let Some(scroll) = scroll else {
        return false;
    };
    let top = duel
        .board
        .as_ref()
        .and_then(|board| board.stack.first())
        .map(|item| item.id);
    if top.is_none()
        || duel.hovered != top
        || scroll.object != top
        || !scroll.runs_over
        || duel.ending().is_some()
        || duel
            .interaction
            .as_ref()
            .is_some_and(baylee_client_core::Interaction::edits_number)
    {
        return false;
    }
    let nudge = if fired.has(Action::NumberUp) {
        TextNudge::LineUp
    } else if fired.has(Action::NumberDown) {
        TextNudge::LineDown
    } else if fired.has(Action::TextPageUp) {
        TextNudge::PageUp
    } else if fired.has(Action::TextPageDown) {
        TextNudge::PageDown
    } else {
        return false;
    };
    scroll.nudge = Some(nudge);
    true
}

/// Keeps the top entry's sentence where it was scrolled to, takes the
/// keyboard's steps, and stands its scrollbar only while the sentence runs
/// over.
///
/// Every write is compared first: this runs every frame, and a box, a bar or
/// a resource written at rest would relay out the HUD or wake what watches
/// it (`docs/perf-client.md`).
pub fn stack_text(
    mut scroll: ResMut<StackTextScroll>,
    added: Query<(Entity, &StackTextBox), Added<StackTextBox>>,
    mut boxes: Query<(&StackTextBox, &mut ScrollPosition, &ComputedNode)>,
    mut bars: Query<(&StackTextBar, &mut Visibility)>,
) {
    // A box spawned this frame: the same entry's stands where it was, and
    // another entry's starts at its top.
    for (entity, text_box) in &added {
        if scroll.object != Some(text_box.object) {
            *scroll = StackTextScroll {
                object: Some(text_box.object),
                ..StackTextScroll::default()
            };
        }
        if let Ok((_, mut position, _)) = boxes.get_mut(entity)
            && moved(position.y, scroll.offset)
        {
            position.y = scroll.offset;
        }
        for (bar, mut shown) in &mut bars {
            if bar.text_box == entity {
                shown.set_if_neq(if scroll.runs_over {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
            }
        }
    }
    let nudge = scroll.nudge;
    for (bar, mut shown) in &mut bars {
        let Ok((text_box, mut position, computed)) = boxes.get_mut(bar.text_box) else {
            continue;
        };
        let scale = computed.inverse_scale_factor();
        let view = computed.size().y * scale;
        // Not laid out yet: what the spawn stood it at holds.
        if view <= 0.0 || scroll.object != Some(text_box.object) {
            continue;
        }
        let content = computed.content_size().y * scale;
        if let Some(nudge) = nudge {
            let to = super::scroll::scrolled(
                position.y,
                nudge.travel(view),
                computed.size().y,
                computed.content_size().y,
                scale,
            );
            if moved(position.y, to) {
                position.y = to;
            }
        }
        let over = crate::face::thumb(view, content, 0.0).is_some();
        shown.set_if_neq(if over {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if scroll.runs_over != over {
            scroll.runs_over = over;
        }
        if moved(scroll.offset, position.y) {
            scroll.offset = position.y;
        }
    }
    if nudge.is_some() {
        scroll.nudge = None;
    }
}

/// Whether two offsets differ: the compare before a write.
fn moved(from: f32, to: f32) -> bool {
    (from - to).abs() > f32::EPSILON
}

type StackViewportQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Node, &'static mut Visibility),
    (With<StackViewport>, Without<StackPanel>),
>;

/// Where the stack panel stands and how tall it may grow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PanelRoom {
    /// Its top edge, from the window's top.
    pub top: f32,
    /// Its right edge, from the window's right.
    pub right: f32,
    /// Its width.
    pub width: f32,
    /// The most it may grow to.
    pub max_height: f32,
    /// Whether the window is a phone's: the panel's
    /// compact shape, [`STACK_PHONE_FULL_HEIGHT`]'s top row. See
    /// [`baylee_client_core::tableview::TableFrame::Phone`].
    pub phone: bool,
}

impl PanelRoom {
    /// The body's ceiling under the panel's own chrome — its padding, its
    /// head and, off a phone, the controls above the list.
    pub(super) fn body_cap(self) -> f32 {
        let chrome = if self.phone {
            2.0 * STACK_PHONE_PAD + STACK_PHONE_HEAD
        } else {
            58.0
        };
        (self.max_height - chrome).max(0.0)
    }
}

/// The stack panel's place in a window `window` logical pixels big, whose
/// hand drawer is drawn `shown` open (0 shut, 1 open; off a phone it is
/// always open) and whose players' strip ends `strip_right` logical pixels
/// from the window's left edge (0 where none stands).
///
/// Off a phone: under the report button, down to the hand zone, as it has
/// always stood. **On a phone** (08./09.10.2026: at 844 × 390 the panel had
/// room for its head and its controls and showed no entry at all — its cap
/// was 103 px, measured against the whole hand zone, which a phone's drawer
/// keeps shut) it stands at the window's top, left of the drawer's tab and
/// of the corner buttons, so it never covers the tab and the corner's top
/// is its own, and it reaches down to the bar where the drawer has it now.
/// The players' strip stands on that bar at its left: the panel narrows to
/// end beside it, and where that would leave too narrow a panel (a narrow
/// phone, many seats) it keeps its width and stops above the strip instead.
#[must_use]
pub(super) fn panel_room(window: Vec2, shown: f32, strip_right: f32) -> PanelRoom {
    use baylee_client_core::tableview::TableFrame;
    if TableFrame::of(window.x, window.y) == TableFrame::Phone {
        let zone_top = window.y - hand::HAND_ZONE_H + super::hand_drawer::drop_at(shown);
        let right = STACK_PHONE_RIGHT;
        let wide = STACK_PANEL_W.min(window.x - right - EDGE).max(0.0);
        let beside = window.x - right - strip_right - 2.0 * STACK_PHONE_GAP;
        let (width, foot) = if beside >= STACK_PHONE_MIN_W {
            (wide.min(beside), zone_top)
        } else {
            (wide, zone_top - super::ledge::players::STRIPS_H)
        };
        PanelRoom {
            top: EDGE,
            right,
            width,
            max_height: (foot - EDGE - STACK_PHONE_AIR).max(0.0),
            phone: true,
        }
    } else {
        PanelRoom {
            top: TOP_CLEAR,
            right: EDGE,
            width: STACK_PANEL_W,
            max_height: (window.y - hand::HAND_ZONE_H - TOP_CLEAR - 36.0)
                .min(window.y * 0.76)
                .max(80.0),
            phone: false,
        }
    }
}

/// On a phone, how far the panel stands from the window's right edge: clear
/// of the hand drawer's tab, which stands at the bar's right end at every
/// drawer position, and so of the report button and the square beside it.
const STACK_PHONE_RIGHT: f32 = EDGE + super::hand_drawer::TAB_W + 8.0;
/// Where the players' strip ends, in logical pixels from the window's left
/// edge, as `bevy_ui` laid it out (a frame old, which a strip standing still
/// does not mind); 0 while none stands.
pub(super) fn strip_right<'a>(
    strips: impl Iterator<
        Item = (
            &'a ComputedNode,
            &'a UiGlobalTransform,
            &'a InheritedVisibility,
        ),
    >,
) -> f32 {
    strips
        .filter(|(computed, _, seen)| seen.get() && computed.size().x > 0.0)
        .map(|(computed, place, _)| {
            (place.translation.x + computed.size().x / 2.0) * computed.inverse_scale_factor
        })
        .fold(0.0, f32::max)
}

/// The players' strip, as [`strip_right`] reads it.
pub(crate) type StripQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static ComputedNode,
        &'static UiGlobalTransform,
        &'static InheritedVisibility,
    ),
    With<super::ledge::players::PlayersStrip>,
>;

/// On a phone, the narrowest the panel is made to end beside the players'
/// strip: a queued card, a name and a target's thumbnail.
const STACK_PHONE_MIN_W: f32 = 240.0;
/// On a phone, the air between the panel's foot and the bar.
const STACK_PHONE_AIR: f32 = 6.0;
/// On a phone, the panel's padding.
const STACK_PHONE_PAD: f32 = 6.0;
/// On a phone, the gap between the panel's parts.
const STACK_PHONE_GAP: f32 = 4.0;
/// On a phone, the panel's head and the gap under it: the toggle's 26 px
/// and the panel's row gap.
const STACK_PHONE_HEAD: f32 = 26.0 + STACK_PHONE_GAP;

/// Writes `value` into `slot` only when it differs: a changed `Node` lays
/// the whole interface out again.
fn set_px(slot: &mut Val, value: f32) -> bool {
    if matches!(*slot, Val::Px(now) if (now - value).abs() <= 0.01) {
        return false;
    }
    *slot = px(value);
    true
}

/// A phone's standing answers, as [`fold_the_stack`] stands or folds them.
type StackControlsQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Node,
        &'static mut Visibility,
        &'static ComputedNode,
    ),
    (
        With<StackControls>,
        Without<StackPanel>,
        Without<StackViewport>,
    ),
>;

/// Animate clipping instead of scaling text, so the stack remains readable;
/// and keep the panel where [`panel_room`] puts it, which on a phone follows
/// the hand drawer.
#[allow(clippy::too_many_arguments)] // a Bevy system: the panel's parts
pub fn fold_the_stack(
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut fold: ResMut<StackFold>,
    windows: Query<&Window>,
    ui: Option<Res<UiScale>>,
    duel: Option<Res<Duel>>,
    mut bodies: StackViewportQuery,
    mut panels: Query<&mut Node, (With<StackPanel>, Without<StackViewport>)>,
    mut controls: StackControlsQuery,
    strips: StripQuery,
    mut toggles: Query<&mut Text, With<StackToggle>>,
) {
    let target = if fold.collapsed { 0.0 } else { 1.0 };
    if (fold.open - target).abs() > f32::EPSILON {
        let open = if prefs.all().reduce_motion {
            target
        } else {
            fold.open + (target - fold.open) * (1.0 - (-16.0 * time.delta_secs()).exp())
        };
        fold.open = if (open - target).abs() < 0.001 {
            target
        } else {
            open
        };
    }
    let room = windows.single().map_or(
        PanelRoom {
            top: TOP_CLEAR,
            right: EDGE,
            width: STACK_PANEL_W,
            max_height: 618.0,
            phone: false,
        },
        |w| {
            panel_room(
                crate::hud::scale::space(w, ui.as_deref()),
                // No table, no drawer to follow: drawn open.
                duel.as_ref().map_or(1.0, |duel| duel.hand_shown),
                strip_right(strips.iter()),
            )
        },
    );
    for mut node in &mut panels {
        // Through `bypass_change_detection` and marked changed only on a
        // real move: every frame used to write all of these.
        let node_ref = node.bypass_change_detection();
        let moved = set_px(&mut node_ref.max_height, room.max_height)
            | set_px(&mut node_ref.top, room.top)
            | set_px(&mut node_ref.right, room.right)
            | set_px(&mut node_ref.width, room.width);
        if moved {
            node.set_changed();
        }
    }
    let cap = room.body_cap() * fold.open;
    // On a phone the body keeps one top row's height whatever stands
    // beside it in the panel: the entry is what the panel is for.
    let floor = if room.phone {
        STACK_PHONE_FULL_HEIGHT.min(cap)
    } else {
        0.0
    };
    for (mut node, mut visibility) in &mut bodies {
        let node_ref = node.bypass_change_detection();
        if set_px(&mut node_ref.max_height, cap) | set_px(&mut node_ref.min_height, floor) {
            node.set_changed();
        }
        visibility.set_if_neq(if fold.open == 0.0 {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        });
    }
    // A phone's standing answers stand while they fit whole under one top
    // row, and fold away (no height, not drawn) where they would be cut —
    // the drawer open on a short phone. What fits is read off what they
    // hold, which a folded node still lays out.
    for (mut node, mut visibility, computed) in &mut controls {
        let wants = computed.content_size().y * computed.inverse_scale_factor();
        // Nothing measured yet (a node spawned this frame) is not a fit.
        let fits = wants > 0.0
            && fold.open > 0.0
            && room.body_cap() - STACK_PHONE_FULL_HEIGHT - STACK_PHONE_GAP >= wants;
        let node_ref = node.bypass_change_detection();
        let ceiling = if fits { Val::Auto } else { px(0) };
        if node_ref.max_height != ceiling {
            node_ref.max_height = ceiling;
            node.set_changed();
        }
        visibility.set_if_neq(if fits {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
    let mark = if fold.collapsed { "+" } else { "−" };
    for mut text in &mut toggles {
        if text.0 != mark {
            **text = mark.into();
        }
    }
}

/// The card a stack entry is drawn at, at the top of the panel.
const STACK_CARD_W: f32 = 72.0;
/// Height of that card.
const STACK_CARD_H: f32 = STACK_CARD_W * 88.0 / 63.0;
/// The card a *queued* entry is drawn at — about two thirds of the top one,
/// which is the whole of the ordering cue.
const STACK_QUEUED_W: f32 = 46.0;
/// Height of that card.
const STACK_QUEUED_H: f32 = STACK_QUEUED_W * 88.0 / 63.0;
/// The smaller card a *target* is drawn at, so the two never read as peers:
/// the thing on the stack is the sentence, its targets are its objects.
const STACK_TARGET_W: f32 = 40.0;
/// Height of a target thumbnail.
const STACK_TARGET_H: f32 = STACK_TARGET_W * 88.0 / 63.0;
/// A target of a queued entry, at the same two thirds.
const STACK_QUEUED_TARGET_W: f32 = 26.0;
/// Height of that thumbnail.
const STACK_QUEUED_TARGET_H: f32 = STACK_QUEUED_TARGET_W * 88.0 / 63.0;

/// Panel width.
///
/// It was 296, which was wide enough for a card, an arrow and three targets
/// and not wide enough for the **names**. The name column of a queued row is
/// the panel less the padding either side, the row's own padding, the rail,
/// the card and the gap — `W − 20 − 8 − 3 − card − 8` — which came to 187 px,
/// and 187 px is not a name: measured over every face in the pool with the
/// advance widths of Inter, which the client shipped then, 58 of the 1475
/// (3.9%) did not fit at the size they are drawn, and a further two dozen
/// were cut by the estimator below although they would have. (Faustina is
/// the narrower face for a *name* — 3 of 1475 would overrun that column, not
/// 58 — so the widening is not what the serif needed. It is what the
/// estimator needed, and that argument is unchanged.)
///
/// At **352** — a fifth of the 1728-pixel window this client is developed
/// against, so still a panel and not a second window — that column is 267 px
/// and **every** face in the pool fits, with the longest
/// (`Okina, Temple to the Grandfathers`, 202 px at 14 in Faustina, 245 in
/// Inter) still 65 px short of the edge.
const STACK_PANEL_W: f32 = 352.0;

/// Bounded visible rows, including overscan for smooth scrolling.
const STACK_COMPACT_ROWS: usize = 16;
pub(super) const STACK_FULL_HEIGHT: f32 = 164.0;
/// The top row on a phone: a queued row's card, and beside it the name with
/// its targets on one line and the sentence's two lines under them — no
/// subtitle (the head says whose answer is awaited). Its padding either side
/// and the taller of the card and that column.
pub(super) const STACK_PHONE_FULL_HEIGHT: f32 = 2.0 * STACK_PHONE_ROW_PAD
    + STACK_QUEUED_H
        .max(STACK_QUEUED_TARGET_H + 3.0 + STACK_SENTENCE_LINES_PHONE * STACK_SENTENCE_LINE);
/// The top row's padding on a phone, as on a desktop's.
const STACK_PHONE_ROW_PAD: f32 = 6.0;
const STACK_ROW_HEIGHT: f32 = 82.0;

/// First rendered queued row, including a small overscan above the viewport.
pub(super) fn window_start(scroll: f32, full_height: f32) -> usize {
    (((scroll - full_height).max(0.0) / STACK_ROW_HEIGHT) as usize).saturating_sub(2)
}

fn rows_height(start: usize, end: usize, full_height: f32) -> f32 {
    if end <= start {
        return 0.0;
    }
    (end - start) as f32 * STACK_ROW_HEIGHT
        + if start == 0 {
            full_height - STACK_ROW_HEIGHT
        } else {
            0.0
        }
}

fn spacer(commands: &mut Commands, height: f32) -> Entity {
    commands
        .spawn((
            Node {
                height: px(height),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// The name on the full row, which is the largest thing in the panel.
const STACK_NAME_PT: f32 = 16.0;
/// Its line box, stated rather than left to [`bevy::text`]'s 1.2 default,
/// because [`STACK_NAME_LINES`] is a height in pixels and has to know it.
const STACK_NAME_LINE: f32 = STACK_NAME_PT * super::SERIF_SCALE * 1.2;
/// How many lines of it the full row will give up its height for.
///
/// Two, and it is the one place in the panel that wraps. A name is what the
/// row *is*, and the nine faces in the pool that do not fit 237 px at this
/// size are the ones a player is most likely not to know by sight — the
/// legendary lands with a title after the comma. The cap is a `max_height`
/// over a clipped node rather than a claim about the pool: a printing this
/// client has never seen gets two lines and a clean edge, not a third line
/// that pushes the queue out of the panel.
///
/// The queued rows below stay at one line. Their height is what the panel is
/// budgeted against, and at 267 px every face in the pool fits on it.
const STACK_NAME_LINES: f32 = 2.0;
/// The name on a queued row.
const STACK_QUEUED_NAME_PT: f32 = 14.0;

/// The printed sentence under the name on the full row.
///
/// Twelve, the size the subtitle beside it is already set at, and four under
/// the name: the name is what the row *is* and stays the largest thing on it,
/// while the sentence and the subtitle are both things the row says about
/// itself and read as one block at one size.
const STACK_SENTENCE_PT: f32 = 12.0;
/// How many lines of that sentence the row shows at once.
///
/// Measured rather than chosen: the body is about 237 px wide, which is
/// around 38 characters at this size, and the longest loyalty ability in the
/// pool is 113 characters — three lines. Four is that plus the slack word
/// wrapping leaves at the end of a line. It was a *cut* until 08.10.2026:
/// past it the sentence ended in an ellipsis, and on a row of a fixed height
/// the lines it did keep pushed the targets out of the bottom of the row —
/// the owner's report, *"you can't see the target"*. It is the height of a
/// box now ([`StackTextBox`]): the sentence is whole inside it, and what
/// does not fit scrolls, under a scrollbar that stands only while it does.
pub(super) const STACK_SENTENCE_LINES: f32 = 4.0;
/// The same box on a phone's short window (below
/// [`crate::shellkit::size::PHONE_HEIGHT`]), where the panel under the
/// corner buttons is a few lines tall in all and four would hide the rest of
/// the row behind the sentence.
pub(super) const STACK_SENTENCE_LINES_PHONE: f32 = 2.0;
/// One line of that sentence, in logical pixels: its size, the serif's
/// scale and [`STACK_LINE`]. Set on the text as its line height, so the box
/// is a whole number of lines tall and never shows half of one.
pub(super) const STACK_SENTENCE_LINE: f32 = STACK_SENTENCE_PT * super::SERIF_SCALE * STACK_LINE;
/// The air between the text box and its scrollbar.
const STACK_TEXT_GAP: f32 = 4.0;

/// How many lines of a full row's sentence its box shows in a window this
/// tall.
pub(super) fn text_lines(window_height: f32) -> f32 {
    if window_height < crate::shellkit::size::PHONE_HEIGHT {
        STACK_SENTENCE_LINES_PHONE
    } else {
        STACK_SENTENCE_LINES
    }
}
/// A mana mark in that sentence, as a fraction of the prose's own size.
///
/// The same 0.72 [`crate::manaui::spawn_pip`] sets a glyph at inside its
/// disc, and it is the right number for a different reason here: the marks
/// have to sit at the prose's cap height, and the font makes them nearly a
/// full em tall — a colour pip measures 1.001 em out of `mana.ttf`, so 0.72
/// of the size is 0.72 of the line.
///
/// Rasterized at the device pixels this sentence is drawn at on a Retina
/// screen, the pip stands **17.3** px against the prose's cap. The prose has
/// changed face underneath it and the number did not move: Faustina is asked
/// for [`super::SERIF_SCALE`] times the nominal 12 and caps at 0.648 em,
/// which is 17.1 px; Inter was asked for 12 flat and capped at 0.728 em,
/// which was 17.5. A taller size in a shorter face and a shorter size in a
/// taller one land within a pixel of each other, so 0.72 is the match for
/// both — which is luck, and is recorded here so a third face is *measured*
/// rather than assumed to inherit it. The tap arrow is the font's shortest
/// mark at 0.784 em and lands at 13.5, reading as the heavier glyph it is
/// rather than as a taller one.
const STACK_MARK: f32 = 0.72;

/// A planeswalker's loyalty badge standing before the sentence, as a fraction
/// of the prose's own size.
///
/// The 0.88 [`crate::manaui::spawn_rich`] sets every symbol at inside a line
/// of prose, so an initial here and a mark quoted mid-sentence are set to one
/// another exactly as they are on the ability sheet. It is deliberately not
/// [`STACK_MARK`]: that one is a *glyph* out of `mana.ttf` that has to reach
/// the prose's cap height, and this is a shape built out of two boxes whose
/// size is its own.
const STACK_INITIAL: f32 = 0.88;

/// How tall one line of that sentence is, as a share of its font size.
///
/// `bevy_text`'s own default, written out because the initial is centred
/// against the **first** line: a badge centred on the whole block would sit
/// halfway down a four-line paragraph.
const STACK_LINE: f32 = 1.2;

/// The air between the initial and the sentence it stands before.
const STACK_INITIAL_GAP: f32 = 6.0;

/// How fast a row arrives, per second.
///
/// The same exponential everything in this client eases with, at a rate that
/// puts a row at about nine tenths after a fifth of a second. Deliberately
/// slower than [`crate::ambience::FEEL_RATE`]: that one answers a pointer,
/// where any delay is lag, and this one announces an event the player did not
/// necessarily cause.
const ARRIVE_RATE: f32 = 13.0;

/// How far above its resting place an entry starts, in logical pixels.
///
/// Up rather than down, because the stack grows downwards on the screen and a
/// new object lands on *top* of it: a row rising into the gap it just made
/// says which end of the queue it joined.
const ARRIVE_LIFT: f32 = 14.0;

/// The same for the panel, which slides rather than grows.
const PANEL_LIFT: f32 = 10.0;

/// The scale an arriving row starts at.
///
/// Small enough to read as a movement towards the reader, far enough from
/// zero that the row is never a dot: a card that scaled from nothing would
/// be an effect, and this is a notification.
const ARRIVE_SCALE: f32 = 0.96;

/// How far *below* its resting place a promoted row starts.
///
/// The other direction, and that is the whole of the resolution animation.
/// A row is promoted when the object above it has resolved and left, which is
/// the one event in this panel a player most wants to see — and it was being
/// drawn as an arrival, lifting into the slot from above, which says the
/// opposite of what happened. A promotion comes *up* from the row it was in.
const PROMOTE_LIFT: f32 = 10.0;

/// The scale a promoted row starts at.
///
/// Below [`ARRIVE_SCALE`] on purpose: an arriving row lands and a promoted
/// one *grows*, having been drawn at two thirds the size a moment earlier.
const PROMOTE_SCALE: f32 = 0.92;

/// How fast a promoted row's rail cools from its landing colour to its own.
///
/// Slower than [`ARRIVE_RATE`], so the light is still settling after the row
/// has stopped moving — about 290 ms to nine tenths. That lag is the point:
/// the accent rail was on the row that resolved a moment ago, and a bright
/// mark appearing one slot lower and cooling into place is "a spell resolved"
/// drawn as the movement it is, without drawing a ghost of the object the
/// view no longer carries.
const SETTLE_RATE: f32 = 8.0;

/// What the pointer and the standing question say about a row.
///
/// Three facts that arrive together and are read together, bundled so the
/// panel's builders take one argument for them rather than three. They are
/// the same three the hand zone reads — a spell on the stack is a legal
/// target of anything that says "target spell", and a player choosing one is
/// doing exactly what they do in the hand.
pub(super) struct Picks<'a> {
    /// The object under the pointer, or the one the keyboard cursor names.
    pub hovered: Option<ObjectId>,
    /// What has been picked towards the answer so far.
    pub selected: &'a [ObjectId],
    /// What the pending question would accept.
    pub selectable: &'a [ObjectId],
}

impl Picks<'_> {
    /// The light a row's picture wears, if it wears one.
    ///
    /// The same light the hand zone draws, at the same three weights and from
    /// the same function — deliberately, and it is the whole argument for
    /// putting this on the *picture* rather than on the row. "The rules will
    /// accept this as an answer" is one claim, and a player who met it as a
    /// glow around a card in their hand must meet it as a glow around a card
    /// here; two grammars for one fact is two things to learn.
    ///
    /// It also leaves the row's own teal alone. The rail marks a **slot** —
    /// a position in the queue, drawn as a bar — and this marks an
    /// **object**, drawn as a glow around a picture, so a counterspell aimed
    /// at the top of the stack reads as "this card, in this slot" rather than
    /// as a louder slot.
    fn light(&self, object: ObjectId) -> Option<BoxShadow> {
        let hovered = self.hovered == Some(object);
        if self.selected.contains(&object) {
            return Some(super::hand::halo(palette::CANDLE, 1.0));
        }
        if self.selectable.contains(&object) {
            return Some(super::hand::halo(
                palette::CANDLE,
                if hovered { 0.85 } else { 0.70 },
            ));
        }
        // A row that is not an answer to anything still hovers; its ground
        // says so, and a glow would be claiming the question accepts it.
        None
    }
}

/// Which part of the panel a node's arrival is remembered under.
///
/// The *shape* is part of the key on purpose. A row promoted from compact to
/// full — which is exactly what a resolution looks like from the panel's side
/// — is a different drawing of the same object, and a fresh key is what makes
/// the promotion ease rather than cut. It also means departure needs no
/// animation at all: the object that resolved is gone from the view, drawing
/// a ghost of it would be drawing something the view no longer carries, and
/// the *visible* event is the new top rising into the full row — which it
/// now literally does, up out of the slot it was queued in rather than down
/// from above. See [`PROMOTE_LIFT`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StackKey {
    /// The panel itself, which arrives when the stack stops being empty.
    Panel,
    /// One entry, and whether it is drawn as the full row.
    Entry(ObjectId, bool),
}

/// One row's movement into its place.
#[derive(Clone, Copy)]
struct Rise {
    /// The row this is about.
    key: StackKey,
    /// How far along it is, `0.0`…`1.0`.
    at: f32,
    /// Whether the row **grew** into this slot rather than landing in it —
    /// the object above it resolved. It comes up from below and starts
    /// smaller; see [`PROMOTE_LIFT`].
    promoted: bool,
    /// The promoted row's rail, on its own slower ramp; see [`SETTLE_RATE`].
    /// Unused by anything that merely arrived.
    cooled: f32,
}

/// How far each row of the stack panel has arrived, `0.0`…`1.0`.
///
/// A `Vec` rather than a map: a stack with more than a dozen objects on it is
/// a rules oddity rather than a case to optimise for, and a linear scan over
/// eight entries is cheaper than hashing one.
#[derive(Resource, Default)]
pub struct StackMotion {
    /// One entry per row on screen, in no particular order.
    rows: Vec<Rise>,
}

impl StackMotion {
    /// How far `key` has arrived; `1.0` for anything not being tracked, so a
    /// node whose row has finished is simply drawn.
    fn progress(&self, key: StackKey) -> f32 {
        self.rows
            .iter()
            .find(|rise| rise.key == key)
            .map_or(1.0, |rise| rise.at)
    }

    /// The whole of `key`'s movement, for the row itself — which is the one
    /// node that needs to know *which way* it came.
    fn rise(&self, key: StackKey) -> Option<Rise> {
        self.rows.iter().copied().find(|rise| rise.key == key)
    }
}

/// Which of a node's colours the arrival is written to.
///
/// Load-bearing rather than tidy. [`Node`] *requires* a [`BackgroundColor`]
/// and a [`BorderColor`], so every node in this panel carries all three
/// colours whether it draws them or not — a line of text has a background of
/// [`Color::NONE`]. A system that simply wrote whichever colour it found
/// would take that transparent black, multiply its alpha back up and give
/// every label in the panel an opaque plate. The first live shot of this
/// panel is exactly that: eight black rectangles where the words should be.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Paints {
    /// A line of text: its [`TextColor`] and nothing else.
    Ink,
    /// A fill: its [`BackgroundColor`].
    Fill,
    /// A fill that runs the other way — opaque while the row arrives and gone
    /// once it has landed. It is how the card picture fades: the art is a
    /// [`MaterialNode`] on a material shared with every card that looks the
    /// same (`a_look_is_shared_by_exactly_what_looks_the_same`), so an alpha
    /// written there would fade the player's whole hand with it.
    Veil,
}

/// A node that eases in with the row it belongs to.
///
/// Every piece of a row carries one — the row's own fill, each line of text,
/// each chip — because [`bevy_ui`] has no opacity inheritance: fading a
/// subtree means writing an alpha per entity, and an entity that knows its
/// own row needs no tree walk to be found.
#[derive(Component)]
pub struct Arriving {
    /// The row whose progress this node follows.
    key: StackKey,
    /// The alpha the node is drawn at once it has arrived.
    base: f32,
    /// Which colour that alpha belongs to.
    paints: Paints,
}

impl Arriving {
    /// Text that fades up to the alpha of the colour it is set in.
    const fn ink(key: StackKey, base: f32) -> Self {
        Self {
            key,
            base,
            paints: Paints::Ink,
        }
    }

    /// A fill that fades up to the alpha of the colour it is drawn in.
    const fn fill(key: StackKey, base: f32) -> Self {
        Self {
            key,
            base,
            paints: Paints::Fill,
        }
    }

    /// A veil over a card picture, opaque until the row has landed.
    const fn veil(key: StackKey) -> Self {
        Self {
            key,
            base: 1.0,
            paints: Paints::Veil,
        }
    }

    /// The alpha this node should be drawn at, `progress` of the way in.
    fn alpha(&self, progress: f32) -> f32 {
        self.base
            * if self.paints == Paints::Veil {
                1.0 - progress
            } else {
                progress
            }
    }
}

/// The row (or the panel) itself, which also lifts and grows into place.
#[derive(Component)]
pub struct ArrivingRow {
    /// How far above its resting place it starts, in logical pixels.
    lift: f32,
    /// The scale it starts at; `1.0` for something that only slides.
    from: f32,
    /// The border it rests at, so the accent rail on the top row arrives with
    /// the row rather than standing there alone while the row fades in behind
    /// it. Kept here rather than read back off the node, because the resting
    /// colour of a row with no rail is [`Color::NONE`] and reading *that* back
    /// as a base is the black-plate trap one component up.
    rail: Color,
}

/// The stack, next-to-resolve at the top; pinned left of the phase rail.
///
/// Drawn as cards rather than as a list of names, because the stack is the one
/// place in the game where "what is about to happen, and to what" has to be
/// read in a hurry — and a player who has to match two names against a board
/// of twelve permanents is doing the engine's bookkeeping by hand. Each entry
/// is the spell (or the permanent whose ability it is), an arrow, and a
/// picture of everything it points at.
#[allow(clippy::too_many_arguments)] // a panel, a view, and the material store
#[allow(clippy::too_many_lines)] // a title, a queue and a tally, built flat
pub(super) fn spawn_stack_panel(
    commands: &mut Commands,
    selected: Option<ObjectId>,
    orders: &[baylee_client_core::automation::AbilityOrder],
    scroll: ScrollPosition,
    (full_height, text_lines, room): (f32, f32, PanelRoom),
    lang: Lang,
    board: &baylee_client_core::BoardModel,
    view: &PlayerView,
    statics: &GameStatic,
    picks: &Picks<'_>,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
    mut cards: Option<&mut UiCards<'_>>,
) -> Entity {
    let key = StackKey::Panel;
    let panel = commands
        .spawn((
            StackPanel,
            Node {
                position_type: PositionType::Absolute,
                // Under the report button (#309), which has the corner: the
                // draw offer and the concession that used to sit above it
                // are on the shelf (AX §4.3). On a phone at the top, left of
                // the corner and the hand's tab (`panel_room`).
                right: px(room.right),
                top: px(room.top),
                width: px(room.width),
                max_height: px(room.max_height),
                flex_direction: FlexDirection::Column,
                row_gap: px(if room.phone { STACK_PHONE_GAP } else { 6.0 }),
                padding: UiRect::all(px(if room.phone { STACK_PHONE_PAD } else { 10.0 })),
                overflow: Overflow::clip(),
                border_radius: BorderRadius::all(px(5)),
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(palette::DIALOG),
            BorderColor::all(palette::DIALOG_LINE),
            ZIndex(Z_STACK),
            upward_shadow(),
            Pickable::IGNORE,
            Arriving::fill(key, palette::DIALOG.alpha()),
            ArrivingRow {
                lift: PANEL_LIFT,
                from: 1.0,
                rail: palette::DIALOG_LINE,
            },
        ))
        .id();

    // One row and not two. The title and the note about whose answer the
    // table is waiting for are one line of information, and stacking them
    // spent a second line of a panel that has to fit `STACK_COMPACT_ROWS`
    // entries under it — and read as a second heading rather than as a note
    // beside the first.
    let head = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: px(8),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(panel).add_child(head);

    let title = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(6),
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
            children![
                (
                    Text::new(Phrase::StackTitle.text(lang)),
                    tf(fonts, 13.0),
                    TextColor(palette::MUTED),
                    Arriving::ink(key, palette::MUTED.alpha()),
                ),
                (
                    Node {
                        padding: UiRect::axes(px(6.0), px(1.0)),
                        border_radius: BorderRadius::all(px(8)),
                        ..default()
                    },
                    BackgroundColor(palette::DIALOG_LIT),
                    Arriving::fill(key, palette::DIALOG_LIT.alpha()),
                    children![(
                        Text::new(board.stack.len().to_string()),
                        tf(fonts, 13.0),
                        TextColor(palette::INK),
                        Arriving::ink(key, palette::INK.alpha()),
                    )],
                ),
            ],
        ))
        .id();
    commands.entity(head).add_child(title);

    // Whose answer the table is waiting for. It is the one thing about the
    // stack the prompt slip cannot say — the slip speaks to *this* seat, and
    // between two of an opponent's spells there is nothing on it at all —
    // and it costs one lookup. Nothing once the game is over, which is the
    // only question the engine asks nobody.
    if let Some(holder) = view.awaiting {
        let waiting = commands
            .spawn((
                Text::new(waiting_line(
                    lang,
                    statics.seat_name(holder),
                    holder == view.seat,
                )),
                tf(fonts, 11.0),
                TextColor(palette::MUTED),
                Arriving::ink(key, palette::MUTED.alpha()),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(head).add_child(waiting);
    }

    let toggle = commands
        .spawn((
            UiButton,
            Node {
                width: px(28),
                height: px(26),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(palette::DIALOG_LIT),
            BorderColor::all(palette::DIALOG_LINE),
            Feel::tinting_to(palette::DIALOG_LIT, palette::CANDLE_WASH_LIT),
            children![(
                StackToggle,
                Text::new("−"),
                tf_bold(fonts, 19.0),
                TextColor(palette::CANDLE),
                Pickable::IGNORE
            )],
        ))
        .observe(|mut click: On<PointerClick>, mut fold: ResMut<StackFold>| {
            click.propagate(false);
            fold.collapsed = !fold.collapsed;
        })
        .id();
    commands.entity(head).add_child(toggle);
    // Above the list, except on a phone, where the list comes first and the
    // controls give way to it (`spawn_controls`).
    if !room.phone {
        spawn_controls(commands, panel, selected, orders, lang, view, fonts, false);
    }
    let start = window_start(scroll.y, full_height).min(board.stack.len().saturating_sub(1));
    let end = (start + STACK_COMPACT_ROWS).min(board.stack.len());
    let body = commands
        .spawn((
            StackBody {
                top: board.stack.first().map(|item| item.id),
                rows: board.stack.len(),
                floor: if room.phone {
                    STACK_PHONE_FULL_HEIGHT
                } else {
                    STACK_FULL_HEIGHT
                },
            },
            Scrolls,
            Node {
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                min_height: px(0),
                min_width: px(0),
                flex_grow: 1.0,
                ..default()
            },
            scroll,
        ))
        .id();
    let track = super::ledge::log::scrollbar(commands, body, bar_colours(), fill_bar);
    let viewport = commands
        .spawn((
            StackViewport,
            Node {
                min_height: px(0),
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
                overflow: Overflow::clip(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .add_children(&[body, track])
        .id();
    commands.entity(panel).add_child(viewport);

    if start > 0 {
        let gap = spacer(commands, rows_height(0, start, full_height));
        commands.entity(body).add_child(gap);
    }
    for item in &board.stack[start..end] {
        let entry = spawn_stack_entry(
            commands,
            lang,
            item,
            item.depth == 0,
            selected == Some(item.id),
            view,
            statics,
            picks,
            textures,
            assets,
            fonts,
            faces,
            cards.as_deref_mut(),
            (text_lines, room),
        );
        commands.entity(body).add_child(entry);
    }

    if end < board.stack.len() {
        let gap = spacer(commands, rows_height(end, board.stack.len(), full_height));
        commands.entity(body).add_child(gap);
    }
    if room.phone {
        spawn_controls(commands, panel, selected, orders, lang, view, fonts, true);
    }
    panel
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod folding_tests;

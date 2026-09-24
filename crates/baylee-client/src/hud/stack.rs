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
use baylee_client_core::card_face::TextBlock;
use baylee_client_core::manapip;

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
pub struct StackBody;
#[derive(Component)]
pub struct StackToggle;

/// Animate clipping instead of scaling text, so the stack remains readable.
pub fn fold_the_stack(
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut fold: ResMut<StackFold>,
    windows: Query<&Window>,
    mut bodies: Query<(&mut Node, &mut Visibility), With<StackBody>>,
    mut toggles: Query<&mut Text, With<StackToggle>>,
) {
    let target = if fold.collapsed { 0.0 } else { 1.0 };
    fold.open = if prefs.all().reduce_motion {
        target
    } else {
        fold.open + (target - fold.open) * (1.0 - (-16.0 * time.delta_secs()).exp())
    };
    if (fold.open - target).abs() < 0.001 {
        fold.open = target;
    }
    let cap = windows
        .single()
        .map_or(460.0, |w| (w.height() * 0.62 - 58.0).max(80.0));
    for (mut node, mut visibility) in &mut bodies {
        node.max_height = px(cap * fold.open);
        *visibility = if fold.open == 0.0 {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    for mut text in &mut toggles {
        **text = if fold.collapsed { "+" } else { "−" }.into();
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
const STACK_FULL_HEIGHT: f32 = 164.0;
const STACK_ROW_HEIGHT: f32 = 82.0;

/// First rendered queued row, including a small overscan above the viewport.
pub(super) fn window_start(scroll: f32) -> usize {
    (((scroll - STACK_FULL_HEIGHT).max(0.0) / STACK_ROW_HEIGHT) as usize).saturating_sub(2)
}

fn rows_height(start: usize, end: usize) -> f32 {
    if end <= start {
        return 0.0;
    }
    (end - start) as f32 * STACK_ROW_HEIGHT
        + if start == 0 {
            STACK_FULL_HEIGHT - STACK_ROW_HEIGHT
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
/// How many lines of that sentence the row will give up its height for.
///
/// Measured rather than chosen: the body is about 237 px wide, which is
/// around 38 characters at this size, and the longest loyalty ability in the
/// pool is 113 characters — three lines. Four is that plus the slack word
/// wrapping leaves at the end of a line, and it is a *cut*, not a wrap limit:
/// past it the sentence ends in an ellipsis so one pathological card cannot
/// push the queue out of the panel.
const STACK_SENTENCE_LINES: f32 = 4.0;
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

/// Reconciles what is being tracked against what is on screen.
///
/// Three rules, and each is a different answer to "this key was not here last
/// frame".
///
/// A row that steps **down** is not an arrival. When a spell lands on a stack
/// that already had one, yesterday's top is drawn queued from this frame on —
/// a different key for the same object, which would otherwise be seeded at
/// nothing and fade in beside the newcomer, so two rows would announce
/// themselves and only one of them would be new. Carrying the progress across
/// the key change leaves it standing where it was, and it is the *current*
/// progress and not `1.0` because the house AI answers within a frame or two
/// of priority: the common case is a spell demoted while it is still
/// arriving, and seeding that at rest would snap it to full.
///
/// A row that steps **up** is a promotion — the object above it resolved —
/// and that one is *not* carried across, because it is the movement the
/// player is meant to see. It is marked instead, and moves the other way; see
/// [`PROMOTE_LIFT`]. The marking has to happen here, before the retain below
/// drops the queued key the question is asked about.
///
/// Anything else is new and starts at nothing.
fn track(motion: &mut StackMotion, live: &[StackKey]) {
    for &key in live {
        let StackKey::Entry(id, false) = key else {
            continue;
        };
        let known = motion.rows.iter().any(|rise| rise.key == key);
        let stepped_down = motion
            .rows
            .iter()
            .find(|rise| rise.key == StackKey::Entry(id, true))
            .map(|rise| rise.at);
        if !known && let Some(progress) = stepped_down {
            motion.rows.push(Rise {
                key,
                at: progress,
                promoted: false,
                cooled: 1.0,
            });
        }
    }

    let promoted: Vec<StackKey> = live
        .iter()
        .copied()
        .filter(|key| {
            let StackKey::Entry(id, true) = *key else {
                return false;
            };
            !motion.rows.iter().any(|rise| rise.key == *key)
                && motion
                    .rows
                    .iter()
                    .any(|rise| rise.key == StackKey::Entry(id, false))
        })
        .collect();

    motion.rows.retain(|rise| live.contains(&rise.key));
    for &key in live {
        if !motion.rows.iter().any(|rise| rise.key == key) {
            let promoted = promoted.contains(&key);
            motion.rows.push(Rise {
                key,
                at: 0.0,
                promoted,
                // Only a promotion has a rail to cool. Seeded at nothing for
                // everything else, the slower ramp would keep `moving` true
                // for a third of a second after the row had settled, and
                // every node in the panel would go on being written to with
                // nothing left to say.
                cooled: if promoted { 0.0 } else { 1.0 },
            });
        }
    }
}

/// Eases every arriving row towards its resting state.
///
/// Runs after [`sync_overlay`] and deliberately: a row spawned this frame is
/// spawned at *rest*, and without the ordering it would be drawn once at full
/// strength before this system ever saw it — a one-frame flash at the top of
/// the panel, which is the one place in the interface a player is watching.
///
/// [`sync_overlay`]: super::sync_overlay
pub fn ease_the_stack_in(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut motion: ResMut<StackMotion>,
    mut fades: Query<(
        &Arriving,
        Option<&mut TextColor>,
        Option<&mut BackgroundColor>,
    )>,
    mut rows: Query<(&Arriving, &ArrivingRow, &mut UiTransform, &mut BorderColor)>,
) {
    // Which rows are on screen. Anything else has resolved, or the panel has
    // gone with the last object on it, and its progress goes with it — an id
    // kept after its row left would greet the *next* spell of that name with
    // no animation at all.
    //
    // `fades` is the wider query — a row carries an [`Arriving`] too — so it
    // is the one that sees every key.
    let mut live: Vec<StackKey> = Vec::new();
    for (arriving, ..) in &fades {
        if !live.contains(&arriving.key) {
            live.push(arriving.key);
        }
    }
    track(&mut motion, &live);

    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let ease = |rate: f32| {
        if still {
            1.0
        } else {
            1.0 - (-rate * time.delta_secs()).exp()
        }
    };
    let step = ease(ARRIVE_RATE);
    let settle = ease(SETTLE_RATE);
    let mut moving = false;
    for rise in &mut motion.rows {
        if rise.at >= 1.0 && rise.cooled >= 1.0 {
            continue;
        }
        rise.at = (rise.at + (1.0 - rise.at) * step).min(1.0);
        if rise.at > 0.999 {
            rise.at = 1.0;
        }
        rise.cooled = (rise.cooled + (1.0 - rise.cooled) * settle).min(1.0);
        if rise.cooled > 0.999 {
            rise.cooled = 1.0;
        }
        moving = true;
    }
    // Every node is spawned at rest, so a panel that has finished arriving
    // needs nothing written to it at all — including on the frame it is
    // rebuilt under the pointer.
    if !moving {
        return;
    }

    for (arriving, ink, fill) in &mut fades {
        let alpha = arriving.alpha(motion.progress(arriving.key));
        match arriving.paints {
            Paints::Ink => {
                if let Some(mut ink) = ink {
                    ink.0 = ink.0.with_alpha(alpha);
                }
            }
            Paints::Fill | Paints::Veil => {
                if let Some(mut fill) = fill {
                    fill.0 = fill.0.with_alpha(alpha);
                }
            }
        }
    }
    for (arriving, row, mut transform, mut border) in &mut rows {
        let rise = motion.rise(arriving.key);
        let progress = rise.map_or(1.0, |rise| rise.at);
        // Which way the row came, which is the whole of the difference
        // between a spell being cast and a spell resolving. An arrival drops
        // in from above (negative y is up); a promotion comes up from the
        // slot below and grows, because that is what happened to it.
        let (lift, from) = if rise.is_some_and(|rise| rise.promoted) {
            (PROMOTE_LIFT, PROMOTE_SCALE)
        } else {
            (-row.lift, row.from)
        };
        transform.translation.y = px(lift * (1.0 - progress));
        transform.scale = Vec2::splat(from + (1.0 - from) * progress);
        // The rail lands bright and cools into its own colour, on the slower
        // ramp, so the light is still settling after the row has stopped —
        // an accent mark appearing one slot down from where it was is the
        // resolution, told without a ghost of the object that left.
        let rail = match rise {
            Some(rise) if rise.promoted => palette::INK.mix(&row.rail, rise.cooled),
            _ => row.rail,
        };
        *border = BorderColor::all(rail.with_alpha(row.rail.alpha() * progress));
    }
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
            Node {
                position_type: PositionType::Absolute,
                right: px(EDGE),
                // The corner is the panel's own now: the draw offer and the
                // concession that used to sit above it are on the shelf
                // (AX §4.3), so there is nothing left up here to clear.
                top: px(EDGE),
                width: px(STACK_PANEL_W),
                max_height: percent(62),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                padding: UiRect::all(px(10)),
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
            Button,
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
        .observe(
            |mut click: On<Pointer<Click>>, mut fold: ResMut<StackFold>| {
                click.propagate(false);
                fold.collapsed = !fold.collapsed;
            },
        )
        .id();
    commands.entity(head).add_child(toggle);
    spawn_controls(commands, panel, selected, orders, lang, view, fonts);
    let start = window_start(scroll.y).min(board.stack.len().saturating_sub(1));
    let end = (start + STACK_COMPACT_ROWS).min(board.stack.len());
    let body = commands
        .spawn((
            StackBody,
            Scrolls,
            Node {
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                min_height: px(0),
                ..default()
            },
            scroll,
        ))
        .id();
    commands.entity(panel).add_child(body);

    if start > 0 {
        let gap = spacer(commands, rows_height(0, start));
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
        );
        commands.entity(body).add_child(entry);
    }

    if end < board.stack.len() {
        let gap = spacer(commands, rows_height(end, board.stack.len()));
        commands.entity(body).add_child(gap);
    }
    panel
}

fn control_button(
    commands: &mut Commands,
    parent: Entity,
    label: &str,
    active: bool,
    fonts: &UiFonts,
) -> Entity {
    let button = commands
        .spawn((
            Button,
            Node {
                min_height: px(28),
                padding: UiRect::axes(px(8), px(5)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(4)),
                border: UiRect::all(px(1)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(if active {
                palette::CANDLE_WASH_LIT
            } else {
                palette::DIALOG_LIT
            }),
            BorderColor::all(if active {
                palette::CANDLE
            } else {
                palette::DIALOG_LINE
            }),
            children![(
                Text::new(label),
                tf(fonts, 11.0),
                TextColor(palette::INK),
                Pickable::IGNORE
            )],
        ))
        .id();
    commands.entity(parent).add_child(button);
    button
}

fn pass_check(commands: &mut Commands, fonts: &UiFonts, enabled: bool) -> Entity {
    commands
        .spawn((
            Node {
                width: px(14),
                height: px(14),
                margin: UiRect::right(px(6)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(3)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BorderColor::all(palette::CANDLE),
            Pickable::IGNORE,
            children![(
                Text::new(if enabled {
                    glyph::CHECK.to_string()
                } else {
                    String::new()
                }),
                icon_tf(fonts, 10.0),
                TextColor(palette::CANDLE),
                Pickable::IGNORE,
            )],
        ))
        .id()
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn spawn_controls(
    commands: &mut Commands,
    panel: Entity,
    selected: Option<ObjectId>,
    orders: &[baylee_client_core::automation::AbilityOrder],
    lang: Lang,
    view: &PlayerView,
    fonts: &UiFonts,
) {
    use baylee_client_core::automation::{ability_order, set_ability_order};
    use baylee_engine::choice::StandingAnswer;
    let marked = selected.and_then(|id| view.stack.iter().find(|item| item.id == id));
    let controls = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(5),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(panel).add_child(controls);
    let hint = if marked.is_some() {
        Phrase::StackStopHint
    } else {
        Phrase::StackSelectHint
    };
    let text = commands
        .spawn((
            Text::new(hint.text(lang)),
            tf(fonts, 10.0),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(controls).add_child(text);
    let running = view.priority_held;
    let label = if running {
        Phrase::HoldRelease
    } else if marked.is_some() {
        Phrase::StackRunTo
    } else {
        Phrase::StackRun
    };
    let button = control_button(commands, controls, label.text(lang), running, fonts);
    commands
        .entity(button)
        .observe(|mut click: On<Pointer<Click>>, mut duel: ResMut<Duel>| {
            click.propagate(false);
            if let Some(action) = duel.hold_action(false) {
                duel.submit(action);
            }
        });
    let ability = marked
        .or_else(|| view.stack.last())
        .and_then(|item| match item.stack_item {
            Some(baylee_view::StackItem::Ability { ability, .. }) => ability,
            _ => None,
        });
    if let Some(ability) = ability {
        let order = ability_order(orders, ability);
        let title = commands
            .spawn((
                Text::new(Phrase::StackAbilityPolicy.text(lang)),
                tf(fonts, 10.0),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(controls).add_child(title);
        let pass = control_button(
            commands,
            controls,
            Phrase::StackAlwaysPass.text(lang),
            order.pass,
            fonts,
        );
        let mark = pass_check(commands, fonts, order.pass);
        commands.entity(pass).insert_children(0, &[mark]);
        commands.entity(pass).observe(
            move |mut click: On<Pointer<Click>>, mut prefs: ResMut<crate::prefs::Prefs>| {
                click.propagate(false);
                let mut order = ability_order(&prefs.all().ability_orders, ability);
                order.pass = !order.pass;
                set_ability_order(&mut prefs.edit().ability_orders, order);
            },
        );
        let choices = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(4),
                    flex_wrap: FlexWrap::Wrap,
                    row_gap: px(4),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(controls).add_child(choices);
        for (label, answer) in [
            (Phrase::StackAsk, None),
            (Phrase::StackAlwaysYes, Some(StandingAnswer::Yes)),
            (Phrase::StackAlwaysNo, Some(StandingAnswer::No)),
        ] {
            let button = control_button(
                commands,
                choices,
                label.text(lang),
                order.answer == answer,
                fonts,
            );
            commands.entity(button).observe(
                move |mut click: On<Pointer<Click>>, mut prefs: ResMut<crate::prefs::Prefs>| {
                    click.propagate(false);
                    let mut order = ability_order(&prefs.all().ability_orders, ability);
                    order.answer = answer;
                    set_ability_order(&mut prefs.edit().ability_orders, order);
                },
            );
        }
        let hint = commands
            .spawn((
                Text::new(Phrase::StackPolicyHint.text(lang)),
                tf(fonts, 10.0),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(controls).add_child(hint);
    }
    if !orders.is_empty() {
        let reset = control_button(
            commands,
            controls,
            Phrase::StackResetRules.text(lang),
            false,
            fonts,
        );
        commands.entity(reset).observe(
            |mut click: On<Pointer<Click>>, mut prefs: ResMut<crate::prefs::Prefs>| {
                click.propagate(false);
                prefs.edit().ability_orders.clear();
            },
        );
    }
}

/// One row of the stack panel: the object, what it is, and what it points at.
///
/// `full` is the top of the stack — the sentence the panel is about. Every
/// other row is the queue behind it and is drawn at two thirds, with no
/// subtitle and no arrow: a queued entry answers "what else is coming", and
/// the seat and the kind are questions a player asks about the thing that is
/// *next*.
#[allow(clippy::too_many_arguments)] // the same slot arguments, one level down
#[allow(clippy::too_many_lines)] // two shapes of one row, built flat
fn spawn_stack_entry(
    commands: &mut Commands,
    lang: Lang,
    item: &baylee_client_core::board::StackItem,
    full: bool,
    marked: bool,
    view: &PlayerView,
    statics: &GameStatic,
    picks: &Picks<'_>,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
    mut cards: Option<&mut UiCards<'_>>,
) -> Entity {
    let key = StackKey::Entry(item.id, full);
    // The next thing to resolve is lit and railed; the one behind it carries
    // a hint of the same fill so "this resolves second" is visible, and
    // everything under *that* is flat. Depth is the only ordering a player
    // has to trust here, and past the second row it is carried by position.
    //
    // Under the pointer each of the three is one step lighter than what it
    // already had, which is how a hover can mean the same thing on rows that
    // are not peers. It is built into the tree rather than animated by a
    // [`Feel`], and that is particular to this panel: `ease_the_stack_in`
    // writes this very `BackgroundColor` for as long as a row is arriving,
    // and a second writer with its own opinion about the alpha would fight it
    // for a quarter of a second every time a spell is cast. The overlay is
    // rebuilt whenever the hover changes anyway — `HudRevision` counts it —
    // so the row can simply be *built* lit, which is what the hand zone has
    // always done with its halo.
    let hovered = picks.hovered == Some(item.id);
    let fill = if marked {
        palette::CANDLE_WASH_LIT
    } else if full {
        if hovered {
            palette::PANEL_HOT
        } else {
            palette::DIALOG_LIT
        }
    } else if item.depth == 1 {
        palette::DIALOG_LIT.with_alpha(if hovered { 0.62 } else { 0.45 })
    } else if hovered {
        palette::DIALOG_LIT.with_alpha(0.30)
    } else {
        Color::NONE
    };
    let rail = if full || marked {
        palette::CANDLE
    } else {
        Color::NONE
    };
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                height: px(if full {
                    STACK_FULL_HEIGHT
                } else {
                    STACK_ROW_HEIGHT
                }),
                flex_shrink: 0.0,
                overflow: Overflow::clip(),
                column_gap: px(8),
                padding: UiRect::all(px(if full { 6.0 } else { 4.0 })),
                border: UiRect::left(px(3)),
                align_items: AlignItems::FlexStart,
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(rail),
            // The whole row answers for the spell on it, and that is the fix
            // for a duel that could not be played: a `ChooseTargets` whose
            // only option was a spell on the stack had no way to be answered
            // at all, because the panel's one pickable node was the 66-pixel
            // picture and every other node — the row included — carried
            // `Pickable::IGNORE`. `Interaction::toggle` has always taken "a
            // spell on the stack"; what was missing was a way to *say* it.
            // A click anywhere on the row — the picture, the name, the
            // printed sentence — now goes through `activate_card` like a
            // click on a card in the hand, and lands on `toggle` for the
            // same reason it does there: nothing earlier in that chain is
            // true of an object on the stack.
            HandCardVisual { object: item.id },
            crate::hud::StackRowCard,
            Arriving::fill(key, fill.alpha()),
            ArrivingRow {
                lift: ARRIVE_LIFT,
                from: ARRIVE_SCALE,
                rail,
            },
        ))
        .id();

    let (width, height) = if full {
        (STACK_CARD_W, STACK_CARD_H)
    } else {
        (STACK_QUEUED_W, STACK_QUEUED_H)
    };
    let art = spawn_stack_card(
        commands,
        lang,
        key,
        // The row speaks for the entry; see `spawn_stack_card`.
        None,
        item.art,
        stack_face(item, view, faces, textures),
        width,
        height,
        statics,
        textures,
        assets,
        fonts,
        cards.as_deref_mut(),
    );
    // The light that says the pending question would take this spell as its
    // answer, and the brighter one that says it already has. It replaces the
    // slot's drop shadow rather than joining it, exactly as in the hand:
    // `BoxShadow` is one component, and a card is lit or it is at rest.
    if let Some(light) = picks.light(item.id) {
        commands.entity(art).insert(light);
    }
    commands.entity(row).add_child(art);

    let body = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                flex_grow: 1.0,
                min_width: px(0),
                overflow: Overflow::clip_x(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_child(body);

    // The body is the panel less its padding, the row's own padding, the rail
    // and the card. The two rows spend it differently, and that is the whole
    // of the ramp once more: the **full** row's name may wrap to
    // [`STACK_NAME_LINES`], because it is the thing the row is about and nine
    // faces in the pool are longer than one line of it; a **queued** name is
    // cut to fit one line, because its height is what the panel is budgeted
    // against and at 267 px every face in the pool fits on it anyway.
    let ink = if full { palette::CANDLE } else { palette::INK };
    let size = if full {
        STACK_NAME_PT
    } else {
        STACK_QUEUED_NAME_PT
    };
    let room = STACK_PANEL_W - 20.0 - if full { 12.0 } else { 8.0 } - 3.0 - width - 8.0;
    // The name a player reads rather than the one the engine projects. It has
    // to be looked up here and not taken from `item.name`, because
    // `BoardModel` is built in a crate that links neither the catalog nor a
    // socket to fetch it over.
    let title = view.object(item.id).map_or_else(
        || item.name.clone(),
        |o| crate::face::name_of(o, view, faces.texts),
    );
    let title = heading(item, full, title, view, faces);
    let mut name = commands.spawn((
        // A card's name is set in the card's face, not the interface's:
        // Faustina carries every printed word this client draws, wherever
        // the ground under it happens to be.
        super::tf_serif(fonts, size, 400),
        TextColor(ink),
        Arriving::ink(key, ink.alpha()),
        // The row is what the pointer is on, and a label is a node: left
        // pickable it would take the hover for itself and the row around
        // it would never light.
        Pickable::IGNORE,
    ));
    if full {
        name.insert((
            Text::new(title),
            bevy::text::LineHeight::Px(STACK_NAME_LINE),
            // A cap in pixels rather than a claim about the pool: a printing
            // this client has never seen gets two lines and a clean edge
            // instead of a third line that pushes the queue out of the panel.
            Node {
                max_height: px(STACK_NAME_LINES * STACK_NAME_LINE),
                overflow: Overflow::clip(),
                ..default()
            },
        ));
    } else {
        name.insert((
            Text::new(fit(&title, room, size)),
            TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
        ));
    }
    let name = name.id();
    commands.entity(body).add_child(name);

    // What kind of thing this is, and whose — on the top row only. An ability
    // names its source even when the source has left: the picture above may
    // be missing, the sentence must not be. A *spell* names nothing, because
    // the picture already said it.
    if full {
        let kind = match item.kind {
            baylee_client_core::board::StackKind::Spell => None,
            baylee_client_core::board::StackKind::Ability { source, .. } => {
                Some(view.object(source).map_or_else(
                    || Phrase::StackAbilityBare.text(lang).to_string(),
                    |o| {
                        Phrase::StackAbility
                            .fill(lang, &[&crate::face::name_of(o, view, faces.texts)])
                    },
                ))
            }
        };
        let subtitle = commands
            .spawn((
                Text::default(),
                super::tf_serif(fonts, STACK_SENTENCE_PT, 400),
                TextColor(palette::MUTED),
                Arriving::ink(key, palette::MUTED.alpha()),
                Pickable::IGNORE,
            ))
            .id();
        if let Some(kind) = kind {
            let span = commands
                .spawn((
                    TextSpan::new(format!("{kind} — ")),
                    super::tf_serif(fonts, STACK_SENTENCE_PT, 400),
                    TextColor(palette::MUTED),
                    Arriving::ink(key, palette::MUTED.alpha()),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(subtitle).add_child(span);
        }
        // The seat in the slant, and only the seat: it is the one word of the
        // subtitle that names a *person*, and italic Inter at this size over
        // this ground is legible for a word and tiring for a sentence.
        let seat = commands
            .spawn((
                TextSpan::new(if item.controller == view.seat {
                    Phrase::You.text(lang).to_string()
                } else {
                    statics.seat_name(item.controller).to_string()
                }),
                super::tf_serif_italic(fonts, STACK_SENTENCE_PT, 400),
                TextColor(palette::MUTED),
                Arriving::ink(key, palette::MUTED.alpha()),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(subtitle).add_child(seat);
        commands.entity(body).add_child(subtitle);

        // What the ability *does*, in the player's own printing and
        // language. This is the whole reason the host says which sentence a
        // stack entry is: "+1" tells a player nothing, and a stack of three
        // triggers that all read `Ability · Ondu Cleric` tells them less.
        //
        // The full row only. A queued row answers "what else is coming", and
        // six sentences stacked under one another would be a wall of text
        // where the size ramp used to carry the order.
        if let Some(blocks) = stack_sentence(item, view, faces) {
            let line = spawn_stack_sentence(commands, fonts, key, blocks, room);
            commands.entity(body).add_child(line);
        }
    }

    if !item.targets.is_empty() {
        let targets = spawn_stack_targets(
            commands, lang, item, key, full, view, statics, textures, assets, fonts, faces, cards,
        );
        commands.entity(body).add_child(targets);
    }

    row
}

/// The printed sentence on a full row, with a planeswalker's badge before it.
///
/// A walker prints its cost at the head of the line — `+2: Look at the top
/// card…` — and that badge is the mark a player recognises a walker's ability
/// by. Set as letters it is three characters of prose at the front of a
/// sentence; set as the shape the card draws, it is an **initial**, and what
/// is left beside it is what the ability does. Both halves come from one cut
/// ([`baylee_client_core::abilitysheet::loyalty_cut`]), so the cost cannot be
/// both drawn and printed, nor dropped without being drawn.
///
/// Everything else on the stack keeps its whole line. A cost paid in mana is
/// already a row of marks inside the sentence and has no second shape to
/// stand as, and a trigger has no cost at all.
fn spawn_stack_sentence(
    commands: &mut Commands,
    fonts: &UiFonts,
    key: StackKey,
    blocks: Vec<TextBlock>,
    room: f32,
) -> Entity {
    use baylee_client_core::abilitysheet;

    let (initial, blocks) = abilitysheet::loyalty_cut(blocks);
    let sentence = commands
        .spawn((
            Text::default(),
            super::tf_serif(fonts, STACK_SENTENCE_PT, 400),
            TextColor(palette::INK),
            Arriving::ink(key, palette::INK.alpha()),
            Pickable::IGNORE,
            if initial.is_some() {
                // Sharing the line with the badge, and able to shrink back
                // off its own natural width — prose that cannot shrink pushes
                // the row wide instead of wrapping inside it.
                Node {
                    flex_grow: 1.0,
                    flex_basis: Val::Auto,
                    min_width: px(0),
                    ..default()
                }
            } else {
                Node::default()
            },
        ))
        .id();

    // One budget across the spans, spent in printed order, so a long ability
    // cannot grow the row past the queue it is ordering — and so the reminder
    // is what gets cut first, which is the order a player would drop them in
    // too. The initial takes its own width out of that room first, or the cut
    // would be measured against pixels the prose no longer has.
    let prose = room
        - initial.map_or(0.0, |_| {
            STACK_SENTENCE_PT * STACK_INITIAL * crate::manaui::BADGE_SPAN + STACK_INITIAL_GAP
        });
    let room = budget(prose * STACK_SENTENCE_LINES, STACK_SENTENCE_PT);
    for piece in spans_of(&blocks, Some(room)) {
        let ink = if piece.reminder {
            palette::MUTED
        } else {
            palette::INK
        };
        let span = commands
            .spawn((
                TextSpan::new(piece.text),
                if piece.mark {
                    crate::manaui::mana_tf(fonts, STACK_SENTENCE_PT * STACK_MARK)
                } else if piece.reminder {
                    super::tf_serif_italic(fonts, STACK_SENTENCE_PT, 400)
                } else {
                    super::tf_serif(fonts, STACK_SENTENCE_PT, 400)
                },
                TextColor(ink),
                Arriving::ink(key, ink.alpha()),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(sentence).add_child(span);
    }

    let Some(loy) = initial else {
        return sentence;
    };
    spawn_walker_line(commands, fonts, key, loy, sentence)
}

/// The badge and the sentence, as one line.
///
/// The badge is laid on the **panel**, not on parchment, so it is drawn the
/// other way up from the sheet's: a light body carrying a dark number. That
/// is the same claim the sheet's dark-on-parchment badge makes, made on the
/// ground this panel actually has — a `PARCHMENT_INK` lozenge here would be a
/// hole in the panel with nothing legible in it.
fn spawn_walker_line(
    commands: &mut Commands,
    fonts: &UiFonts,
    key: StackKey,
    loy: manapip::Loyalty,
    sentence: Entity,
) -> Entity {
    let line = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::FlexStart,
                column_gap: px(STACK_INITIAL_GAP),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    // Exactly one line tall, with the badge centred in it: an initial belongs
    // on the sentence's first line, and a badge aligned to the block would
    // either ride above that line's own middle or sink into the paragraph.
    let seat = commands
        .spawn((
            Node {
                height: px(STACK_SENTENCE_PT * super::SERIF_SCALE * STACK_LINE),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let badge = crate::manaui::spawn_loyalty_badge(
        commands,
        fonts,
        loy,
        STACK_SENTENCE_PT * STACK_INITIAL,
        palette::INK,
        palette::DIALOG_LIT,
    );
    // There is no subtree opacity in `bevy_ui`, so each piece of the badge
    // arrives on the row's own progress or none of it does — and a solid mark
    // standing at full strength over a row that has not faded in yet is the
    // one-frame flash [`ease_the_stack_in`] exists to prevent. Both pieces are
    // **ink** now: the body is the badge's own glyph rather than two painted
    // boxes, so a `fill` here would animate a `BackgroundColor` nothing has.
    commands
        .entity(badge.body)
        .insert(Arriving::ink(key, palette::INK.alpha()));
    commands
        .entity(badge.numeral)
        .insert(Arriving::ink(key, palette::DIALOG_LIT.alpha()));
    commands.entity(seat).add_child(badge.root);
    commands.entity(line).add_children(&[seat, sentence]);
    line
}

/// The arrow and everything a stack entry points at, as one wrapping row.
#[allow(clippy::too_many_arguments)] // the same slot arguments again
fn spawn_stack_targets(
    commands: &mut Commands,
    lang: Lang,
    item: &baylee_client_core::board::StackItem,
    key: StackKey,
    full: bool,
    view: &PlayerView,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
    mut cards: Option<&mut UiCards<'_>>,
) -> Entity {
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
                align_items: AlignItems::Center,
                flex_wrap: FlexWrap::Wrap,
                row_gap: px(4),
                margin: UiRect::top(px(2)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    // The arrow belongs to the sentence, not to the queue: a queued row is
    // already the narrow one, and a glyph that is a third of its body width
    // buys nothing a thumbnail sitting under a name does not already say.
    if full {
        let arrow = commands
            .spawn((
                Text::new("→"),
                tf(fonts, 14.0),
                TextColor(palette::CANDLE),
                Arriving::ink(key, palette::CANDLE.alpha()),
                // A label, and the row's hover is the row's — as above.
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(row).add_child(arrow);
    }

    for target in &item.targets {
        let chip = spawn_stack_target(
            commands,
            lang,
            target,
            key,
            full,
            view,
            statics,
            textures,
            assets,
            fonts,
            faces,
            cards.as_deref_mut(),
        );
        commands.entity(row).add_child(chip);
    }
    row
}

/// The face a stack entry should show instead of its art, if any.
///
/// A spell on the stack has its own object and its own projected text; an
/// ability has neither, so it keeps its picture and its name.
fn stack_face(
    item: &baylee_client_core::board::StackItem,
    view: &PlayerView,
    faces: &FaceCtx<'_>,
    textures: &CardTextures,
) -> Option<CardFace> {
    let object = view.object(item.id)?;
    faces.object(object, textures, item.art)
}

/// A card in the stack panel: its art, its face, or a plain plate with the
/// card's back colour when the seat may not know what it is.
#[allow(clippy::too_many_arguments)] // the slot, the card, and the stores
fn spawn_stack_card(
    commands: &mut Commands,
    lang: Lang,
    key: StackKey,
    object: Option<ObjectId>,
    art: Option<ImageKey>,
    face: Option<CardFace>,
    width: f32,
    height: f32,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    cards: Option<&mut UiCards<'_>>,
) -> Entity {
    let slot = commands
        .spawn((
            Node {
                width: px(width),
                height: px(height),
                flex_shrink: 0.0,
                border_radius: card_radius(width),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(palette::DIALOG_LIT),
            soft_shadow(),
            Arriving::fill(key, palette::DIALOG_LIT.alpha()),
        ))
        .id();
    // Who speaks for this picture. A stack card is drawn an inch across —
    // enough to recognise a spell, nowhere near enough to read one — and the
    // stack is where a player most needs to read. `HandCardVisual` is what
    // `pointer_hover` looks for, and it already speaks for every card the HUD
    // draws rather than the felt; a card whose object this seat may not know
    // (something cast face down) reports nothing and simply does not preview.
    //
    // An **entry** passes `None` and is not the exception it looks like: its
    // whole row carries the component instead, so that the name and the
    // printed sentence answer for the spell as well as the picture does. A
    // picture that stayed pickable inside a pickable row would also swallow
    // the row's own hover — a `Node` under the pointer is the hover, and its
    // parent is then not hovered at all — which is the mistake the hand zone's
    // labels have already made once. A **target** chip is its own object and
    // keeps its own handle.
    match object {
        Some(object) => {
            commands.entity(slot).insert(HandCardVisual { object });
        }
        None => {
            commands.entity(slot).insert(Pickable::IGNORE);
        }
    }
    if let Some(key) = art {
        let image = textures.get(key, statics, assets);
        let visual = spawn_card_art(
            commands,
            lang,
            image,
            face.as_ref(),
            width,
            height,
            crate::face::Detail::Compact,
            fonts,
            // Nothing on the stack is on a battlefield, so no glow: the
            // border says what the rules have made a *permanent*, and a
            // spell wearing one would be claiming something untrue.
            CardLook::art(key, finish_of(statics, Some(key)), 0),
            cards,
        );
        commands.entity(slot).add_child(visual);
    }
    // The picture's own fade, laid over it rather than mixed into it. The art
    // is a `MaterialNode` on a material shared with every card that looks the
    // same, so an alpha written there would fade the hand as well; a veil the
    // colour of the empty slot fades exactly this one, and costs one node.
    let veil = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                ..default()
            },
            BackgroundColor(palette::DIALOG_LIT.with_alpha(0.0)),
            Pickable::IGNORE,
            Arriving::veil(key),
        ))
        .id();
    commands.entity(slot).add_child(veil);
    slot
}

/// One target of a stack entry: its picture when it has one, a chip when it
/// does not — a player, a token this seat cannot name, a face-down permanent.
#[allow(clippy::too_many_arguments)] // as above
fn spawn_stack_target(
    commands: &mut Commands,
    lang: Lang,
    target: &baylee_client_core::board::StackTarget,
    key: StackKey,
    full: bool,
    view: &PlayerView,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
    cards: Option<&mut UiCards<'_>>,
) -> Entity {
    let (width, height) = if full {
        (STACK_TARGET_W, STACK_TARGET_H)
    } else {
        (STACK_QUEUED_TARGET_W, STACK_QUEUED_TARGET_H)
    };
    if target.art.is_some() {
        return spawn_stack_card(
            commands,
            lang,
            key,
            target.object(),
            target.art,
            None,
            width,
            height,
            statics,
            textures,
            assets,
            fonts,
            cards,
        );
    }
    // A player is a name and a heart, not a rectangle pretending to be a
    // card. Anything else with no picture (a token, something face down) is
    // its name in the same chip, so the row never has a hole in it.
    let (glyph, label) = match target.player() {
        Some(player) => (Some(glyph::HEART), statics.seat_name(player).to_string()),
        None => (
            None,
            target
                .object()
                .and_then(|id| view.object(id))
                .map(|o| crate::face::name_of(o, view, faces.texts))
                .or_else(|| target.name.clone())
                .unwrap_or_else(|| "?".into()),
        ),
    };
    let size = if full { 12.0 } else { 11.0 };
    let chip = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(6.0), px(3.0)),
                border_radius: BorderRadius::all(px(9)),
                ..default()
            },
            BackgroundColor(palette::DIALOG_LIT),
            Pickable::IGNORE,
            Arriving::fill(key, palette::DIALOG_LIT.alpha()),
        ))
        .id();
    if let Some(glyph) = glyph {
        let icon = commands
            .spawn((
                Text::new(glyph.to_string()),
                icon_tf(fonts, size - 2.0),
                TextColor(palette::DANGER),
                Arriving::ink(key, palette::DANGER.alpha()),
            ))
            .id();
        commands.entity(chip).add_child(icon);
    }
    let text = commands
        .spawn((
            Text::new(label),
            tf(fonts, size),
            TextColor(palette::INK),
            Arriving::ink(key, palette::INK.alpha()),
        ))
        .id();
    commands.entity(chip).add_child(text);
    chip
}

/// The line under the stack's title: whose answer the table is waiting for.
///
/// The local seat is **addressed**, not named. Its display name is the
/// language's own pronoun — offline it is literally `Phrase::You` — and this
/// sentence needs the accusative in German, so filling the slot with it reads
/// "wartet auf Du". [`Phrase::WaitingForYou`] is the sentence written out
/// instead, which is the fix that generalises: a slot that needs a different
/// word in different sentences is a slot that gets filled wrongly.
fn waiting_line(lang: Lang, name: &str, is_me: bool) -> String {
    if is_me {
        Phrase::WaitingForYou.text(lang).to_string()
    } else {
        Phrase::WaitingFor.fill(lang, &[name])
    }
}

/// The printed sentence a stack entry stands for, in the player's own
/// language, or `None` when there is nothing trustworthy to draw.
///
/// Two things have to line up and either may be missing, which is why every
/// step is a `?` and the panel falls back to the label it drew before. The
/// host has to know which sentence it is (it does not for a token's ability,
/// an emblem's, or one a continuous effect granted), and the source has to
/// still be findable, because the card is what the text is filed under and
/// an ability on the stack outlives its source (CR 113.7a). The words are
/// [`crate::cardtext::sentence`]'s, which falls to the card's English Oracle
/// when the player's language has none that pairs — offline, always.
///
/// The face is the host's answer and not the source's current one, for
/// the reason `baylee_view::StackText::face` gives.
pub(super) fn stack_sentence(
    item: &baylee_client_core::board::StackItem,
    view: &PlayerView,
    faces: &FaceCtx<'_>,
) -> Option<Vec<TextBlock>> {
    let baylee_client_core::board::StackKind::Ability {
        source,
        text,
        rules,
    } = item.kind
    else {
        return None;
    };
    let (text, card) = (text?, rules?.card);
    let print = view
        .object(source)
        .and_then(|source| crate::cardtext::print_of(source, card));
    crate::cardtext::sentence(Some(faces.texts), card, print, text)
}

/// What a **queued** ability row is headed, which is not its source's name.
///
/// A queued row answers "what else is coming", and for an ability the
/// source's name answers it badly: #132 was reported off a stack of five
/// where two rows read `Sheoldred, the Apocalypse` and nothing told them
/// apart. This file already makes that argument for the full row —
/// *"a stack of three triggers that all read `Ability · Ondu Cleric` tells
/// them less"* — and the queue was simply not held to it.
///
/// An ability has no name of its own, so the printed sentence *is* its
/// identity, and the picture beside the row already says which permanent it
/// came from. That is also why this costs no height: it is the line the row
/// already had, carrying something that distinguishes it.
///
/// Reminder text is dropped. It is parenthetical by definition (CR 207.2)
/// and this is one line — the reminder would be the half a player does not
/// need, taking the room from the half they do. Mana marks go in as their
/// printed source (`{T}`) rather than as glyphs, because a queued row is a
/// single `Text` and not the span chain a full row builds.
///
/// [`None`] whenever the sentence is not *known*, and the caller then draws
/// the name as before: the host sends no line index for some abilities, and
/// the catalog's text arrives over a socket that a client playing offline
/// against the house may not have at all. A row must never come out blank
/// because a lookup missed.
fn queued_ability_line(
    item: &baylee_client_core::board::StackItem,
    view: &PlayerView,
    faces: &FaceCtx<'_>,
) -> Option<String> {
    ability_line(&stack_sentence(item, view, faces)?)
}

/// What a row is headed, given the `name` its object is drawn under.
///
/// The **full** row keeps the name whatever it is: it carries the printed
/// sentence on its own line underneath, so the name is the one thing there
/// that is not already said. A **queued ability** does not, and its name is
/// its *source's* — two triggers off one permanent were the same row twice.
///
/// The fallback is the name, not an empty line: see [`queued_ability_line`]
/// for the two ordinary ways the sentence is simply not known yet.
fn heading(
    item: &baylee_client_core::board::StackItem,
    full: bool,
    name: String,
    view: &PlayerView,
    faces: &FaceCtx<'_>,
) -> String {
    if full {
        return name;
    }
    queued_ability_line(item, view, faces).unwrap_or(name)
}

/// The prose half of [`queued_ability_line`], with the two lookups taken
/// out — which is what makes the rule testable without a print table and a
/// catalog behind it.
///
/// Re-joining on whitespace rather than concatenating is doing work: a
/// reminder cut out of the middle of a sentence leaves the space that was in
/// front of it and the space that was behind it, and two spaces in the
/// middle of a one-line heading read as a missing word.
fn ability_line(blocks: &[TextBlock]) -> Option<String> {
    let joined = blocks
        .iter()
        .filter_map(|block| match block {
            TextBlock::Rules(text) => Some(text.as_str()),
            TextBlock::Reminder(_) => None,
        })
        .collect::<Vec<_>>()
        .join(" ");
    let line = joined.split_whitespace().collect::<Vec<_>>().join(" ");
    (!line.is_empty()).then_some(line)
}

/// One run of a sentence as it is drawn: prose, or a single mana mark.
///
/// A mark is its own piece because it is set in a different font at a
/// different size ([`STACK_MARK`]), and a reminder is flagged rather than
/// separated because the brackets it is drawn inside are pieces too and have
/// to carry the same slant and the same ink.
pub(super) struct Piece {
    /// The characters.
    pub text: String,
    /// One glyph of `mana.ttf`, rather than prose.
    pub mark: bool,
    /// Reminder text (CR 207.2), including its own `" ("` and `")"`.
    pub reminder: bool,
}

/// Every run of `blocks`, in printed order, with a reminder's brackets in
/// place and the whole sentence held to `room` characters.
///
/// `room` is `None` for a surface that **wraps** instead of cutting, which is
/// the difference between the two places this is drawn. The stack panel's full
/// row is budgeted — its height is what the queue under it is measured
/// against, so a pathological card must not be able to push the queue off the
/// panel — while the slip under the hover preview is the place that answers
/// "what *exactly* is about to happen" and cuts nothing at all.
///
/// The order inside a block is split-then-budget, and that is the part worth
/// keeping: `{T}` is three characters of source and one mark on screen, so
/// cutting the line before the split drops text there was room for.
///
/// The three characters taken off a reminder's budget are its `" (…)"`, and
/// they come off before its first piece is measured — so what a block hands
/// on to the next one is what it did not spend.
pub(super) fn spans_of(blocks: &[TextBlock], room: Option<usize>) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut left = room;
    for block in blocks {
        let reminder = matches!(block, TextBlock::Reminder(_));
        let mut room_for = left.map(|left| {
            if reminder {
                left.saturating_sub(3)
            } else {
                left
            }
        });
        if room_for == Some(0) {
            break;
        }
        let mut pieces: Vec<(String, bool)> = Vec::new();
        for piece in manapip::inline(block.text()) {
            if room_for == Some(0) {
                break;
            }
            match piece {
                manapip::Inline::Mark(mark) => {
                    room_for = room_for.map(|room| room - 1);
                    pieces.push((mark.to_string(), true));
                }
                manapip::Inline::Text(run) => {
                    // Prose, so the cut is [`cut_words`] — see its doc for
                    // why a name and a sentence are cut differently.
                    let run = match room_for {
                        Some(room) => cut_words(&run, room),
                        None => run,
                    };
                    room_for = room_for.map(|room| room - run.chars().count());
                    pieces.push((run, false));
                }
            }
        }
        if pieces.is_empty() {
            continue;
        }
        left = room_for;
        if reminder {
            pieces.insert(0, (" (".to_string(), false));
            pieces.push((")".to_string(), false));
        }
        out.extend(pieces.into_iter().map(|(text, mark)| Piece {
            text,
            mark,
            reminder,
        }));
    }
    out
}

/// The average advance of a lower-case letter, as a fraction of the size.
///
/// Measured, not guessed, and it survived the change of face by arithmetic
/// rather than by luck. Inter's lower case averaged 0.531 of its size and
/// 0.52 was that, rounded down to be generous. Alegreya Sans measures 0.445
/// of the size it is *rendered* at and Faustina 0.473, and they are rendered
/// at 1.2x and 1.1x the nominal size a caller passes — **0.534 and 0.520**
/// against the nominal, which brackets the same number. That is what let the
/// scales be scales rather than three hundred new constants.
pub(super) const CHAR_WIDTH: f32 = 0.52;

/// A card name cut to one line `room` pixels wide, set at `size`.
///
/// A serif's lower case averages a little over half its point size and a card
/// name is mostly lower case, so `0.52` is the ratio the budget is taken at —
/// deliberately generous, because the cost of guessing narrow is one word
/// clipped by the body's own `overflow` and the cost of guessing wide is a
/// name cut short that would have fitted.
///
/// **It is a guess and the widening is what makes it a safe one.** Measured
/// against the shipped `Faustina.ttf`'s own advance widths, a real name runs
/// between 0.36 (`Teferi's Isle`) and 0.65 (`Damn`) of its length times its
/// size, not 0.52 — a ratio that is wrong in **both** directions at once, so
/// at the old 187-pixel column it both cut names that would have fitted and
/// passed names that then clipped. (In `Inter.ttf`, which this replaced, the
/// spread was 0.41 to 0.70 and the widest name set 227 px against Faustina's
/// 202.) The only caller of `fit` is the
/// **queued** row, whose column is now 267 px at 14: over all 1475 faces in
/// the pool the budget is 36 characters, nothing clips and nothing is cut
/// that would have fitted — the estimator is exercised and wrong about
/// nothing. The full row does not call it at all; it wraps. If a name column
/// is ever narrowed again, this is the thing to replace with a real
/// measurement rather than to re-tune.
///
/// **[`budget`] has a second consumer, and the sentence it serves is the one
/// this estimate is worst for.** That paragraph above said "the only caller
/// left" and meant `fit`; [`spawn_stack_sentence`] calls `budget` directly
/// for a *wrapped* run of prose, where a per-character average is not the
/// quantity wanted at all. Wrapping breaks at words, so each break gives up
/// part of a line — a German compound gives up most of one — and the
/// character count that fits in four lines of prose is well under four times
/// the character count that fits in one. #132 reports the consequence from
/// the drawing: a sentence cut mid-word with room still in the panel. The
/// mid-word half is fixed ([`cut_words`]); the budget itself is not, because
/// choosing it needs the measurement the report has and this file does not.
///
/// The ellipsis replaces characters rather than joining them, so the result
/// never grows past the budget, and the cut is on `char` boundaries because a
/// card name is not ASCII (Æther Vial, Márton Stromgald).
fn fit(name: &str, room: f32, size: f32) -> String {
    cut(name, budget(room, size))
}

/// How many characters of `size`-point text fit in `room` pixels.
///
/// [`CHAR_WIDTH`] is the ratio, and it is a nominal size that goes in: the
/// two faces are asked for a scaled one ([`super::UI_SCALE`],
/// [`super::SERIF_SCALE`]), and the scales were chosen so that the product
/// lands back where Inter was.
///
/// Split out of [`fit`] because a *sentence* spends one budget across several
/// spans — rules text and its reminder are two — and each span cutting itself
/// to the full budget would let the pair run twice as long as the room they
/// share.
#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
pub(super) fn budget(room: f32, size: f32) -> usize {
    (room / (size * CHAR_WIDTH)).max(4.0) as usize
}

/// `text`, cut to `budget` characters with an ellipsis if it is longer.
pub(super) fn cut(text: &str, budget: usize) -> String {
    if text.chars().count() <= budget {
        return text.to_string();
    }
    let mut cut: String = text.chars().take(budget.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

/// The same, cut at the last **word** boundary that fits.
///
/// A card name is cut mid-word without anybody minding — a name is one thing
/// and half of it still points at the card. A *sentence* is not: a rules line
/// that stops at `…kannst du eine +1/+1-Marke auf den Klin…` reads as a
/// rendering fault rather than as text that goes on, and the fragment is
/// worth nothing. Which is why this is a second function and not a change to
/// [`cut`]: the two callers want different things, and a name cut to its last
/// whole word would lose most of `Asmoranomardicadaistinaculdacar`.
///
/// **A word longer than the whole budget falls back to the character cut.**
/// Backing off to the previous boundary would then be backing off to nothing
/// and returning a bare ellipsis, which says less than a cut word does — and
/// it is a real case, because the budget shrinks as spans are spent and the
/// last span can be handed four characters.
///
/// Trailing space goes before the ellipsis rather than after the cut, so the
/// mark sits against the word it follows.
pub(super) fn cut_words(text: &str, budget: usize) -> String {
    if text.chars().count() <= budget {
        return text.to_string();
    }
    let head: String = text.chars().take(budget.saturating_sub(1)).collect();
    let Some(at) = head.rfind(char::is_whitespace) else {
        return cut(text, budget);
    };
    let kept = head[..at].trim_end();
    if kept.is_empty() {
        return cut(text, budget);
    }
    format!("{kept}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A name that fits is left exactly as printed — the common case, and the
    /// one where a stray ellipsis would be a lie about the card.
    #[test]
    fn a_short_name_is_not_cut() {
        assert_eq!(fit("Shock", 187.0, 15.0), "Shock");
    }

    /// The table waits for *you*, not for a pronoun in the wrong case.
    ///
    /// Seen live as "wartet auf You" and, once the offline seat was named in
    /// German, one slot away from "wartet auf Du". An opponent is still named,
    /// because a name is what a name slot is for.
    #[test]
    fn the_table_waits_for_you_rather_than_for_your_name() {
        assert_eq!(waiting_line(Lang::De, "Du", true), "wartet auf dich");
        assert_eq!(waiting_line(Lang::En, "You", true), "waiting for you");
        assert_eq!(
            waiting_line(Lang::De, "sharp 1", false),
            "wartet auf sharp 1"
        );
    }

    /// A long one is cut *and* stays inside the budget it was cut to. The
    /// first attempt appended the ellipsis to a full-budget slice and came
    /// out one character wider than the room it was given.
    #[test]
    fn a_long_name_is_cut_to_fit() {
        let room = 187.0;
        let size = 15.0;
        let cut = fit("Asmoranomardicadaistinaculdacar", room, size);
        assert!(cut.ends_with('…'), "cut without saying so: {cut}");
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let budget = (room / (size * CHAR_WIDTH)) as usize;
        assert!(
            cut.chars().count() <= budget,
            "{cut} is {} chars, over the {budget} it had",
            cut.chars().count()
        );
    }

    /// A sentence is cut at a word, and the report's own line is the case.
    ///
    /// Fails against the old code, which cut `…auf den Klin…` — the fragment
    /// #132 was written from. A name may be cut mid-word and is; prose may
    /// not, which is why the two have different functions.
    #[test]
    fn a_sentence_is_cut_at_a_word() {
        let line = "kannst du eine +1/+1-Marke auf den Klingenmeister legen";
        let cut = cut_words(line, 34);
        assert_eq!(cut, "kannst du eine +1/+1-Marke auf…");
        assert!(
            cut.chars().count() <= 34,
            "{cut} is over the budget it was given"
        );
    }

    /// A word that does not fit at all is cut through rather than dropped.
    ///
    /// The budget shrinks as spans are spent, so the last span of a sentence
    /// can be handed four characters — and backing off to the previous word
    /// boundary there means backing off to nothing. A bare `…` says less
    /// than a cut word does, and the same holds for one long compound with
    /// no space in it anywhere.
    #[test]
    fn a_word_longer_than_the_budget_falls_back_to_the_character_cut() {
        assert_eq!(
            cut_words("Verzauberungskreatur", 8),
            cut("Verzauberungskreatur", 8)
        );
        assert!(cut_words("Verzauberungskreatur", 8).chars().count() <= 8);
        // And the same when the first word alone overruns: there is a space
        // in the string, but none of it is inside the budget.
        assert_eq!(
            cut_words("Verzauberungskreatur legen", 8),
            cut("Verzauberungskreatur", 8)
        );
    }

    /// Prose that fits is left exactly as printed, ellipsis and all absent.
    #[test]
    fn a_sentence_that_fits_is_not_cut() {
        assert_eq!(cut_words("Ziehe eine Karte.", 40), "Ziehe eine Karte.");
        // Exactly at the budget is not over it.
        assert_eq!(cut_words("Ziehe eine Karte.", 17), "Ziehe eine Karte.");
    }

    /// The cut lands on character boundaries. A byte-wise slice of a name
    /// with an accent in it panics, and the pool has several.
    #[test]
    fn a_name_that_is_not_ascii_survives_the_cut() {
        let cut = fit("Æther Vial of Márton Stromgald’s Æther", 60.0, 13.0);
        assert!(cut.chars().count() < 20, "{cut} was not cut at all");
    }

    /// A printing whose text is two printed **lines**, which is what
    /// `baylee_core::oracle::sentences` splits on and what
    /// `StackText::line` indexes into — not two sentences of prose.
    fn two_line_printing() -> crate::cardtext::CardTexts {
        use baylee_client_core::card_face::{CardTextEntry, FaceText};
        crate::cardtext::CardTexts::filed(
            baylee_core::ids::PrintRef::new(7),
            CardTextEntry {
                scryfall_id: "abc".to_string(),
                lang: "de".to_string(),
                faces: vec![FaceText {
                    name: "Sheoldred".to_string(),
                    english_name: "Sheoldred".to_string(),
                    type_line: "Kreatur".to_string(),
                    oracle_text: "Immer wenn ein Spieler eine Karte zieht, \
                                  erhältst du 2 Lebenspunkte.\n\
                                  Immer wenn ein Gegner eine Karte zieht, \
                                  verliert er 2 Lebenspunkte."
                        .to_string(),
                    mana_cost: String::new(),
                }],
            },
        )
    }

    /// One permanent, and one stack ability per entry of `lines` — `None`
    /// for an ability the host sent no line index for.
    fn a_stack_of(lines: &[Option<u8>]) -> (baylee_client_core::BoardModel, PlayerView) {
        use baylee_client_core::board::Openings;
        use baylee_client_core::test_support::{ViewBuilder, printed, token};

        let on_stack: Vec<_> = lines
            .iter()
            .enumerate()
            .map(|(at, line)| {
                let mut ability = token(30 + at as u32, 0, "Sheoldred", 0, 0);
                ability.card = None;
                ability.stack_item = Some(baylee_view::StackItem::Ability {
                    source: ObjectId::new(7, 0),
                    ability: None,
                    rules: Some(baylee_view::RulesFace {
                        card: baylee_core::ids::CardIndex::new(7),
                        face: 0,
                    }),
                    text: line.map(|line| baylee_view::StackText {
                        face: 0,
                        line,
                        of: 2,
                    }),
                });
                ability
            })
            .collect();
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![printed(7, 0, "Sheoldred", 7)])
            .with_stack(on_stack)
            .build();
        let board = baylee_client_core::BoardModel::from_view(
            &view,
            Openings::none(),
            |_| 800.0,
            &[],
            crate::cardart::registry(),
        );
        (board, view)
    }

    /// Two abilities of one permanent are two different rows in the queue.
    ///
    /// The defect #132 was reported from, run: a stack of five where two
    /// rows read `Sheoldred, the Apocalypse` and nothing told them apart.
    /// The test carries its own evidence — [`crate::face::name_of`] is what
    /// the queue used to head these rows with, and it is asserted to give
    /// one string for both, so the old behaviour fails beside the new one
    /// passing. It goes through [`heading`], the door the row calls, rather
    /// than through the lookup underneath it.
    #[test]
    fn two_abilities_of_one_permanent_are_two_different_queued_rows() {
        let texts = two_line_printing();
        let (board, view) = a_stack_of(&[Some(0), Some(1)]);
        let mode = crate::face::FaceMode::default();
        let settings = crate::settings::ClientSettings::default();
        let faces = FaceCtx {
            texts: &texts,
            mode: &mode,
            settings: &settings,
            view: Some(&view),
        };
        assert_eq!(board.stack.len(), 2, "two abilities are on the stack");

        let named = |item: &baylee_client_core::board::StackItem| {
            view.object(item.id).map_or_else(
                || item.name.clone(),
                |o| crate::face::name_of(o, &view, faces.texts),
            )
        };
        let headings: Vec<String> = board
            .stack
            .iter()
            .map(|item| heading(item, false, named(item), &view, &faces))
            .collect();

        assert_ne!(
            headings[0], headings[1],
            "two triggers of one permanent must not be the same row twice"
        );
        // On the set, not on the order: `BoardModel` walks the stack top
        // first, and which end that is has nothing to do with this claim.
        for clause in ["Immer wenn ein Spieler", "Immer wenn ein Gegner"] {
            assert!(
                headings.iter().any(|h| h.starts_with(clause)),
                "a row is headed by its own clause: {headings:?}"
            );
        }

        // The old heading, run: one string for both rows.
        assert_eq!(
            named(&board.stack[0]),
            named(&board.stack[1]),
            "the name the queue used to draw cannot tell these two apart"
        );
        // And the full row keeps that name, because it carries the sentence
        // already and the name is the one thing it does not otherwise say.
        for item in &board.stack {
            assert_eq!(
                heading(item, true, named(item), &view, &faces),
                named(item),
                "a full row is headed by its name"
            );
        }
    }

    /// A row never comes out blank because a lookup missed.
    ///
    /// The host sends no line index for some abilities, and the catalog's
    /// text arrives over a socket a client playing the house offline may not
    /// have at all — so the heading falls back to the name.
    #[test]
    fn an_ability_with_no_sentence_keeps_the_name() {
        let texts = crate::cardtext::CardTexts::default();
        let (board, view) = a_stack_of(&[None]);
        let mode = crate::face::FaceMode::default();
        let settings = crate::settings::ClientSettings::default();
        let faces = FaceCtx {
            texts: &texts,
            mode: &mode,
            settings: &settings,
            view: Some(&view),
        };
        assert!(queued_ability_line(&board.stack[0], &view, &faces).is_none());
        assert_eq!(
            heading(
                &board.stack[0],
                false,
                "Sheoldred".to_string(),
                &view,
                &faces
            ),
            "Sheoldred"
        );
    }

    /// A reminder is not what the one line is spent on, and the runs it is
    /// cut out of do not leave a double space behind.
    #[test]
    fn a_queued_line_drops_the_reminder_and_the_gap_it_left() {
        let blocks = vec![
            TextBlock::Rules("Fliegend".to_string()),
            TextBlock::Reminder("kann nur von Kreaturen geblockt werden".to_string()),
            TextBlock::Rules("und Wachsamkeit".to_string()),
        ];
        assert_eq!(
            ability_line(&blocks).as_deref(),
            Some("Fliegend und Wachsamkeit")
        );
        assert_eq!(
            ability_line(&[TextBlock::Reminder("nur dies".to_string())]),
            None,
            "a sentence that is nothing but a reminder heads no row"
        );
    }

    /// A queued row is the same object as the full row it will be promoted
    /// to, and a *different* key — which is the whole reason a resolution
    /// eases instead of cutting.
    #[test]
    fn promotion_is_a_new_key() {
        let id = ObjectId::new(7, 0);
        assert_ne!(StackKey::Entry(id, false), StackKey::Entry(id, true));
        assert_eq!(StackKey::Entry(id, true), StackKey::Entry(id, true));
    }

    /// Progress is remembered per row and forgotten for a row that is not on
    /// screen, so a rebuilt panel re-attaches to the arrival in progress and
    /// an object that resolved leaves nothing behind.
    #[test]
    fn a_row_that_is_not_tracked_is_simply_drawn() {
        let mut motion = StackMotion::default();
        let id = ObjectId::new(3, 0);
        assert!((motion.progress(StackKey::Entry(id, true)) - 1.0).abs() < f32::EPSILON);
        motion.rows.push(Rise {
            key: StackKey::Entry(id, true),
            at: 0.4,
            promoted: false,
            cooled: 1.0,
        });
        assert!((motion.progress(StackKey::Entry(id, true)) - 0.4).abs() < f32::EPSILON);
        assert!((motion.progress(StackKey::Panel) - 1.0).abs() < f32::EPSILON);
    }

    /// A veil is the inverse of ink: opaque while the row arrives, gone once
    /// it has. Without that the card picture — which is on a shared material
    /// and cannot be faded itself — would pop in at full strength.
    #[test]
    fn a_veil_clears_as_the_ink_comes_up() {
        let key = StackKey::Panel;
        let ink = Arriving::ink(key, 0.88);
        let veil = Arriving::veil(key);
        assert!(ink.alpha(0.0).abs() < f32::EPSILON);
        assert!((veil.alpha(0.0) - 1.0).abs() < f32::EPSILON);
        assert!((ink.alpha(1.0) - 0.88).abs() < f32::EPSILON);
        assert!(veil.alpha(1.0).abs() < f32::EPSILON);
    }

    // ---- the system, actually run ---------------------------------------
    //
    // The arithmetic above is the easy half. "Declared but never wired" is a
    // bug this client has shipped before, so the rest of these run the system
    // in an `App` and assert on what a frame left behind.

    use crate::prefs::Prefs;

    /// An app with the system in it and nothing else.
    fn harness() -> App {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<Prefs>()
            .init_resource::<StackMotion>()
            .add_systems(Update, ease_the_stack_in);
        app
    }

    /// One row: a fill that fades and a transform that lifts, plus a line of
    /// text under it, which is the part no opacity inheritance would reach.
    fn a_row(app: &mut App, key: StackKey) -> (Entity, Entity) {
        let row = app
            .world_mut()
            .spawn((
                Node::default(),
                BackgroundColor(palette::DIALOG_LIT),
                Arriving::fill(key, palette::DIALOG_LIT.alpha()),
                ArrivingRow {
                    lift: ARRIVE_LIFT,
                    from: ARRIVE_SCALE,
                    rail: palette::CANDLE,
                },
            ))
            .id();
        let ink = app
            .world_mut()
            .spawn((
                Node::default(),
                TextColor(palette::INK),
                Arriving::ink(key, palette::INK.alpha()),
            ))
            .id();
        (row, ink)
    }

    /// A sixtieth of a second, the frame this client is tuned against.
    fn a_frame(app: &mut App) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(1.0 / 60.0));
        app.update();
    }

    fn alpha_of(app: &App, row: Entity) -> f32 {
        app.world()
            .entity(row)
            .get::<BackgroundColor>()
            .unwrap()
            .0
            .alpha()
    }

    fn lift_of(app: &App, row: Entity) -> f32 {
        match app
            .world()
            .entity(row)
            .get::<UiTransform>()
            .unwrap()
            .translation
            .y
        {
            Val::Px(y) => y,
            other => panic!("the lift is not in pixels: {other:?}"),
        }
    }

    /// A row comes up from nothing, over several frames, and then holds
    /// exactly where it belongs. Both ends matter: a fade that never finished
    /// would leave the panel permanently dim.
    #[test]
    fn a_row_arrives_and_then_holds_still() {
        let mut app = harness();
        let key = StackKey::Entry(ObjectId::new(1, 0), true);
        let (row, ink) = a_row(&mut app, key);

        a_frame(&mut app);
        let part = alpha_of(&app, row);
        assert!(
            part > 0.0 && part < palette::DIALOG_LIT.alpha(),
            "one frame in, the row is part way: {part}"
        );
        assert!(lift_of(&app, row) < -0.5, "and still above its place");
        let text = app
            .world()
            .entity(ink)
            .get::<TextColor>()
            .unwrap()
            .0
            .alpha();
        assert!(
            text > 0.0 && text < 1.0,
            "the text fades with it, having no inheritance to ride: {text}"
        );

        for _ in 0..40 {
            a_frame(&mut app);
        }
        assert!(
            (alpha_of(&app, row) - palette::DIALOG_LIT.alpha()).abs() < 1e-4,
            "the row lands at the alpha it was drawn in"
        );
        assert!(lift_of(&app, row).abs() < 1e-4, "and at its resting place");
        let scale = app.world().entity(row).get::<UiTransform>().unwrap().scale;
        assert!((scale.x - 1.0).abs() < 1e-4, "at full size: {scale:?}");
    }

    /// The whole reason the progress is a resource: the HUD is rebuilt on
    /// hover, and a row that restarted its fade every time the pointer moved
    /// would flicker instead of arriving.
    #[test]
    fn a_rebuilt_row_carries_on_where_it_was() {
        let mut app = harness();
        let key = StackKey::Entry(ObjectId::new(2, 0), false);
        let (row, _) = a_row(&mut app, key);
        for _ in 0..4 {
            a_frame(&mut app);
        }
        let midway = alpha_of(&app, row);
        assert!(midway > 0.1, "far enough in to tell a restart from a fade");

        // What a `HudRevision` change does: the whole subtree goes and is
        // built again from scratch, at rest, with the same key.
        app.world_mut().entity_mut(row).despawn();
        let (again, _) = a_row(&mut app, key);
        a_frame(&mut app);
        assert!(
            alpha_of(&app, again) > midway,
            "the rebuild continued the arrival rather than restarting it"
        );
    }

    /// A row that steps down holds still.
    ///
    /// This is the headline case — a spell lands on a stack that already had
    /// one — and the key carries the row's shape, so the object that was on
    /// top is under a *new* key the moment it is drawn queued. Seeded like an
    /// arrival it would fade in beside the newcomer and the player would see
    /// two spells land where one did.
    #[test]
    fn a_row_that_steps_down_does_not_announce_itself() {
        let mut app = harness();
        let old = ObjectId::new(8, 0);
        let (top, top_ink) = a_row(&mut app, StackKey::Entry(old, true));
        for _ in 0..40 {
            a_frame(&mut app);
        }

        // A spell lands: the panel is rebuilt, the old top in the queued
        // shape and the newcomer full above it.
        app.world_mut().entity_mut(top).despawn();
        app.world_mut().entity_mut(top_ink).despawn();
        let (stepped, _) = a_row(&mut app, StackKey::Entry(old, false));
        let (landed, _) = a_row(&mut app, StackKey::Entry(ObjectId::new(9, 0), true));
        a_frame(&mut app);

        assert!(
            (alpha_of(&app, stepped) - palette::DIALOG_LIT.alpha()).abs() < 1e-4,
            "the demoted row stood where it was"
        );
        assert!(lift_of(&app, stepped).abs() < 1e-4, "and did not drop in");
        let arriving = alpha_of(&app, landed);
        assert!(
            arriving > 0.0 && arriving < palette::DIALOG_LIT.alpha(),
            "while the spell that did land is still arriving: {arriving}"
        );
    }

    /// And it is carried at the progress it had, not at rest. The house AI
    /// answers within a frame or two of priority, so the common demotion is
    /// of a row that is still arriving; seeding that at 1.0 would snap a
    /// half-faded spell to full on the frame the counter landed.
    #[test]
    fn a_row_demoted_mid_arrival_keeps_its_place_in_the_fade() {
        let mut app = harness();
        let old = ObjectId::new(11, 0);
        let (top, top_ink) = a_row(&mut app, StackKey::Entry(old, true));
        a_frame(&mut app);
        a_frame(&mut app);
        let partway = alpha_of(&app, top);
        assert!(
            partway > 0.0 && partway < palette::DIALOG_LIT.alpha() * 0.9,
            "the row under test has to still be arriving: {partway}"
        );

        // The answer lands before the first spell has finished arriving.
        app.world_mut().entity_mut(top).despawn();
        app.world_mut().entity_mut(top_ink).despawn();
        let (stepped, _) = a_row(&mut app, StackKey::Entry(old, false));
        a_row(&mut app, StackKey::Entry(ObjectId::new(12, 0), true));
        a_frame(&mut app);

        let carried = alpha_of(&app, stepped);
        assert!(
            carried > partway && carried < palette::DIALOG_LIT.alpha() * 0.95,
            "it continues from {partway}, it does not jump to full: {carried}"
        );
    }

    /// The other direction is not carried, and that is the point: a queued
    /// row becoming full is what a resolution looks like from the panel.
    #[test]
    fn a_row_that_is_promoted_still_arrives() {
        let mut app = harness();
        let id = ObjectId::new(10, 0);
        let (queued, queued_ink) = a_row(&mut app, StackKey::Entry(id, false));
        for _ in 0..40 {
            a_frame(&mut app);
        }
        app.world_mut().entity_mut(queued).despawn();
        app.world_mut().entity_mut(queued_ink).despawn();
        let (full, _) = a_row(&mut app, StackKey::Entry(id, true));
        a_frame(&mut app);
        let part = alpha_of(&app, full);
        assert!(
            part > 0.0 && part < palette::DIALOG_LIT.alpha(),
            "a resolution is meant to be seen: {part}"
        );
    }

    /// And it arrives from the **other direction**, which is the difference
    /// between a spell being cast and a spell resolving.
    ///
    /// A promoted row grew out of the slot below it: the object above it has
    /// resolved and left. Drawn as an arrival it lifted into place from
    /// above — the movement of something landing on the stack, which is the
    /// opposite of what happened. This is the one assertion that can tell the
    /// two apart, because the alpha ramp is identical for both.
    #[test]
    fn a_promoted_row_comes_up_from_the_slot_it_was_in() {
        let mut app = harness();
        let id = ObjectId::new(14, 0);
        let (queued, queued_ink) = a_row(&mut app, StackKey::Entry(id, false));
        for _ in 0..40 {
            a_frame(&mut app);
        }
        app.world_mut().entity_mut(queued).despawn();
        app.world_mut().entity_mut(queued_ink).despawn();
        let (full, _) = a_row(&mut app, StackKey::Entry(id, true));
        a_frame(&mut app);

        let lift = lift_of(&app, full);
        assert!(
            lift > 0.0 && lift <= PROMOTE_LIFT,
            "a promotion starts below its place and rises: {lift}"
        );

        // The counter-test, and the reason the sign is worth asserting at
        // all: a spell that is merely *cast* still drops in from above.
        let (landed, _) = a_row(&mut app, StackKey::Entry(ObjectId::new(15, 0), true));
        a_frame(&mut app);
        assert!(
            lift_of(&app, landed) < 0.0,
            "an arrival comes from the other side"
        );
    }

    /// The promoted row's rail lands bright and cools into the accent.
    ///
    /// The light was on the row that resolved a moment ago; a bright mark
    /// appearing one slot lower and settling is the resolution drawn as the
    /// movement it is. It is on the slower ramp deliberately, so it is still
    /// settling after the row has stopped moving — which is what this asserts
    /// by reading the border on the frame the movement is nearly over.
    #[test]
    fn the_rail_of_a_promoted_row_cools_into_place() {
        let mut app = harness();
        let id = ObjectId::new(16, 0);
        let (queued, queued_ink) = a_row(&mut app, StackKey::Entry(id, false));
        for _ in 0..40 {
            a_frame(&mut app);
        }
        app.world_mut().entity_mut(queued).despawn();
        app.world_mut().entity_mut(queued_ink).despawn();
        let (full, _) = a_row(&mut app, StackKey::Entry(id, true));

        let mut seen_warmer = false;
        for _ in 0..12 {
            a_frame(&mut app);
            let rail = app
                .world()
                .entity(full)
                .get::<BorderColor>()
                .expect("a Node always has one")
                .top
                .to_srgba();
            // Whiter than the accent it settles at: `INK` is brighter in
            // every channel; green carries the contrast against warm gold.
            if rail.green > palette::CANDLE.to_srgba().green * 1.1 {
                seen_warmer = true;
            }
        }
        assert!(seen_warmer, "the rail landed brighter than it rests");

        for _ in 0..60 {
            a_frame(&mut app);
        }
        let settled = app
            .world()
            .entity(full)
            .get::<BorderColor>()
            .expect("a Node always has one")
            .top
            .to_srgba();
        let accent = palette::CANDLE.to_srgba();
        assert!(
            (settled.red - accent.red).abs() < 0.02
                && (settled.green - accent.green).abs() < 0.02
                && (settled.blue - accent.blue).abs() < 0.02,
            "and cooled all the way to the accent: {settled:?}"
        );
    }

    /// A player who has asked for stillness gets the panel, not the arrival.
    #[test]
    fn holding_still_puts_the_row_straight_where_it_belongs() {
        let mut app = harness();
        app.world_mut().resource_mut::<Prefs>().edit().reduce_motion = true;
        let key = StackKey::Entry(ObjectId::new(3, 0), true);
        let (row, _) = a_row(&mut app, key);
        a_frame(&mut app);
        assert!(
            (alpha_of(&app, row) - palette::DIALOG_LIT.alpha()).abs() < 1e-4,
            "there on the first frame"
        );
        assert!(lift_of(&app, row).abs() < 1e-4);
    }

    /// A line of text keeps the background it does not have.
    ///
    /// [`Node`] requires a [`BackgroundColor`], so every label in this panel
    /// carries a transparent one; a fade that wrote whichever colour it found
    /// turned each of those into an opaque black plate, and the first live
    /// shot of the panel was eight of them where the words should be. The
    /// [`Paints`] tag is what stops it, and this is the test that would have
    /// caught it — the arithmetic tests all passed while the panel was
    /// unreadable.
    #[test]
    fn fading_a_label_does_not_give_it_a_plate() {
        let mut app = harness();
        let key = StackKey::Entry(ObjectId::new(5, 0), true);
        let (_, ink) = a_row(&mut app, key);
        for _ in 0..3 {
            a_frame(&mut app);
        }
        let plate = app
            .world()
            .entity(ink)
            .get::<BackgroundColor>()
            .expect("a Node always has one")
            .0;
        assert!(
            plate.alpha().abs() < f32::EPSILON,
            "the label grew a background: {plate:?}"
        );
        assert!(
            app.world()
                .entity(ink)
                .get::<TextColor>()
                .unwrap()
                .0
                .alpha()
                > 0.0,
            "while its ink did come up"
        );
    }

    /// The accent rail arrives with the row rather than standing there alone
    /// while the row fades in behind it — and a row with no rail never grows
    /// one, which is the same `Color::NONE` trap one component along.
    #[test]
    fn the_rail_arrives_with_its_row() {
        let mut app = harness();
        let key = StackKey::Entry(ObjectId::new(6, 0), true);
        let (row, _) = a_row(&mut app, key);
        a_frame(&mut app);
        let part = app.world().entity(row).get::<BorderColor>().unwrap().left;
        assert!(
            part.alpha() > 0.0 && part.alpha() < 1.0,
            "the rail comes up with the row: {part:?}"
        );
        for _ in 0..40 {
            a_frame(&mut app);
        }
        let rested = app.world().entity(row).get::<BorderColor>().unwrap().left;
        assert!((rested.alpha() - 1.0).abs() < 1e-4, "and lands lit");
    }

    /// A resolved spell leaves nothing behind. Without the prune, the *next*
    /// object at that id and shape would find its arrival already finished
    /// and appear with no animation at all.
    #[test]
    fn a_row_that_leaves_the_panel_is_forgotten() {
        let mut app = harness();
        let key = StackKey::Entry(ObjectId::new(4, 0), true);
        let (row, ink) = a_row(&mut app, key);
        a_frame(&mut app);
        assert_eq!(app.world().resource::<StackMotion>().rows.len(), 1);

        app.world_mut().entity_mut(row).despawn();
        app.world_mut().entity_mut(ink).despawn();
        a_frame(&mut app);
        assert!(
            app.world().resource::<StackMotion>().rows.is_empty(),
            "the panel emptied and took its progress with it"
        );
    }
}

#[cfg(test)]
mod folding_tests {
    use super::*;
    #[test]
    fn folding_survives_rebuilds_and_reopens() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<crate::prefs::Prefs>()
            .init_resource::<StackFold>()
            .add_systems(Update, fold_the_stack);
        let body = app.world_mut().spawn((StackBody, Node::default())).id();
        app.world_mut().resource_mut::<StackFold>().collapsed = true;
        let frame = |app: &mut App| {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_secs_f32(0.016));
            app.update();
        };
        frame(&mut app);
        let progress = app.world().resource::<StackFold>().open;
        assert!(progress > 0.0 && progress < 1.0);
        app.world_mut().entity_mut(body).despawn();
        let rebuilt = app.world_mut().spawn((StackBody, Node::default())).id();
        frame(&mut app);
        assert!(app.world().resource::<StackFold>().open < progress);
        for _ in 0..40 {
            frame(&mut app);
        }
        assert_eq!(
            *app.world().get::<Visibility>(rebuilt).unwrap(),
            Visibility::Hidden
        );
        app.world_mut().resource_mut::<StackFold>().collapsed = false;
        frame(&mut app);
        assert_eq!(
            *app.world().get::<Visibility>(rebuilt).unwrap(),
            Visibility::Inherited
        );
    }
}

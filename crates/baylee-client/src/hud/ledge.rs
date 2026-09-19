//! The ledge: the top edge of the hand zone, and the place a player answers
//! the game from.
//!
//! It is named for `tabletop::MAT_LEDGE`, the shoulder of a seat's mat that
//! the seat bar is written along. The hand zone gets the same shoulder, at
//! its top — the mana pool at one end, the engine's question and its answers
//! in the middle, the two ways to leave the game at the other. Nothing here
//! floats: the four things that used to hover over the table are one edge.
//!
//! This file holds the shelf and the middle of it: the question, its answers
//! and the armed card that replaces them. The mana pool and the two ways out
//! arrive in their own steps — their columns are already here and already the
//! width they will be, so that nothing the middle does moves when they land.
//!
//! Everything taller than one line belongs in the [`drawer`], which grows
//! upward out of this shelf and has its own file and its own counter.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;

pub(super) mod drawer;
pub(super) mod pool;
// Not `hud::tray`, which is the zone dialog. This is the strip the
// dialog is put away into; the collision and why it stands are in the
// module's own doc.
pub(super) mod tray;

/// Where the shelf put the middle of itself, for the drawer to stand over.
///
/// A resource rather than a second call to
/// [`baylee_client_core::ledge::arrange`], because arranging takes the widths
/// of all three columns and the drawer has no business measuring the shelf's
/// buttons. Two estimates of one number would also be two chances to be
/// wrong about it, and the middle's width is already an estimate —
/// see [`mid_width`].
///
/// Written whenever [`sync_ledge`] rebuilds, which is whenever it can change:
/// the arrangement is a function of the revision and the window.
#[derive(Resource)]
pub struct LedgeLayout {
    /// The centre of the middle column, in logical pixels from the left edge.
    pub(super) mid_x: f32,
    /// The window it was measured in.
    pub(super) window_w: i32,
}

impl Default for LedgeLayout {
    /// The middle of a window of the width `sync_ledge` assumes before it has
    /// seen one, so a drawer drawn before the first arrangement is centred
    /// rather than thrown to the left edge.
    fn default() -> Self {
        Self {
            mid_x: 600.0,
            window_w: 1200,
        }
    }
}

/// How far the middle of the shelf is from the middle of the window.
///
/// Doubled, because this is paid as *padding on one side*: a centred child in
/// a box padded by `p` on the left has its centre at `(p + w) / 2`, so `p` is
/// twice the slide. Positive means the middle sits right of centre.
pub(super) fn mid_shift(mid_x: f32, window_w: i32) -> f32 {
    #[allow(clippy::cast_precision_loss)]
    let half = window_w as f32 / 2.0;
    2.0 * (mid_x - half)
}

/// That slide as padding, on whichever side has to carry it.
///
/// The one place this arithmetic lives. The middle column and the drawer both
/// stand on the same centre, and a drawer that worked it out for itself would
/// be over the question until the day one of the two was adjusted.
pub(super) fn mid_padding(mid_x: f32, window_w: i32) -> UiRect {
    let shift = mid_shift(mid_x, window_w);
    if shift >= 0.0 {
        UiRect::left(px(shift))
    } else {
        UiRect::right(px(-shift))
    }
}

/// The shelf itself: the one node in the overlay's retained tree that
/// **outlives a rebuild**.
///
/// [`super::HudRevision`] counts the hover, so `sync_overlay` tears its tree
/// down and builds it again whenever the pointer crosses a card — hundreds of
/// times a turn. Everything in that tree is content that follows the pointer
/// (the preview *is* the hover) or is cheap enough not to care. The shelf is
/// neither: it is the edge the window ends at, it is there at the first frame
/// and never goes, and the buttons on it carry a [`crate::ambience::Feel`]
/// whose warmth is state on the entity — a shelf rebuilt under the pointer
/// snaps the button the player is reaching for back to rest.
///
/// So `sync_overlay` keeps the root and this node across a rebuild and
/// despawns the rest, and the shelf's own contents answer to their own
/// revision. It is **not** a second root, and that is a measurement rather
/// than a preference: a root sorts wholesale against the others, and the
/// shelf has to stand *over* the table veil (`Z_VEIL`, the whole window) and
/// *under* the hover preview (`Z_PREVIEW`, which `hand::beside` may put over
/// the shelf when it describes a card near the bottom of the screen). Both
/// are children of [`HudRoot`], so the shelf has to be one too.
#[derive(Component)]
pub struct LedgeShelf;

/// Spawns the shelf: opaque, full width, [`hand::LEDGE_H`] tall, standing on
/// the top of the hand zone.
///
/// **A sibling of the zone and not a child of it**, which is the one thing
/// about this node that is not free to change. `ZIndex` counts among
/// siblings, and the zone sits at [`Z_HAND`] — under the table veil the zone
/// dialog paints over the whole window at [`Z_VEIL`], deliberately, because
/// the hand cannot answer what that dialog is asking. The question *can*, and
/// a question drawn dimmed is a question the player is being told not to
/// answer. The slip stood above that veil for exactly this reason and the
/// ledge inherits its place.
///
/// Two more things that look like details and are not. The shelf is **no
/// longer opaque** — §3.3 said it was, and gave a reason ("a translucent edge
/// reads as a veil, not as a shelf"), and the owner reversed it on 14.09.2026:
/// *"Make them a little bit transparent with slow moving shader animation, so
/// it gets a little bit breathing live."* What makes that readable rather
/// than merely dimmer is the other half of the same instruction: the hand
/// zone's cloth now runs **behind** this node, so the shelf is composited over
/// a container and not over the sky. And its overflow is **visible**: the
/// drawer grows up out of it, and so do the two casts below, so a `clip()`
/// copied from the zone would leave a drawer nobody can see and a shelf that
/// stands off nothing.
pub(super) fn spawn_ledge(
    commands: &mut Commands,
    cloth: Option<Handle<crate::frontal::FrontalMaterial>>,
) -> Entity {
    let shelf = commands
        .spawn((
            LedgeShelf,
            Node {
                position_type: PositionType::Absolute,
                bottom: px(hand::HAND_ZONE_H - hand::LEDGE_H),
                left: px(0),
                right: px(0),
                height: px(hand::LEDGE_H),
                // The lip is paid for out of the **top** padding, which is why
                // this is not `UiRect::axes`. `BoxSizing::DEFAULT` is
                // `BorderBox`, so `LEDGE_H` is the outside of the shelf and
                // the border eats into it: 6 + 28 + 6 is 40 only if the line
                // is not there, and with it a 28-px button overflows by one.
                // Taking the pixel off the padding rather than adding it to
                // the shelf keeps the rule the whole height budget is built on
                // — every pixel of ledge is a pixel of table — and costs
                // nothing to look at, because 1 + 5 above the button reads as
                // the 6 below it.
                padding: UiRect::new(px(EDGE), px(EDGE), px(LEDGE_PAD_Y - LIP), px(LEDGE_PAD_Y)),
                // The lip is still paid for out of the border box, because the
                // padding arithmetic above is measured against it — but it is
                // *painted* by the cloth (`frontal.wgsl`, the top row of the
                // rail), because a `BorderColor` on a `MaterialNode` is a
                // question and the line is not optional.
                border: UiRect::top(px(LIP)),
                // The two corners the owner asked for on 14.09.2026, cut by
                // the cloth itself and stated here too so the headless
                // fallback is the same shape. It rounds the **top** only; the
                // other three corners of the controls bar are off the frame.
                border_radius: BorderRadius::top(px(crate::frontal::CORNER)),
                overflow: Overflow::visible(),
                ..default()
            },
            // Nothing of its own. The rail is a surface, and it is the same
            // cloth as the zone below — one dye, one pair of clocks, one fold
            // field indexed on the window's x, so the two read as one piece
            // with a rail across the top. Without a render world there is no
            // handle and the flat dye is drawn instead.
            match cloth {
                Some(_) => BackgroundColor(Color::NONE),
                None => BackgroundColor(palette::DOCK_GROUND.with_alpha(RAIL_FALLBACK)),
            },
            ZIndex(Z_LEDGE),
            // The shelf itself answers nothing and must not swallow a click
            // meant for the table — but its children are buttons, and a
            // button in a node the pointer cannot see is a button that cannot
            // be pressed. Hoverable, blocking nothing, exactly as the zone
            // below it is.
            //
            // `should_block_lower: false` does mean a click on the bare shelf
            // reaches whatever 3D geometry is behind it, which is not
            // obviously right. It is harmless today because the only thing
            // down there is the slab's margin and nothing on it is pickable;
            // if the layout ever puts a card under the shelf, this is the
            // line that has to change.
            Pickable {
                should_block_lower: false,
                is_hoverable: true,
            },
        ))
        .id();
    if let Some(handle) = cloth {
        commands
            .entity(shelf)
            .insert((MaterialNode(handle), crate::frontal::Hanging));
    }
    // The two casts, as children outside the shelf's own box rather than as a
    // `BoxShadow` on it. See `LIFT_UP_Y` for why they had to stop being one.
    for cast in [lift_up(commands), lift_down(commands)] {
        commands.entity(shelf).add_child(cast);
    }
    shelf
}

/// What [`sync_ledge`] leaves alone: everything the shelf was spawned with.
type Retained = Or<(With<pool::PoolColumn>, With<LedgeCast>)>;

/// One of the shelf's two casts, so the rebuild leaves them where they are.
///
/// They are spawned with the shelf and never change: an elevation is a fact
/// about the shelf and not about the question standing on it.
#[derive(Component, Clone, Copy)]
pub struct LedgeCast;

/// The shelf's cast on the table above it.
///
/// A child, `Pickable::IGNORE`, drawn entirely **outside** the shelf's
/// rectangle, which is the whole reason it is not a `ShadowStyle` any more.
///
/// The [`LIP`] in the offset is not a nudge. An absolutely-positioned child's
/// insets are measured from its containing block's **padding** box, and the
/// shelf carries `border: UiRect::top(px(LIP))` — so a bare `-LIFT_UP_H` put
/// this gradient's darkest end one pixel *inside* the bar, on top of the one
/// line the cloth paints. Measured at 3008 x 1630: the lip along the straight
/// top edge composited to (36, 31, 19) where the same lip round the corner,
/// below the cast, came out at `DIALOG_LINE`'s own (55, 48, 31) — the same
/// ratio, 0.65, on all three channels, which is a black veil and not a
/// different colour. A shadow an object casts must not fall on the object.
fn lift_up(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            LedgeCast,
            Node {
                position_type: PositionType::Absolute,
                top: px(-LIFT_UP_H - LIP),
                left: px(0),
                right: px(0),
                height: px(LIFT_UP_H),
                ..default()
            },
            BackgroundGradient::from(LinearGradient::to_bottom(vec![
                ColorStop::percent(palette::SHADOW.with_alpha(0.0), 0.0),
                ColorStop::percent(
                    palette::SHADOW.with_alpha(palette::SHADOW.alpha() * 0.18),
                    40.0,
                ),
                ColorStop::percent(
                    palette::SHADOW.with_alpha(palette::SHADOW.alpha() * 0.55),
                    70.0,
                ),
                ColorStop::percent(palette::SHADOW, 100.0),
            ])),
            Pickable::IGNORE,
        ))
        .id()
}

/// And its cast on the cards below, the lighter of the two.
///
/// The [`LIP`] comes off this one for the reason it goes onto [`lift_up`]:
/// both are measured from the padding box, so `LEDGE_H` alone would start the
/// cast a pixel below the shelf and leave an unshadowed line under it.
fn lift_down(commands: &mut Commands) -> Entity {
    let share = |f: f32| palette::SHADOW.with_alpha(palette::SHADOW.alpha() * LIFT_DOWN_SHARE * f);
    commands
        .spawn((
            LedgeCast,
            Node {
                position_type: PositionType::Absolute,
                top: px(hand::LEDGE_H - LIP),
                left: px(0),
                right: px(0),
                height: px(LIFT_DOWN_H),
                ..default()
            },
            BackgroundGradient::from(LinearGradient::to_bottom(vec![
                ColorStop::percent(share(1.0), 0.0),
                ColorStop::percent(share(0.48), 35.0),
                ColorStop::percent(share(0.12), 70.0),
                ColorStop::percent(share(0.0), 100.0),
            ])),
            Pickable::IGNORE,
        ))
        .id()
}

/// How far the shelf stands off the table above it, and how soft that cast is.
///
/// §3.3 asked for one shadow, downwards, and gave the reason: a card running
/// under an opaque bar reads as a card on a shelf rather than a card cut by a
/// line. The owner asked for the other one on 14.09.2026 — *"zum Tisch hin
/// als auch zur Hand hin (zur Hand etwas leichter)"* — and it is the same
/// argument turned round. A slab that casts one way is a lid; one that casts
/// both ways is standing off both, which is what a shelf at the edge of a
/// table is doing.
///
/// The table's side is the deeper of the two because it is the side that can
/// afford it. Above the shelf is open felt with nothing on it to compete
/// with; below it is a row of cards each carrying a glow that is a *rules*
/// statement, and a heavy cast there argues with the one light on this screen
/// a player is meant to read as information.
///
/// **Why these are heights and not offsets and blurs.** They were a
/// `BoxShadow` with two `ShadowStyle`s, and a `BoxShadow` is the node's own
/// rectangle offset and blurred — so the up-cast's rectangle is this shelf
/// shifted five pixels up and covers nearly the whole of its interior at
/// `SHADOW`'s full 0.55, and the down-cast covers it again from about ten
/// pixels down. All of that was *behind an opaque node and never seen*, which
/// was true right up until the owner asked for the shelf to be translucent on
/// 14.09.2026. On a translucent one it is roughly two thirds black across the
/// whole width, which is the removed panel arriving by the back door — the
/// same failure `frontal.rs`'s hem paid for once ("a hem at 0.97 under a cast
/// at 0.25 is 0.99 over the first sixteen pixels").
///
/// So each cast is a gradient child lying entirely outside the shelf's box:
/// nothing of either falls on the shelf, the up-cast still lands on the felt
/// and the down-cast still lands on the cards, and both still ride at
/// [`Z_LEDGE`] because they are children of the node that carries it. The
/// depths are what the old blurs reached — about fourteen pixels up and
/// twelve down — so the picture is the one §3.3 and AV1 measured.
const LIFT_UP_H: f32 = 14.0;
const LIFT_DOWN_H: f32 = 12.0;

/// What the hand's side of the cast is worth against the table's.
///
/// A share and not a second colour, so "somewhat lighter" cannot quietly stop
/// being true when [`palette::SHADOW`] is retuned — which is what the assert
/// below is for.
///
/// It was 0.45 while the two casts were one blurred rectangle each: the
/// up-cast's own lower edge was blurred too and about eleven pixels of it
/// landed on the hand, so the hand's side was being paid for twice and a
/// share of 0.70 measured *deeper* there than the single cast it replaced.
/// Neither cast reaches the other's side now, so the share is the plain
/// statement it was always meant to be and goes back to the 0.70 AV1 asked
/// for.
const LIFT_DOWN_SHARE: f32 = 0.70;
const _: () = assert!(LIFT_DOWN_SHARE < 1.0 && LIFT_DOWN_SHARE > 0.0);

/// How dense the shelf is where there is no render world to draw the cloth.
///
/// The cloth's own density at the rail, **read** from [`crate::frontal::RAIL`]
/// rather than restated, for the reason `hand::GROUND` gives: a fallback that
/// put the ground somewhere else would move every headless measurement with
/// it.
const RAIL_FALLBACK: f32 = crate::frontal::RAIL;

/// The air above and below a button on the shelf.
///
/// Twice this plus a button's [`BUTTON_H`] is [`hand::LEDGE_H`], and that
/// equation is the only reason either number is what it is. The [`LIP`] comes
/// out of the top of it rather than out of the shelf; the node says why.
pub(super) const LEDGE_PAD_Y: f32 = 6.0;

/// The line along the top of the shelf, where the table stops.
pub(crate) const LIP: f32 = 1.0;

/// How tall anything a player presses on the shelf is.
///
/// A keycap with the button's own air around it: [`KEYCAP_SIDE`] is 1.9 and
/// [`CAP_PT`] is 10.5, so the cap is a 20-pixel square and `20 + 2 · 4` is
/// this. Written out rather than computed all the same, because the equation
/// below is the one that actually holds it — a cap drawn at another size
/// would have to leave the shelf's height alone, not move it.
///
/// Not the 44 a phone would ask for: this is a desktop table, the shelf is
/// 40 px of a window that would rather be table, and a 44-px target would take
/// another 16 px off the hand. A deliberate refusal, written down as one.
pub(super) const BUTTON_H: f32 = 28.0;

/// The shelf's arithmetic, as one statement rather than four comments.
const _: () = assert!(LEDGE_PAD_Y * 2.0 + BUTTON_H == hand::LEDGE_H);

// ------------------------------------------------------- what stands on it
//
// Three absolutely positioned children rather than one flex row with
// `space-between`, because the question belongs on the **window's** centre —
// which is the middle of the player's own mat, where their eyes already are
// — and `space-between` would put it in the middle of whatever room the
// other two columns left over, so it would shuffle sideways every time a
// mana pip arrived. `docs`' AX §2.3 has the argument and the widths.
//
// One thing about absolute children here is a taffy fact rather than a
// choice: `perform_absolute_layout_on_absolute_children` measures an inset
// against `container_size - border`, and **not** minus padding. So the
// shelf's own `padding` does nothing for these three, and each states the
// band it stands in out of the same two constants the shelf is built from.

/// Which snapshot the shelf currently shows.
///
/// Its own counter and not [`super::HudRevision`], which counts the hover: the
/// shelf would otherwise be rebuilt every time the pointer crossed a card,
/// hundreds of times a turn, and a button's [`Feel`] would lose its warmth
/// under the player's own pointer. [`LedgeShelf`] is what makes that possible
/// — it outlives the rebuild the rest of the overlay goes through.
///
/// Compared **whole** rather than field by field, which is the one way this
/// differs from [`super::HudRevision`] and the reason is that struct's own
/// test: a field there can be compared and never assigned (the tree redraws
/// every frame) or assigned and never compared (a control that silently does
/// nothing), and `hud/tests.rs` reads the source to catch both. `PartialEq`
/// and one assignment cannot express either mistake, so there is nothing for
/// such a test to find.
///
/// There is no `hovered` here and there must not be. That is the whole point
/// of the struct, and `the_shelf_does_not_follow_the_pointer` holds it.
// Nine bools, and the lint's advice — "a state machine, or two-variant enums"
// — is the one shape this must not take. Each of these is an independent fact
// about a different thing, and what the struct does with them is compare all
// of them at once; folding any pair into an enum would claim they cannot both
// be true, which is a claim about the game and not about the drawing.
#[allow(clippy::struct_excessive_bools)]
#[derive(Resource, Default, Clone, PartialEq)]
pub struct LedgeRevision {
    hand_order: crate::hand_order::HandOrder,
    /// Which snapshot of the game.
    pub(super) seq: Option<u64>,
    /// Whether the game has ended, which empties the right column.
    ///
    /// Everything else the ending changes it changes through a field that is
    /// already here — the sentence, the answers — so this looks redundant and
    /// is not: the two ways out of a game are drawn from `can_offer_draw` and
    /// `concede_armed` alone, and neither of those moves when the last player
    /// falls over.
    pub(super) over: bool,
    /// The question, as the sentence says it.
    pub(super) prompt: Option<String>,
    /// The engine's refusal, which replaces that sentence.
    pub(super) error: Option<String>,
    /// What the connection has to say, which replaces it first.
    pub(super) link_note: Option<baylee_client_core::i18n::Phrase>,
    /// Whether this seat is being asked at all — the sentence's weight, and
    /// whether there are answers under it.
    pub(super) waiting: bool,
    /// Whether the zone browser's dialog is holding the answer, which is what
    /// keeps a second Confirm off the shelf.
    pub(super) elsewhere: bool,
    /// What has been picked so far, because `can_confirm` reads it and the
    /// lone OK answer appears the moment it turns true.
    pub(super) selected: Vec<ObjectId>,
    /// What is armed and waiting for its second press. It never leaves the
    /// client, so nothing else here moves when it changes.
    pub(super) armed: Option<crate::Armed>,
    /// Whether the client's own cast chooser is standing, which takes the
    /// answers away: the engine's window behind it is an ordinary priority,
    /// and "Pass priority" under "Choose how it is cast" is two primary
    /// answers saying opposite things.
    pub(super) cast_menu: bool,
    /// Whether "resolve the stack" is one of the answers — [`Duel::
    /// can_hold_for_stack`], written down rather than left to `seq`. Every
    /// other field here is a fact the drawing reads, derived from the view or
    /// the interaction and listed anyway; this one is no different, and the
    /// alternative is a claim about what `PlayerView::seq` counts.
    pub(super) holdable: bool,
    /// Whether the engine would take a draw offer right now, which is the
    /// difference between a secondary button and a dead one.
    pub(super) can_offer_draw: bool,
    /// Whether the concession is armed and waiting for its second press.
    ///
    /// It follows nothing but the pointer — no snapshot, no question — so
    /// without it here the button would keep the word it was drawn with. It
    /// is also what takes the draw offer away: see [`ways_out`].
    pub(super) concede_armed: bool,
    /// Whether this seat has a running priority hold, which is what the
    /// middle says instead of a question — and what puts a keycap on the way
    /// out of it.
    pub(super) priority_held: bool,
    /// Whether the client's own autopilot is running, which draws the same
    /// sentence and the same button with **no** cap: no key ends it (§4.4),
    /// and a cap that promised one would be a lie. Entirely client-side, so
    /// nothing else here moves when it starts or stops.
    pub(super) autopilot: bool,
    /// The language the shelf is written in.
    pub(super) lang: Option<Lang>,
    /// Every keycap's legend comes out of the keymap, so a rebinding has to
    /// reach the shelf on the next frame rather than at the next question.
    pub(super) keys: Option<baylee_client_core::prefs::Keymap>,
    /// The window's width, rounded to whole pixels: it is what decides
    /// whether the question keeps its keycaps.
    pub(super) window_w: i32,
}

/// The size the shelf's own prose is set at.
const SENTENCE_PT: f32 = 14.0;

/// The size an answer's label is set at.
const LABEL_PT: f32 = 13.0;

/// The size a keycap's legend is set at, which is what sets the cap's side:
/// `10.5 · KEYCAP_SIDE` is the 20-pixel square §4.2 asks for.
const CAP_PT: f32 = 10.5;

/// Between a keycap and the words it belongs to.
const CAP_GAP: f32 = 6.0;

/// An answer's own air, left and right of its contents.
const BUTTON_PAD_X: f32 = 10.0;

/// And above and below, which the button's [`BUTTON_H`] mostly settles: with
/// `BoxSizing::BorderBox` a 28-pixel outside and a 1-pixel border leave 26,
/// and this is what a 13-point line is given of it.
const BUTTON_PAD_Y: f32 = 4.0;

/// What the left column takes: the mana pool at its widest.
///
/// **Reserved rather than followed**, and at the *worst* case: the label, six
/// entries and the gaps between them, plus [`EDGE`]. §2.3 refuses to centre
/// the question between its neighbours precisely so that it does not move
/// when a mana pip arrives, and a reservation that followed the pool would be
/// that refusal undone one level down: the question would hold still against
/// its neighbour's *edge* and wander with its contents instead.
///
/// The number is **measured**, which is §10.1 item 3's acceptance and is the
/// one place it changed something. §2.3 estimated 325 from `0.52 × pt` per
/// character; the shipped face and this row's own padding give
///
/// ```text
///   EDGE                                        12.0
///   "Manavorrat", Medium 11 × UI_SCALE          60.5   (estimated 57)
///   label gap                                   10.0
///   6 entries: 1 + 3 + 16 + 4 + 14.0 + 3 + 1   252.1   (estimated 6 × 36)
///   5 gaps between them                         30.0
///                                              ─────
///                                              364.6
/// ```
///
/// The estimator was four pixels low on the label and the design's per-entry
/// figure had left out the restriction rim's own padding. Under-reserving is
/// the one direction that costs something: `arrange` slides the question to
/// clear *this* number, so a column wider than it says crowds the question by
/// the difference. The German label is the wider of the two languages (the
/// English "Mana pool" is 54.7) and a two-digit count is not covered — `×12`
/// is 6.8 px wider than `×8`, and six kinds of mana in double figures is
/// past what a duel does.
///
/// Six is §2.3's worst case and it is the count of mana *colours*, colourless
/// included. [`baylee_client_core::manapool::row`] lists restricted mana as
/// entries of its own after the plain ones (CR 106.6), so a seat floating
/// both kinds of one colour has more than six, and the column then grows past
/// its reservation and under the question instead of being clipped. Noted
/// rather than solved: reaching it takes seven sources of mana in one window,
/// one of them restricted, with none of it spent.
const LEFT_RESERVED: f32 = 365.0;

/// The same for the right column: two buttons, the gap between them and the
/// edge.
///
/// **Measured** like [`LEFT_RESERVED`] and for the same reason — `arrange`
/// slides the question to clear what it is told the neighbours take, so a
/// column wider than it says crowds the question by the difference. §2.3
/// estimated 214; the shipped Bold at 13 points gives
///
/// ```text
///   EDGE                                        12.0
///   "Remis anbieten" + 2·PAD_X + 2·border      119.1
///   BUTTON_GAP                                   8.0
///   "Aufgeben"  + 2·PAD_X + 2·border            82.9
///                                              ─────
///                                              222.0
/// ```
///
/// German is the wider of the two (English is 193.4) and is what is reserved,
/// as on the left.
///
/// The **armed** concession is not in this number, and that is the finding of
/// §10.2 step 5 rather than an omission: "Aufgeben? Nochmal drücken" is 200.4
/// wide, so §4.3's pair-that-grows-leftwards would be 339.5 here. Reserving
/// *that* would drop every 1280 window to `Compact` for the whole game to
/// pay for a state that lasts one click; drawing it unreserved puts it 71.5
/// px over the middle's last button — and the right column is spawned after
/// the middle, so the grown button would win the pick over the right end of
/// "Zug überspringen" and a skip-turn click would concede the game. So the
/// armed concession **stands alone**: the draw offer is not drawn beside it
/// (212.4 in German, 164.9 in English, both inside this reservation), which
/// is also the right answer on its own terms — a second button beside a
/// decision with no undo is a misclick target.
const RIGHT_RESERVED: f32 = 222.0;

/// What the left column calls itself, set quietly.
///
/// Smaller than an answer's label and smaller than the sentence, because it
/// is the one piece of text on the shelf that never changes: a player reads
/// it once and afterwards reads only what stands beside it.
const POOL_LABEL_PT: f32 = 11.0;

/// A floating mana's disc.
const POOL_PIP: f32 = 16.0;

/// The numeral beside it, in Bold — the one number on the shelf.
const POOL_COUNT_PT: f32 = 12.0;

/// From the label to the first entry: a wider step than between the entries,
/// because the label names the row and is not part of it.
const POOL_LABEL_GAP: f32 = 10.0;

/// Between two entries of the pool.
const POOL_ENTRY_GAP: f32 = 6.0;

/// Between an entry's disc and its numeral, which are one thing.
const POOL_PIP_GAP: f32 = 4.0;

/// Builds what stands on the shelf, when what is written on it changes.
///
/// After `sync_overlay`, because the shelf it fills is spawned there — and
/// the emptiness check below is what makes that ordering a preference rather
/// than a requirement: a shelf that has just been built afresh is filled even
/// when nothing in the revision moved.
#[allow(clippy::too_many_lines)] // one retained-UI rebuild, sectioned by comments
#[allow(clippy::too_many_arguments)]
pub fn sync_ledge(
    mut commands: Commands,
    duel: Res<Duel>,
    mut revision: ResMut<LedgeRevision>,
    mut layout: ResMut<LedgeLayout>,
    shelf: Query<(Entity, Option<&Children>), With<LedgeShelf>>,
    // Two exemptions from the rebuild below, and one query for both: the
    // pool's column, which outlives a rebuild so a mana can be drawn arriving
    // (`ledge/pool.rs`), and the two casts, which are spawned with the shelf
    // and never change.
    retained: Query<(), Retained>,
    fonts: Res<UiFonts>,
    settings: Res<crate::settings::ClientSettings>,
    prefs: Res<crate::prefs::Prefs>,
    windows: Query<&Window>,
) {
    let Ok((shelf, standing)) = shelf.single() else {
        return;
    };
    let lang = Lang::of(&settings.lang);
    // A finished game is the one question this shelf does not answer: who won
    // is the end screen's whole subject, set at four times the size. The
    // column simply empties — see `sync_overlay`'s own note, which this is
    // the other half of.
    let over = duel.ending().is_some();
    let turn = duel
        .view
        .as_ref()
        .map_or(baylee_client_core::Turn::Mine, |v| {
            baylee_client_core::Turn::of(v.active, v.seat)
        });
    let waiting = !duel.is_my_turn_to_act();
    let elsewhere = duel.browser.answers_here(duel.interaction.as_ref());
    let prompt = duel
        .cast_menu
        .as_ref()
        .filter(|_| !over)
        .map(|m| m.prompt().headline(lang, turn, duel.statics.as_ref()))
        .or_else(|| {
            duel.interaction
                .as_ref()
                .filter(|_| !over)
                .map(|i| i.prompt().headline(lang, turn, duel.statics.as_ref()))
        });
    #[allow(clippy::cast_possible_truncation)]
    let window_w = windows.single().map_or(1200, |w| w.width() as i32);
    let next = LedgeRevision {
        hand_order: duel.hand_order,
        seq: duel.board.as_ref().map(|b| b.seq),
        over,
        prompt,
        error: duel.last_error.clone().filter(|_| !over),
        link_note: duel.link_note.filter(|_| !over),
        waiting,
        elsewhere,
        selected: duel
            .interaction
            .as_ref()
            .map(|i| i.selected().collect())
            .unwrap_or_default(),
        armed: duel.armed.clone(),
        cast_menu: duel.cast_menu.is_some() && !over,
        holdable: duel.can_hold_for_stack(),
        can_offer_draw: duel.can_offer_draw(),
        concede_armed: duel.concede_armed,
        priority_held: duel.priority_held(),
        autopilot: duel.autopilot.is_some(),
        lang: Some(lang),
        keys: Some(prefs.keymap().clone()),
        window_w,
    };
    // The second half of the gate is what covers a shelf that was spawned
    // afresh with the revision still describing the tree before it. Everything
    // spawned *with* the shelf is exempt from the rebuild below and is
    // therefore also not evidence that the rebuild has run, so the question is
    // whether anything else is standing there.
    let filled = standing.is_some_and(|c| c.iter().any(|child| retained.get(child).is_err()));
    if *revision == next && filled {
        return;
    }
    *revision = next;

    for child in standing.into_iter().flatten() {
        // Everything the shelf was not spawned with. `ledge/pool.rs` says why
        // the pool's column is exempt: mana arrives and is spent *inside* one
        // question, and §4.1 wants that drawn arriving, which takes an entity
        // that outlives this rebuild. The two casts are exempt because they
        // are the shelf's own elevation and answer to nothing this system
        // knows about.
        if retained.get(*child).is_err() {
            commands.entity(*child).despawn();
        }
    }

    let tools = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(LEFT_RESERVED),
                top: px(LEDGE_PAD_Y - LIP + 2.0),
                column_gap: px(4),
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(shelf).add_child(tools);
    let orders = if window_w >= 1400 {
        crate::hand_order::HandOrder::ALL.to_vec()
    } else {
        vec![duel.hand_order]
    };
    for order in orders {
        let label = if window_w >= 1400 {
            order.label(lang).to_string()
        } else {
            format!("Hand: {} ›", order.label(lang))
        };
        let weight = if order == duel.hand_order {
            Weight::Candle
        } else {
            Weight::Secondary
        };
        let button = hand_tool(&mut commands, &fonts, &label, order, weight);
        commands.entity(button).insert(MenuButton {
            action: MenuAction::SortHand(if window_w >= 1400 {
                order
            } else {
                order.next()
            }),
        });
        commands.entity(tools).add_child(button);
    }

    // The middle is built first because it is the only one that knows how
    // wide it is, and how wide it is decides the arrangement of all three.
    let answers = answers_for(&duel, lang, over, waiting, elsewhere);
    let armed = duel
        .armed
        .as_ref()
        .filter(|_| !over)
        .and_then(|a| super::overlay::armed_label(&duel, lang, a));
    let sentence = revision
        .link_note
        .map(|note| (note.text(lang).to_string(), true))
        .or_else(|| revision.error.clone().map(|text| (text, true)))
        // An armed card says what it is about to do on the button itself, so
        // the question above it would be the same sentence a second time —
        // §6: the shelf never shows two sentences, and the armed row is the
        // one state that takes the sentence away rather than replacing it.
        //
        // A running hold replaces it instead, and that is the whole of §4.4:
        // "why is nobody asking me?" is a question about the middle, so it is
        // answered in the middle, where the question would have been.
        .or_else(|| {
            if holding(&duel, over, waiting) {
                Some(Phrase::HoldingPriority.text(lang).to_string())
            } else {
                revision.prompt.clone()
            }
            .filter(|_| armed.is_none())
            .map(|text| (text, false))
        });

    let caps = keys_for(&prefs, &answers, armed.is_some(), duel.priority_held());
    let caps_w: f32 = caps.iter().flatten().map(|c| cap_width(c) + CAP_GAP).sum();
    let mid = mid_width(sentence.as_ref().map(|(t, _)| t.as_str()), &answers, &caps);
    #[allow(clippy::cast_precision_loss)]
    let arrangement = baylee_client_core::ledge::arrange(
        window_w as f32,
        baylee_client_core::ledge::Columns {
            left: LEFT_RESERVED + if window_w >= 1400 { 310.0 } else { 145.0 },
            mid,
            right: RIGHT_RESERVED,
        },
        caps_w,
    );
    // The drawer stands over the question, so it has to be told where the
    // question ended up. Written here rather than read from the node, because
    // a `Node`'s padding is bevy_ui's to lay out and would be a frame stale by
    // the time anything read it back.
    layout.mid_x = arrangement.mid_x;
    layout.window_w = window_w;

    // Two, not three. The left column is `pool::PoolColumn` and is retained:
    // it was spawned with the shelf, the despawn above skipped it, and it
    // fills itself against a revision of its own so an arriving mana can be
    // drawn arriving. See `ledge/pool.rs`.
    let columns = [
        column_node(Side::Mid(arrangement.mid_x, window_w)),
        column_node(Side::Right),
    ]
    .map(|node| commands.spawn(node).id());
    commands.entity(shelf).add_children(&columns);

    ways_out(&mut commands, &fonts, lang, columns[1], &revision);

    let middle = columns[0];

    // `Split` is the rung that sends the sentence into the drawer, and there
    // is no drawer yet. Until there is, it draws what `Compact` draws: a
    // question the player has already read is worth less than the buttons,
    // which is exactly why `Split` gives it up — but dropping it on the floor
    // instead of putting it somewhere is not the same trade.
    let shows_sentence = arrangement.density.shows_sentence()
        || arrangement.density == baylee_client_core::ledge::Density::Split;
    if let Some((text, alarming)) = sentence.filter(|_| shows_sentence) {
        // `LEDGE_SOFT` and not `DIALOG_SOFT`: this sentence stands on the
        // shelf's own ground, which is no longer opaque. The constant says
        // what that costs and why it is only for ink standing here.
        let ink = if alarming {
            palette::DANGER
        } else if waiting {
            palette::LEDGE_SOFT
        } else {
            palette::DIALOG_INK
        };
        let line = self::sentence(&mut commands, &fonts, &text, SENTENCE_PT, ink);
        commands.entity(middle).add_child(line);
    }

    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(baylee_client_core::ledge::BUTTON_GAP),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(middle).add_child(row);

    if let Some(words) = armed {
        // The armed card replaces the answers rather than joining them: the
        // first button *is* the answer, and the second is the way back.
        armed_row(&mut commands, &fonts, lang, &prefs, row, &words);
    } else {
        for (i, (says, label)) in answers.iter().enumerate() {
            let cap = if arrangement.density.shows_keycaps() {
                caps[i].as_deref()
            } else {
                None
            };
            // The candle is the first *answer*'s and stays there whatever
            // else joins the row: a command is never the thing the shelf is
            // inviting, and the invitation is what the candle is for. Which
            // is why this reads the `Says` and not only the index — the hold
            // row is one command standing alone at zero, and a burning "Ask
            // me again" would say the game is waiting for it.
            let weight = match says {
                Says::Command(super::MenuAction::ReleaseHold) => Weight::Ghost,
                Says::Answer(_) if i == 0 => Weight::Candle,
                Says::Answer(_) | Says::Command(_) => Weight::Secondary,
            };
            let button = answer(&mut commands, &fonts, label, weight, cap);
            match *says {
                Says::Answer(action) => {
                    commands.entity(button).insert(PromptButton { action });
                }
                Says::Command(action) => {
                    commands.entity(button).insert(super::MenuButton { action });
                }
            }
            commands.entity(row).add_child(button);
        }
    }
}

/// Compact, framed category controls; icons and text share one hit target.
fn hand_tool(
    commands: &mut Commands,
    fonts: &UiFonts,
    label: &str,
    order: crate::hand_order::HandOrder,
    weight: Weight,
) -> Entity {
    use crate::hand_order::HandOrder;
    let mark = match order {
        HandOrder::Draw => baylee_client_core::tableicons::ZONES[0],
        HandOrder::Mana => '\u{f162}', // numeric ascending
        HandOrder::Name => '\u{f15d}', // alphabetic ascending
        HandOrder::Type => glyph::LIBRARY,
        HandOrder::Color => '\u{f53f}', // palette
    };
    let (fill, edge, ink) = weight.colours();
    let button = commands
        .spawn((
            Node {
                height: px(24),
                align_items: AlignItems::Center,
                column_gap: px(4),
                padding: UiRect::axes(px(6), px(2)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(edge),
        ))
        .id();
    if let Some(feel) = weight.feel() {
        commands.entity(button).insert(feel);
    }
    let icon = commands
        .spawn((
            Text::new(mark.to_string()),
            table_icon_tf(fonts, mark, 10.0),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    let label = commands
        .spawn((
            Text::new(label),
            tf_bold(fonts, 11.0),
            TextLayout::no_wrap(),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(button).add_children(&[icon, label]);
    button
}

/// What one button in the middle sends.
///
/// Two mechanisms wearing one shape, which is the honest way round: an
/// [`Answer`](Says::Answer) replies to the question the engine asked and rides
/// a [`PromptButton`]; a [`Command`](Says::Command) states a condition and
/// rides a [`MenuButton`], reaching the game by the road that button's key
/// already takes.
///
/// §10.1 item 7 of the design is about the one command there is: "resolve the
/// stack" is a condition rather than a reply, and stands in the row of replies
/// anyway, because while there *is* a stack it answers the question above it —
/// no, to none of that.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Says {
    /// An answer to the engine's question.
    Answer(PromptAction),
    /// Something done to the game beside answering it.
    Command(super::MenuAction),
}

/// Which answers this question takes, in the order they are offered.
///
/// Lifted out of `sync_overlay` with the two suppressions that are easy to
/// read as bugs intact. `cast_menu` takes the answers away because the
/// engine's window *behind* the client's own chooser is an ordinary priority,
/// and "Pass priority" under "Choose how it is cast" is two primary answers
/// saying opposite things. `elsewhere` takes the Confirm away because the zone
/// browser's footer already draws one, and a player ticking a fetchland's
/// target saw the same word twice on one screen.
///
/// What did not come from `sync_overlay` is the middle button of a priority,
/// which had no button at all before the shelf: see [`Says`].
fn answers_for(
    duel: &Duel,
    lang: Lang,
    over: bool,
    waiting: bool,
    elsewhere: bool,
) -> Vec<(Says, String)> {
    use baylee_engine::choice::Pending;
    // Above the suppression and not below it: the hold exists **only** while
    // this seat is not being asked, so a branch under that `return` would be
    // dead code that looked like a feature.
    if holding(duel, over, waiting) {
        return vec![(
            Says::Command(super::MenuAction::ReleaseHold),
            Phrase::HoldRelease.text(lang).to_string(),
        )];
    }
    if over || waiting || duel.cast_menu.is_some() {
        return Vec::new();
    }
    let say = |action: PromptAction, phrase: Phrase| {
        (Says::Answer(action), phrase.text(lang).to_string())
    };
    match duel
        .interaction
        .as_ref()
        .map(baylee_client_core::Interaction::pending)
    {
        Some(Pending::Mulligan { .. }) => vec![
            say(PromptAction::Keep, Phrase::KeepHand),
            say(PromptAction::Mulligan, Phrase::TakeMulligan),
        ],
        Some(Pending::YesNo { .. }) => vec![
            say(PromptAction::Yes, Phrase::ActAnswerYes),
            say(PromptAction::No, Phrase::ActAnswerNo),
        ],
        // Priority is not confirmed, it is *passed*, and the two words are
        // not interchangeable on a button: "OK" acknowledges something that
        // has already happened. "Skip turn" beside it is the same decision at
        // the larger size, and is where the phase rail's fast-forward went.
        //
        // Between them, and only while there is a stack to let go: three
        // buttons, three mechanisms — the engine's own answer, an engine hold,
        // and a client-side autopilot that never reaches the wire at all. They
        // look alike on purpose; what they have in common is that each of them
        // is a way of saying "not now".
        Some(Pending::Priority { .. }) => {
            let mut row = vec![say(PromptAction::Confirm, Phrase::PassPriority)];
            if duel.can_hold_for_stack() {
                row.push((
                    Says::Command(super::MenuAction::HoldForStack),
                    Phrase::ResolveTheStack.text(lang).to_string(),
                ));
            }
            row.push(say(PromptAction::SkipTurn, Phrase::SkipTheTurn));
            row
        }
        // Combat always offers all three, including with nothing declared:
        // "none" is a real answer, and the step does not end without one.
        Some(Pending::ChooseAttackers { .. }) => vec![
            say(PromptAction::AimNext, Phrase::AimNext),
            say(PromptAction::Confirm, Phrase::Attack),
            say(PromptAction::DeclareNothing, Phrase::DeclareNone),
        ],
        Some(Pending::ChooseBlockers { .. }) => vec![
            say(PromptAction::AimNext, Phrase::AimNext),
            say(PromptAction::Confirm, Phrase::Block),
            say(PromptAction::DeclareNothing, Phrase::DeclareNone),
        ],
        Some(Pending::DiscardChoice { count, .. })
            if duel
                .interaction
                .as_ref()
                .is_some_and(baylee_client_core::Interaction::can_confirm) =>
        {
            vec![(
                Says::Answer(PromptAction::Confirm),
                Phrase::counted(
                    usize::from(*count),
                    Phrase::DiscardCard,
                    Phrase::DiscardCards,
                )
                .fill(lang, &[&count.to_string()]),
            )]
        }
        Some(_)
            if !elsewhere
                && duel
                    .interaction
                    .as_ref()
                    .is_some_and(baylee_client_core::Interaction::can_confirm) =>
        {
            vec![say(PromptAction::Confirm, Phrase::ConfirmOk)]
        }
        _ => Vec::new(),
    }
}

/// Whether the shelf is showing a running hold instead of a question.
///
/// A hold is the one game state with **no other symptom**: the middle is
/// empty precisely *because* the seat is not being asked, which is exactly
/// what an idle shelf looks like. A player who set one two turns ago and
/// forgot would watch the game play itself with nothing on screen to blame.
///
/// Two mechanisms, one picture (§4.4): the engine's own hold, which a view
/// reports, and the client's autopilot, which never reaches the wire at all.
/// They differ in one thing only and it is the keycap — see [`keys_for`].
///
/// `cast_menu` takes it away for the reason it takes the answers away: the
/// engine's window behind the client's own chooser is an ordinary priority,
/// and the chooser is a question this seat is very much being asked.
fn holding(duel: &Duel, over: bool, waiting: bool) -> bool {
    !over
        && waiting
        && duel.cast_menu.is_none()
        && (duel.priority_held() || duel.autopilot.is_some())
}

/// The legend on each answer's keycap, or `None` where the answer has no key.
///
/// **Always out of the keymap**, never out of a string in the code: a player
/// who rebinds `Space` sees the new chord on the next frame.
/// [`baylee_client_core::ledge::shortcut_for`] is the bridge from an answer to
/// the action that sends it, and `chords` is the account's own binding of
/// that action. `first` and not `[0]`, because an action a player has unbound
/// is an answer with no cap rather than a panic.
///
/// A [`Says::Command`] names its action here rather than through that bridge,
/// which reaches `PromptAction` alone. That is one line per command and the
/// alternative is worse: `shortcut_for` lives in client-core, where
/// `MenuAction` is a renderer type it does not know and should not learn.
///
/// `held` is the one cap that depends on the *game* and not on the button:
/// the way out of an engine hold wears `F6`, because `F6` is what cancels a
/// running hold, and the way out of the **autopilot** wears nothing, because
/// no key ends that one (§4.4). One button, two mechanisms behind it, and a
/// cap that promised what its key does not do would be worse than no cap.
fn keys_for(
    prefs: &crate::prefs::Prefs,
    answers: &[(Says, String)],
    armed: bool,
    held: bool,
) -> Vec<Option<String>> {
    let legend = |action| {
        prefs
            .keymap()
            .chords(action)
            .first()
            .map(baylee_client_core::prefs::Chord::display)
    };
    if armed {
        // Not a `PromptAction` between them: an armed deed is the client's
        // own two-stage commit, fired by `Action::Primary` and taken back by
        // `Action::Cancel` (`input::armed_keys`), so the bridge does not
        // reach it and these two are named directly.
        return vec![
            legend(baylee_client_core::prefs::Action::Primary),
            legend(baylee_client_core::prefs::Action::Cancel),
        ];
    }
    answers
        .iter()
        .map(|(says, _)| match says {
            Says::Answer(action) => baylee_client_core::ledge::shortcut_for(*action),
            // The same key the button's own press takes: `menu_click` and the
            // key handler both go through `Duel::hold_action`, so the cap is
            // the truth about what the button does and not merely about what
            // else would do it.
            Says::Command(super::MenuAction::HoldForStack) => {
                Some(baylee_client_core::prefs::Action::HoldForStack)
            }
            // The same key again, and the same reason: `Duel::hold_action`
            // answers `PriorityHold::Always` while a hold is running, so F6
            // and this button send one thing. The autopilot has no key and
            // gets no cap.
            Says::Command(super::MenuAction::ReleaseHold) => {
                held.then_some(baylee_client_core::prefs::Action::HoldForStack)
            }
            Says::Command(_) => None,
        })
        .map(|action| action.and_then(legend))
        .collect()
}

/// Which of the three columns a node is, and where its centre goes.
enum Side {
    Left,
    /// The middle, with the centre [`baylee_client_core::ledge::arrange`] put
    /// it on and the width of the window it was measured against.
    Mid(f32, i32),
    Right,
}

/// One column, standing in the shelf's own band.
///
/// The vertical inset is written out rather than inherited, because the
/// shelf's padding does not reach an absolutely positioned child: taffy
/// measures an inset against the border box less the *border*, so the band
/// below the lip is `LEDGE_PAD_Y - LIP` down from the top and `LEDGE_PAD_Y`
/// up from the bottom, which is exactly [`BUTTON_H`] of room.
fn column_node(side: Side) -> impl Bundle {
    let mut node = Node {
        position_type: PositionType::Absolute,
        top: px(LEDGE_PAD_Y - LIP),
        bottom: px(LEDGE_PAD_Y),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(baylee_client_core::ledge::SENTENCE_GAP),
        ..default()
    };
    match side {
        Side::Left => {
            node.left = px(EDGE);
            node.justify_content = JustifyContent::Start;
            // The pool's own step, not the sentence's: these are entries in
            // one row rather than two things standing next to each other.
            node.column_gap = px(POOL_ENTRY_GAP);
        }
        Side::Right => {
            node.right = px(EDGE);
            node.justify_content = JustifyContent::End;
            // Two buttons, so the step between them is a button's and not a
            // sentence's — and [`RIGHT_RESERVED`] is measured with this one.
            node.column_gap = px(baylee_client_core::ledge::BUTTON_GAP);
        }
        // Full width and centred, so the question stands on the **window's**
        // middle — which is the middle of this seat's own mat. When `arrange`
        // has had to slide it off centre, the slide is paid for out of one
        // side's padding ([`mid_padding`]), which is the same arithmetic the
        // drawer stands on.
        Side::Mid(mid_x, window_w) => {
            node.left = px(0);
            node.right = px(0);
            node.justify_content = JustifyContent::Center;
            node.padding = mid_padding(mid_x, window_w);
        }
    }
    // The middle lies over the other two across the whole width, so without
    // this a draw offer and a concession would be dead or alive depending on
    // the order the three were spawned in — "a label swallows the hover", one
    // level up. The buttons inside it are pickable in their own right.
    (node, Pickable::IGNORE)
}

/// The right column: the two ways out of a game that is still being played.
///
/// It was a row of pills in the window's top-right corner, over the felt,
/// with the priority hold's chip beside it. The hold went to the middle
/// (§4.4, and it is the answer to a question the middle is asking), and these
/// two came here, which is where they were always about to be: the shelf has
/// three columns, and leaving the game belongs to no seat and to no question.
///
/// **After `GameOver` the column is empty** — there is nothing left to
/// concede and nobody left to offer a draw to, and the pair used to stay lit
/// in the corner under the end screen, hovering and answering nothing because
/// `DuelSet::Input` does not run in `Finished`.
///
/// Neither button wears a keycap and neither is going to: a draw offer is not
/// a thing to press by accident, and a concession is that twice over. What
/// the concession has instead is [`Weight::Danger`] and a second press
/// ([`crate::Duel::concede_armed`]), which every other key and every other
/// click take back.
///
/// The armed concession **stands alone**, which is this step's one deviation
/// from §4.3's "Remis rückt mit" and is measured in [`RIGHT_RESERVED`]: the
/// pair would be 339.5 px wide in German, the reservation is 222, and the
/// column is spawned *after* the middle — so the grown button would take the
/// press meant for the right end of "Zug überspringen" and a skipped turn
/// would concede the game.
fn ways_out(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    column: Entity,
    revision: &LedgeRevision,
) {
    if revision.over {
        return;
    }
    let armed = revision.concede_armed;
    if !armed {
        // A draw needs this seat's own priority (CR 104.4i, and `offer_draw`
        // refuses anything else), so the button says so rather than being a
        // live control whose usual answer is a refusal in the sentence above.
        let weight = if revision.can_offer_draw {
            Weight::Secondary
        } else {
            Weight::Dead
        };
        let offer = answer(commands, fonts, Phrase::OfferADraw.text(lang), weight, None);
        if weight != Weight::Dead {
            commands.entity(offer).insert(super::MenuButton {
                action: super::MenuAction::OfferDraw,
            });
        }
        commands.entity(column).add_child(offer);
    }
    let (words, weight) = if armed {
        (Phrase::ConcedeConfirm, Weight::Danger)
    } else {
        (Phrase::Concede, Weight::Secondary)
    };
    let concede = answer(commands, fonts, words.text(lang), weight, None);
    commands.entity(concede).insert(super::MenuButton {
        action: super::MenuAction::Concede,
    });
    commands.entity(column).add_child(concede);
}

/// The shelf's prose: the question, or whatever has replaced it.
///
/// The slip's voice on the dialog's ground. The slant is what carried the
/// question on parchment and it carries it here — a question is *written*,
/// where a button is stamped — but nothing else of the sheet comes with it:
/// no bleed, which is what a nib does to fibres, and no parchment ink.
/// Bracketed asides go grey through the same
/// [`baylee_client_core::prose::bracketed`] the slip used, because a key to
/// press or a count the board already shows is not part of the sentence.
/// `size` because the drawer writes in this voice too and writes quieter: a
/// hint about where to click is not as loud as the question it is under.
fn sentence(commands: &mut Commands, fonts: &UiFonts, text: &str, size: f32, ink: Color) -> Entity {
    let line = commands
        .spawn((
            Text::default(),
            tf_italic(fonts, size),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    for (run, aside) in baylee_client_core::prose::bracketed(text) {
        let span = commands
            .spawn((
                TextSpan::new(run.to_string()),
                tf_italic(fonts, size),
                TextColor(if aside { palette::LEDGE_SOFT } else { ink }),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(line).add_child(span);
    }
    line
}

/// How loud an answer is.
///
/// Five, and the last two are the right column's, which is why they arrived
/// with it in §10.2 step 5: a concession waiting for its second press is the
/// loudest thing this shelf ever says, and a draw the engine would refuse is
/// the quietest. [`palette::LEDGE_DEAD`] came in for the empty pool's em dash
/// and says exactly the same thing on a button — a place where something
/// would be.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Weight {
    /// The answer the engine is asking for — pass, attack, keep, yes, OK.
    ///
    /// Exactly one per question, and it is the **marking of the default
    /// button**: the one answer `Space` sends is the one that burns. Rejected:
    /// brass, which is a light on a *card* and means a thing already taken;
    /// and an underline as well, which is a second mark for one claim.
    Candle,
    /// Every other answer to the same question.
    Secondary,
    /// A way back out rather than an answer: cancel, ask again.
    Ghost,
    /// A concession that has been armed and is waiting for its second press.
    ///
    /// The one weight louder than the candle, and the only one on the shelf
    /// that is not an invitation. Rejected: a concession drawn in this colour
    /// at rest — then the second stage says nothing new — and a concession
    /// drawn as a ghost, which is too quiet for the hardest thing the button
    /// does.
    Danger,
    /// A control that is drawn because its place is reserved, and that cannot
    /// be pressed: a draw offer outside this seat's own priority.
    ///
    /// Not a greyed-out fill but **no fill at all**, so it reads as a place
    /// something would be rather than as a button somebody has switched off.
    /// It carries no [`Feel`] and no `MenuButton`, and takes
    /// `Pickable::IGNORE`: a control the pointer warms and the press ignores
    /// is a lie one frame long.
    Dead,
}

impl Weight {
    /// Fill, border, ink.
    const fn colours(self) -> (Color, Color, Color) {
        match self {
            // `DIALOG` on `CANDLE` is 7.39 : 1. Light ink on the candle is
            // 1.86 : 1 and is forbidden outright — see the palette.
            Self::Candle => (palette::CANDLE, palette::CANDLE, palette::DIALOG),
            // A dark inset key with a champagne edge, subordinate to the
            // candle-filled default without disappearing into the dock.
            Self::Secondary => (palette::DOCK_GROUND, palette::DOCK_EDGE, palette::DOCK_INK),
            Self::Ghost => (Color::NONE, Color::NONE, palette::DIALOG_INK),
            // `DIALOG` on `DANGER` is the second pair this shelf is held to
            // — §3.2 measures it at 6.11 : 1, and it is the same dark ink the
            // candle carries, because the two loud buttons are one register.
            Self::Danger => (palette::DANGER, palette::DANGER, palette::DIALOG),
            // A border and nothing behind it: the box is where the button
            // would be, and `LEDGE_DEAD` is the ink the empty pool writes its
            // em dash in.
            Self::Dead => (Color::NONE, palette::DIALOG_LINE, palette::LEDGE_DEAD),
        }
    }

    /// How it answers the pointer, or nothing where it must not answer at all.
    ///
    /// `Feel::new` shades a colour towards white and **keeps its alpha**, so
    /// a ghost resting at `Color::NONE` would be lifted to a brighter nothing
    /// and never answer the pointer at all. Its hot end is therefore stated.
    ///
    /// [`Dead`](Self::Dead) is the one weight with no answer: a button that
    /// warms under the pointer and does nothing when pressed is worse than a
    /// button that is plainly not there.
    fn feel(self) -> Option<Feel> {
        Some(match self {
            Self::Candle => Feel::new(palette::CANDLE),
            Self::Secondary => Feel::new(palette::DOCK_GROUND),
            Self::Ghost => Feel::rising_to(Color::NONE, palette::DIALOG_LIT),
            Self::Danger => Feel::new(palette::DANGER),
            Self::Dead => return None,
        })
    }

    /// The cap's own fill, border and legend, which are not the button's.
    ///
    /// A keycap is a dark key **on** the answer, whatever the answer is, so
    /// the fill is the shelf's own ground in all three cases. What differs is
    /// the legend: `CANDLE` on `DIALOG` reads at 7.39 : 1 where `DIALOG_SOFT`
    /// on the same ground reads at 4.69 : 1, and the brighter of the two
    /// belongs on the button the question is asking for.
    ///
    /// A ghost's cap is the secondary's exactly. `DIALOG_LIT` as its fill
    /// would measure 4.28 : 1 against the legend and fall under 4.5.
    ///
    /// The right column's two wear none at all — §4.3 refuses a key for
    /// either, a draw offer because it is not a thing to press by accident
    /// and a concession for the same reason twice over — so their arm here is
    /// the quiet one and is never reached by a drawing.
    const fn cap_colours(self) -> (Color, Color, Color) {
        match self {
            Self::Candle => (palette::DIALOG, palette::DIALOG, palette::CANDLE),
            Self::Secondary | Self::Ghost | Self::Danger | Self::Dead => {
                (palette::DIALOG, palette::DIALOG_LINE, palette::DIALOG_SOFT)
            }
        }
    }
}

/// One answer on the shelf: a keycap, and what pressing it does.
///
/// A sibling of `overlay::answer_button` rather than a change to it, and
/// deliberately: that one draws on **parchment**, and the end screen and the
/// lobby's own `Press` still stand on paper. Two registers, two functions,
/// one shape.
///
/// The label carries `Pickable::IGNORE` for the reason every label in this
/// client does: a `Text` is a `Node`, so a pickable one sits in front of the
/// button and `Feel` animates the padding while the middle goes dead.
pub(super) fn answer(
    commands: &mut Commands,
    fonts: &UiFonts,
    label: &str,
    weight: Weight,
    cap: Option<&str>,
) -> Entity {
    let (fill, edge, ink) = weight.colours();
    let button = commands
        .spawn((
            Node {
                height: px(BUTTON_H),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(CAP_GAP),
                padding: UiRect::axes(px(BUTTON_PAD_X), px(BUTTON_PAD_Y)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(edge),
        ))
        .id();
    // A dead control gets neither, which is the whole of what makes it dead:
    // no warmth under the pointer, and no press to be swallowed by a box that
    // was never going to answer it.
    if let Some(feel) = weight.feel() {
        commands.entity(button).insert(feel);
    } else {
        commands.entity(button).insert(Pickable::IGNORE);
    }
    // A machined key, not a floating pill. Both bevels stay inside the
    // existing border box and neither adds a target or changes measurement.
    if matches!(weight, Weight::Candle | Weight::Secondary | Weight::Danger) {
        for (top, colour) in [
            (true, palette::DOCK_INK.with_alpha(0.18)),
            (false, palette::DOCK_GROUND.with_alpha(0.65)),
        ] {
            let bevel = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(3),
                        right: px(3),
                        top: if top { px(1) } else { Val::Auto },
                        bottom: if top { Val::Auto } else { px(1) },
                        height: px(1),
                        ..default()
                    },
                    BackgroundColor(colour),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(button).add_child(bevel);
        }
    }
    if let Some(legend) = cap {
        let (cap_fill, cap_edge, cap_ink) = weight.cap_colours();
        let key = keycap(commands, fonts, legend, cap_fill, cap_ink, cap_edge, CAP_PT);
        commands.entity(button).add_child(key);
    }
    let words = commands
        .spawn((
            Text::new(label.to_string()),
            tf_bold(fonts, LABEL_PT),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(button).add_child(words);
    button
}

/// The armed deed as a pair of buttons: the deed itself, and the way back.
///
/// Two buttons and no sentence between them, because the first one *is* the
/// sentence — a line reading "Play this card" beside a button called "Send"
/// says the same thing twice and leaves a player to work out which half is
/// the button. It is the one state that takes the question off the shelf.
fn armed_row(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    prefs: &crate::prefs::Prefs,
    row: Entity,
    words: &super::overlay::ArmedWords,
) {
    let caps = keys_for(prefs, &[], true, false);
    let cancel = super::overlay::ArmedWords {
        text: Phrase::ArmedCancel.text(lang).to_string(),
        cost: None,
    };
    for (i, (action, words, weight)) in [
        (MenuAction::SendArmed, words, Weight::Candle),
        (MenuAction::CancelArmed, &cancel, Weight::Ghost),
    ]
    .into_iter()
    .enumerate()
    {
        let (_, _, ink) = weight.colours();
        let button = answer(commands, fonts, "", weight, caps[i].as_deref());
        commands.entity(button).insert(MenuButton { action });
        // The phrase splits at its `{0}`; one with none, or a deed with no
        // price to quote, is one piece and the pips are skipped. A cost is
        // **drawn** and never spelled — `{4}{U}{U}` as letters is the thing
        // the deck builder deliberately does not do.
        let (head, tail) = words
            .cost
            .and_then(|_| words.text.split_once("{0}"))
            .unwrap_or((words.text.as_str(), ""));
        put_words(commands, fonts, button, head.trim(), ink);
        if let Some(cost) = words.cost {
            for pip in baylee_client_core::manapip::cost(&cost) {
                let mark = crate::manaui::spawn_pip(commands, fonts, pip, 15.0);
                commands.entity(button).add_child(mark);
            }
        }
        put_words(commands, fonts, button, tail.trim(), ink);
        commands.entity(row).add_child(button);
    }
}

/// One piece of a button's words, or nothing at all when the piece is empty.
fn put_words(commands: &mut Commands, fonts: &UiFonts, button: Entity, text: &str, ink: Color) {
    if text.is_empty() {
        return;
    }
    let node = crate::manaui::spawn_rich_label(commands, fonts, text, LABEL_PT, ink);
    commands.entity(button).add_child(node);
}

/// How wide a keycap's box is, legend and air together.
///
/// [`KEYCAP_SIDE`] is a floor and not a width: a cap grows with its legend,
/// so `F6` sits in the 20-pixel square and `⇧Tab` does not.
fn cap_width(legend: &str) -> f32 {
    (CAP_PT * KEYCAP_SIDE).max(super::text_width(legend, CAP_PT, true) + 2.0 * CAP_PT * 0.45)
}

/// How wide the middle column will be, before it is laid out.
///
/// The estimate [`baylee_client_core::ledge::arrange`] is fed, and the reason
/// [`super::text_width`] exists — `bevy_ui` measures text during layout, and
/// this is a decision the layout depends on.
fn mid_width(sentence: Option<&str>, answers: &[(Says, String)], caps: &[Option<String>]) -> f32 {
    let buttons: f32 = answers
        .iter()
        .enumerate()
        .map(|(i, (_, label))| {
            let cap = caps
                .get(i)
                .and_then(Option::as_deref)
                .map_or(0.0, |c| cap_width(c) + CAP_GAP);
            cap + super::text_width(label, LABEL_PT, true) + 2.0 * BUTTON_PAD_X
        })
        .sum();
    #[allow(clippy::cast_precision_loss)]
    let gaps = baylee_client_core::ledge::BUTTON_GAP * answers.len().saturating_sub(1) as f32;
    let words = sentence.map_or(0.0, |text| {
        super::text_width(text, SENTENCE_PT, false) + baylee_client_core::ledge::SENTENCE_GAP
    });
    words + buttons + gaps
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shelf is a **dialog**, and nothing on it is borrowed from the
    /// parchment or from the table.
    ///
    /// Its own scan rather than an addition to `sheet.rs`'s, which is bound
    /// to `sheet.rs` and `overlay.rs` by name and would never have looked at
    /// a new file. Three things are forbidden and each for its own reason:
    ///
    /// - `palette::ACCENT` is the teal `docs/design.md` §1.2 retires. Candle
    ///   is what replaces it, and a shelf that kept one teal control would be
    ///   the retirement half-done in the most visible place there is.
    /// - `BRASS` as a **letter** is gilt, which on this client means a thing
    ///   already *taken* — an armed deed, a place in an ordering. The shelf
    ///   asks; it does not report. (As a fill it never appears here at all,
    ///   so the needle is the `TextColor`.)
    /// - `PARCHMENT` in any form is the sheet a question was written on, and
    ///   §3.1 is the whole argument for why it is not written on one now.
    #[test]
    fn the_ledge_speaks_only_dialog() {
        for forbidden in [
            "palette::ACCENT",
            "TextColor(palette::BRASS",
            "palette::PARCHMENT",
            // The four the mana pool brought with it out of the chip it was.
            // `ACTIVE` is the one §3.2 names with this very pip as its
            // example — brass is a light at the edge of a *card* — and the
            // other three are the cool near-black panel register the whole
            // dialog ground replaces. Written as `palette::INK)` because
            // `palette::INK` is a prefix of `INK_DANGER` and `INK_BRASS`.
            "palette::ACTIVE",
            "palette::PANEL",
            "palette::MUTED",
            "palette::DEAD",
            "palette::INK)",
        ] {
            assert!(
                !drawn().contains(forbidden),
                "`{forbidden}` is on the shelf, which is a dialog: the ledge \
                 has one register and this is not in it"
            );
        }
    }

    /// The half of this file that draws, which is what a scan is about.
    ///
    /// Everything below `#[cfg(test)]` is these tests, and the needles they
    /// name are written out here in full — a scan over the whole file finds
    /// its own list and reports the rule as the violation. Cutting at the
    /// attribute is the shortest honest answer; the alternative is counting
    /// occurrences, which passes the moment a second one appears in a doc
    /// comment.
    ///
    /// The drawer is read with it (§10.2 step 6), which is why this hands
    /// back a `String` rather than the `&'static str` it used to: the drawer
    /// is the same surface under the same rules, and a scan that read only
    /// the half of it standing on the shelf would be `sheet.rs`'s file-bound
    /// scan made twice. It carries no `#[cfg(test)]` of its own — what it
    /// draws is asserted by driving it, in `overlay.rs`' harness — so it
    /// joins whole.
    ///
    /// The pool joined it in step 6b, on the same argument and on the day it
    /// still *passed*: its three inks are `DIALOG_SOFT`, `LEDGE_DEAD` and
    /// `DIALOG`, every one of them a pair this module already measures. A scan
    /// is worth widening while it is green — widening one to make a failure go
    /// away is how a bound gets loosened to fit what it found.
    fn drawn() -> String {
        let shelf = include_str!("ledge.rs")
            .split_once("#[cfg(test)]")
            .expect("the tests are still where they were")
            .0;
        format!(
            "{shelf}{}{}",
            include_str!("ledge/drawer.rs"),
            include_str!("ledge/pool.rs")
        )
    }

    /// What an answer is written in has to be readable on what it is written
    /// on.
    ///
    /// The pair the shelf lives or dies by: `DIALOG` on `CANDLE` is the ink
    /// of every default button, and the *inverse* — light ink on the candle —
    /// is 1.86 : 1 and is forbidden outright by §3.2. A test on the ratio
    /// alone would pass on either, so both ends are stated.
    ///
    /// `LEDGE_DEAD / DIALOG` is the third pair, and it is the one bounded on
    /// **both** sides on purpose: §3.2 puts it at 3.08 : 1, over the 3.0 a
    /// large glyph is held to and under the 4.5 prose needs, because an empty
    /// pool's em dash has to be legible without being something to attend to.
    /// A one-sided assertion here would let it drift up into the register of
    /// the things that are actually there.
    #[test]
    fn a_candle_is_dark_enough_to_write_on() {
        fn linear(c: f32) -> f32 {
            if c <= 0.040_45 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        fn contrast(a: Color, b: Color) -> f32 {
            let luma = |c: Color| {
                let s = c.to_srgba();
                0.2126f32.mul_add(
                    linear(s.red),
                    0.7152f32.mul_add(linear(s.green), 0.0722 * linear(s.blue)),
                )
            };
            let (one, two) = (luma(a), luma(b));
            (one.max(two) + 0.05) / (one.min(two) + 0.05)
        }

        let ink = contrast(palette::DIALOG, palette::CANDLE);
        assert!(
            ink >= 4.5,
            "the candle's own label has to be readable: {ink:.2}:1"
        );
        // The armed concession is the second loud button and carries the same
        // dark ink, which is what makes the pair one register rather than two
        // colours that happen to be bright. §3.2 measures it at 6.11 : 1.
        let danger = contrast(palette::DIALOG, palette::DANGER);
        assert!(
            danger >= 4.5,
            "the loudest thing the shelf says has to be readable while it \
             says it: {danger:.2}:1"
        );
        let wrong = contrast(palette::DIALOG_INK, palette::CANDLE);
        assert!(
            wrong < 3.0,
            "this test's premise is that light ink on a candle cannot be \
             read, and it measured {wrong:.2}:1"
        );
        // The keycap on a secondary answer: a dark key on the shelf's own
        // ground, and the quiet legend still has to carry 11-point text.
        let legend = contrast(palette::DIALOG_SOFT, palette::DIALOG);
        assert!(
            legend >= 4.5,
            "a keycap nobody can read is a key nobody presses: {legend:.2}:1"
        );
        // The em dash of an empty pool, and a draw offer the engine would
        // refuse: a thing that is not there.
        let absent = contrast(palette::LEDGE_DEAD, palette::DIALOG);
        assert!(
            (3.0..4.5).contains(&absent),
            "`LEDGE_DEAD` says \"nothing here\" and has to be read without \
             being read *at*: {absent:.2}:1 is outside 3.0 … 4.5"
        );
    }

    /// Which row of the drawer is taken is said by its border, and the wash is
    /// only allowed to not get in the way.
    ///
    /// §5 named [`drawer::PICKED_WASH`] and nothing held it to anything, which
    /// is the shape of claim this file measures rather than believes. It is
    /// bounded on **both** sides, and the two bounds point opposite ways: the
    /// ink has to survive the wash (the row still carries words), and the wash
    /// must not be credited with the claim it cannot make — 1.08 : 1 against
    /// the fill every other row already has is not a difference, so a drawer
    /// that lost the candle border would go on passing a test that only asked
    /// whether the picked row was washed.
    #[test]
    fn a_picked_row_is_said_by_its_border() {
        /// sRGB → linear, as in `a_candle_is_dark_enough_to_write_on`.
        fn linear(c: f32) -> f32 {
            if c <= 0.040_45 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        fn contrast(a: Color, b: Color) -> f32 {
            let luma = |c: Color| {
                let s = c.to_srgba();
                0.2126f32.mul_add(
                    linear(s.red),
                    0.7152f32.mul_add(linear(s.green), 0.0722 * linear(s.blue)),
                )
            };
            let (one, two) = (luma(a), luma(b));
            (one.max(two) + 0.05) / (one.min(two) + 0.05)
        }
        /// `over` at its own alpha, laid on an opaque `under`.
        fn over(over: Color, under: Color) -> Color {
            let (o, u) = (over.to_srgba(), under.to_srgba());
            let mix = |a: f32, b: f32| a.mul_add(o.alpha, b * (1.0 - o.alpha));
            Color::srgb(
                mix(o.red, u.red),
                mix(o.green, u.green),
                mix(o.blue, u.blue),
            )
        }

        let taken = over(
            palette::CANDLE.with_alpha(drawer::PICKED_WASH),
            palette::DOCK_GROUND,
        );
        let (fill, edge, ink) = drawer::PANEL_KEY;

        let words = contrast(ink, taken);
        assert!(
            words >= 4.5,
            "a row that has been picked is still a row to read: {words:.2}:1"
        );
        let said = contrast(palette::CANDLE, edge);
        assert!(
            said >= 3.0,
            "the border is the whole of what says a row is taken: {said:.2}:1"
        );
        let alone = contrast(taken, fill);
        assert!(
            alone < 1.2,
            "this test's premise is that the wash cannot say it on its own, \
             and it measured {alone:.2}:1 against an untaken row's fill"
        );
    }

    /// The shelf does not follow the pointer, and the struct is where that is
    /// decided.
    ///
    /// [`LedgeRevision`] exists *because* [`super::HudRevision`] counts the
    /// hover; a `hovered` field here would put the shelf back in the rebuild
    /// it was taken out of, and every `Feel` on it back to rest whenever the
    /// pointer crossed a hand card. Nothing about that is visible at the call
    /// site — the field would simply be compared and assigned like any other
    /// — so it is read out of the source, the way `hud/tests.rs` reads
    /// `HudRevision`'s own fields.
    #[test]
    fn the_shelf_does_not_follow_the_pointer() {
        let source = include_str!("ledge.rs");
        let body = source
            .split_once("pub struct LedgeRevision {")
            .expect("the struct is still called that")
            .1;
        let body = body.split_once("\n}").expect("and still closes").0;
        let fields: Vec<&str> = body
            .lines()
            .filter_map(|line| {
                let name = line.trim().strip_prefix("pub(super) ")?;
                let (name, _) = name.split_once(':')?;
                name.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
                    .then_some(name)
            })
            .collect();
        assert!(fields.len() > 8, "the fields did not parse: {fields:?}");
        for field in fields {
            assert!(
                !field.contains("hover"),
                "`{field}` puts the shelf back in the hover's rebuild, which \
                 is the one thing this counter exists to keep it out of"
            );
        }
    }

    /// A question that has to give something up gives up its keycaps before
    /// its words, and its words before its buttons.
    ///
    /// The renderer's half of `client-core`'s ladder: `arrange` decides the
    /// rung and this is what the rung is spent on. `Split` is the rung that
    /// sends the sentence to the drawer, and until there is a drawer it draws
    /// what `Compact` draws — a question the player has already read is worth
    /// less than the buttons, which is why `Split` gives it up, but dropping
    /// it on the floor instead of putting it somewhere is a different trade.
    #[test]
    fn the_rungs_are_spent_on_the_caps_first() {
        use baylee_client_core::ledge::Density;
        assert!(Density::Full.shows_keycaps() && Density::Full.shows_sentence());
        assert!(!Density::Compact.shows_keycaps() && Density::Compact.shows_sentence());
        assert!(!Density::Split.shows_keycaps() && !Density::Split.shows_sentence());
        // And the one deviation, stated where it can be found again.
        assert!(
            include_str!("ledge.rs").contains("|| arrangement.density == "),
            "`Split` no longer borrows `Compact`'s sentence: either the \
             drawer exists and this line should go, or the question is being \
             dropped"
        );
    }

    /// A keycap is a floor and not a width.
    ///
    /// `F6` sits in the 20-pixel square [`KEYCAP_SIDE`] gives it and
    /// `Shift+Tab` does not, which is the whole reason `sheet::cap`'s box
    /// grows with its legend — and the reason §2.3's own worked example is 18
    /// pixels light: it measures `[Space]` at the square, and "Space" is five
    /// characters.
    #[test]
    fn a_long_chord_gets_a_wider_key() {
        let square = CAP_PT * KEYCAP_SIDE;
        assert!(
            (cap_width("6") - square).abs() < f32::EPSILON,
            "one character sits in the square: {}",
            cap_width("6")
        );
        for chord in ["Shift+Tab", "Space"] {
            assert!(
                cap_width(chord) > square,
                "`{chord}` does not, and clipping a chord would be worse than \
                 growing its key: {}",
                cap_width(chord)
            );
        }
        // And the slip in §2.3's own worked example, written down where it
        // can be checked: it measures `[Space]` at the square, which is 18
        // pixels light. The priority middle is 606 and not 588 — still
        // `Full` at 1280 against the design's own 325 and 214, so nothing
        // downstream moved when it was found. (Both of those are measured
        // numbers now, 365 and 222, and 623 is still `Full`.)
        assert!(
            cap_width("Space") - square > 17.0,
            "the design's arithmetic was out by {}",
            cap_width("Space") - square
        );
    }

    /// A command's cap names the key that does the same thing, and it comes
    /// out of the keymap like every other cap on the shelf.
    ///
    /// The bridge [`keys_for`] uses for an answer —
    /// `baylee_client_core::ledge::shortcut_for` — reaches `PromptAction`
    /// alone, so a [`Says::Command`] names its action in the renderer. This is
    /// what makes that a mapping rather than a guess: unbind
    /// `Action::HoldForStack` in the default keymap and the cap goes, which is
    /// right; point it at another action and this fails, which is the part
    /// worth having.
    #[test]
    fn a_command_wears_the_key_that_does_the_same_thing() {
        let prefs = crate::prefs::Prefs::default();
        let row = vec![
            (Says::Answer(PromptAction::Confirm), "Pass".to_string()),
            (
                Says::Command(super::MenuAction::HoldForStack),
                "Resolve the stack".to_string(),
            ),
        ];
        let caps = keys_for(&prefs, &row, false, false);
        assert_eq!(
            caps.get(1).and_then(Option::as_deref),
            Some("F6"),
            "the whole claim of the cap is that this key does this: {caps:?}"
        );
        // The counter-half: a command with no key of its own wears none,
        // rather than borrowing the one beside it.
        let row = vec![(
            Says::Command(super::MenuAction::Concede),
            "Concede".to_string(),
        )];
        assert_eq!(
            keys_for(&prefs, &row, false, false),
            vec![None],
            "a concession has no key and must not grow one here"
        );
    }

    /// The way out of a hold wears the key that ends a hold, and the way out
    /// of the autopilot wears nothing.
    ///
    /// One button and one sentence for two mechanisms (§4.4), which is right
    /// — a player who has stopped being asked does not care which of them did
    /// it — and the keycap is the one place the difference is real. `F6`
    /// cancels a running engine hold; **no** key ends the autopilot, so a cap
    /// on that button would promise a way out that the keyboard does not
    /// have.
    #[test]
    fn the_way_out_of_a_hold_wears_a_key_and_the_way_out_of_the_pilot_does_not() {
        let prefs = crate::prefs::Prefs::default();
        let row = vec![(
            Says::Command(super::MenuAction::ReleaseHold),
            "Ask me again".to_string(),
        )];
        assert_eq!(
            keys_for(&prefs, &row, false, true),
            vec![Some("F6".to_string())],
            "a running hold is cancelled by F6, so the button says F6"
        );
        assert_eq!(
            keys_for(&prefs, &row, false, false),
            vec![None],
            "no key ends the autopilot, so the same button wears no cap"
        );
    }
}

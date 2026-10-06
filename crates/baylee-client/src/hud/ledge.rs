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

pub(super) mod ai_log;
pub(super) mod drawer;
pub(super) mod log;
pub(super) mod menu;
pub(super) mod players;
pub(super) mod pool;
// Not `hud::tray`, which is the zone dialog. These are the two doors in the
// bar the dialog is put away into; the collision and why it stands are in
// the module's own doc.
pub(super) mod tray;

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
    /// An explicitly selected row can make Confirm available without changing the board.
    chosen_index: Option<usize>,
    /// Identity of the damage question represented by the confirmation button.
    decision_id: Option<baylee_client_core::interaction::DecisionId>,
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
    /// A refusal, which replaces that sentence.
    ///
    /// A [`Refusal`] rather than a rendered `String` for the reason
    /// `Duel::last_error` carries one: the slot takes this client's own
    /// sentences *and* another process's prose, and only the first of those
    /// can be translated.
    ///
    /// Held **unrendered**, like the `link_note` beside it and unlike
    /// `prompt`. That costs nothing here, because `lang` is a field of this
    /// revision: a player changing language mid-game rebuilds the shelf
    /// whichever form this takes, and comparing the refusal itself is the
    /// narrower question of the two.
    pub(super) error: Option<baylee_client_core::i18n::Refusal>,
    /// What the connection has to say, which replaces it first.
    pub(super) link_note: Option<baylee_client_core::i18n::Phrase>,
    /// Whether this seat is being asked at all — the sentence's weight, and
    /// whether there are answers under it.
    pub(super) waiting: bool,
    /// Whether a decision countdown is standing beside the sentence.
    ///
    /// The **presence** of the number and never its value. A revision
    /// carrying the seconds would rebuild this whole tree once a second for
    /// the last minute of every question; what is gated here is the cell
    /// appearing and going away, which happens twice per question, and
    /// `count_down_the_decision` writes the digits into it without touching
    /// a `Node`.
    pub(super) clock: bool,
    /// Whether the zone browser's dialog is holding the answer, which is what
    /// keeps a second Confirm off the shelf.
    pub(super) elsewhere: bool,
    /// What has been picked so far, because `can_confirm` reads it and the
    /// lone OK answer appears the moment it turns true.
    pub(super) selected: Vec<ObjectId>,
    /// Player targets also change whether the answer can be confirmed (#182).
    pub(super) selected_players: Vec<PlayerId>,
    /// Combat pairings are not part of the ordinary selected-object list.
    pub(super) declared: usize,
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
    /// without it here the panel's rows would keep the words they were drawn
    /// with. It no longer takes the draw offer away — [`menu`] keeps it as a
    /// dead row — but it still reaches the shelf, because this is what the
    /// panel's own revision reads it through.
    pub(super) concede_armed: bool,
    /// Whether the game menu is open, which is the burger's ground.
    ///
    /// The same shape as `concede_armed`: entirely the client's, following
    /// nothing but a press, and here because without it the door would not
    /// say which side of it the player is on. [`tray::StripRevision`] carries
    /// `open` for the zones button for exactly this reason.
    pub(super) menu_open: bool,
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

/// What the left of the shelf takes: the hand's sorting buttons, at the
/// window's own edge.
///
/// It was the mana pool's worst case, 365 px of label and six entries, until
/// the owner moved the pool off this shelf and put these in its place —
/// *"Dafür verschiebe die Hand Sorting Buttons in der Actions-Bar ganz nach
/// links und mache sie etwas kleiner"* (19.09.2026). The tools used to stand
/// at `LEFT_RESERVED` with their own measured width added on top of it; now
/// they start at [`EDGE`] and this is the whole of the left column.
///
/// **Reserved rather than followed**, still, and that is not the same
/// argument it was. §2.3 refuses to centre the question between its
/// neighbours so that it does not move when a mana pip arrives; the pool's
/// contents changed *inside* one question and a reservation that followed
/// them would have been that refusal undone one level down. These do not.
/// What changes this row's width is the interface language and whether the
/// window is wide enough for all five buttons, neither of which can move
/// while a question stands — so reserving the wider language is caution and
/// no longer a requirement.
///
/// The number is **measured**, as its predecessor was, because
/// [`super::text_width`] is an estimator that was four pixels low on the
/// pool's own label — and it is measured in two halves, because the client
/// draws one language at a time. The German row was photographed at 1728 and
/// runs from [`EDGE`] to 281.0, so it is **269.0** wide; the five labels'
/// own widths came out of the shipped `AlegreyaSans-Bold` at the pixel size
/// this row sets it (`TOOL_PT` × [`super::UI_SCALE`]), where English is 14.5
/// wider than German across the five. 269.0 + 14.5 is 283.5 and this rounds
/// up. The current readability scale adds ten percent; reserve that increase
/// over the whole row so the marks and padding keep some spare room too.
///
/// English is the wider of the two here, which is the opposite way round from
/// [`RIGHT_RESERVED`]: "Draw order" is 10.6 px wider than "Zugfolge" and
/// "Type" 5.6 wider than "Typ", against "Color" saving 1.8 on "Farbe".
const TOOLS_WIDE: f32 = 312.0;

/// The same row when the window is too narrow for all five, and one button
/// cycles through the orders instead.
///
/// Its label is the longest of the five wearing `Hand: … ›`, so it is wider
/// than any single button of the wide row and narrower than two of them.
/// `Hand: Draw order ›` sets 89.8 wide in the same face at the same size, and
/// what a button puts around its label — two borders, the air either side,
/// the step to the mark and the mark itself — measured 23.2 to 25.7 across
/// the five photographed buttons, the spread being the marks' own advances.
/// The former scale reserved 89.8 + 25.7, rounded up. The current reservation
/// includes the ten-percent readability increase.
const TOOLS_NARROW: f32 = 128.0;

/// What the left column takes at this width, [`EDGE`] included.
///
/// The threshold is [`sync_ledge`]'s own — a window at least 1400 px wide is
/// given all five buttons — and it is written in both places rather than
/// passed, because the row is *built* there and *reserved* here and a
/// parameter would only make it look as though one of them decided.
fn tools_reserved(window_w: i32) -> f32 {
    EDGE + if window_w >= WIDE_ENOUGH_FOR_FIVE {
        TOOLS_WIDE
    } else {
        TOOLS_NARROW
    }
}

/// How wide a window has to be before the hand is sorted by five buttons
/// rather than by one that cycles.
const WIDE_ENOUGH_FOR_FIVE: i32 = 1400;

/// How tall one of those buttons is.
///
/// *"und mache sie etwas kleiner"*, which is a comparison and needs the thing
/// it is smaller **than**: [`BUTTON_H`], the height of anything a player
/// presses on this shelf. These are the one row here that is not an answer to
/// the question — a standing preference, set once and read at a glance — so
/// they are the one row allowed under that height. Eight pixels of it, which
/// is twice what a strip's own button gives up ([`STRIP_H`] takes four) — the
/// strip is still a control the player presses and this row is a label they
/// read, so the two are smaller than a button for different amounts and for
/// different reasons.
const TOOL_H: f32 = BUTTON_H - 8.0;

/// A sorting button's own air, left and right of its contents.
const TOOL_PAD_X: f32 = 5.0;

/// Between the mark and the word it belongs to.
const TOOL_MARK_GAP: f32 = 3.0;

/// And between two of the buttons.
const TOOL_GAP: f32 = 3.0;

/// The size a sorting button's word is set at.
const TOOL_PT: f32 = 10.0;

/// And its mark, which is a picture and carries the smaller of the two.
const TOOL_MARK_PT: f32 = 9.0;

/// The same for the right column: the burger, and the log's and the zones'
/// doors beside it.
///
/// **Measured** like [`TOOLS_WIDE`] and for the same reason — `arrange`
/// slides the question to clear what it is told the neighbours take, so a
/// column wider than it says crowds the question by the difference. Here the
/// measurement is short, because no button in it has a label: [`EDGE`],
/// [`menu::BURGER`] and [`tray::WIDTH`], 108 px against the 222 the pair
/// took. The doors are not the shelf's children ([`tray`] says why), but
/// they stand in its row, so their room is reserved here all the same
/// (#264).
///
/// What the 222 was, kept because it is what the number is *against*: the
/// shipped Bold at 13 points put "Remis anbieten" at 119.1 as a button,
/// "Aufgeben" at 82.9, `BUTTON_GAP` between them and `EDGE` outside — German
/// being the wider of the two, English 193.4. §2.3 had estimated 214.
///
/// And the **armed** concession was never in it, which was §10.2 step 5's
/// finding rather than an omission: "Aufgeben? Nochmal drücken" is 200.4
/// wide, so §4.3's pair-that-grows-leftwards would have been 339.5. Reserving
/// that would have dropped every 1280 window to `Compact` for the whole game
/// to pay for a state lasting one click; drawing it unreserved put it 71.5 px
/// over the middle's last button — and the right column is spawned after the
/// middle, so the grown button would win the pick over the right end of "Zug
/// überspringen" and a skip-turn click would concede the game. The answer was
/// that the armed concession **stood alone**, with the draw offer taken out
/// from beside it.
///
/// That whole knot is gone rather than smaller. The concession is a row in a
/// panel of fixed width now ([`menu`]), so there is nothing for it to grow
/// over and no reason to take its neighbour away — and the draw offer keeps
/// its place as a dead row, which is what stops the confirm row sliding up
/// under a pointer that is about to press it.
const RIGHT_RESERVED: f32 = EDGE + menu::BURGER + tray::WIDTH;

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

mod answers;
mod buttons;
mod clock;
mod shelf;
mod sync;

pub(super) use answers::*;
pub(crate) use buttons::*;
pub use clock::*;
pub use shelf::*;
pub use sync::*;

#[cfg(test)]
mod tests;

//! The 2D overlay: everything a player reads rather than manipulates.
//!
//! It used to open with two bands across the top of the window — a strip of
//! seat tabs and a twelve-step phase rail under it, a hundred and ten pixels
//! of every screen, in every game. Both are gone. What they said is drawn on
//! the table now, once per seat, on the shelf at the front of that seat's own
//! mat: [`seatbar`] is the whole of it, and `docs/client.md` §"Every seat's
//! bar" is normative. The camera got those hundred and ten pixels back, which
//! is what [`crate::table::Canvas`] spends on cards.
//!
//! What is left up here belongs to the window rather than to a seat:
//!
//! - **Top right** — the two controls that end a game, offer a draw and
//!   concede, as a row of pills over the felt rather than a band across it.
//! - **Right** — the stack, drawn as cards, under those pills.
//! - **The middle** — the prompt slip, the zone browser, the hover preview.
//! - **Bottom** — the hand bar: card images, overlapping but never less
//!   than 30% visible, horizontally scrollable when even that overflows,
//!   with a large hover tooltip for reading a card.
//!
//! The overlay is retained-UI: it is rebuilt only when something it shows
//! actually changed (snapshot, prompt, hover, selection, orders). The seat
//! bars are a **second** retained tree with a revision of their own, because
//! [`HudRevision`] counts the hover and a bar that was rebuilt on every
//! pointer move would be rebuilt some hundreds of times a turn.

use baylee_client_core::i18n::{Lang, Phrase};

use crate::Duel;
use crate::ambience::Feel;
use crate::cardmat::{CardLook, CardUiMaterial, UiCardMaterials, UiCards, finish_of};
use crate::textures::CardTextures;
use baylee_client_core::automation::{AutoPilot, RailRow};
use baylee_client_core::browser::Placement;
use baylee_client_core::card_face::CardFace;
use baylee_client_core::images::{ArtSize, FinishTreatment, ImageKey};
use baylee_client_core::interaction::CombatFocus;
use baylee_core::ids::{Defender, ObjectId, PlayerId};
use baylee_view::{GameStatic, PlayerView};
use bevy::prelude::*;

/// The four UI fonts: Inter upright and italic for text, Font Awesome Solid
/// for icons, and the `mana` font for mana symbols.
/// Bundled OFL/CC-BY fonts (see NOTICE) — the default font has none of the
/// weight range, the icon glyphs or the mana symbols.
#[derive(Resource, Clone)]
pub struct UiFonts {
    /// Text font (Inter, variable weight).
    pub text: Handle<Font>,
    /// The same family, slanted.
    ///
    /// A second file rather than a switch, because there is nowhere to put
    /// the switch: `Inter.ttf` is a variable font whose axes are `opsz` and
    /// `wght` and nothing else, and [`TextFont`] carries a face and a size —
    /// no style, no synthetic oblique. A slant this client cannot ask for is
    /// a slant it has to ship.
    pub italic: Handle<Font>,
    /// Icon font (Font Awesome 6 Free, solid).
    pub icons: Handle<Font>,
    /// Mana symbols (the `mana` font, SIL OFL). `docs/legal.md` §2 names it
    /// as the one symbol font this project may bundle.
    pub mana: Handle<Font>,
}

/// Loads the bundled fonts at startup.
pub fn setup_fonts(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(UiFonts {
        text: assets.load("fonts/Inter.ttf"),
        italic: assets.load("fonts/Inter-Italic.ttf"),
        icons: assets.load("fonts/fa-solid-900.ttf"),
        mana: assets.load("fonts/mana.ttf"),
    });
}

/// A text-font handle at a size.
pub(crate) fn tf(fonts: &UiFonts, size: f32) -> TextFont {
    TextFont {
        font: bevy::text::FontSource::Handle(fonts.text.clone()),
        font_size: bevy::text::FontSize::Px(size),
        ..default()
    }
}

/// The same at a slant — the prompt slip's own voice.
pub(crate) fn tf_italic(fonts: &UiFonts, size: f32) -> TextFont {
    TextFont {
        font: bevy::text::FontSource::Handle(fonts.italic.clone()),
        font_size: bevy::text::FontSize::Px(size),
        ..default()
    }
}

/// An icon-font handle at a size.
fn icon_tf(fonts: &UiFonts, size: f32) -> TextFont {
    TextFont {
        font: bevy::text::FontSource::Handle(fonts.icons.clone()),
        font_size: bevy::text::FontSize::Px(size),
        ..default()
    }
}

// Font Awesome glyph codepoints used across the overlay (fa-solid-900).
mod glyph {
    /// Heart (life total).
    pub const HEART: char = '\u{f004}';
    /// Hand (cards in hand).
    pub const HAND: char = '\u{f256}';
    /// Layer group (library).
    pub const LIBRARY: char = '\u{f5fd}';
    /// Skull (graveyard).
    pub const SKULL: char = '\u{f54c}';
    /// Ban (exile).
    pub const EXILE: char = '\u{f05e}';
    /// Skull and crossbones (poison counters).
    ///
    /// Drawn by nothing between the commit that took the seat tab off the top
    /// of the window and the one that gives the seat sheet its body: poison
    /// and energy are two of the things the sheet says and the bar has no
    /// room for.
    #[expect(dead_code, reason = "the seat sheet says it next")]
    pub const POISON: char = '\u{f714}';
    /// Bolt (energy counters). See [`POISON`].
    #[expect(dead_code, reason = "the seat sheet says it next")]
    pub const ENERGY: char = '\u{f0e7}';
    /// Caret down (speech-bubble tail).
    pub const CARET_DOWN: char = '\u{f0d7}';
    /// Expand (resize handle).
    pub const EXPAND: char = '\u{f065}';
    /// Crown (the command zone). See [`POISON`].
    #[expect(dead_code, reason = "the seat sheet says it next")]
    pub const COMMAND: char = '\u{f521}';
    /// Times (close a panel). The text font has no U+2715, so the cross has
    /// to come from here or it draws as a missing glyph.
    pub const CLOSE: char = '\u{f00d}';
}

/// Root of the overlay.
#[derive(Component)]
pub struct HudRoot;

/// A hand card on the overlay: the 2D counterpart of [`crate::table::CardVisual`],
/// so clicks and hover treat hand and battlefield the same way.
#[derive(Component)]
pub struct HandCardVisual {
    /// The object the card represents and that input reports.
    pub object: ObjectId,
}

/// A seat's bar: click inspects that seat's board.
///
/// It was a tab in a strip along the top of the window and is the bar on that
/// seat's own mat now. The component outlived the tab because what it says is
/// "this is seat N, and a click here is about seat N" — which is true of a
/// bar written on a seat's ground more plainly than it ever was of a tab in a
/// row of tabs.
#[derive(Component)]
pub struct PlayerTab {
    /// The seat this bar represents.
    pub player: PlayerId,
}

/// The button for the step the game is in, and how far it has lit up.
///
/// Zero at spawn and eased to one by [`rail::light_the_current_step`]. The
/// HUD tree is rebuilt whenever the step changes, so the button carrying this
/// is always a *new* entity — which is what makes a value that only ever
/// climbs from zero the whole transition.
///
/// Nothing spawns one at the moment: the rail that did is gone, and the seat
/// bar draws its now-light as two rings rather than as a border and a shadow.
/// The system and this marker are kept for the commit that gives the bar its
/// baton — see the note at the top of [`rail`].
#[derive(Component, Default)]
pub struct PhaseNow {
    /// How far the light has come, 0 to 1.
    pub lit: f32,
}

/// The block that says whether it is day or night, and which designation it
/// was built for.
///
/// Nothing spawns one at the moment, for [`PhaseNow`]'s reason: it stood in
/// the rail's head, and the seat bar carries the designation on its hinge
/// instead.
///
/// The value is carried on the component rather than read back out of the
/// view, because the flash that marks a change has to be anchored to the
/// *change* and the HUD tree is rebuilt on every hover: a light that eased
/// from zero at spawn, the way [`PhaseNow`] does, would fire again every
/// time the pointer crossed a card. [`rail::flash_the_designation`] keeps
/// the last value it saw beside the clock and compares.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub struct Designation(pub baylee_view::DayNight);

/// One of the pills in the window's top-right corner.
#[derive(Component)]
pub struct MenuButton {
    /// What the button does.
    pub action: MenuAction,
}

/// What a menu button does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuAction {
    /// Leave the game (sends the engine's own concession).
    Concede,
    /// Offer a draw: every other player still in the game has to accept
    /// (CR 104.4a).
    OfferDraw,
    /// Cancel a running priority hold, so the seat is asked again.
    ///
    /// Only ever drawn while one is running, which is why there is no
    /// matching "set a hold" button: choosing between "until the stack is
    /// empty" and "the rest of this turn" is a two-key decision, and the way
    /// out of either is one.
    ReleaseHold,
    /// Send the armed deed (`crate::Armed`).
    SendArmed,
    /// Put it back with nothing on the wire.
    ///
    /// Both of these are `MenuButton`s rather than a component of their own
    /// because `pointer` is already at Bevy's system-parameter limit, and a
    /// button is a button: what distinguishes them is what they do, which is
    /// exactly what this enum is for.
    CancelArmed,
}

/// The card preview's resize handle (bottom-right corner).
#[derive(Component)]
pub struct PreviewResize;

/// An answer button under the prompt headline.
#[derive(Component)]
pub struct PromptButton {
    /// Which answer the button sends.
    pub action: PromptAction,
}

/// One offered ability, in the chooser under the prompt.
///
/// Carries the position rather than the action itself, because the list is
/// rebuilt from `LegalActions` when the button is pressed: a bar that was
/// drawn a frame ago must not be able to send an ability the engine has since
/// stopped offering.
#[derive(Component)]
pub struct AbilityButton {
    /// Position in the list [`crate::abilities::options`] returns.
    pub index: usize,
}

/// One row of an indexed choice, in the chooser under the prompt.
///
/// Position, not the answer itself, for the same reason [`AbilityButton`]
/// carries one: the list is rebuilt from the *current* prompt when the button
/// is pressed, so a bar drawn a frame ago cannot answer a question the engine
/// has since replaced.
#[derive(Component)]
pub struct ChoiceButton {
    /// Position in the list [`crate::choices::options`] returns.
    pub index: usize,
}

/// What a prompt button answers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PromptAction {
    /// Yes.
    Yes,
    /// No.
    No,
    /// Keep the hand.
    Keep,
    /// Take the mulligan.
    Mulligan,
    /// Confirm / pass / OK.
    Confirm,
    /// Declare no attackers, or no blockers.
    DeclareNothing,
    /// Aim the next declaration at the next defender (or attacker).
    AimNext,
    /// Hand the rest of this turn to the autopilot.
    ///
    /// The other half of the arrow buttons the rail lost. It belongs here
    /// rather than on a strip of its own for the reason the slip exists at
    /// all: "pass this window" and "pass every window until my next turn" are
    /// the same decision at two sizes, and a player who has just been offered
    /// the first should not have to look somewhere else for the second.
    SkipTurn,
    /// One arm of the number stepper: `+1` or `-1`.
    ///
    /// A prompt button rather than a component of its own, because that is
    /// what it is — the one choice with nothing on the table to click, and
    /// the arms belong in the same bar as every other answer. It also keeps
    /// `input::pointer` off Bevy's system-parameter limit.
    Step(i32),
}

/// One card in the zone browser.
///
/// Carries the object, not an index: a tray row is a card, and a click on a
/// card means exactly what a click on the same card on the table means —
/// which is the whole reason the browser exists rather than a second way of
/// answering choices.
#[derive(Component)]
pub struct TrayCard {
    /// The object this row stands for.
    pub object: ObjectId,
}

/// A zone tab in the browser. `None` is the "all zones" tab.
#[derive(Component)]
pub struct TrayTab {
    /// The zone this tab shows.
    pub zone: Option<baylee_client_core::browser::BrowseZone>,
}

/// The browser's close button.
#[derive(Component)]
pub struct TrayClose;

/// The browser's sheet itself — the node a drag or a resize writes to.
///
/// Marked so that `crate::input::tray_drag` can find one node per frame and
/// move it *without* going through [`HudRevision`]. The overlay is a retained
/// tree rebuilt from scratch whenever the revision changes, and rebuilding
/// two hundred nodes for every pixel of a drag would make the sheet
/// unusable — the same reason the deckbuilder's hover preview is not part of
/// its list.
#[derive(Component)]
pub struct TrayPanel;

/// The sheet's title row: what a drag takes hold of.
///
/// The header and not a separate grip glyph, which is the convention every
/// window in every desktop follows and therefore needs no explaining. The
/// title text keeps `Pickable::IGNORE`, so the whole row minus the close
/// button is the handle.
#[derive(Component)]
pub struct TrayGrip;

/// The sheet's resize corner.
#[derive(Component)]
pub struct TrayResize;

/// Which of the two a pointer is holding.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TrayDragKind {
    /// The header: the sheet moves.
    Move,
    /// The corner: the sheet grows and shrinks.
    Resize,
}

/// The browser's filter box.
///
/// A `Button` because it is a field a player *gives* the keyboard to: a box
/// that swallowed every keystroke while the panel merely stood open would be
/// the end of playing with the graveyard visible.
#[derive(Component)]
pub struct TrayFilter;

/// The browser's sort control.
///
/// One button rather than a menu, because four keys and a direction is not a
/// menu's worth: a click steps to the next key, and the arrow beside it turns
/// the current one round.
#[derive(Component)]
pub struct TraySort {
    /// `true` for the arrow that reverses, `false` for the key itself.
    pub reverse: bool,
}

/// The scrolling strip inside the hand bar.
#[derive(Component)]
pub struct HandStrip;

/// Hand card geometry: the size every hand card renders at.
pub const HAND_CARD_W: f32 = 110.0;
/// Height, keeping the 63:88 card aspect.
pub const HAND_CARD_H: f32 = HAND_CARD_W * 88.0 / 63.0;
/// The fraction of a card that must stay visible when cards overlap.
const MIN_VISIBLE: f32 = 0.3;

/// How the hand bar lays out `count` cards of `card_w` width in
/// `available_w` pixels: the distance between card starts, the total
/// content width, and whether scrolling is required.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HandLayout {
    /// Distance between the left edges of neighboring cards.
    pub step: f32,
    /// Total width of the laid-out cards.
    pub content_width: f32,
    /// How far in from the left edge the first card starts.
    ///
    /// Half of whatever the cards did not use, so a hand stands in the middle
    /// of the bar rather than against its left edge. Zero as soon as the hand
    /// fills the bar, which is also when it starts scrolling: a scroll offset
    /// measured from a moving origin would drift.
    pub lead: f32,
    /// Whether the content overflows and must scroll.
    pub scrollable: bool,
}

/// The hand layout rule: fully visible and evenly spread while that fits;
/// overlapping with at least [`MIN_VISIBLE`] of every card showing when it
/// does not; scrollable when even the minimum overlap overflows.
#[must_use]
pub fn hand_layout(count: usize, card_w: f32, available_w: f32) -> HandLayout {
    if count == 0 {
        return HandLayout {
            step: card_w,
            content_width: 0.0,
            lead: 0.0,
            scrollable: false,
        };
    }
    let natural = count as f32 * card_w;
    if natural <= available_w {
        // Even spread: cards fully visible, spare space becomes gaps — up to
        // a point. The step is capped at a card and a little air, because a
        // two-card hand spread across a whole monitor is two cards a player
        // has to look for. What the cap leaves over is what `lead` centres.
        let step = if count > 1 {
            ((available_w - card_w) / (count - 1) as f32).min(card_w + 8.0)
        } else {
            card_w
        };
        let content_width = (count - 1) as f32 * step + card_w;
        return HandLayout {
            step,
            content_width,
            lead: (available_w - content_width).max(0.0) * 0.5,
            scrollable: false,
        };
    }
    let step = ((available_w - card_w) / (count - 1) as f32).max(card_w * MIN_VISIBLE);
    let content_width = (count - 1) as f32 * step + card_w;
    HandLayout {
        step,
        content_width,
        // A hand this wide has no spare room to share out, and a scroll
        // offset measured from a moving origin would drift as cards are
        // played.
        lead: (available_w - content_width).max(0.0) * 0.5,
        scrollable: content_width > available_w,
    }
}

/// Which snapshot the overlay currently shows.
#[derive(Resource, Default)]
pub struct HudRevision {
    seq: Option<u64>,
    prompt: Option<String>,
    /// Cursor position and choice selection — they change without a new
    /// snapshot (hover is per-frame, selection never leaves the client).
    hovered: Option<ObjectId>,
    selected: Vec<ObjectId>,
    /// Standing orders, autopilot, and inspected seat.
    orders: Option<baylee_client_core::automation::PhaseOrders>,
    autopilot: Option<AutoPilot>,
    focus: Option<PlayerId>,
    /// Preview size (resized via handle or shortcut).
    preview_scale: f32,
    /// Whether cards are drawing their constructed face. Held on a key, so
    /// it changes between snapshots and has to be part of the redraw gate.
    faces: bool,
    /// How many printings have text. Text arrives over the network mid-game,
    /// and a face built before it lands says a good deal less.
    texts: usize,
    /// The last refusal, so that one appearing rebuilds the bar.
    ///
    /// Separate from `prompt` because it no longer replaces it: a refusal
    /// happens while a question is standing, and the two lines are drawn
    /// together.
    error: Option<String>,
    /// What the connection is doing, so losing or regaining it redraws.
    ///
    /// The one thing in this struct that changes without the game changing at
    /// all: a socket goes away between snapshots, and the bar has to say so
    /// without waiting for a view that is not coming.
    link_note: Option<baylee_client_core::i18n::Phrase>,
    /// How much art has arrived, failed or been evicted.
    ///
    /// Same reason as `texts`, one asset later: a hand card is built with
    /// either its art or its constructed face, and which of those it should
    /// be changes when a load finishes. The table re-decides every frame and
    /// needs nothing here; a retained tree does.
    arrivals: u64,
    /// Where the combat focus points and how many declarations stand.
    ///
    /// Both change without a new snapshot — aiming and declaring never leave
    /// the client until the answer is sent — so without them here the combat
    /// line would be drawn once and then stay wrong for the whole step.
    combat: Option<(usize, usize, usize)>,
    /// Which permanent's abilities the chooser is offering. Opened by a click
    /// and closed by the next one, neither of which is a new snapshot, so
    /// without it here the chooser would never appear.
    ability_menu: Option<ObjectId>,
    /// Which entry of that chooser the keyboard is on — same reason, and
    /// without it the highlight would never move.
    ability_pick: usize,
    /// What the zone browser is showing. Opened by a choice arriving and
    /// by a tap on the top card of a pile, neither of which need be a new
    /// snapshot,
    /// and the tab and filter move with no snapshot at all.
    browser: (
        bool,
        Option<baylee_client_core::browser::BrowseZone>,
        String,
        bool,
    ),
    /// The value a number choice stands at. It changes with no new snapshot —
    /// stepping X never leaves the client until Confirm — so without it the
    /// stepper would draw the opening value and then stay wrong.
    number: Option<u32>,
    /// Which entry of the *answer* chooser is picked — a colour, a seat, a
    /// cast option, a creature type.
    ///
    /// The same reason as `number`, and it was missing for the same reason it
    /// is easy to miss: picking one never leaves the client until Confirm, so
    /// nothing else in this struct moved and the brass highlight stayed on
    /// whichever entry happened to be picked when the tree was last built.
    /// It is also what makes a `Feel` safe on those buttons — `feel` owns
    /// their `BackgroundColor` from the first frame, so a fill that depends
    /// on state is only honest while the state is in this gate.
    choice: Option<usize>,
    /// The two menu buttons' states: whether a draw may be offered at all,
    /// and whether concede is waiting for its second press. The first follows
    /// the pending choice, the second nothing but the pointer, and a button
    /// whose label changes has to be redrawn when it does.
    menu: (bool, bool),
    /// What is armed and waiting for its second tap. Unlike a priority hold,
    /// which always arrives with a new `seq`, this never leaves the client at
    /// all — so without it here the armed row would never be drawn.
    armed: Option<crate::Armed>,
    /// The window's logical size, rounded to whole pixels.
    ///
    /// Nothing in this struct followed the *window* before, so a HUD built
    /// for one size stood there unchanged after a resize — latent everywhere
    /// the overlay reads `windows` (the hand bar does, at
    /// `overlay.rs`'s `spawn_hand_bar`), and no longer latent at all once the
    /// zone browser is placed from a band whose height is the window's. A
    /// rebuild on resize is cheaper than a system clamping the sheet every
    /// frame, and it fixes the hand bar on the way past.
    window: (i32, i32),
}

/// Palette, kept in one place so the overlay reads as one design.
pub(crate) mod palette {
    use bevy::prelude::Color;

    /// Panel background.
    pub const PANEL: Color = Color::srgba(0.05, 0.06, 0.08, 0.88);
    /// Slightly lighter panel (active tab, tooltip).
    pub const PANEL_LIT: Color = Color::srgba(0.10, 0.13, 0.16, 0.94);
    /// Primary text.
    pub const INK: Color = Color::srgb(0.90, 0.93, 0.94);
    /// Secondary text.
    pub const MUTED: Color = Color::srgb(0.58, 0.64, 0.68);
    /// A seat that has lost.
    pub const DEAD: Color = Color::srgb(0.30, 0.32, 0.34);
    /// The accent used for anything asking for a decision.
    pub const ACCENT: Color = Color::srgb(0.33, 0.75, 0.71);
    /// Danger: lethal damage, a seat about to lose.
    pub const DANGER: Color = Color::srgb(0.91, 0.47, 0.42);
    /// The active seat's marker.
    pub const ACTIVE: Color = Color::srgb(0.84, 0.64, 0.31);
    /// A card the client is offering to tap lands for: an offer, not a
    /// legal action, and drawn as the weaker claim it is.
    pub const REACHABLE: Color = Color::srgb(0.50, 0.47, 0.84);
    /// Soft shadow under raised elements.
    pub const SHADOW: Color = Color::srgba(0.0, 0.0, 0.0, 0.55);

    /// Parchment, matching the middle of
    /// [`tabletop::parchment`](baylee_client_core::tabletop::parchment).
    ///
    /// Felt is the ground, parchment is a sheet, brass draws the lines and
    /// gold belongs to the local seat. A panel that is *read* is a sheet; a
    /// panel that is *worked in* stays [`PANEL`]. That is the whole rule, and
    /// it is why the prompt is parchment and the seat tabs are not.
    pub const PARCHMENT: Color = Color::srgb(0.929, 0.890, 0.800);
    /// Where a sheet lying on a table loses the light — its border, and the
    /// rim of the generated sheet it has to meet.
    pub const PARCHMENT_EDGE: Color = Color::srgb(0.706, 0.639, 0.502);
    /// Ink.
    pub const PARCHMENT_INK: Color = Color::srgb(0.098, 0.082, 0.062);
    /// The quieter ink: hints, tallies, what a key does.
    pub const PARCHMENT_SOFT: Color = Color::srgb(0.290, 0.251, 0.204);
    /// Brass: the answer a sheet is asking for.
    pub const BRASS: Color = Color::srgb(0.788, 0.635, 0.153);
    /// Danger on parchment. [`DANGER`] is tuned to carry on 88% black and is
    /// unreadable on a sheet; this is the same claim at the same weight.
    pub const INK_DANGER: Color = Color::srgb(0.620, 0.200, 0.129);
    /// The shadow a sheet lying above the table casts.
    pub const SHEET_SHADOW: Color = Color::srgba(0.0, 0.0, 0.0, 0.66);

    /// The prompt slip's own ink, and the aside's.
    ///
    /// [`PARCHMENT_INK`] and [`PARCHMENT_SOFT`] with a little of the sheet
    /// showing through. The alpha is the whole point: the slip is written
    /// *on* the parchment rather than printed over it, so the grain the sheet
    /// carries comes up through the letters instead of stopping at them. It
    /// is small — at 0.94 a stroke is still a stroke — because ink you can
    /// see the table through is a watermark, not a question.
    pub const SLIP_INK: Color = Color::srgba(0.098, 0.082, 0.062, 0.94);
    /// The quieter of the two, for a hint or a tally.
    pub const SLIP_SOFT: Color = Color::srgba(0.290, 0.251, 0.204, 0.92);
    /// An aside — whatever the sheet said in brackets.
    ///
    /// Grey rather than a lighter ink: an aside is a different *kind* of
    /// sentence (a key to press, a count already implied by the board), and
    /// draining the warmth out of it says so where another shade of brown
    /// would only say "further away".
    pub const SLIP_ASIDE: Color = Color::srgba(0.404, 0.376, 0.337, 0.86);
    /// What a letter on the slip casts.
    ///
    /// Warm and barely there: a hard black shadow under 13 px text reads as
    /// a rendering fault, and the job here is only to lift the line off a
    /// sheet that is the same family of colour as the ink is.
    pub const SLIP_SHADOW: Color = Color::srgba(0.161, 0.129, 0.086, 0.32);
    /// The fill under an answer that is not the one the sheet is asking for.
    ///
    /// Not [`Color::NONE`]: a button with no fill and a drop shadow renders
    /// as a dark rounded hole in the parchment, because the shadow is drawn
    /// under a surface that is not there. A few percent of the sheet's own
    /// colour is enough to be a surface and not enough to compete with
    /// [`BRASS`].
    pub const SLIP_GHOST: Color = Color::srgb(0.878, 0.835, 0.741);
}

/// The generated surfaces the overlay is drawn on.
#[derive(Resource, Clone)]
pub struct UiSheets {
    /// A sheet of parchment, stretched over whatever it is drawn on.
    pub parchment: Handle<Image>,
}

/// Generates the overlay's own surfaces at startup.
///
/// Arithmetic rather than a picture, for the reason `docs/legal.md` §2 gives
/// about the felt: ornament is the easiest thing to borrow by accident.
pub fn setup_sheets(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.insert_resource(UiSheets {
        parchment: images.add(crate::table::image_of(
            &baylee_client_core::tabletop::parchment(512),
        )),
    });
}

/// A sheet of parchment as a node's own surface.
///
/// Stretched, so a slip and a dialog are each *one* sheet rather than a wall
/// of tiles — see [`tabletop::parchment`](baylee_client_core::tabletop::parchment).
pub(crate) fn sheet(sheets: &UiSheets) -> ImageNode {
    ImageNode {
        // `Auto` is the default and it sizes the *node* to the image, which
        // on a 512-pixel sheet drops a bright square into the middle of every
        // panel it is put on. Stretch is what makes it a surface rather than
        // a picture.
        image_mode: NodeImageMode::Stretch,
        ..ImageNode::new(sheets.parchment.clone())
    }
}

/// What a sheet lying above the table casts: further out and much softer than
/// [`soft_shadow`], because it is a physical thing over the board rather than
/// a panel in the same plane as one.
pub(crate) fn sheet_shadow() -> BoxShadow {
    BoxShadow::new(
        palette::SHEET_SHADOW,
        Val::Px(0.0),
        Val::Px(10.0),
        Val::Px(2.0),
        Val::Px(26.0),
    )
}

/// The corner a sheet is cut with. Rounder than a button, because it is a
/// larger object and a sheet with a button's radius reads as a big button.
pub(crate) fn sheet_radius() -> BorderRadius {
    BorderRadius::all(px(14))
}

/// The parchment as a *child* of the node it covers, rather than as that
/// node's own image.
///
/// This is a bug fix with a shape worth keeping. [`sheet`] inserted on the
/// panel itself paints the **content box** — so a sheet with `padding` on it
/// drew the parchment in the middle and flat [`palette::PARCHMENT`] in a ring
/// around it, sixteen pixels wide on the browser and twenty-two on the prompt
/// slip, with the sheet's own rounded corners cut *inside* the panel's. That
/// is the "the background still looks strange" the owner reported: two
/// concentric rounded rectangles in two different colours where there should
/// have been one sheet.
///
/// An absolutely-positioned child is measured against its parent's padding
/// box, which is exactly the area that was missing, and it is out of flow so
/// it costs the column no gap and the layout no row. The radius is the
/// parent's less the one-pixel border it sits inside, so the two curves are
/// concentric rather than nested.
///
/// [`Pickable::IGNORE`] because it is a surface, not a control: without it
/// the sheet is the topmost hit under every pointer on the panel.
pub(crate) fn sheet_surface(sheets: &UiSheets) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            right: px(0),
            top: px(0),
            bottom: px(0),
            border_radius: BorderRadius::all(px(13)),
            ..default()
        },
        sheet(sheets),
        Pickable::IGNORE,
    )
}

/// The soft radius + shadow every button and panel shares.
pub(crate) fn soft_shadow() -> BoxShadow {
    BoxShadow::new(
        palette::SHADOW,
        Val::Px(0.0),
        Val::Px(2.0),
        Val::Px(0.0),
        Val::Px(6.0),
    )
}

/// An upward shadow for a panel standing over the table.
fn upward_shadow() -> BoxShadow {
    BoxShadow::new(
        palette::SHADOW,
        Val::Px(0.0),
        Val::Px(-4.0),
        Val::Px(0.0),
        Val::Px(14.0),
    )
}

/// The soft corner radius for buttons and tabs.
pub(crate) fn btn_radius() -> BorderRadius {
    BorderRadius::all(px(6))
}

/// How far anything fixed to the edge of the window stands off it.
///
/// One number, because the seat-tab strip, the phase rail under it, the mana
/// chip and the zone browser's own margin were a single column of things down
/// the left of the screen standing at 8, 10, 12 and 12. Nothing about the
/// difference meant anything — it was three people picking a number — and an
/// eye reading down that edge sees the disagreement long before it can name
/// it. Two of those four are gone now (the seat bars are on the table), and
/// the number outlived them: it is what the menu pills, the stack, the tray
/// and the browser all stand off by.
///
/// The hand bar keeps its own ten: its edge is never seen (it is full-width
/// and its cards are centred), and the number is load-bearing arithmetic in
/// [`hand`]'s spread rather than an inset.
pub(crate) const EDGE: f32 = 12.0;

/// How tall the two controls in the top-right corner are drawn.
///
/// They are the whole of what is left of a strip that was fifty-six pixels
/// tall with a fifty-four pixel rail under it, and they no longer sit on a
/// band at all: a pill over the felt is as tall as the finger that presses it
/// and no taller. Thirty-two is small for a *touch* target — the lobby's
/// phone frame asks forty-four — and these two are a desktop control apiece,
/// with a second press behind the dangerous one.
pub(crate) const MENU_H: f32 = 32.0;

/// Where the top of the free window begins for anything pinned to the
/// **right**, which is the one column the menu pills stand in.
///
/// The rest of the window's top edge is `EDGE` and nothing more: the strip
/// that used to run across it is on the table now. This is the exception, and
/// it is stated rather than measured because a stack panel that discovered
/// the pills by overlapping them would do so only in the games that have a
/// stack at all.
pub(crate) const MENU_BAND: f32 = EDGE + MENU_H + EDGE;

/// How far the two things that float above the hand bar — the prompt slip
/// and the mana chip — stand off it.
///
/// One constant for the same reason [`EDGE`] is: they sit side by side at
/// the same height and were pinned at `HAND_BAR_H + 12` and `+ 10`.
pub(crate) const ABOVE_HAND: f32 = 12.0;

/// A card's corner radius for a given rendered width.
///
/// The number is the printed one — 3 mm on a 63 mm card, 4.76% — and it has
/// to be, because the card material cuts its own corners at exactly that
/// radius (`PRINTED_CORNER` in `shaders/card_ui.wgsl`). This node clips its
/// child and carries the drop shadow, so a rounder radius here would slice
/// into the printed border and leave the shadow hanging off the corners. It
/// used to be 10%, from before the shader cut anything at all.
fn card_radius(width: f32) -> BorderRadius {
    BorderRadius::all(px(width * 0.0476))
}

/// The preview's corner radius — a touch rounder than a physical card
/// (~8%), subtler than the cards on the table.
fn preview_radius(width: f32) -> BorderRadius {
    BorderRadius::all(px(width * 0.08))
}

/// One color per team, so allied seats read as one side at a glance.
/// `None` is the neutral default.
#[must_use]
pub fn team_color(team: Option<u8>) -> Color {
    match team {
        None => palette::MUTED,
        Some(0) => Color::srgb(0.45, 0.62, 0.90),
        Some(1) => Color::srgb(0.70, 0.50, 0.88),
        Some(2) => Color::srgb(0.42, 0.80, 0.55),
        Some(3) => Color::srgb(0.88, 0.55, 0.55),
        _ => Color::srgb(0.80, 0.80, 0.60),
    }
}

/// The abilities the chooser should draw for `object`.
fn ability_options(
    duel: &Duel,
    lang: Lang,
    object: ObjectId,
) -> Option<Vec<crate::abilities::AbilityOption>> {
    let view = duel.view.as_ref()?;
    let interaction = duel.interaction.as_ref()?;
    let options = crate::abilities::options(lang, view, interaction, object);
    (!options.is_empty()).then_some(options)
}

mod card;
mod hand;
mod overlay;
pub(crate) mod rail;
pub(crate) mod seatbar;
mod stack;
mod tray;

#[cfg(test)]
mod tests;

use card::{FaceCtx, spawn_card_art};
use hand::{PreviewAt, preview_anchor, preview_face, preview_place, spawn_hand_bar};
use rail::{combat_line, incoming_line};
use stack::spawn_stack_panel;

pub use hand::apply_hand_scroll;
pub use hand::{HAND_BAR_H, OVERLAY_CARD_H, OVERLAY_CARD_W};
pub use overlay::{despawn_overlay, sync_overlay};
pub use rail::same_team;
pub use rail::{DesignationFlash, flash_the_designation, light_the_current_step};
pub use seatbar::{
    BarRevision, SeatBar, SeatBarRoot, SeatInk, SeatStep, Shelf, Shelves, measure_shelves,
    place_seat_bars, sync_seat_bars,
};
pub use stack::{StackMotion, ease_the_stack_in};
pub(crate) use tray::band_of;

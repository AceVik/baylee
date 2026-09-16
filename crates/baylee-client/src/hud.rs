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
//! - **Right** — the stack, drawn as cards, in a corner that is its own: the
//!   two controls that end a game were a row of pills above it and are on the
//!   shelf's right-hand column now (AX §4.3).
//! - **The middle** — the prompt slip, the zone browser, the hover preview.
//! - **Bottom** — the hand zone: card images, overlapping but never less
//!   than 30% visible, horizontally scrollable when even that overflows,
//!   with a large hover tooltip for reading a card.
//!
//! The overlay is retained-UI: it is rebuilt only when something it shows
//! actually changed (snapshot, prompt, hover, selection, orders). The seat
//! bars are a **second** retained tree with a revision of their own, because
//! [`HudRevision`] counts the hover and a bar that was rebuilt on every
//! pointer move would be rebuilt some hundreds of times a turn.

use baylee_client_core::i18n::{Lang, Phrase, seat_name};

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

/// The faces the interface is set in.
///
/// Two text families and two symbol faces, and the two families are two
/// *voices*. **Alegreya Sans** carries the interface — a humanist sans whose
/// strokes still remember a pen, which is the "leicht geschwungene" the
/// owner asked for, while its terminals are a text face's and not a
/// handwriting's. **Faustina** carries card names and rules text: a
/// newspaper serif, because a rules paragraph is *quoted* text — the same
/// reason the stack sentence draws marks and not discs — and the only serif
/// whose hairlines survived this client's 12 px 1× raster.
///
/// This replaces Inter in every role, so it **overrides `docs/design.md`
/// §1.2**, which says three cuts are shipped, there is no fourth, and Inter
/// carries all of them. Both families are OFL 1.1 with no Reserved Font
/// Name; see `NOTICE` and `assets/fonts/licenses/`.
#[derive(Resource, Clone)]
pub struct UiFonts {
    /// Interface text (Alegreya Sans Regular).
    pub text: Handle<Font>,
    /// The same family one weight up.
    ///
    /// A file and not a [`TextFont::weight`]: these are *static* cuts, and
    /// that field only reaches a variable font. Small text takes this one —
    /// [`tf`] says where the line is and why.
    pub medium: Handle<Font>,
    /// The family's Bold, and the only face a **control** is set in.
    ///
    /// A fifth file for exactly the reason there is a fourth: `TextFont::weight`
    /// does not reach a static cut, so a bold label is a bold *file* or it is
    /// nothing. What it buys is a distinction this interface could not draw —
    /// a word a player can press against a word a player can only read. See
    /// [`tf_bold`] for where that line falls.
    pub bold: Handle<Font>,
    /// Interface text, slanted.
    ///
    /// A second file rather than a switch, for the same reason it always
    /// was: [`TextFont`] carries a face and a size, and a slant this client
    /// cannot ask for is a slant it has to ship.
    pub italic: Handle<Font>,
    /// Slanted and one weight up, for small italics.
    pub medium_italic: Handle<Font>,
    /// Card names and rules text (Faustina, variable `wght` 300–800).
    ///
    /// Variable, so [`TextFont::weight`] does reach this one — which is what
    /// pays for the ink: a pen set down on parchment spreads, and weight 500
    /// is that spread.
    pub serif: Handle<Font>,
    /// The same, slanted — a reminder's aside.
    pub serif_italic: Handle<Font>,
    /// Icon font (Font Awesome 6 Free, solid).
    pub icons: Handle<Font>,
    /// Mana symbols (the `mana` font, SIL OFL). `docs/legal.md` §2 names it
    /// as the one symbol font this project may bundle.
    pub mana: Handle<Font>,
}

/// Loads the bundled fonts at startup.
pub fn setup_fonts(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(UiFonts {
        text: assets.load("fonts/AlegreyaSans-Regular.ttf"),
        medium: assets.load("fonts/AlegreyaSans-Medium.ttf"),
        bold: assets.load("fonts/AlegreyaSans-Bold.ttf"),
        italic: assets.load("fonts/AlegreyaSans-Italic.ttf"),
        medium_italic: assets.load("fonts/AlegreyaSans-MediumItalic.ttf"),
        serif: assets.load("fonts/Faustina.ttf"),
        serif_italic: assets.load("fonts/Faustina-Italic.ttf"),
        icons: assets.load("fonts/fa-solid-900.ttf"),
        mana: assets.load("fonts/mana.ttf"),
    });
}

/// What a nominal size is multiplied by before Alegreya Sans is asked for it.
///
/// Alegreya Sans is authored small: its x-height is 0.458 of the em against
/// Inter's 0.546, so at the same nominal size the whole interface reads about
/// two steps smaller. Every size in this client was chosen against Inter, so
/// the correction lives *here* and not in three hundred call sites.
///
/// It has a second, load-bearing effect. The width estimator in
/// [`stack::CHAR_WIDTH`] is 0.52 of the size per character, measured on
/// Inter; Alegreya Sans measures 0.445, and 0.445 × 1.2 is 0.534. The
/// apparent size and the character budget therefore both stay where they
/// were, which is why this is a scale and not a set of new constants.
pub(crate) const UI_SCALE: f32 = 1.2;

/// The same for Faustina, whose x-height is 0.494.
pub(crate) const SERIF_SCALE: f32 = 1.1;

/// Below this nominal size the Regular cut is asked to do too much.
///
/// Alegreya Sans has a light Regular, and under a 12 px raster its thin
/// strokes drop below a pixel and the stems go grey. Medium is the reading
/// weight down there; above it Medium reads as emphasis nobody asked for.
const SMALL_TEXT: f32 = 14.0;

/// Lining figures, always.
///
/// Alegreya Sans defaults to **old-style** figures, whose 3, 4, 7 and 9 hang
/// below the baseline. That is right in a paragraph and wrong in every place
/// this client puts a number: a life total, a mana value, a turn number, a
/// power and toughness. `lnum` is therefore not a flourish — it is what keeps
/// a 7 from looking like it fell out of the seat bar.
fn lining() -> bevy::text::FontFeatures {
    bevy::text::FontFeatures::builder()
        .enable(bevy::text::FontFeatureTag::LINING_FIGURES)
        .build()
}

/// A text-font handle at a size.
pub(crate) fn tf(fonts: &UiFonts, size: f32) -> TextFont {
    let face = if size < SMALL_TEXT {
        &fonts.medium
    } else {
        &fonts.text
    };
    TextFont {
        font: bevy::text::FontSource::Handle(face.clone()),
        font_size: bevy::text::FontSize::Px(size * UI_SCALE),
        font_features: lining(),
        ..default()
    }
}

/// A **control's own label**, at a size.
///
/// The one rule this face draws, and it is worth stating because it is not
/// "important text is bold": a label a player can **press** is set in Bold,
/// and a sentence that merely happens to lie inside a control is not. So the
/// word on a button, a chip, a tray tab, an answer on the prompt slip and the
/// digit in an ability row's keycap are all bold; the ability's printed
/// sentence beside that digit, a card's name in a browser row and the slip's
/// prose are not. A control that is *only* a glyph — the eye at the end of a
/// password box, the phase tiles' icons — is set in the icon face and reaches
/// none of this.
///
/// There is no small-size branch, and that is the difference from [`tf`].
/// Medium exists down there because the Regular's stems go grey under a 12 px
/// raster; Bold has no such trouble at any size this client sets, so the face
/// is the same one all the way down and a 8 px tile label is the same weight
/// as a 20 px button.
///
/// Bold is 3.5% wider than Regular on lower case (0.4610 of the em against
/// 0.4453, measured out of the shipped files), which is why no width estimate
/// moved with it: [`stack::CHAR_WIDTH`] budgets card *names*, and a card name
/// is never a control's label.
pub(crate) fn tf_bold(fonts: &UiFonts, size: f32) -> TextFont {
    TextFont {
        font: bevy::text::FontSource::Handle(fonts.bold.clone()),
        font_size: bevy::text::FontSize::Px(size * UI_SCALE),
        font_features: lining(),
        ..default()
    }
}

/// The same at a slant — the prompt slip's own voice.
pub(crate) fn tf_italic(fonts: &UiFonts, size: f32) -> TextFont {
    let face = if size < SMALL_TEXT {
        &fonts.medium_italic
    } else {
        &fonts.italic
    };
    TextFont {
        font: bevy::text::FontSource::Handle(face.clone()),
        font_size: bevy::text::FontSize::Px(size * UI_SCALE),
        font_features: lining(),
        ..default()
    }
}

/// Faustina at a size and a weight — a card's own words.
///
/// The weight is an argument because this is the one face here that is
/// variable, and because parchment wants a heavier stroke than a panel does:
/// [`INK_WEIGHT`] is what a sheet asks for and 400 is what everything else
/// does. Faustina prints lining figures by default, so no feature is set.
pub(crate) fn tf_serif(fonts: &UiFonts, size: f32, weight: u16) -> TextFont {
    TextFont {
        font: bevy::text::FontSource::Handle(fonts.serif.clone()),
        font_size: bevy::text::FontSize::Px(size * SERIF_SCALE),
        weight: bevy::text::FontWeight(weight),
        ..default()
    }
}

/// The same, slanted — a reminder in brackets.
pub(crate) fn tf_serif_italic(fonts: &UiFonts, size: f32, weight: u16) -> TextFont {
    TextFont {
        font: bevy::text::FontSource::Handle(fonts.serif_italic.clone()),
        font_size: bevy::text::FontSize::Px(size * SERIF_SCALE),
        weight: bevy::text::FontWeight(weight),
        ..default()
    }
}

/// The weight a pen has when it is set down on parchment.
///
/// Not decoration: this is half of what the owner asked for when they asked
/// for ink. A nib laid on a rough sheet spreads, and 500 on a variable face
/// is that spread — where the other half, [`bleed`], is the halo the fibres
/// wick it into.
pub(crate) const INK_WEIGHT: u16 = 500;

/// The shadow under text on parchment: a **bleeding front**, not a drop.
///
/// The client already had a `TextShadow` here and it said the wrong thing. A
/// shadow offset straight down doubles every stroke and reads as "the
/// letters are floating above the sheet"; ink sits *in* the sheet. So the
/// correction is the direction, not the amount — half a pixel down, no
/// sideways component until the size is large enough to carry one, and the
/// colour of an iron-gall bleed front (a warm amber-brown) rather than
/// black.
///
/// Under 12 px there is no halo at all: at that size a second coloured copy
/// of a stem is the whole stem, and the umlauts clot. The test is one
/// sentence — **if a player can see the shadow, it is too strong.**
pub(crate) fn bleed(size: f32) -> Option<TextShadow> {
    let (offset, alpha) = if size < 12.0 {
        return None;
    } else if size < 16.0 {
        (Vec2::new(0.0, 0.5), 0.26)
    } else if size < 25.0 {
        (Vec2::new(0.25, 0.75), 0.28)
    } else {
        (Vec2::new(0.5, 1.0), 0.30)
    };
    Some(TextShadow {
        offset,
        color: palette::BLEED.with_alpha(alpha),
    })
}

/// An icon-font handle at a size.
pub(crate) fn icon_tf(fonts: &UiFonts, size: f32) -> TextFont {
    TextFont {
        font: bevy::text::FontSource::Handle(fonts.icons.clone()),
        font_size: bevy::text::FontSize::Px(size),
        ..default()
    }
}

// Font Awesome glyph codepoints used across the overlay (fa-solid-900).
pub(crate) mod glyph {
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
    // Used by `hud::tests`, which is where the icon face is read, so the
    // expectation only holds in a build that is not the test one.
    #[cfg_attr(not(test), expect(dead_code, reason = "the seat sheet says it next"))]
    pub const POISON: char = '\u{f714}';
    /// Bolt (energy counters). See [`POISON`].
    // Used by `hud::tests`, which is where the icon face is read, so the
    // expectation only holds in a build that is not the test one.
    #[cfg_attr(not(test), expect(dead_code, reason = "the seat sheet says it next"))]
    pub const ENERGY: char = '\u{f0e7}';
    /// Caret down (speech-bubble tail).
    pub const CARET_DOWN: char = '\u{f0d7}';
    /// Expand (resize handle).
    pub const EXPAND: char = '\u{f065}';
    /// Crown (the command zone). See [`POISON`].
    // Used by `hud::tests`, which is where the icon face is read, so the
    // expectation only holds in a build that is not the test one.
    #[cfg_attr(not(test), expect(dead_code, reason = "the seat sheet says it next"))]
    pub const COMMAND: char = '\u{f521}';
    /// Times (close a panel). The text font has no U+2715, so the cross has
    /// to come from here or it draws as a missing glyph.
    pub const CLOSE: char = '\u{f00d}';
    /// Check (a ticked box in the zone browser). Read out of the shipped
    /// font's own cmap rather than looked up: a codepoint a search agrees
    /// about is not the same claim as a glyph this file has.
    pub const CHECK: char = '\u{f00c}';
    /// A list of rows, each with a block at its left: the detailed view.
    ///
    /// The three below are the zone browser's view buttons, and they are a
    /// *set* — read out of the shipped font's cmap together and rendered
    /// together, because what each one has to say it says by not looking like
    /// the other two. This one is the only one of the three with a column
    /// down its left side, which is the thumbnail.
    pub const VIEW_ROWS: char = '\u{f00b}';
    /// Three fat items with a mark beside each: the large view. Fewer things,
    /// bigger, and no left column — which is exactly what that view drops.
    pub const VIEW_BIG: char = '\u{f03a}';
    /// Four large cells: the grid. Two by two rather than the denser
    /// three-by-three beside it in the font, because the tiles are cards and
    /// a grid of nine would read as a spreadsheet.
    pub const VIEW_GRID: char = '\u{f009}';
    /// Eye: show a masked field.
    pub const EYE: char = '\u{f06e}';
    /// Eye with a line through it: cover it again.
    pub const EYE_SLASH: char = '\u{f070}';
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

/// A card that is actually standing in the player's own hand row.
///
/// [`HandCardVisual`] is deliberately wider than that — it means "a card the
/// HUD draws rather than the felt", and the stack panel puts it on every row
/// so a spell on the stack hovers, previews and answers a click like any
/// other card. That is
/// right for hover and for clicks and wrong for anything that *writes* to the
/// node: [`crate::touch::settle`] sets a card's `top`, and with only the
/// wider component to go on it set the stack slots' too, for as long as a
/// card cast out of the hand still had a touch left to spend. So the row says
/// that it is the row, and the two questions stop being one.
#[derive(Component)]
pub struct HandRowCard;

/// A whole row of the stack panel, standing for the object drawn on it.
///
/// The same shape as [`HandRowCard`] and for the same reason, one surface
/// along. The row carries [`HandCardVisual`] so that a click anywhere on it
/// — the picture, the name, the printed sentence — is a click on the spell
/// it draws, and this says *which* node is the row. `devctl`'s
/// `/state.cards` reads it to report the stack as a third zone beside the
/// table and the hand: a driver that cannot find a stack object on screen
/// cannot answer a `ChooseTargets` that names one.
#[derive(Component)]
pub struct StackRowCard;

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

/// A button that does something to the game rather than answering a question.
///
/// It began as the pair of pills in the window's top-right corner and every
/// one of them is on the shelf now: the armed row's two buttons, "resolve the
/// stack", the way out of a hold, and the pair itself. What the component
/// says is what it always said — this control's press goes to
/// `input::menu_click` — and where it is drawn is the caller's business.
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
    /// (CR 104.4i).
    OfferDraw,
    /// Cancel a running priority hold, so the seat is asked again.
    ///
    /// Only ever drawn while one is running.
    ReleaseHold,
    /// Hold priority until the stack is empty: `PriorityHold::UntilStackEmpty`,
    /// the same thing `Action::HoldForStack` sends and by the same road.
    ///
    /// The one of the two holds that has a button, and [`ReleaseHold`] used to
    /// say there was none. The other is F7's `UntilEndOfTurn`, which asks again
    /// at every trigger — so it answers no question and is a setting rather
    /// than a reply. This one, on a stack that is not empty, *is* an answer to
    /// the question standing above it ("do you want to respond to that?"): no,
    /// to none of it. Which is why it stands in the row of answers and why it
    /// is drawn only while there is a stack to resolve — on an empty one it
    /// would be a button promising to do nothing.
    ///
    /// [`ReleaseHold`]: MenuAction::ReleaseHold
    HoldForStack,
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
///
/// Defined in `client-core` and named here, because
/// [`baylee_client_core::ledge::shortcut_for`] is the bridge from an answer
/// to the key that sends it, and a keymap is `prefs`' business. The component
/// it rides on stays a renderer thing; the enum is a fact about the question.
pub use baylee_client_core::ledge::PromptAction;

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

/// The band the sheet is placed in: the dialog's outermost node, and the one
/// that is actually a child of [`HudRoot`].
///
/// A second marker rather than moving [`TrayPanel`] outwards, because the two
/// nodes answer two questions. `TrayPanel` is the rectangle a drag writes and
/// has to be the sheet itself; this is what [`OverlayTree`] passes over and
/// [`TrayTree`] tears down, and a sweep that named the inner one would despawn
/// the band and take the sheet with it — which is exactly what it did for one
/// test run.
#[derive(Component)]
pub struct TrayBand;

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

/// A pointer holding the sheet: what it took hold of, where it started and
/// where it was on the frame before.
///
/// `origin` is what separates a *drag* on the corner from a *click* on it,
/// and it has to be recorded rather than derived: a resize ends with the
/// pointer still over the corner, because the corner travelled with the hand,
/// so the release alone says nothing about whether anything moved.
#[derive(Clone, Copy, Debug)]
pub struct TrayDrag {
    /// The control under the press.
    pub kind: TrayDragKind,
    /// Where the pointer went down.
    pub origin: Vec2,
    /// Where it was on the frame before this one.
    pub last: Vec2,
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

/// One of the browser's three view buttons.
///
/// Three buttons and not one that cycles, which is where this differs from
/// the sort key beside it: a sort key is a ring of four *equivalent* answers
/// and the button can say which one is current, but a view is a shape the
/// player is looking at, and a control that has to be pressed twice to get
/// back to the list would be asking them to guess what the next press does.
/// Three marks, one lit — the current one is visible without pressing
/// anything.
#[derive(Component)]
pub struct TrayView {
    /// The mode this button selects.
    pub mode: baylee_client_core::browser::ViewMode,
}

/// The "none of them" button, drawn only when the question's minimum is zero.
///
/// Not a [`PromptAction::Confirm`] button with different words, though that
/// is what it *sends*: Confirm sends the answer that is assembled and this
/// one sends the **empty** answer, clearing the selection first so a player
/// who ticked a card and then changed their mind does not have that card sent
/// under it (`docs/redesign-proposal.md` §6).
///
/// It was `TrayCancel`, under a button reading "Cancel", and both were wrong
/// the same way: there is no cancel on the wire, so this button *answers the
/// question* and the game moves on. A player who read it as a way out had
/// already spent the choice. The word is [`Phrase::BrowseNone`] now and the
/// type is named for what it does, because the next reader of this file is
/// exactly the person the old name would have fooled.
#[derive(Component)]
pub struct TrayNone;

/// The veil over the table, behind a dialog that holds the whole answer.
///
/// It is a child of [`HudRoot`] and therefore part of the retained tree, so it
/// is torn down and rebuilt with everything else — which is why how far it has
/// risen is kept in [`Veil`] and not on this node. Marked here so
/// [`tray::dim_the_table`] can find it on the frames between two rebuilds.
#[derive(Component)]
pub struct TableVeil;

/// How far the veil has risen, `0.0` to `1.0` of [`palette::VEIL`]'s alpha.
///
/// A resource and not a field of the node, because the node does not survive a
/// rebuild and a question is answered *through* rebuilds: every tick of a
/// checkbox changes [`HudRevision`], and a fade that started again at each of
/// them would be a table that flickered while a player chose a card.
///
/// There is only ever one veil, so one number is the whole state.
#[derive(Resource, Default)]
pub struct Veil {
    /// The eased fraction.
    pub lit: f32,
}

/// The scrolling strip inside the hand zone.
#[derive(Component)]
pub struct HandStrip;

/// Hand card geometry: the size every hand card renders at.
///
/// It was 110, and the owner asked for a hand of slightly less height. A hand
/// card keeps 63:88, so height is not a number this file has — it is the
/// width, seen the other way round, and 110 → 92 is 25.1 pixels off the
/// bottom of the window. That is what pays for [`hand::LEDGE_H`]: the zone is
/// 1.9 px taller than the bar it replaces, which the camera cannot see.
///
/// Not 84. The card's printed name is rastered across this width (`face.rs`),
/// and under about 88 px it falls below the legibility floor the preview card
/// is held to — a hand of cards nobody can read is not a smaller hand.
pub const HAND_CARD_W: f32 = 92.0;
/// Height, keeping the 63:88 card aspect.
pub const HAND_CARD_H: f32 = HAND_CARD_W * 88.0 / 63.0;
/// The fraction of a card that must stay visible when cards overlap.
const MIN_VISIBLE: f32 = 0.3;

/// How much of the window the hand may not lay itself out in.
///
/// It is written as the zone's padding and read by [`hand_available`], and
/// those are two different facts about it — because the padding moves
/// **nothing**. The strip is an absolutely-positioned child, and taffy lays
/// one of those out against the padding *box*, which is the border box less
/// the border and has no padding taken off it. What actually holds the row
/// off the window's edges is [`HAND_STRIP_INSET`] on one side and the
/// layout's centring `lead` on the other, and they balance because `lead` is
/// computed from a width this has already been subtracted from.
///
/// So: a term in the row's **width**, never a term in a card's **position**.
/// It was both for as long as it existed, which put the hover preview ten
/// pixels to the right of its own card — see `hand::hand_card_x`.
pub const HAND_BAR_PAD: f32 = 10.0;

/// Where the strip's own left edge sits inside that padding box.
///
/// The margin the scroll system writes is this plus the layout's `lead`,
/// less the scroll offset. It is a named constant because it was written out
/// as a bare `10.0` in three places and read as the same ten pixels in only
/// two of them.
pub const HAND_STRIP_INSET: f32 = 10.0;

/// How wide the hand may lay itself out: the window less the bar's padding.
///
/// One function because two callers answered it differently, and the
/// disagreement was visible. The overlay took 110 pixels off the right-hand
/// end for a commander zone and the per-frame scroll system did not, so a
/// seat with a commander had its row spawned centred in one width and
/// re-centred in a wider one on the very next frame — half the zone,
/// sideways and back, on every rebuild. The hover is part of
/// [`HudRevision`], so a pointer crossing the hand rebuilt it continuously
/// and the row shook rather than jumped.
///
/// Neither of them should have been subtracting it. The commander zone was
/// drawn in this bar once and is not any more: it is a public zone (CR
/// 903.6) and it stands beside the mat on the table with the graveyard and
/// the exile pile. The reservation outlived the thing it was reserving for,
/// which is why the hand of a commander deck was laid out around a hole
/// nothing has occupied for some time.
#[must_use]
pub fn hand_available(window_w: f32) -> f32 {
    (window_w - 2.0 * HAND_BAR_PAD).max(0.0)
}

/// How the hand zone lays out `count` cards of `card_w` width in
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

/// Everything about the zone browser that the drawn panel depends on.
///
/// Its own struct and not a tuple in [`HudRevision`], because it was a tuple
/// and two of these six were missing from it. The panel is a **retained**
/// tree: nothing in the browser reaches the screen except by appearing here
/// and being found to have changed, so a field left out is a control that
/// silently does nothing. The sort key and its direction were left out, and
/// clicking either button changed the order of a list that was never redrawn.
///
/// Named fields are the whole point — an omission from a list of names is
/// visible where an omission from `(bool, Option<_>, String, bool)` is not.
///
/// It sat in [`HudRevision`] until the dialog got a retained tree of its own;
/// it is [`tray::TrayRevision`]'s now, and the browser reaches `sync_overlay`
/// nowhere else at all.
#[derive(Default, Clone, PartialEq, Eq)]
struct BrowserGate {
    /// Whether the panel stands at all. Opened by a choice arriving and by a
    /// tap on the top card of a pile, neither of which need be a new
    /// snapshot.
    open: bool,
    /// Which zone boxes are ticked, empty being "Alle".
    ///
    /// The whole set and not one zone, because ticking a second pile merges
    /// it into the list without changing anything else the gate can see: a
    /// comparison that kept only "which tab" would draw the merge once and
    /// then never redraw it.
    ticked: std::collections::BTreeSet<baylee_client_core::browser::BrowseZone>,
    /// What is typed in the filter — **and where the caret and the selection
    /// stand in it**, which is why this is the buffer rather than its string.
    ///
    /// It was a `String`, so a caret that moved changed nothing this gate
    /// could see, while `tray::filter_runs` draws the box out of
    /// [`TextBuffer::segments`] — head, selection, tail and the bar between
    /// them, every one of them read off the state the comparison had thrown
    /// away. Measured in the running client on 14.09.2026: `abcdef` typed,
    /// then five `ArrowLeft`s, three of them holding shift, and the box did
    /// not change by a single pixel — then a `Backspace` deleted the **b**,
    /// which is the model saying the caret had been standing at 2 the whole
    /// time. The same defect the sort buttons had, one field further in.
    ///
    /// [`TextBuffer::segments`]: baylee_client_core::textbuf::TextBuffer::segments
    filter: baylee_client_core::textbuf::TextBuffer,
    /// Whether the filter box is holding the keyboard, which draws its rim.
    typing: bool,
    /// Which key the rows are in, and
    sort: baylee_client_core::browser::SortKey,
    /// which way up.
    descending: bool,
    /// Which shape the rows are drawn in.
    ///
    /// Read out of `ClientSettings` rather than the `Browser`, which is the
    /// one field here that does not come off the model — and it is here for
    /// the reason the two above it are: the mode is a control on this panel,
    /// so a panel that could not see it changing is three buttons that do
    /// nothing.
    view: baylee_client_core::browser::ViewMode,
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
    /// What is armed and waiting for its second tap. Unlike a priority hold,
    /// which always arrives with a new `seq`, this never leaves the client at
    /// all — so without it here the armed row would never be drawn.
    armed: Option<crate::Armed>,
    /// The cast chooser this client opens before it floats anything: which
    /// card, how many ways, and which row the cursor is on.
    ///
    /// `armed`'s neighbour and for its reason: the chooser is entirely the
    /// client's, so opening it, walking it and closing it all happen with no
    /// new snapshot and nothing else in this struct moving. The three fields
    /// are what a redraw depends on; the ways themselves are rebuilt from
    /// `LegalActions` whenever the list is read, so a change in them arrives
    /// with the `seq` that caused it.
    cast_menu: Option<(ObjectId, usize, usize)>,
    /// The window's logical size, rounded to whole pixels.
    ///
    /// Nothing in this struct followed the *window* before, so a HUD built
    /// for one size stood there unchanged after a resize — latent everywhere
    /// the overlay reads `windows` (the hand zone does, at
    /// `overlay.rs`'s `spawn_hand_zone`), and no longer latent at all once the
    /// zone browser is placed from a band whose height is the window's. A
    /// rebuild on resize is cheaper than a system clamping the sheet every
    /// frame, and it fixes the hand zone on the way past.
    window: (i32, i32),
}

/// Palette, kept in one place so the overlay reads as one design.
pub(crate) mod palette {
    use bevy::prelude::Color;

    /// Panel background.
    pub const PANEL: Color = Color::srgba(0.05, 0.06, 0.08, 0.88);
    /// Slightly lighter panel (active tab, tooltip).
    pub const PANEL_LIT: Color = Color::srgba(0.10, 0.13, 0.16, 0.94);
    /// [`PANEL_LIT`] with the pointer on it.
    ///
    /// One step and no more: the stack's top row is already the loud thing in
    /// its panel, and a hover that also brightened its ink or its rail would
    /// be answering a second time — the card preview the same hover opens is
    /// the real answer. Lifted in all three channels rather than in alpha,
    /// because this row is the one that rests at almost full opacity and has
    /// nowhere left to go there.
    pub const PANEL_HOT: Color = Color::srgba(0.13, 0.17, 0.20, 0.94);
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
    /// Life gained — [`DANGER`]'s answer, and the only green on this
    /// overlay.
    ///
    /// Leaf rather than emerald, which is what keeps it away from [`ACCENT`]:
    /// the accent is a cyan-leaning teal and means *this is asking you
    /// something*, and a life gain is asking nothing. Two greens that had to
    /// be told apart would be a third claim the player never agreed to read.
    pub const HEAL: Color = Color::srgb(0.53, 0.80, 0.44);
    /// The active seat's marker.
    pub const ACTIVE: Color = Color::srgb(0.84, 0.64, 0.31);
    /// A card the client is offering to tap lands for: an offer, not a
    /// legal action, and drawn as the weaker claim it is.
    pub const REACHABLE: Color = Color::srgb(0.50, 0.47, 0.84);
    /// The fill behind selected text in a field.
    ///
    /// [`ACCENT`] at a quarter, because the same accent rings the field that
    /// has the caret: a selection and a focus are the same claim about the
    /// same box — *this is where the typing goes* — and saying it in two
    /// colours would make them two facts.
    pub const SELECTION: Color = Color::srgba(0.33, 0.75, 0.71, 0.25);
    /// Soft shadow under raised elements.
    pub const SHADOW: Color = Color::srgba(0.0, 0.0, 0.0, 0.55);

    /// Parchment, matching the middle of
    /// [`tabletop::parchment`](baylee_client_core::tabletop::parchment) —
    /// and, since this value, the card shader's own `PARCHMENT`.
    ///
    /// Felt is the ground, parchment is a sheet, brass draws the lines and
    /// gold belongs to the local seat. A panel that is *read* is a sheet; a
    /// panel that is *worked in* stays [`PANEL`]. That is the whole rule, and
    /// it is why the prompt is parchment and the seat tabs are not.
    ///
    /// It was #EDE3CC while the saga page the card shader draws was #E0D4B0,
    /// which is one material in two colours — and the wrong way round, the
    /// *smaller* surface being the darker one where a colour field the size
    /// of a fingernail already reads darker than a sheet of paper. The card's
    /// is what both took. `the_parchment_is_the_same_paper_in_both_languages`
    /// reads it back out of the WGSL, the way the rail and the plate are
    /// already held.
    pub const PARCHMENT: Color = Color::srgb(0.88, 0.83, 0.69);
    /// Where a sheet lying on a table loses the light — its border, and the
    /// rim of the generated sheet it has to meet.
    pub const PARCHMENT_EDGE: Color = Color::srgb(0.669, 0.596, 0.433);
    /// Ink.
    pub const PARCHMENT_INK: Color = Color::srgb(0.098, 0.082, 0.062);
    /// The quieter ink: hints, tallies, what a key does.
    pub const PARCHMENT_SOFT: Color = Color::srgb(0.290, 0.251, 0.204);
    /// Brass: the answer a sheet is asking for.
    pub const BRASS: Color = Color::srgb(0.788, 0.635, 0.153);
    /// The accent on parchment — a sheet's own heading, set in the house
    /// colour rather than in body ink.
    ///
    /// [`BRASS`] is a *light*: it fills a keycap, a wash, a border, each of
    /// which is its own ground. As letters on [`PARCHMENT`] it measures
    /// 1.6:1, which is why nothing on a sheet is written in it. This is the
    /// same hue taken down to an ink weight — every channel of [`BRASS`] at
    /// 0.53, which keeps the mix exactly and lands on 4.8:1 — the contrast
    /// the parchment's own danger ink carried, which retired with the prompt
    /// slip in §10.2 step 6 (the shelf and the drawer say danger in
    /// [`DANGER`], on dialog and not on paper).
    pub const INK_BRASS: Color = Color::srgb(0.418, 0.337, 0.081);
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
    /// The colour a nib bleeds into the fibres around it.
    ///
    /// Iron-gall ink does not cast a grey shadow; it wicks outwards and the
    /// front of it is warm amber-brown, because the iron oxidises before the
    /// gall darkens. That is the whole reason [`super::bleed`] is not
    /// [`SLIP_SHADOW`] at a different alpha: the old shadow was the right
    /// idea in the wrong colour and the wrong direction.
    pub const BLEED: Color = Color::srgb(0.290, 0.200, 0.090);
    /// The fill under an answer that is not the one the sheet is asking for.
    ///
    /// Not [`Color::NONE`]: a button with no fill and a drop shadow renders
    /// as a dark rounded hole in the parchment, because the shadow is drawn
    /// under a surface that is not there. A few percent of the sheet's own
    /// colour is enough to be a surface and not enough to compete with
    /// [`BRASS`].
    pub const SLIP_GHOST: Color = Color::srgb(0.832, 0.779, 0.639);

    // ------------------------------------------------------ a dialog's own
    //
    // `docs/redesign-proposal.md` §1.3 draws the line and §6 applies it:
    // **parchment is a sheet you read from, a panel is a place you work.**
    // The prompt slip and the ability sheet are read; the zone browser has a
    // checkbox, a tally and a Confirm in it, and that is work. So it is a
    // panel — and a *warm* one, because the table it lies on is candlelit and
    // [`PANEL`]'s cool near-black is the one surface in this client that was
    // never on it.

    /// The dialog's ground.
    pub const DIALOG: Color = Color::srgb(0.110, 0.098, 0.075);
    /// A dialog's ground where it is lifted: the head and the footer bands.
    pub const DIALOG_LIT: Color = Color::srgb(0.149, 0.129, 0.098);
    /// Every line drawn on a dialog: its border, and the rules between rows.
    pub const DIALOG_LINE: Color = Color::srgb(0.216, 0.188, 0.122);
    /// What a dialog says.
    pub const DIALOG_INK: Color = Color::srgb(0.925, 0.890, 0.816);
    /// The quieter half of it: a type line, a tally, a badge.
    pub const DIALOG_SOFT: Color = Color::srgb(0.557, 0.514, 0.424);
    /// The same quiet ink, on the **actions row**, which is no longer opaque.
    ///
    /// [`DIALOG_SOFT`] reads 4.69 : 1 on an opaque [`DIALOG`] — 0.19 of
    /// headroom over the 4.5 prose needs, which is all it ever had. The owner
    /// asked for the row to be "a little bit transparent" on 14.09.2026
    /// (`ax-design.md` §3.3 had said opaque, with a reason, and this reverses
    /// it), and the row is composited over the sky: at a density of 0.92 the
    /// same ink falls to 4.07 : 1 at the cloth's worst pose, and the sentence
    /// standing on the row stops being readable.
    ///
    /// So the ink pays for the transparency, where the transparency is:
    /// `DIALOG_SOFT` scaled by 1.33 in **linear** light, which keeps its hue
    /// exactly and takes it to 5.18 : 1 at rest and 4.95 : 1 at the worst
    /// pose. `frontal.rs`'s
    /// `the_rail_stays_readable_at_every_pose_the_cloth_can_take` is what
    /// holds it, scanning the poses the two clocks can actually reach.
    ///
    /// Only for ink standing **directly on the row**: the waiting sentence,
    /// its bracketed asides, and the pool's own word. A keycap's legend sits
    /// on the cap's own opaque fill and keeps [`DIALOG_SOFT`]; so does every
    /// line in the drawer, which is opaque [`DIALOG`]; and the zone dialog —
    /// which the owner named as the colour to match — is not touched at all.
    pub const LEDGE_SOFT: Color = Color::srgb(0.635, 0.586, 0.485);
    /// What a dialog says about a thing that is not there.
    ///
    /// Quieter than [`DIALOG_SOFT`], and **deliberately** under the 4.5 : 1
    /// that prose has to clear: 3.08 : 1 on [`DIALOG`], which is above the
    /// 3.0 a large glyph or a disabled control is held to and below the ratio
    /// that would make it read as something to attend to. AX §3.2 names the
    /// number, `a_candle_is_dark_enough_to_write_on` holds both ends of it.
    ///
    /// Two readers, and they are the same claim twice: the em dash standing
    /// where an empty mana pool's entries would be, and — from §10.2 step 5 —
    /// a draw offer the engine would refuse. Not [`DEAD`], which is the cool
    /// near-black grey of the old panels and has no business on a candlelit
    /// shelf.
    pub const LEDGE_DEAD: Color = Color::srgb(0.430, 0.400, 0.330);
    /// Candle: an offer, at the energy of something the engine is asking for.
    ///
    /// The one hue the redesign leaves for "this is live" — [`ACCENT`]'s
    /// teal is what it replaces. [`BRASS`] is its neighbour and not its
    /// twin: brass is gilt, the colour of a thing already *taken* (an armed
    /// deed, a place in an ordering), and candle is the invitation.
    pub const CANDLE: Color = Color::srgb(0.878, 0.604, 0.227);
    /// [`CANDLE`] laid over a dialog: the fill under a chosen row.
    ///
    /// A wash rather than a fill, because a row that is chosen is still a row
    /// being read — the tick and the name carry the claim, and a bar of
    /// saturated candle across the list would make the chosen row the only
    /// thing on the sheet anyone can see.
    pub const CANDLE_WASH: Color = Color::srgba(0.878, 0.604, 0.227, 0.10);
    /// The rail down the edge of the row the keyboard is standing on.
    ///
    /// [`CANDLE`] dimmed, which is the grammar's own word for it: bright
    /// candle is an offer the *engine* made, and this is the client
    /// volunteering where the next press would land — a claim about the
    /// keyboard, not about the game. A rail and not a wash because
    /// [`CANDLE_WASH`] already means "chosen" on the same row, and the focus
    /// stands on rows that are not chosen and leaves rows that are.
    ///
    /// More opaque than either wash and still short of [`CANDLE`], because a
    /// two-pixel edge and a full-width fill do not read alike at one alpha:
    /// the wash is faint precisely because it covers the whole row.
    pub const CANDLE_EDGE: Color = Color::srgba(0.878, 0.604, 0.227, 0.55);
    /// The veil drawn over the table behind a dialog that holds the whole
    /// answer, at full strength.
    ///
    /// **Cold, and that is the whole of it.** A dialog this dark cannot be
    /// separated from a veiled table by brightness — [`DIALOG`] is srgb8
    /// (28, 25, 19) and the veiled baize measures (15, 29, 26), so the panel
    /// is the *darker* of the two in green and the difference cannot be read
    /// as depth. What separates them is **temperature**: a blue-black veil
    /// pulls the cloth towards teal and drains the leather rail of its
    /// warmth, and the dialog and its [`CANDLE`] are then the only warm
    /// things in the window. A pure-black veil would have darkened everything
    /// and separated nothing.
    ///
    /// It used to be the same blue-black the hand zone's own ground was, one
    /// step deeper, and the argument for that was that the zone is itself a
    /// veil over the felt so a second hue would read as two materials. **The
    /// zone is no longer a veil.** The owner asked for a container on
    /// 14.09.2026 and `crate::frontal` is what draws it: one dye,
    /// [`DIALOG`], at 0.88 falling to 0.92, with the table still showing
    /// through and nothing cool about it. So this alpha is now on its own —
    /// the veil is over the *table*, the container is the dialog's own
    /// colour, and the two no longer have to match.
    ///
    /// The alpha was **measured on screen, not reasoned about**: a
    /// `BackgroundColor` composites in linear space, where an alpha buys far
    /// less darkening than sRGB arithmetic predicts — `frontal::SKIRT`
    /// carries the same warning and the numbers that earned it. Measured at
    /// 1728×1052 against a live search, one screenshot either side of the
    /// same `Confirm`: the felt goes (29, 53, 43) → (15, 29, 26), the rail
    /// (21, 51, 40) → (12, 29, 25), a seat bar's ink 173 → 100 — a little
    /// over half, everywhere, and everything still legible. 0.60 was the
    /// first value and took the felt only to (18, 34, 29), which read as
    /// weather rather than as a table that has been put down.
    pub const TABLE_VEIL: Color = Color::srgba(0.020, 0.030, 0.055, 0.70);
    /// [`CANDLE_WASH`] with the pointer on it.
    ///
    /// Stated rather than derived, because `Feel`'s default hover shades a
    /// colour towards white and keeps its alpha: a wash at a tenth lifted
    /// that way is a slightly paler tenth, which is not an answer a player
    /// can see. Twice the wash is.
    pub const CANDLE_WASH_LIT: Color = Color::srgba(0.878, 0.604, 0.227, 0.20);
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

/// The corner a sheet is cut with, as a number.
///
/// Spelled out as well as wrapped for [`BTN_R`]'s reason and one more: a child
/// drawn inside a sheet has to be cut *concentrically* with it, which is this
/// number less the border it sits inside, and a child that guessed would show
/// as a square corner in a round one.
pub(crate) const SHEET_R: f32 = 14.0;

/// The corner a sheet is cut with. Rounder than a button, because it is a
/// larger object and a sheet with a button's radius reads as a big button.
pub(crate) fn sheet_radius() -> BorderRadius {
    BorderRadius::all(px(SHEET_R))
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
            border_radius: BorderRadius::all(px(SHEET_R - 1.0)),
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

/// The soft corner radius for buttons and tabs, as a number.
///
/// Spelled out as well as wrapped, because one thing on this screen wants it
/// on two corners and not on the other two: the drawer grows out of the shelf,
/// so it is rounded at the top and square where it meets the lip.
pub(crate) const BTN_R: f32 = 6.0;

/// The soft corner radius for buttons and tabs.
pub(crate) fn btn_radius() -> BorderRadius {
    BorderRadius::all(px(BTN_R))
}

/// Roughly how wide `text` is at `size`, before anything has been laid out.
///
/// `bevy_ui` measures text *during* layout, which is one frame too late for a
/// decision the layout itself depends on: the ledge has to know how wide its
/// question is in order to decide whether the question fits beside its
/// neighbours. So this is the estimator `hud::stack` already cuts card names
/// with — `stack::CHAR_WIDTH` of the nominal point size per character — with
/// the one correction a bold face needs, which Inter's own advance widths put
/// at about 3.5%.
///
/// It is an estimate and is allowed to be. What it feeds is a choice between
/// three arrangements with tens of pixels between them
/// ([`baylee_client_core::ledge::arrange`]), not a position: the rungs are
/// far enough apart that a few percent either way picks the same one. Nothing
/// is ever *placed* from this.
#[must_use]
pub(crate) fn text_width(text: &str, size: f32, bold: bool) -> f32 {
    #[allow(clippy::cast_precision_loss)]
    let chars = text.chars().count() as f32;
    chars * size * stack::CHAR_WIDTH * if bold { 1.035 } else { 1.0 }
}

/// A keycap's side, as a multiple of the legend on it.
///
/// A square and not a disc, because what it stands for is a **key**: the
/// character on it is the one a player presses, and a keyboard has no round
/// keys. It was a roundel and read as a bullet — an ornament numbering a list
/// rather than a control naming a keystroke.
///
/// A *ratio* and not a side, because caps are drawn at more than one size. It
/// was 21 px flat, which is 1.9 times the ability sheet's 11-pt rows and was
/// right there and wrong everywhere else: that sheet's smaller footer legend
/// sat in the same 21-px box, so the quietest key on it had the largest cap.
pub(crate) const KEYCAP_SIDE: f32 = 1.9;

/// The keycap's corner radius, and the ability sheet's row radius.
///
/// One constant for both, so a cap reads as a key sitting *in* its row rather
/// than as a second, differently-cornered object on it. A key is a square
/// with its corners taken off, which is what a small radius on a 21-pixel
/// square is; half the side is the circle it used to be.
pub(crate) const KEYCAP_R: f32 = 4.0;

/// One key, drawn as the key it is.
///
/// A key gets named in more than one register: on a row of the ability sheet,
/// where the digit is what arms it; in that sheet's footer, where `Esc` is
/// the way out; and on the ledge, where an answer carries the chord that
/// sends it. A player should not have to learn twice that a small square
/// means "press this". `lobby::ui::chip` is the same idea in the panel
/// register, which is where the settings screen draws every binding.
///
/// It **grows with its legend**: a digit is one character and `⇧Tab` is four,
/// so the side is a floor and not a width. Anything else would either clip
/// the word or make every digit sit in a box wide enough for the longest key
/// on the keyboard. The box grows with the *size* too — see [`KEYCAP_SIDE`].
///
/// Three colours and none of them derived from another, because they say
/// three different things. `ink` is the legend's: on a row the cap is a
/// control and is written in full ink, in a footer it is a reminder of a key
/// that is always there and is written in the same grey as the words beside
/// it. `border` is the cap's own edge, and it is the one this function used
/// to decide for itself — *parchment* knowledge (ink at the edge of brass,
/// soft grey otherwise) baked into a shape that is not parchment's. The ledge
/// draws caps on a dialog, where that rule gives the wrong answer, so the
/// edge is the caller's to name and `sheet::cap` is where the old one lives
/// on.
pub(crate) fn keycap(
    commands: &mut Commands,
    fonts: &UiFonts,
    legend: &str,
    fill: Color,
    ink: Color,
    border: Color,
    size: f32,
) -> Entity {
    let side = size * KEYCAP_SIDE;
    let key = commands
        .spawn((
            Node {
                min_width: px(side),
                height: px(side),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::horizontal(px(size * 0.45)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(KEYCAP_R)),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(border),
            Pickable::IGNORE,
        ))
        .id();
    let glyph = commands
        .spawn((
            Text::new(legend.to_string()),
            tf_bold(fonts, size),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(key).add_child(glyph);
    key
}

// ------------------------------------------------ what lies over what, and why
//
// Six numbers rather than six literals scattered over four files, because a
// `ZIndex` only ever orders a node **among its own parent's children**: every
// one of these is a direct child of [`HudRoot`], they are therefore all
// comparable, and the only way to see whether they are right is to read them
// together. Two siblings sharing a number are settled by the order they were
// spawned in, which is how the seat bars — a *second* retained tree, whose own
// root was tied with this one at zero — came to be drawn straight through the
// zone browser's dialog.
//
// These order **nothing outside `HudRoot`**. bevy sorts root nodes by
// `(GlobalZIndex, ZIndex)` and only then walks each subtree, so every other
// root in this client is either wholly above or wholly below all six: the seat
// bars at `GlobalZIndex(-1)` are under them, and [`crate::depart`]'s flying
// card, [`sheet`]'s ability sheet and [`crate::lifeflash`]'s number are over
// them, in that order.
//
// The order says one thing: **a surface that is answering a question stands
// over a surface that is merely showing one.** The veil is the hinge — what is
// below it goes dark, what is above it stays lit.

/// The stack of spells waiting to resolve.
pub(crate) const Z_STACK: i32 = 1;
/// The hand zone: the cards and the ground they lie on.
pub(crate) const Z_HAND: i32 = 2;
/// The veil over the table, behind a dialog holding the whole answer.
pub(crate) const Z_VEIL: i32 = 3;
/// The ledge: the edge the hand zone ends at, carrying the question, its
/// answers, the mana pool and the two ways out — and so the one surface a
/// veil must never dim.
///
/// It was `Z_SLIP`, for the prompt slip that used to float here and said the
/// same thing about itself. The name changed; the reason did not.
pub(crate) const Z_LEDGE: i32 = 4;
/// The zone browser's dialog — and the end screen's sheet, which is over a
/// veil of its own under a different root and wants the same answer.
pub(crate) const Z_SHEET: i32 = 5;
/// The hover preview, which describes whatever is under the pointer and so has
/// to stand over all of it — including a row of the dialog.
pub(crate) const Z_PREVIEW: i32 = 10;

/// How far anything fixed to the edge of the window stands off it.
///
/// One number, because the seat-tab strip, the phase rail under it, the mana
/// chip and the zone browser's own margin were a single column of things down
/// the left of the screen standing at 8, 10, 12 and 12. Nothing about the
/// difference meant anything — it was three people picking a number — and an
/// eye reading down that edge sees the disagreement long before it can name
/// it. Two of those four are gone now (the seat bars are on the table), and
/// the number outlived them: it is what the stack, the tray and the browser
/// all stand off by — and since §10.2 step 5 put the two ways out of a game
/// on the shelf, it is what *everything* pinned to an edge stands off by.
/// There is no exception left.
///
/// The hand zone keeps its own ten: its edge is never seen (it is full-width
/// and its cards are centred), and the number is load-bearing arithmetic in
/// [`hand`]'s spread rather than an inset.
pub(crate) const EDGE: f32 = 12.0;

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
    let options = crate::abilities::options_for(
        lang,
        view,
        interaction,
        object,
        (duel.ability_menu == Some(object))
            .then(|| duel.asking_tap())
            .flatten(),
    );
    (!options.is_empty()).then_some(options)
}

/// The two resources that say how a card is moving right now.
///
/// One `SystemParam` and not two parameters, and the reason is a ceiling
/// rather than taste: bevy implements `IntoSystemSet` for system functions of
/// up to sixteen parameters, and [`sync_overlay`] already had sixteen. A
/// seventeenth does not fail where it is written — it fails at every
/// `.before()` and `.after()` that names the system, with an error about the
/// *ordering*, which is a long way from the line that caused it.
#[derive(bevy::ecs::system::SystemParam)]
pub struct CardMotion<'w> {
    /// When each card last caught the light.
    pub(crate) sheen: Res<'w, crate::sheen::Sheen>,
    /// Which card is under the finger, and where that has put it.
    pub(crate) touch: Res<'w, crate::touch::Touched>,
}

/// The overlay's retained tree, as the two things a rebuild has to tell
/// apart.
///
/// One `SystemParam` for the reason [`CardMotion`] gives — [`sync_overlay`]
/// stands at bevy's sixteen-parameter ceiling and a seventeenth fails at
/// every `.before()` that names it rather than where it is written.
///
/// The pair exists because a rebuild is no longer a clean sweep.
/// [`HudRevision`] counts the hover, so the tree is torn down whenever the
/// pointer moves; [`ledge::LedgeShelf`] is the one node that must not be, and
/// its doc comment has the whole reason. So the root is kept, its children
/// are despawned, and the shelf is passed over.
#[derive(bevy::ecs::system::SystemParam)]
pub struct OverlayTree<'w, 's> {
    /// The root, and whatever hangs off it.
    pub(crate) root: Query<'w, 's, (Entity, Option<&'static Children>), With<HudRoot>>,
    /// The shelf, so a child can be recognised as the one to keep.
    pub(crate) shelf: Query<'w, 's, Entity, With<ledge::LedgeShelf>>,
    /// The drawer's node, which is kept for the same reason and is a second
    /// query rather than an `Or` with the shelf: two things survive the sweep
    /// and they survive it for two arguments, so a reader of this struct
    /// should have to see both.
    pub(crate) drawer: Query<'w, 's, Entity, With<ledge::drawer::DrawerRoot>>,
    /// The zone dialog's veil, kept for a third argument: the dialog is
    /// rebuilt on a gate of its own ([`tray::TrayRevision`]), because it draws
    /// from no hover at all and this system's gate counts one.
    pub(crate) veil: Query<'w, 's, Entity, With<TableVeil>>,
    /// And the dialog's panel. A fourth query rather than an `Or` with the
    /// veil, and here the pair is *forced* rather than merely clearer: a
    /// `ZIndex` orders a node among its own parent's children, and the veil
    /// ([`Z_VEIL`]) and the panel ([`Z_SHEET`]) have the shelf's [`Z_LEDGE`]
    /// between them. Wrapping the two in one node would put the shelf behind
    /// the veil — and the shelf is where the question the dialog is answering
    /// is written.
    pub(crate) panel: Query<'w, 's, Entity, With<TrayBand>>,
}

/// The zone dialog's own nodes, and the root they hang from.
///
/// [`tray::sync_tray`]'s counterpart to [`OverlayTree`]. It asks for the root
/// rather than being handed one because the two systems are independent: the
/// overlay can take the root away between two frames of the dialog standing —
/// a game that ends, a board that has not arrived — and the dialog has to find
/// that out rather than be told.
#[derive(bevy::ecs::system::SystemParam)]
pub struct TrayTree<'w, 's> {
    /// The overlay's root, which both nodes are children of.
    pub(crate) root: Query<'w, 's, Entity, With<HudRoot>>,
    /// The veil, at [`Z_VEIL`].
    pub(crate) veil: Query<'w, 's, Entity, With<TableVeil>>,
    /// The panel, at [`Z_SHEET`].
    pub(crate) panel: Query<'w, 's, Entity, With<TrayBand>>,
}

mod card;
mod finish;
mod hand;
mod ledge;
mod motion;
mod overlay;
pub(crate) mod rail;
mod scroll;
pub(crate) mod seatbar;
mod sheet;
mod slip;
mod stack;
mod tray;

#[cfg(test)]
mod tests;

use card::{FaceCtx, spawn_card_art};
use hand::{
    PreviewAt, preview_anchor, preview_art_size, preview_face, preview_place, spawn_hand_zone,
    underneath_place,
};
use overlay::BUTTON_GAP;
use stack::spawn_stack_panel;

pub(crate) use finish::{FinishExits, despawn_finish, settle_the_sheet, spawn_finish};
pub use hand::apply_hand_scroll;
pub use hand::{ARMED_RAISE, HAND_ZONE_H, LEDGE_H, OVERLAY_CARD_H, OVERLAY_CARD_W};
/// The one line the actions row carries, which `frontal` paints because the
/// row is drawn by a `MaterialNode` and a border on one is a question.
pub(crate) use ledge::LIP as LEDGE_LIP;
pub use ledge::drawer::{DrawerRevision, DrawerRoot, sync_drawer, zoom_the_drawer};
pub use ledge::pool::{PoolRevision, sync_pool, zoom_the_pool};
pub use ledge::{LedgeLayout, LedgeRevision, LedgeShelf, sync_ledge};
pub(crate) use overlay::answer_button;
pub use overlay::{despawn_overlay, sync_overlay};
pub use rail::same_team;
pub use rail::{DesignationFlash, flash_the_designation, light_the_current_step};
pub(crate) use scroll::scrolled;
pub use scroll::{HandScroll, Scrolls, scrolls};
pub use seatbar::{
    BarRevision, LifeCell, SeatBar, SeatBarRoot, SeatInk, SeatStep, SeatTile, Shelf, Shelves,
    measure_shelves, place_seat_bars, stretch_step_tiles, sync_seat_bars,
};
pub use sheet::{
    AbilitySheet, AbilitySheetRoot, SheetClose, SheetNub, SheetPager, SheetRevision, SheetZoom,
    place_ability_sheet, sync_ability_sheet, zoom_the_sheet,
};
pub use slip::{SlipWash, wash_the_slip_in};
pub use stack::{StackMotion, ease_the_stack_in};
pub(crate) use tray::band_of;
pub(crate) use tray::dim_the_table;
pub use tray::{TrayRevision, sync_tray};

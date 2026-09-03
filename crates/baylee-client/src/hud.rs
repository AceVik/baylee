//! The 2D overlay: everything a player reads rather than manipulates.
//!
//! Three regions, always in the same place:
//!
//! - **Top** — the player tabs: every *other* seat with life and zone
//!   counts, the active seat highlighted, lost seats grayed out, teams
//!   sharing a color. Click or `Shift+1..9` inspects a seat's board.
//! - **Left** — the phase rail: the five phases, current one highlighted,
//!   per-phase standing orders (green = take priority, red = skip), plus
//!   the "next phase" and "end turn" autopilot buttons.
//! - **Bottom** — the hand bar: card images, overlapping but never less
//!   than 30% visible, horizontally scrollable when even that overflows,
//!   with a large hover tooltip for reading a card.
//!
//! The overlay is retained-UI: it is rebuilt only when something it shows
//! actually changed (snapshot, prompt, hover, selection, orders).

use baylee_client_core::i18n::{Lang, Phrase};

use crate::Duel;
use crate::cardmat::{CardLook, CardUiMaterial, UiCardMaterials, UiCards, finish_of, glow_bits};
use crate::textures::CardTextures;
use baylee_client_core::automation::{AutoPilot, RailRow};
use baylee_client_core::card_face::CardFace;
use baylee_client_core::images::{ArtSize, FinishTreatment, ImageKey};
use baylee_client_core::interaction::CombatFocus;
use baylee_core::ids::{Defender, ObjectId, PlayerId};
use baylee_view::{GameStatic, PlayerView};
use bevy::prelude::*;

/// The three UI fonts: Inter for text, Font Awesome Solid for icons, and
/// the `mana` font for mana symbols.
/// Bundled OFL/CC-BY fonts (see NOTICE) — the default font has none of the
/// weight range, the icon glyphs or the mana symbols.
#[derive(Resource, Clone)]
pub struct UiFonts {
    /// Text font (Inter, variable weight).
    pub text: Handle<Font>,
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
    /// Forward one step (next phase).
    pub const STEP: char = '\u{f051}';
    /// Fast-forward (end turn).
    pub const FAST: char = '\u{f050}';
    /// Skull and crossbones (poison counters).
    pub const POISON: char = '\u{f714}';
    /// Bolt (energy counters).
    pub const ENERGY: char = '\u{f0e7}';
    /// Caret down (speech-bubble tail).
    pub const CARET_DOWN: char = '\u{f0d7}';
    /// Expand (resize handle).
    pub const EXPAND: char = '\u{f065}';
    /// Crown (the command zone).
    pub const COMMAND: char = '\u{f521}';
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

/// A player tab at the top: click inspects that seat's board.
#[derive(Component)]
pub struct PlayerTab {
    /// The seat this tab represents.
    pub player: PlayerId,
}

/// A phase/step button on the rail: click toggles its standing order.
#[derive(Component)]
pub struct PhaseButton {
    /// Which rail (opponents' / your phases) this button belongs to.
    pub side: baylee_client_core::automation::RailSide,
    /// The rail row (step) this button controls.
    pub row: RailRow,
}

/// One of the two autopilot buttons at the rail's foot.
#[derive(Component)]
pub enum RailButton {
    /// Pass priority until the phase changes.
    NextPhase,
    /// Fast-forward to the next turn.
    EndTurn,
}

/// A game-menu button at the tab bar's right end.
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

/// The sliding own-board overlay (the panel, positioned by animation).
#[derive(Component)]
pub struct OwnBoardOverlay;

/// The knob on the overlay's top edge: click toggles it open/closed.
#[derive(Component)]
pub struct OverlayKnob;

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

/// A zone count that opens the browser on that pile when clicked.
///
/// The counts were already drawn — in the seat tabs and in the own-board
/// overlay — and were the one place the interface said "there are seven
/// cards here" and gave the player no way to look at them.
#[derive(Component)]
pub struct PileChip {
    /// The pile it opens, or `None` for every zone at once.
    pub zone: Option<baylee_client_core::browser::BrowseZone>,
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
            scrollable: false,
        };
    }
    let natural = count as f32 * card_w;
    if natural <= available_w {
        // Even spread: cards fully visible, spare space becomes gaps.
        let step = if count > 1 {
            ((available_w - card_w) / (count - 1) as f32).min(card_w + 8.0)
        } else {
            card_w
        };
        return HandLayout {
            step,
            content_width: (count - 1) as f32 * step + card_w,
            scrollable: false,
        };
    }
    let step = ((available_w - card_w) / (count - 1) as f32).max(card_w * MIN_VISIBLE);
    let content_width = (count - 1) as f32 * step + card_w;
    HandLayout {
        step,
        content_width,
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
    /// The own-board overlay's open/closed state (knob arrow).
    overlay_open: bool,
    /// Preview size (resized via handle or shortcut).
    preview_scale: f32,
    /// Whether cards are drawing their constructed face. Held on a key, so
    /// it changes between snapshots and has to be part of the redraw gate.
    faces: bool,
    /// How many printings have text. Text arrives over the network mid-game,
    /// and a face built before it lands says a good deal less.
    texts: usize,
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
    /// by a click on a pile chip, neither of which need be a new snapshot,
    /// and the tab and filter move with no snapshot at all.
    browser: (
        bool,
        Option<baylee_client_core::browser::BrowseZone>,
        String,
    ),
    /// The value a number choice stands at. It changes with no new snapshot —
    /// stepping X never leaves the client until Confirm — so without it the
    /// stepper would draw the opening value and then stay wrong.
    number: Option<u32>,
    /// The two menu buttons' states: whether a draw may be offered at all,
    /// and whether concede is waiting for its second press. The first follows
    /// the pending choice, the second nothing but the pointer, and a button
    /// whose label changes has to be redrawn when it does.
    menu: (bool, bool),
    /// What is armed and waiting for its second tap. Unlike a priority hold,
    /// which always arrives with a new `seq`, this never leaves the client at
    /// all — so without it here the armed row would never be drawn.
    armed: Option<crate::Armed>,
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
    /// Standing order "take priority" (green).
    pub const ORDER_GO: Color = Color::srgba(0.16, 0.35, 0.22, 0.95);
    /// Standing order "skip" (red).
    pub const ORDER_SKIP: Color = Color::srgba(0.40, 0.15, 0.15, 0.95);
    /// A card the client is offering to tap lands for: an offer, not a
    /// legal action, and drawn as the weaker claim it is.
    pub const REACHABLE: Color = Color::srgb(0.50, 0.47, 0.84);
    /// Soft shadow under raised elements.
    pub const SHADOW: Color = Color::srgba(0.0, 0.0, 0.0, 0.55);
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

/// An upward shadow for the own-board overlay.
fn overlay_shadow() -> BoxShadow {
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
mod rail;
mod stack;
mod tray;

#[cfg(test)]
mod tests;

use card::{FaceCtx, spawn_card_art};
use hand::{preview_anchor, preview_face, spawn_hand_bar};
use rail::{combat_line, spawn_phase_rail};
use stack::spawn_stack_panel;

pub use hand::apply_hand_scroll;
pub use hand::{HAND_BAR_H, OVERLAY_CARD_H, OVERLAY_CARD_W, TAB_H};
pub use overlay::{animate_overlay, despawn_overlay, sync_overlay};
pub use rail::RAIL_W;
pub use rail::same_team;

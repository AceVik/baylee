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

use crate::Duel;
use crate::textures::CardTextures;
use baylee_client_core::automation::{AutoPilot, RailRow};
use baylee_client_core::card_face::CardFace;
use baylee_client_core::images::{ArtSize, ImageKey};
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_view::{GameStatic, PlayerView};
use bevy::prelude::*;

/// The two UI fonts: Inter for text, Font Awesome Solid for icons.
/// Bundled OFL/CC-BY fonts (see NOTICE) — the default font has neither
/// the weight range nor the icon glyphs.
#[derive(Resource, Clone)]
pub struct UiFonts {
    /// Text font (Inter, variable weight).
    pub text: Handle<Font>,
    /// Icon font (Font Awesome 6 Free, solid).
    pub icons: Handle<Font>,
}

/// Loads the bundled fonts at startup.
pub fn setup_fonts(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(UiFonts {
        text: assets.load("fonts/Inter.ttf"),
        icons: assets.load("fonts/fa-solid-900.ttf"),
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
    overlay_closed: bool,
    /// Preview size (resized via handle or shortcut).
    preview_scale: f32,
    /// Whether cards are drawing their constructed face. Held on a key, so
    /// it changes between snapshots and has to be part of the redraw gate.
    faces: bool,
    /// How many printings have text. Text arrives over the network mid-game,
    /// and a face built before it lands says a good deal less.
    texts: usize,
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

/// A card's corner radius for a given rendered width (~10%: clearly
/// rounded, reads as a card, not a tile).
fn card_radius(width: f32) -> BorderRadius {
    BorderRadius::all(px(width * 0.10))
}

/// The preview's corner radius — a touch rounder than a physical card
/// (~8%), subtler than the cards on the table.
fn preview_radius(width: f32) -> BorderRadius {
    BorderRadius::all(px(width * 0.08))
}

// ------------------------------------------------- art, or the card's text

/// What the overlay needs in order to choose between a card's art and its own
/// constructed face.
///
/// Bundled because every card-drawing helper here needs all three, and three
/// more parameters on functions that already carry eleven is how a signature
/// stops being readable.
struct FaceCtx<'a> {
    texts: &'a crate::cardtext::CardTexts,
    mode: &'a crate::face::FaceMode,
    settings: &'a crate::settings::ClientSettings,
}

impl FaceCtx<'_> {
    /// Whether every card is showing its face, whatever its art is doing.
    ///
    /// Part of the redraw gate: this one is a held key and a setting, so it
    /// changes without a new snapshot.
    fn always(&self) -> bool {
        self.mode.held || self.settings.prefer_text_view
    }

    /// The face to draw instead of a card's art, or `None` to draw the art.
    fn object(
        &self,
        object: &baylee_view::PublicObject,
        textures: &CardTextures,
        art: Option<ImageKey>,
    ) -> Option<CardFace> {
        crate::face::wants_face(self.mode, self.settings, textures, art)
            .then(|| crate::face::of_object(object, self.texts))
    }

    /// The same, for a card in hand.
    fn hand(
        &self,
        card: &baylee_view::HandObject,
        textures: &CardTextures,
        art: Option<ImageKey>,
    ) -> Option<CardFace> {
        crate::face::wants_face(self.mode, self.settings, textures, art)
            .then(|| crate::face::of_hand(card, self.texts))
    }
}

/// Draws a card into a slot of a fixed size: its art, or its face.
///
/// Every place the overlay shows a card goes through here, which is what makes
/// the two interchangeable: the slot is the same size either way, so holding
/// the modifier reveals text without moving anything on screen.
fn spawn_card_art(
    commands: &mut Commands,
    image: Handle<Image>,
    built: Option<&CardFace>,
    width: f32,
    height: f32,
    detail: crate::face::Detail,
    fonts: &UiFonts,
) -> Entity {
    let slot = commands
        .spawn(Node {
            width: px(width),
            height: px(height),
            overflow: Overflow::clip(),
            ..default()
        })
        .id();
    let child = match built {
        Some(face) => crate::face::spawn_ui(commands, face, width, detail, fonts),
        None => commands
            .spawn((
                ImageNode::new(image),
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
            ))
            .id(),
    };
    commands.entity(slot).add_child(child);
    slot
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

/// Removes the overlay when the duel hands the screen back.
///
/// The 3D stage has always been torn down on `Close`; the overlay was not,
/// because until the client grew a lobby nothing ever closed a duel and came
/// back to something else. The revision goes with it: it describes a tree that
/// no longer exists, and the next duel's first frame has to rebuild rather
/// than compare against it.
pub fn despawn_overlay(
    mut commands: Commands,
    existing: Query<Entity, With<HudRoot>>,
    mut revision: ResMut<HudRevision>,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    *revision = HudRevision::default();
}

/// Rebuilds the overlay when anything it shows changes.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)] // one retained-UI rebuild, sectioned by comments
pub fn sync_overlay(
    mut commands: Commands,
    duel: Res<Duel>,
    mut revision: ResMut<HudRevision>,
    existing: Query<Entity, With<HudRoot>>,
    mut textures: ResMut<CardTextures>,
    assets: Res<AssetServer>,
    windows: Query<&Window>,
    fonts: Res<UiFonts>,
    settings: Res<crate::settings::ClientSettings>,
    texts: Res<crate::cardtext::CardTexts>,
    mode: Res<crate::face::FaceMode>,
) {
    let faces = FaceCtx {
        texts: &texts,
        mode: &mode,
        settings: &settings,
    };
    let seq = duel.board.as_ref().map(|b| b.seq);
    let prompt = duel
        .interaction
        .as_ref()
        .map(|i| i.prompt().headline())
        .or_else(|| duel.last_error.clone());
    let hovered = duel.hovered;
    let selected: Vec<ObjectId> = duel
        .interaction
        .as_ref()
        .map(|i| i.selected().to_vec())
        .unwrap_or_default();
    let orders = duel.orders.clone();
    let autopilot = duel.autopilot;
    let focus = duel.focus;
    let overlay_closed = duel.overlay_closed;
    let preview_scale = settings.preview_scale;

    if revision.seq == seq
        && revision.prompt == prompt
        && revision.hovered == hovered
        && revision.selected == selected
        && revision.orders.as_ref().is_some_and(|o| o.same_as(&orders))
        && revision.autopilot == autopilot
        && revision.focus == focus
        && revision.overlay_closed == overlay_closed
        && (revision.preview_scale - preview_scale).abs() < f32::EPSILON
        && revision.faces == faces.always()
        && revision.texts == texts.len()
        && !existing.is_empty()
    {
        return;
    }
    revision.seq = seq;
    revision.prompt.clone_from(&prompt);
    revision.hovered = hovered;
    revision.selected.clone_from(&selected);
    revision.orders = Some(orders.clone());
    revision.autopilot = autopilot;
    revision.focus = focus;
    revision.overlay_closed = overlay_closed;
    revision.preview_scale = preview_scale;
    revision.faces = faces.always();
    revision.texts = texts.len();

    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let (Some(board), Some(view)) = (duel.board.as_ref(), duel.view.as_ref()) else {
        return;
    };

    let root = commands
        .spawn((
            HudRoot,
            Node {
                width: percent(100),
                height: percent(100),
                ..default()
            },
            // The overlay must never eat clicks meant for the table.
            Pickable::IGNORE,
        ))
        .id();

    // ---- top: the full-width tab bar — ALL players left, menu right ----
    let tabs = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                right: px(0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(8), px(6)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            Pickable::IGNORE,
        ))
        .id();
    let players_row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(8),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for seat in &view.seats {
        let tab = spawn_player_tab(
            &mut commands,
            view,
            duel.statics.as_ref(),
            seat,
            focus,
            &fonts,
        );
        commands.entity(players_row).add_child(tab);
    }
    commands.entity(tabs).add_child(players_row);

    let menu_row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(8),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for (action, label, enabled) in [
        (MenuAction::OfferDraw, "Remis", true),
        (MenuAction::Concede, "Aufgeben", true),
    ] {
        let button = commands
            .spawn((
                MenuButton { action },
                Node {
                    padding: UiRect::axes(px(12), px(6)),
                    border_radius: btn_radius(),
                    ..default()
                },
                BackgroundColor(if enabled {
                    palette::PANEL_LIT
                } else {
                    palette::PANEL
                }),
                soft_shadow(),
                children![(
                    Text::new(label),
                    tf(&fonts, 13.0),
                    TextColor(if enabled { palette::INK } else { palette::DEAD }),
                )],
            ))
            .id();
        commands.entity(menu_row).add_child(button);
    }
    commands.entity(tabs).add_child(menu_row);
    commands.entity(root).add_child(tabs);

    // ---- right: the phase rail (opponents' phases top, yours bottom) ---
    let window_h = windows.single().map_or(800.0, Window::height);
    let rail = spawn_phase_rail(
        &mut commands,
        view,
        &orders,
        autopilot,
        &fonts,
        window_h,
        duel.statics.as_ref(),
    );
    commands.entity(root).add_child(rail);

    // ---- prompt bar (choice headline + answer buttons), above the hand,
    // padded clear of the phase rail ---------------------------------------
    if let Some(text) = prompt {
        let waiting = !duel.is_my_turn_to_act();
        let bar = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    bottom: px(HAND_BAR_H + 10.0),
                    right: px(RAIL_W + 12.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(6),
                    padding: UiRect::axes(px(14), px(8)),
                    border_radius: btn_radius(),
                    ..default()
                },
                BackgroundColor(palette::PANEL),
                soft_shadow(),
            ))
            .id();
        let headline = commands
            .spawn((
                Text::new(text),
                tf(&fonts, 18.0),
                TextColor(if waiting {
                    palette::MUTED
                } else {
                    palette::ACCENT
                }),
            ))
            .id();
        commands.entity(bar).add_child(headline);

        // Answer buttons, matching the pending choice.
        let answers: &[(PromptAction, &str)] = if waiting {
            &[]
        } else {
            match duel
                .interaction
                .as_ref()
                .map(baylee_client_core::Interaction::pending)
            {
                Some(baylee_engine::choice::Pending::Mulligan { .. }) => &[
                    (PromptAction::Keep, "Keep"),
                    (PromptAction::Mulligan, "Mulligan"),
                ],
                Some(baylee_engine::choice::Pending::YesNo { .. }) => {
                    &[(PromptAction::Yes, "Yes"), (PromptAction::No, "No")]
                }
                Some(_)
                    if duel
                        .interaction
                        .as_ref()
                        .is_some_and(baylee_client_core::Interaction::can_confirm) =>
                {
                    &[(PromptAction::Confirm, "OK")]
                }
                _ => &[],
            }
        };
        if !answers.is_empty() {
            let row = commands
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: px(6),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            for (action, label) in answers {
                let button = commands
                    .spawn((
                        PromptButton { action: *action },
                        Node {
                            padding: UiRect::axes(px(12), px(5)),
                            border_radius: btn_radius(),
                            ..default()
                        },
                        BackgroundColor(palette::ACCENT),
                        soft_shadow(),
                        children![(
                            Text::new(*label),
                            tf(&fonts, 13.0),
                            TextColor(palette::PANEL),
                        )],
                    ))
                    .id();
                commands.entity(row).add_child(button);
            }
            commands.entity(bar).add_child(row);
        }
        commands.entity(root).add_child(bar);
    }

    // ---- bottom: the hand bar (always on top) + commander zone ----------
    if let Some(statics) = duel.statics.as_ref() {
        let commanders = view
            .command
            .get(view.seat.get() as usize)
            .map_or(&[][..], Vec::as_slice);
        let cmdr_width = if commanders.is_empty() { 0.0 } else { 110.0 };
        let available = windows
            .single()
            .map_or(1200.0, |w| (w.width() - 20.0 - cmdr_width).max(0.0));
        let layout = hand_layout(board.hand.len(), HAND_CARD_W, available);
        let hand_bar = spawn_hand_bar(
            &mut commands,
            board,
            view,
            statics,
            hovered,
            &selected,
            layout,
            duel.hand_scroll,
            &mut textures,
            &assets,
            &fonts,
            &faces,
        );
        commands.entity(root).add_child(hand_bar);

        // ---- card preview: a speech-bubble tooltip over the hovered
        // card (hand, own battlefield, or command zone). No title text —
        // the image is big enough to read.
        if let Some((art, anchor)) = preview_anchor(board, view, hovered, layout, duel.hand_scroll)
        {
            let scale = settings.preview_scale.clamp(0.5, 1.75);
            let img_w = 308.0 * scale;
            let img_h = img_w * 88.0 / 63.0;
            let panel_w = img_w + 12.0;
            let win_w = windows.single().map_or(1200.0, Window::width);
            let anchor = anchor.unwrap_or(win_w / 2.0);
            // Always fully in the viewport.
            let left = (anchor - panel_w / 2.0).clamp(8.0, (win_w - panel_w - 8.0).max(8.0));
            let key = art.map(|art| ImageKey {
                size: ArtSize::Normal,
                ..art
            });
            // The face first: it only borrows the cache, and the image below
            // needs it mutably.
            let built = hovered.and_then(|id| preview_face(&faces, view, &textures, id, key));
            let image = match key {
                Some(key) => textures.get(key, statics, &assets),
                None => textures.card_back(),
            };
            let visual = spawn_card_art(
                &mut commands,
                image,
                built.as_ref(),
                img_w,
                img_h,
                crate::face::Detail::Full,
                &fonts,
            );
            let tooltip = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: px(HAND_BAR_H + 10.0),
                        left: px(left),
                        padding: UiRect::all(px(6)),
                        border_radius: preview_radius(img_w),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(palette::PANEL_LIT),
                    overlay_shadow(),
                    ZIndex(10),
                    Pickable::IGNORE,
                    children![(
                        // Resize handle, bottom right.
                        PreviewResize,
                        Node {
                            position_type: PositionType::Absolute,
                            right: px(4),
                            bottom: px(4),
                            padding: UiRect::all(px(4)),
                            border_radius: btn_radius(),
                            ..default()
                        },
                        BackgroundColor(palette::PANEL),
                        children![(
                            Text::new(glyph::EXPAND.to_string()),
                            icon_tf(&fonts, 11.0),
                            TextColor(palette::MUTED),
                        )],
                    ),],
                ))
                .id();
            commands.entity(tooltip).add_child(visual);
            commands.entity(root).add_child(tooltip);

            // The speech-bubble tail, pointing at the hovered card.
            let tail = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: px(HAND_BAR_H + 2.0),
                        left: px(anchor - 9.0),
                        ..default()
                    },
                    Pickable::IGNORE,
                    children![(
                        Text::new(glyph::CARET_DOWN.to_string()),
                        icon_tf(&fonts, 18.0),
                        TextColor(palette::PANEL_LIT),
                    )],
                ))
                .id();
            commands.entity(root).add_child(tail);
        }
    }

    // ---- the own-board overlay (sliding layer over the ellipse) --------
    if let Some(statics) = duel.statics.as_ref() {
        let overlay = spawn_own_board_overlay(
            &mut commands,
            board,
            view,
            statics,
            hovered,
            &selected,
            duel.overlay_closed,
            duel.overlay_t,
            window_h,
            &mut textures,
            &assets,
            &fonts,
            &faces,
        );
        commands.entity(root).add_child(overlay);
    }

    // ---- the stack (left of the rail, when non-empty) --------------------
    if !board.stack.is_empty() {
        let stack = spawn_stack_panel(&mut commands, board, &fonts);
        commands.entity(root).add_child(stack);
    }
}

/// One player tab: name, life, zone counts; active highlighted, lost
/// grayed out, team color at the border.
#[allow(clippy::too_many_lines)] // the icon+number spans are naturally flat
fn spawn_player_tab(
    commands: &mut Commands,
    view: &PlayerView,
    statics: Option<&GameStatic>,
    seat: &baylee_view::SeatView,
    focus: Option<PlayerId>,
    fonts: &UiFonts,
) -> Entity {
    let player = seat.player;
    let name = statics.map_or_else(
        || format!("Seat {player}"),
        |s| s.seat_name(player).to_string(),
    );
    let team = statics.and_then(|s| s.seats.iter().find(|i| i.player == player)?.team);
    let exile_count = view.exile.get(player.get() as usize).map_or(0, Vec::len);
    let is_active = view.active == player;
    let is_focused = focus == Some(player);
    let has_priority = view.priority == Some(player);

    let is_local = seat.player == view.seat;
    let (background, ink) = if seat.has_lost {
        (palette::PANEL, palette::DEAD)
    } else if is_active {
        (palette::PANEL_LIT, palette::INK)
    } else {
        (palette::PANEL, palette::INK)
    };
    let border_px = if is_active || is_focused { 2.0 } else { 1.0 };

    let marker = if has_priority { "▶ " } else { "" };
    let display = if is_local {
        format!("You ({name})")
    } else {
        name.clone()
    };
    let counts_color = if seat.has_lost {
        palette::DEAD
    } else {
        palette::MUTED
    };
    let tab = commands
        .spawn((
            PlayerTab { player },
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                padding: UiRect::axes(px(10), px(5)),
                border: UiRect::all(px(border_px)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(background),
            BorderColor::all(if is_active {
                palette::ACTIVE
            } else if is_local {
                palette::ACCENT
            } else {
                team_color(team)
            }),
            soft_shadow(),
            children![(
                // Name and life: name in text font, life with a heart icon.
                Text::new(format!("{marker}{display} ")),
                tf(fonts, 14.0),
                TextColor(if seat.has_lost { palette::DEAD } else { ink }),
                children![
                    (
                        TextSpan::new(glyph::HEART.to_string()),
                        icon_tf(fonts, 11.0),
                        TextColor(if seat.life <= 5 {
                            palette::DANGER
                        } else {
                            palette::ACCENT
                        }),
                    ),
                    (
                        TextSpan::new(format!(" {}", seat.life)),
                        tf(fonts, 14.0),
                        TextColor(if seat.has_lost {
                            palette::DEAD
                        } else if seat.life <= 5 {
                            palette::DANGER
                        } else {
                            ink
                        }),
                    ),
                ],
            ),],
        ))
        .id();

    // Zone counts as icon + number pairs, with experience counters
    // (poison, energy) appearing only when a player actually has them.
    let counts = commands
        .spawn((Text::new(""), tf(fonts, 11.0), TextColor(counts_color)))
        .id();
    commands.entity(tab).add_child(counts);
    let mut span = |icon: char, value: String| {
        let icon_span = commands
            .spawn((
                TextSpan::new(icon.to_string()),
                icon_tf(fonts, 10.0),
                TextColor(counts_color),
            ))
            .id();
        let value_span = commands
            .spawn((
                TextSpan::new(value),
                tf(fonts, 11.0),
                TextColor(counts_color),
            ))
            .id();
        commands.entity(counts).add_child(icon_span);
        commands.entity(counts).add_child(value_span);
    };
    span(glyph::HAND, format!(" {}  ", seat.hand_count));
    span(glyph::LIBRARY, format!(" {}  ", seat.library_count));
    span(glyph::SKULL, format!(" {}  ", seat.graveyard_count));
    span(glyph::EXILE, format!(" {exile_count}"));
    if seat.poison > 0 {
        span(glyph::POISON, format!(" {}", seat.poison));
    }
    if seat.energy > 0 {
        span(glyph::ENERGY, format!(" {}", seat.energy));
    }
    tab
}

/// Icon and the short rail label for a rail row.
fn row_visual(row: RailRow) -> (char, &'static str) {
    match row {
        RailRow::Untap => ('\u{f185}', "UNT"),
        RailRow::Upkeep => ('\u{f0ad}', "UPK"),
        RailRow::Draw => ('\u{f063}', "DRW"),
        RailRow::Main1 => ('\u{f024}', "M1"),
        RailRow::CombatBegin => ('\u{f71d}', "CBT"),
        RailRow::Attackers => ('\u{f70c}', "ATK"),
        RailRow::Blockers => ('\u{f3ed}', "BLK"),
        RailRow::Damage => ('\u{f6e2}', "DMG"),
        RailRow::CombatEnd => ('\u{f11e}', "EOC"),
        RailRow::Main2 => ('\u{f024}', "M2"),
        RailRow::EndStep => ('\u{f253}', "END"),
        RailRow::Cleanup => ('\u{f51a}', "CLN"),
    }
}

/// The rail's width.
pub const RAIL_W: f32 = 56.0;

/// The phase rail, split in two priority-control sections: the phases of
/// *opponents'* turns on top, your own (and teammates') phases at the
/// bottom, the turn number between them, and the autopilot buttons at the
/// foot behind a separator. Spans exactly from the player bar's bottom
/// to the hand bar's top. Row size and font scale with the available
/// height — they only shrink when the space runs out.
#[allow(clippy::too_many_lines)] // two sections + foot, one flat build
fn spawn_phase_rail(
    commands: &mut Commands,
    view: &PlayerView,
    orders: &baylee_client_core::automation::PhaseOrders,
    autopilot: Option<AutoPilot>,
    fonts: &UiFonts,
    window_h: f32,
    statics: Option<&GameStatic>,
) -> Entity {
    use baylee_client_core::automation::RailSide;

    let rail = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(0),
                top: px(TAB_H),
                bottom: px(HAND_BAR_H),
                width: px(RAIL_W),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::axes(px(4), px(6)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            Pickable::IGNORE,
        ))
        .id();

    // Responsive row size: only shrinks when the space runs out.
    let available = (window_h - TAB_H - HAND_BAR_H).max(240.0);
    let reserved = 2.0 * 14.0   // section headers
        + 20.0                  // turn number
        + 1.0 + 8.0             // separator + its margin
        + 2.0 * 30.0 + 4.0      // autopilot buttons + gap
        + 12.0; // rail padding
    let row_h = ((available - reserved) / 24.0).clamp(16.0, 30.0);
    let icon_size = (row_h * 0.42).clamp(8.0, 12.0);
    let label_size = (row_h * 0.30).clamp(6.5, 9.0);
    let show_label = row_h >= 21.0;

    let current = RailRow::current(view.phase, view.step);
    let active_is_mine = same_team(statics, view.active, view.seat);
    let current_side = if active_is_mine {
        RailSide::Mine
    } else {
        RailSide::Theirs
    };

    let spawn_section = |commands: &mut Commands, side: RailSide, header: &str| {
        let section = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(3),
                    align_items: AlignItems::Center,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let head = commands
            .spawn((Text::new(header), tf(fonts, 9.0), TextColor(palette::MUTED)))
            .id();
        commands.entity(section).add_child(head);
        for (row, skipped) in orders.rows_for(side) {
            let is_current = row == current && side == current_side;
            let is_selected = orders.selected() == Some((side, row));
            let (icon, short) = row_visual(row);
            let mut button_children = vec![
                commands
                    .spawn((
                        Text::new(icon.to_string()),
                        icon_tf(fonts, icon_size),
                        TextColor(if is_current {
                            palette::INK
                        } else {
                            palette::MUTED
                        }),
                    ))
                    .id(),
            ];
            if show_label {
                let label = commands
                    .spawn((
                        Text::new(short),
                        tf(fonts, label_size),
                        TextColor(if is_current {
                            palette::INK
                        } else {
                            palette::MUTED
                        }),
                    ))
                    .id();
                button_children.push(label);
            }
            let button = commands
                .spawn((
                    PhaseButton { side, row },
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        height: px(row_h),
                        border: UiRect::all(px(if is_selected || is_current { 2.0 } else { 1.0 })),
                        border_radius: btn_radius(),
                        width: percent(100),
                        ..default()
                    },
                    BackgroundColor(if skipped {
                        palette::ORDER_SKIP
                    } else {
                        palette::ORDER_GO
                    }),
                    BorderColor::all(if is_selected {
                        palette::ACCENT
                    } else if is_current {
                        palette::INK
                    } else {
                        palette::PANEL
                    }),
                ))
                .id();
            for child in button_children {
                commands.entity(button).add_child(child);
            }
            commands.entity(section).add_child(button);
        }
        section
    };

    // Top: the opponents' phases.
    let theirs = spawn_section(commands, RailSide::Theirs, "OPPONENT");
    commands.entity(rail).add_child(theirs);

    // Middle: the turn number between the two sections.
    let turn = commands
        .spawn((
            Node {
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(px(0), px(3)),
                ..default()
            },
            Pickable::IGNORE,
            children![(
                Text::new(format!("T{}", view.turn)),
                tf(fonts, 12.0),
                TextColor(palette::INK),
            )],
        ))
        .id();
    commands.entity(rail).add_child(turn);

    // Bottom: your own (and teammates') phases.
    let mine = spawn_section(commands, RailSide::Mine, "YOU");
    commands.entity(rail).add_child(mine);

    // Foot: separator, then the autopilot buttons with even padding.
    let separator = commands
        .spawn((
            Node {
                height: px(1),
                width: percent(100),
                margin: UiRect::axes(px(0), px(4)),
                ..default()
            },
            BackgroundColor(palette::DEAD),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(rail).add_child(separator);

    let foot = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(6),
                justify_content: JustifyContent::Center,
                padding: UiRect::all(px(2)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for (kind, icon, engaged) in [
        (
            RailButton::NextPhase,
            glyph::STEP,
            matches!(autopilot, Some(AutoPilot::ToNextPhase { .. })),
        ),
        (
            RailButton::EndTurn,
            glyph::FAST,
            matches!(autopilot, Some(AutoPilot::ToNextTurn { .. })),
        ),
    ] {
        let button = commands
            .spawn((
                kind,
                Node {
                    padding: UiRect::all(px(6)),
                    border: UiRect::all(px(1)),
                    border_radius: btn_radius(),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(palette::PANEL_LIT),
                BorderColor::all(if engaged {
                    palette::ACCENT
                } else {
                    palette::PANEL
                }),
                children![(
                    Text::new(icon.to_string()),
                    icon_tf(fonts, 13.0),
                    TextColor(if engaged {
                        palette::ACCENT
                    } else {
                        palette::INK
                    }),
                )],
            ))
            .id();
        commands.entity(foot).add_child(button);
    }
    commands.entity(rail).add_child(foot);

    rail
}

/// Whether two seats play on the same side (same team when teams are
/// set; identical seats otherwise).
#[must_use]
pub fn same_team(statics: Option<&GameStatic>, a: PlayerId, b: PlayerId) -> bool {
    if a == b {
        return true;
    }
    let team_of = |p: PlayerId| {
        statics
            .and_then(|s| s.seats.iter().find(|i| i.player == p))
            .and_then(|i| i.team)
    };
    matches!((team_of(a), team_of(b)), (Some(ta), Some(tb)) if ta == tb)
}

/// The hand bar: a clipping container with the scrolling strip inside,
/// the commander zone pinned to its right end. Always on top of the
/// own-board overlay.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)] // strip + commander zone are one flat build
fn spawn_hand_bar(
    commands: &mut Commands,
    board: &baylee_client_core::BoardModel,
    view: &PlayerView,
    statics: &GameStatic,
    hovered: Option<ObjectId>,
    selected: &[ObjectId],
    layout: HandLayout,
    scroll: f32,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
) -> Entity {
    let bar = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(0),
                left: px(0),
                right: px(0),
                height: px(HAND_BAR_H),
                padding: UiRect::axes(px(10), px(10)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            ZIndex(2),
            Pickable::IGNORE,
        ))
        .id();

    let strip = commands
        .spawn((
            HandStrip,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(10),
                height: px(HAND_CARD_H),
                // Spawn already at the current scroll offset — starting at
                // zero and correcting next frame is the hand's flicker.
                margin: UiRect::left(px(10.0 - scroll)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    for (i, card) in board.hand.iter().enumerate() {
        let is_selected = selected.contains(&card.id);
        let is_hovered = hovered == Some(card.id);
        // No border: the card is rounded like a real one; hover/selection
        // read as a soft accent glow instead of a frame.
        let shadow = if is_selected {
            BoxShadow::new(
                palette::ACCENT,
                Val::Px(0.0),
                Val::Px(0.0),
                Val::Px(2.0),
                Val::Px(10.0),
            )
        } else if is_hovered || card.playable {
            BoxShadow::new(
                if is_hovered {
                    palette::ACCENT
                } else {
                    palette::SHADOW
                },
                Val::Px(0.0),
                Val::Px(0.0),
                Val::Px(0.0),
                Val::Px(if is_hovered { 8.0 } else { 5.0 }),
            )
        } else {
            soft_shadow()
        };
        let built = view
            .hand
            .iter()
            .find(|h| h.id == card.id)
            .and_then(|h| faces.hand(h, textures, Some(card.art)));
        let image = textures.get(card.art, statics, assets);
        let visual = spawn_card_art(
            commands,
            image,
            built.as_ref(),
            HAND_CARD_W,
            HAND_CARD_H,
            crate::face::Detail::Full,
            fonts,
        );
        // Positioned by the layout rule; the strip's margin carries the
        // scroll offset (applied per frame, not rebuilt).
        let left = i as f32 * layout.step;
        let entity = commands
            .spawn((
                HandCardVisual { object: card.id },
                Node {
                    position_type: PositionType::Absolute,
                    left: px(left),
                    top: px(0),
                    width: px(HAND_CARD_W),
                    height: px(HAND_CARD_H),
                    border_radius: card_radius(HAND_CARD_W),
                    overflow: Overflow::clip(),
                    ..default()
                },
                shadow,
            ))
            .id();
        commands.entity(entity).add_child(visual);
        commands.entity(strip).add_child(entity);
    }
    commands.entity(bar).add_child(strip);

    // ---- commander zone (right end): the command zone with the cast
    // counter above the card. Commander format only — hidden otherwise.
    let commanders = view
        .command
        .get(view.seat.get() as usize)
        .map_or(&[][..], Vec::as_slice);
    if !commanders.is_empty() {
        let casts = view
            .seat(view.seat)
            .map_or(&[][..], |s| s.commander_casts.as_slice());
        let zone = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: px(10),
                    bottom: px(10),
                    flex_direction: FlexDirection::Row,
                    column_gap: px(6),
                    padding: UiRect::all(px(6)),
                    border_radius: btn_radius(),
                    ..default()
                },
                BackgroundColor(palette::PANEL_LIT),
                soft_shadow(),
                Pickable::IGNORE,
            ))
            .id();
        for (i, cmd) in commanders.iter().enumerate() {
            let times_cast = casts.get(i).copied().unwrap_or(0);
            let key = cmd
                .card
                .map(|c| ImageKey::new(c.print, c.face, ArtSize::Small));
            let built = faces.object(cmd, textures, key);
            let image = match key {
                Some(key) => textures.get(key, statics, assets),
                None => textures.card_back(),
            };
            let visual = spawn_card_art(
                commands,
                image,
                built.as_ref(),
                OVERLAY_CARD_W * 0.75,
                OVERLAY_CARD_H * 0.75,
                crate::face::Detail::Compact,
                fonts,
            );
            let card = commands
                .spawn((
                    HandCardVisual { object: cmd.id },
                    Node {
                        width: px(OVERLAY_CARD_W * 0.75),
                        height: px(OVERLAY_CARD_H * 0.75),
                        border_radius: card_radius(OVERLAY_CARD_W * 0.75),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    soft_shadow(),
                    children![(
                        // Cast counter, floating above the card's top.
                        Text::new(format!("×{times_cast}")),
                        tf(fonts, 12.0),
                        TextColor(palette::ACCENT),
                        Node {
                            position_type: PositionType::Absolute,
                            top: px(-6),
                            left: percent(50),
                            margin: UiRect::left(px(-10)),
                            padding: UiRect::axes(px(4), px(1)),
                            border_radius: btn_radius(),
                            ..default()
                        },
                        BackgroundColor(palette::PANEL),
                    ),],
                ))
                .id();
            commands.entity(card).add_child(visual);
            commands.entity(zone).add_child(card);
        }
        commands.entity(bar).add_child(zone);
    }
    bar
}

/// Applies the hand scroll offset and keeps the hovered card visible.
///
/// Runs per frame instead of being part of the rebuild: wheel ticks and
/// cursor moves must not respawn the whole strip.
pub fn apply_hand_scroll(
    mut duel: ResMut<Duel>,
    windows: Query<&Window>,
    mut strips: Query<&mut Node, With<HandStrip>>,
) {
    let (Some(board), Ok(window)) = (duel.board.as_ref(), windows.single()) else {
        return;
    };
    let available = (window.width() - 20.0).max(0.0);
    let layout = hand_layout(board.hand.len(), HAND_CARD_W, available);
    let max_scroll = (layout.content_width - available).max(0.0);

    // Keep the hovered card fully in view.
    if let Some(index) = board.hand.iter().position(|c| Some(c.id) == duel.hovered) {
        let start = index as f32 * layout.step;
        let end = start + HAND_CARD_W;
        if start < duel.hand_scroll {
            duel.hand_scroll = start;
        } else if end > duel.hand_scroll + available {
            duel.hand_scroll = end - available;
        }
    }
    duel.hand_scroll = duel.hand_scroll.clamp(0.0, max_scroll);

    for mut node in &mut strips {
        let wanted = UiRect::left(px(10.0 - duel.hand_scroll));
        if node.margin != wanted {
            node.margin = wanted;
        }
    }
}

/// Which card the preview shows and where its anchor (the bubble's tail
/// target) sits horizontally: hand cards anchor at their strip position;
/// everything else anchors at the screen's centre (`None`). Art comes
/// from the hand, the battlefield lanes, or the command zone.
fn preview_anchor(
    board: &baylee_client_core::BoardModel,
    view: &PlayerView,
    hovered: Option<ObjectId>,
    layout: HandLayout,
    scroll: f32,
) -> Option<(Option<ImageKey>, Option<f32>)> {
    let h = hovered?;
    if let Some(i) = board.hand.iter().position(|c| c.id == h) {
        let x = 10.0 + i as f32 * layout.step - scroll + HAND_CARD_W / 2.0;
        return Some((Some(board.hand[i].art), Some(x)));
    }
    for pod in &board.pods {
        for lane in &pod.lanes {
            for group in &lane.groups {
                if group.representative == h {
                    // A token has no art; it still gets a preview, built from
                    // its projected characteristics alone.
                    return Some((group.art, None));
                }
            }
        }
    }
    if let Some(cmd) = view
        .command
        .get(view.seat.get() as usize)
        .and_then(|cmds| cmds.iter().find(|c| c.id == h))
    {
        return Some((
            cmd.card
                .map(|c| ImageKey::new(c.print, c.face, ArtSize::Normal)),
            None,
        ));
    }
    None
}

/// The face for whatever the preview is pointing at.
///
/// The hand is checked first because a hand card is a [`baylee_view::HandObject`]
/// and never appears in [`PlayerView::object`]; everything else — battlefield,
/// stack, graveyard, exile, command zone — is one lookup.
fn preview_face(
    faces: &FaceCtx<'_>,
    view: &PlayerView,
    textures: &CardTextures,
    hovered: ObjectId,
    art: Option<ImageKey>,
) -> Option<CardFace> {
    if let Some(card) = view.hand.iter().find(|c| c.id == hovered) {
        return faces.hand(card, textures, art);
    }
    faces.object(view.object(hovered)?, textures, art)
}

/// Card width in the own-board overlay.
pub const OVERLAY_CARD_W: f32 = 86.0;
/// Card height in the own-board overlay (63:88).
pub const OVERLAY_CARD_H: f32 = OVERLAY_CARD_W * 88.0 / 63.0;
/// Height of the tab bar at the top (the overlay starts below it).
pub const TAB_H: f32 = 48.0;
/// The hand bar's height, including its padding.
pub const HAND_BAR_H: f32 = HAND_CARD_H + 20.0;

/// The own-board overlay: the local player's battlefield as big rounded
/// cards in three lanes, floating above the shared ellipse canvas, with a
/// shadow upwards. Slides down/up (X key or the knob on its top edge).
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)] // panel + knob + lanes are one flat build
fn spawn_own_board_overlay(
    commands: &mut Commands,
    board: &baylee_client_core::BoardModel,
    view: &PlayerView,
    statics: &GameStatic,
    hovered: Option<ObjectId>,
    selected: &[ObjectId],
    closed: bool,
    overlay_t: f32,
    window_h: f32,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
) -> Entity {
    // Spawn already at the current slide position — spawning open and
    // correcting next frame is the battlefield's flicker.
    let open_top = TAB_H;
    let closed_top = window_h - HAND_BAR_H - 14.0;
    let initial_top = open_top + (closed_top - open_top) * overlay_t;
    let panel = commands
        .spawn((
            OwnBoardOverlay,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(RAIL_W), // 100% minus the phase rail
                top: px(initial_top),
                bottom: px(HAND_BAR_H), // 100% minus tabs and the hand bar
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                // No knob row: the knob floats on the panel's edge, only
                // the button itself is visible.
                padding: UiRect {
                    top: px(0),
                    bottom: px(8),
                    left: px(12),
                    right: px(12),
                },
                ..default()
            },
            BackgroundColor(palette::PANEL),
            ZIndex(1),
            overlay_shadow(),
            Pickable::IGNORE,
        ))
        .id();

    // The knob: shallow, centered on the top edge, integrated into the
    // border; the arrow shows the direction the panel will move.
    let knob = commands
        .spawn((
            OverlayKnob,
            Node {
                position_type: PositionType::Absolute,
                top: px(-7),
                left: percent(50),
                margin: UiRect::left(px(-36)),
                width: px(72),
                height: px(14),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius {
                    top_left: px(7),
                    top_right: px(7),
                    ..default()
                },
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            children![(
                Text::new((if closed { '\u{f077}' } else { '\u{f078}' }).to_string()),
                icon_tf(fonts, 9.0),
                TextColor(palette::MUTED),
            )],
        ))
        .id();
    commands.entity(panel).add_child(knob);

    let Some(pod) = board.pods.iter().find(|p| p.is_local) else {
        return panel;
    };
    for lane in &pod.lanes {
        if lane.groups.is_empty() {
            continue;
        }
        let row = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(6),
                    height: px(OVERLAY_CARD_H),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        for group in &lane.groups {
            let is_selected = group.members.iter().any(|m| selected.contains(m));
            let is_hovered = hovered == Some(group.representative);
            let shadow = if is_selected || is_hovered {
                BoxShadow::new(
                    palette::ACCENT,
                    Val::Px(0.0),
                    Val::Px(0.0),
                    Val::Px(0.0),
                    Val::Px(8.0),
                )
            } else {
                soft_shadow()
            };
            let object = view.object(group.representative);
            let built = object.and_then(|o| faces.object(o, textures, group.art));
            // A token has no printing at all, so its face is the only thing
            // there is to draw — before this the overlay skipped it entirely.
            let image = match group.art {
                Some(art) => textures.get(art, statics, assets),
                None => textures.card_back(),
            };
            if built.is_none() && group.art.is_none() {
                continue;
            }
            let visual = spawn_card_art(
                commands,
                image,
                built.as_ref(),
                OVERLAY_CARD_W,
                OVERLAY_CARD_H,
                crate::face::Detail::Compact,
                fonts,
            );
            let card = commands
                .spawn((
                    HandCardVisual {
                        object: group.representative,
                    },
                    Node {
                        width: px(OVERLAY_CARD_W),
                        height: px(OVERLAY_CARD_H),
                        border_radius: card_radius(OVERLAY_CARD_W),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    shadow,
                    children![(
                        // Count chip for grouped stacks.
                        Text::new(if group.count() > 1 {
                            format!("×{}", group.count())
                        } else {
                            String::new()
                        }),
                        tf(fonts, 12.0),
                        TextColor(palette::INK),
                        Node {
                            position_type: PositionType::Absolute,
                            right: px(3),
                            bottom: px(2),
                            ..default()
                        },
                    ),],
                ))
                .id();
            commands.entity(card).add_child(visual);
            commands.entity(row).add_child(card);
        }
        commands.entity(panel).add_child(row);
    }
    panel
}

/// Slides the own-board overlay between its raised and its down position.
/// Raised: pinned under the tab bar. Down: slid beneath the hand (which
/// stays on top), with only the knob peeking above the hand bar so there
/// is always a way back.
pub fn animate_overlay(
    time: Res<Time>,
    mut duel: ResMut<Duel>,
    windows: Query<&Window>,
    mut panels: Query<&mut Node, With<OwnBoardOverlay>>,
) {
    let target = if duel.overlay_closed { 1.0 } else { 0.0 };
    let Ok(window) = windows.single() else {
        return;
    };
    // The `top` is recomputed every frame, so window resizes stay honest
    // even when the animation has settled.
    if (duel.overlay_t - target).abs() >= f32::EPSILON {
        let step = time.delta_secs() * 5.0;
        duel.overlay_t = if (target - duel.overlay_t).abs() <= step {
            target
        } else {
            duel.overlay_t + (target - duel.overlay_t).signum() * step
        };
    }
    let open_top = TAB_H;
    let closed_top = window.height() - HAND_BAR_H - 14.0;
    let top = open_top + (closed_top - open_top) * duel.overlay_t;
    for mut node in &mut panels {
        node.top = px(top);
    }
}

/// The stack, next-to-resolve at the top; pinned left of the phase rail.
fn spawn_stack_panel(
    commands: &mut Commands,
    board: &baylee_client_core::BoardModel,
    fonts: &UiFonts,
) -> Entity {
    let panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(64),
                top: percent(28),
                width: px(220),
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                padding: UiRect::all(px(8)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            Pickable::IGNORE,
        ))
        .id();

    let title = commands
        .spawn((
            Text::new(format!("Stack ({})", board.stack.len())),
            tf(fonts, 13.0),
            TextColor(palette::MUTED),
        ))
        .id();
    commands.entity(panel).add_child(title);

    for item in &board.stack {
        let label = if item.depth == 0 {
            format!("▸ {}", item.name)
        } else {
            format!("  {}", item.name)
        };
        let entity = commands
            .spawn((
                Text::new(label),
                tf(fonts, 14.0),
                TextColor(if item.depth == 0 {
                    palette::ACCENT
                } else {
                    palette::INK
                }),
            ))
            .id();
        commands.entity(panel).add_child(entity);
    }
    panel
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn few_cards_spread_evenly_and_fully_visible() {
        let layout = hand_layout(5, 100.0, 1000.0);
        assert!(!layout.scrollable);
        assert!(layout.step >= 100.0, "cards never overlap when they fit");
        assert!(layout.content_width <= 1000.0);
    }

    #[test]
    fn many_cards_overlap_but_keep_the_minimum_visible() {
        let layout = hand_layout(12, 100.0, 600.0);
        assert!(layout.step >= 30.0, "at least 30% of every card shows");
        assert!(layout.step < 100.0, "they must overlap to fit");
    }

    #[test]
    fn beyond_the_minimum_overlap_the_bar_becomes_scrollable() {
        let layout = hand_layout(30, 100.0, 400.0);
        assert!(layout.scrollable);
        assert!((layout.step - 30.0).abs() < 1e-4, "clamped to the 30% rule");
        assert!(layout.content_width > 400.0);
    }

    #[test]
    fn an_empty_hand_is_not_scrollable() {
        let layout = hand_layout(0, 100.0, 400.0);
        assert!(!layout.scrollable);
        assert!(layout.content_width.abs() < 1e-4);
    }
}

#[cfg(test)]
mod close_tests {
    use super::*;

    #[test]
    fn closing_the_duel_takes_the_overlay_with_it() {
        let mut app = App::new();
        app.init_resource::<HudRevision>()
            .add_systems(Update, despawn_overlay);
        let root = app.world_mut().spawn(HudRoot).id();
        let child = app.world_mut().spawn(Node::default()).id();
        app.world_mut().entity_mut(root).add_child(child);
        app.world_mut().resource_mut::<HudRevision>().overlay_closed = true;

        app.update();

        let mut roots = app.world_mut().query_filtered::<Entity, With<HudRoot>>();
        assert_eq!(roots.iter(app.world()).count(), 0, "the root is gone");
        let mut nodes = app.world_mut().query_filtered::<Entity, With<Node>>();
        assert_eq!(
            nodes.iter(app.world()).count(),
            0,
            "and its children went with it"
        );
        assert!(
            !app.world().resource::<HudRevision>().overlay_closed,
            "a revision describing a tree that no longer exists would make the \
             next duel's first frame skip its own rebuild"
        );
    }
}

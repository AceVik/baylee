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
fn tf(fonts: &UiFonts, size: f32) -> TextFont {
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
    /// Offer a draw — needs mutual agreement, a protocol item; shown
    /// disabled until it exists.
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
}

/// Palette, kept in one place so the overlay reads as one design.
mod palette {
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
fn soft_shadow() -> BoxShadow {
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
fn btn_radius() -> BorderRadius {
    BorderRadius::all(px(6))
}

/// A card's corner radius for a given rendered width (~8%: a little
/// rounder than a physical Magic card, which reads better on screen).
fn card_radius(width: f32) -> BorderRadius {
    BorderRadius::all(px(width * 0.08))
}

/// The preview's corner radius — rounder again than the cards on the
/// table, so the tooltip reads as a preview, not as another card.
fn preview_radius(width: f32) -> BorderRadius {
    BorderRadius::all(px(width * 0.12))
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
) {
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
        && revision
            .orders
            .as_ref()
            .is_some_and(|o| o.rows().eq(orders.rows()) && o.selected() == orders.selected())
        && revision.autopilot == autopilot
        && revision.focus == focus
        && revision.overlay_closed == overlay_closed
        && (revision.preview_scale - preview_scale).abs() < f32::EPSILON
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
        (MenuAction::OfferDraw, "Remis", false),
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

    // ---- right: the phase rail ------------------------------------------
    let rail = spawn_phase_rail(&mut commands, view, &orders, autopilot, &fonts);
    commands.entity(root).add_child(rail);

    // ---- prompt bar (choice headline), floating above the hand bar -----
    if let Some(text) = prompt {
        let waiting = !duel.is_my_turn_to_act();
        let bar = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    bottom: px(HAND_CARD_H + 30.0),
                    right: px(64),
                    padding: UiRect::axes(px(14), px(8)),
                    ..default()
                },
                BackgroundColor(palette::PANEL),
                children![(
                    Text::new(text),
                    tf(&fonts, 18.0),
                    TextColor(if waiting {
                        palette::MUTED
                    } else {
                        palette::ACCENT
                    }),
                )],
            ))
            .id();
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
            &mut textures,
            &assets,
            &fonts,
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
            let key = ImageKey {
                size: ArtSize::Normal,
                ..art
            };
            let image = textures.get(key, statics, &assets);
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
                    children![
                        (
                            ImageNode::new(image),
                            Node {
                                width: px(img_w),
                                height: px(img_h),
                                ..default()
                            },
                        ),
                        (
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
                        ),
                    ],
                ))
                .id();
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
            statics,
            hovered,
            &selected,
            duel.overlay_closed,
            &mut textures,
            &assets,
            &fonts,
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

/// The phase rail: pinned to the right edge, full height minus the hand
/// bar. Every step is one narrow icon button with its standing order;
/// the two autopilot buttons sit at the foot.
#[allow(clippy::too_many_lines)] // one rail, three sections
fn spawn_phase_rail(
    commands: &mut Commands,
    view: &PlayerView,
    orders: &baylee_client_core::automation::PhaseOrders,
    autopilot: Option<AutoPilot>,
    fonts: &UiFonts,
) -> Entity {
    let rail = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(0),
                top: px(TAB_H),
                bottom: px(HAND_CARD_H + 20.0),
                width: px(56),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::axes(px(4), px(6)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            Pickable::IGNORE,
        ))
        .id();

    // Top: turn + local life, then the phase/step buttons.
    let top = commands
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
    let local_life = view.seat(view.seat).map_or(0, |s| s.life);
    let header = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(4), px(4)),
                row_gap: px(1),
                ..default()
            },
            children![
                (
                    Text::new(format!("T{}", view.turn)),
                    tf(fonts, 11.0),
                    TextColor(palette::MUTED),
                ),
                (
                    Text::new("You "),
                    tf(fonts, 12.0),
                    TextColor(palette::MUTED),
                    children![
                        (
                            TextSpan::new(glyph::HEART.to_string()),
                            icon_tf(fonts, 10.0),
                            TextColor(if local_life <= 5 {
                                palette::DANGER
                            } else {
                                palette::ACCENT
                            }),
                        ),
                        (
                            TextSpan::new(format!(" {local_life}")),
                            tf(fonts, 12.0),
                            TextColor(if local_life <= 5 {
                                palette::DANGER
                            } else {
                                palette::INK
                            }),
                        ),
                    ],
                ),
            ],
        ))
        .id();
    commands.entity(top).add_child(header);

    let current_row = RailRow::current(view.phase, view.step);
    for (row, skipped) in orders.rows() {
        let is_current = row == current_row;
        let is_selected = orders.selected() == Some(row);
        let (icon, short) = row_visual(row);
        let button = commands
            .spawn((
                PhaseButton { row },
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(px(2), px(3)),
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
                children![
                    (
                        Text::new(icon.to_string()),
                        icon_tf(fonts, 14.0),
                        TextColor(if is_current {
                            palette::INK
                        } else {
                            palette::MUTED
                        }),
                    ),
                    (
                        Text::new(short),
                        tf(fonts, 10.0),
                        TextColor(if is_current {
                            palette::INK
                        } else {
                            palette::MUTED
                        }),
                    ),
                ],
            ))
            .id();
        commands.entity(top).add_child(button);
    }
    commands.entity(rail).add_child(top);

    // Foot: the autopilot buttons.
    let foot = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
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
                    padding: UiRect::axes(px(4), px(5)),
                    border: UiRect::all(px(1)),
                    justify_content: JustifyContent::Center,
                    width: percent(100),
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
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
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
                left: px(10),
                top: px(10),
                height: px(HAND_CARD_H),
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
        let image = textures.get(card.art, statics, assets);
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
                children![(
                    ImageNode::new(image),
                    Node {
                        width: percent(100),
                        height: percent(100),
                        ..default()
                    },
                )],
            ))
            .id();
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
            let image = match cmd
                .card
                .map(|c| ImageKey::new(c.print, c.face, ArtSize::Small))
            {
                Some(k) => textures.get(k, statics, assets),
                None => textures.card_back(),
            };
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
                    children![
                        (
                            ImageNode::new(image),
                            Node {
                                width: percent(100),
                                height: percent(100),
                                ..default()
                            },
                        ),
                        (
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
                        ),
                    ],
                ))
                .id();
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
) -> Option<(ImageKey, Option<f32>)> {
    let h = hovered?;
    if let Some(i) = board.hand.iter().position(|c| c.id == h) {
        let x = 10.0 + i as f32 * layout.step - scroll + HAND_CARD_W / 2.0;
        return Some((board.hand[i].art, Some(x)));
    }
    for pod in &board.pods {
        for lane in &pod.lanes {
            for group in &lane.groups {
                if group.representative == h {
                    return group.art.map(|art| (art, None));
                }
            }
        }
    }
    if let Some(cmd) = view
        .command
        .get(view.seat.get() as usize)
        .and_then(|cmds| cmds.iter().find(|c| c.id == h))
    {
        return cmd
            .card
            .map(|c| (ImageKey::new(c.print, c.face, ArtSize::Normal), None));
    }
    None
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
    statics: &GameStatic,
    hovered: Option<ObjectId>,
    selected: &[ObjectId],
    closed: bool,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
) -> Entity {
    let panel = commands
        .spawn((
            OwnBoardOverlay,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(TAB_H), // animate_overlay owns this
                bottom: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                // The hand is always on top: the battlefield slides under
                // it, so its content clears the hand bar's height.
                padding: UiRect {
                    top: px(20),
                    bottom: px(HAND_BAR_H + 8.0),
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
            let Some(art) = group.art else {
                continue;
            };
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
            let image = textures.get(art, statics, assets);
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
                    children![
                        (
                            ImageNode::new(image),
                            Node {
                                width: percent(100),
                                height: percent(100),
                                ..default()
                            },
                        ),
                        (
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
                        ),
                    ],
                ))
                .id();
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

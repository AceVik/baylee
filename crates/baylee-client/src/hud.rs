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
use baylee_client_core::automation::{self, AutoPilot};
use baylee_client_core::images::{ArtSize, ImageKey};
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_view::{GameStatic, Phase, PlayerView};
use bevy::prelude::*;

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

/// A phase button on the rail: click toggles its standing order.
#[derive(Component)]
pub struct PhaseButton {
    /// The phase this button controls.
    pub phase: Phase,
}

/// One of the two autopilot buttons at the rail's foot.
#[derive(Component)]
pub enum RailButton {
    /// Pass priority until the phase changes.
    NextPhase,
    /// Fast-forward to the next turn.
    EndTurn,
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

    // ---- top: player tabs (every seat but the local one) ---------------
    let tabs = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(8),
                left: px(8),
                right: px(8),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                column_gap: px(8),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for seat in view.seats.iter().filter(|s| s.player != view.seat) {
        let tab = spawn_player_tab(&mut commands, view, duel.statics.as_ref(), seat, focus);
        commands.entity(tabs).add_child(tab);
    }
    commands.entity(root).add_child(tabs);

    // ---- left: the phase rail ------------------------------------------
    let rail = spawn_phase_rail(&mut commands, view, &orders, autopilot);
    commands.entity(root).add_child(rail);

    // ---- prompt bar (choice headline), floating above the hand bar -----
    if let Some(text) = prompt {
        let waiting = !duel.is_my_turn_to_act();
        let bar = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    bottom: px(HAND_CARD_H + 30.0),
                    right: px(12),
                    padding: UiRect::axes(px(14), px(8)),
                    ..default()
                },
                BackgroundColor(palette::PANEL),
                children![(
                    Text::new(text),
                    TextFont::from_font_size(18.0),
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

    // ---- bottom: the hand bar -------------------------------------------
    if let Some(statics) = duel.statics.as_ref() {
        let available = windows
            .single()
            .map_or(1200.0, |w| (w.width() - 20.0).max(0.0));
        let layout = hand_layout(board.hand.len(), HAND_CARD_W, available);
        let hand_bar = spawn_hand_bar(
            &mut commands,
            board,
            statics,
            hovered,
            &selected,
            layout,
            &mut textures,
            &assets,
        );
        commands.entity(root).add_child(hand_bar);

        // ---- hover tooltip: the hovered hand card, readable -------------
        if let Some(card) = board.hand.iter().find(|c| Some(c.id) == hovered) {
            let key = ImageKey {
                size: ArtSize::Normal,
                ..card.art
            };
            let image = textures.get(key, statics, &assets);
            let tooltip = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: px(HAND_CARD_H + 40.0),
                        left: percent(50),
                        margin: UiRect::left(px(-160)),
                        padding: UiRect::all(px(6)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(4),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(palette::PANEL_LIT),
                    ZIndex(10),
                    Pickable::IGNORE,
                    children![
                        (
                            ImageNode::new(image),
                            Node {
                                width: px(308),
                                height: px(308.0 * 88.0 / 63.0),
                                ..default()
                            },
                        ),
                        (
                            Text::new(card.name.clone()),
                            TextFont::from_font_size(15.0),
                            TextColor(palette::INK),
                        ),
                    ],
                ))
                .id();
            commands.entity(root).add_child(tooltip);
        }
    }

    // ---- the stack (right side, when non-empty) -------------------------
    if !board.stack.is_empty() {
        let stack = spawn_stack_panel(&mut commands, board);
        commands.entity(root).add_child(stack);
    }
}

/// One player tab: name, life, zone counts; active highlighted, lost
/// grayed out, team color at the border.
fn spawn_player_tab(
    commands: &mut Commands,
    view: &PlayerView,
    statics: Option<&GameStatic>,
    seat: &baylee_view::SeatView,
    focus: Option<PlayerId>,
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

    let (background, ink) = if seat.has_lost {
        (palette::PANEL, palette::DEAD)
    } else if is_active {
        (palette::PANEL_LIT, palette::INK)
    } else {
        (palette::PANEL, palette::INK)
    };
    let border_px = if is_active || is_focused { 2.0 } else { 1.0 };

    let marker = if has_priority { "▶ " } else { "" };
    commands
        .spawn((
            PlayerTab { player },
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(1),
                padding: UiRect::axes(px(10), px(5)),
                border: UiRect::all(px(border_px)),
                ..default()
            },
            BackgroundColor(background),
            BorderColor::all(if is_active {
                palette::ACTIVE
            } else {
                team_color(team)
            }),
            children![
                (
                    Text::new(format!("{marker}{name} [{}]", seat.life)),
                    TextFont::from_font_size(14.0),
                    TextColor(if seat.has_lost {
                        palette::DEAD
                    } else if seat.life <= 5 {
                        palette::DANGER
                    } else {
                        ink
                    }),
                ),
                (
                    Text::new(format!(
                        "✋{} 📚{} 🪦{} ⛔{}",
                        seat.hand_count, seat.library_count, seat.graveyard_count, exile_count
                    )),
                    TextFont::from_font_size(11.0),
                    TextColor(if seat.has_lost {
                        palette::DEAD
                    } else {
                        palette::MUTED
                    }),
                ),
            ],
        ))
        .id()
}

/// The phase rail: local seat line, five phase buttons with their
/// standing order, and the two autopilot buttons at the foot.
#[allow(clippy::too_many_lines)] // one rail, three sections
fn spawn_phase_rail(
    commands: &mut Commands,
    view: &PlayerView,
    orders: &baylee_client_core::automation::PhaseOrders,
    autopilot: Option<AutoPilot>,
) -> Entity {
    let rail = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(8),
                top: px(64),
                bottom: px(HAND_CARD_H + 30.0),
                width: px(104),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    // Top: turn + local life, then the phase buttons.
    let top = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
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
                padding: UiRect::axes(px(8), px(5)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            children![
                (
                    Text::new(format!("T{}", view.turn)),
                    TextFont::from_font_size(13.0),
                    TextColor(palette::MUTED),
                ),
                (
                    Text::new(format!("You · {local_life}")),
                    TextFont::from_font_size(14.0),
                    TextColor(if local_life <= 5 {
                        palette::DANGER
                    } else {
                        palette::INK
                    }),
                ),
            ],
        ))
        .id();
    commands.entity(top).add_child(header);

    for (phase, skipped) in orders.rows() {
        let is_current = view.phase == phase;
        let is_selected = orders.selected() == Some(phase);
        let button = commands
            .spawn((
                PhaseButton { phase },
                Node {
                    padding: UiRect::axes(px(8), px(5)),
                    border: UiRect::all(px(if is_selected || is_current { 2.0 } else { 1.0 })),
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
                children![(
                    Text::new(automation::phase_name(phase)),
                    TextFont::from_font_size(12.0),
                    TextColor(if is_current {
                        palette::INK
                    } else {
                        palette::MUTED
                    }),
                )],
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
    for (kind, label, engaged) in [
        (
            RailButton::NextPhase,
            "Next ▶",
            matches!(autopilot, Some(AutoPilot::ToNextPhase { .. })),
        ),
        (
            RailButton::EndTurn,
            "End ⏭",
            matches!(autopilot, Some(AutoPilot::ToNextTurn { .. })),
        ),
    ] {
        let button = commands
            .spawn((
                kind,
                Node {
                    padding: UiRect::axes(px(8), px(6)),
                    border: UiRect::all(px(1)),
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
                    Text::new(label),
                    TextFont::from_font_size(12.0),
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

/// The hand bar: a clipping container with the scrolling strip inside.
#[allow(clippy::too_many_arguments)]
fn spawn_hand_bar(
    commands: &mut Commands,
    board: &baylee_client_core::BoardModel,
    statics: &GameStatic,
    hovered: Option<ObjectId>,
    selected: &[ObjectId],
    layout: HandLayout,
    textures: &mut CardTextures,
    assets: &AssetServer,
) -> Entity {
    let bar = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(0),
                left: px(0),
                right: px(0),
                height: px(HAND_CARD_H + 20.0),
                padding: UiRect::axes(px(10), px(10)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            Pickable::IGNORE,
        ))
        .id();

    let strip = commands
        .spawn((
            HandStrip,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                height: px(HAND_CARD_H),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    for (i, card) in board.hand.iter().enumerate() {
        let is_selected = selected.contains(&card.id);
        let is_hovered = hovered == Some(card.id);
        let (border, border_px) = if is_selected {
            (palette::ACCENT, 3.0)
        } else if is_hovered {
            (palette::ACCENT, 2.0)
        } else if card.playable {
            (palette::ACCENT, 1.0)
        } else {
            (palette::PANEL_LIT, 1.0)
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
                    border: UiRect::all(px(border_px)),
                    ..default()
                },
                BorderColor::all(border),
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
        let wanted = UiRect::left(px(-duel.hand_scroll));
        if node.margin != wanted {
            node.margin = wanted;
        }
    }
}

/// The stack, next-to-resolve at the top.
fn spawn_stack_panel(commands: &mut Commands, board: &baylee_client_core::BoardModel) -> Entity {
    let panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(12),
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
            TextFont::from_font_size(13.0),
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
                TextFont::from_font_size(14.0),
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

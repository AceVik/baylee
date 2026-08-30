//! The 2D overlay: everything a player reads rather than manipulates.
//!
//! Text belongs in screen space. Putting life totals or the stack into the 3D
//! scene would make them shear with the camera and shimmer under perspective,
//! and a player reading a stack under time pressure needs it crisp and always
//! in the same place.
//!
//! The overlay is rebuilt only when the board's sequence number changes. Bevy's
//! UI is retained, so rebuilding every frame would be both wasteful and
//! flickery; rebuilding on a new snapshot is simple, and snapshots arrive at
//! the speed of play rather than the speed of the display.

use crate::Duel;
use baylee_client_core::board::{SeatPod, ThreatSummary};
use baylee_view::{Phase, PlayerView, Step};
use bevy::prelude::*;

/// Root of the overlay.
#[derive(Component)]
pub struct HudRoot;

/// A hand card on the overlay: the 2D counterpart of [`crate::table::CardVisual`],
/// so clicks and hover treat hand and battlefield the same way.
#[derive(Component)]
pub struct HandCardVisual {
    /// The object the card represents and that input reports.
    pub object: baylee_core::ids::ObjectId,
}

/// Which snapshot the overlay currently shows.
#[derive(Resource, Default)]
pub struct HudRevision {
    seq: Option<u64>,
    prompt: Option<String>,
    /// Cursor position and choice selection — they change without a new
    /// snapshot (hover is per-frame, selection never leaves the client).
    hovered: Option<baylee_core::ids::ObjectId>,
    selected: Vec<baylee_core::ids::ObjectId>,
}

/// Palette, kept in one place so the overlay reads as one design.
mod palette {
    use bevy::prelude::Color;

    /// Panel background.
    pub const PANEL: Color = Color::srgba(0.05, 0.06, 0.08, 0.88);
    /// Primary text.
    pub const INK: Color = Color::srgb(0.90, 0.93, 0.94);
    /// Secondary text.
    pub const MUTED: Color = Color::srgb(0.58, 0.64, 0.68);
    /// The accent used for anything asking for a decision.
    pub const ACCENT: Color = Color::srgb(0.33, 0.75, 0.71);
    /// Danger: lethal damage, a seat about to lose.
    pub const DANGER: Color = Color::srgb(0.91, 0.47, 0.42);
    /// The active seat's marker.
    pub const ACTIVE: Color = Color::srgb(0.84, 0.64, 0.31);
    /// Hover/selection background: one step lighter than the panel.
    pub const HOVER_BG: Color = Color::srgba(0.10, 0.13, 0.16, 0.92);
}

/// Rebuilds the overlay when the snapshot changes.
#[allow(clippy::too_many_lines)] // one retained-UI rebuild, sectioned by comments
pub fn sync_overlay(
    mut commands: Commands,
    duel: Res<Duel>,
    mut revision: ResMut<HudRevision>,
    existing: Query<Entity, With<HudRoot>>,
) {
    let seq = duel.board.as_ref().map(|b| b.seq);
    let prompt = duel
        .interaction
        .as_ref()
        .map(|i| i.prompt().headline())
        .or_else(|| duel.last_error.clone());
    let hovered = duel.hovered;
    let selected: Vec<baylee_core::ids::ObjectId> = duel
        .interaction
        .as_ref()
        .map(|i| i.selected().to_vec())
        .unwrap_or_default();

    if revision.seq == seq
        && revision.prompt == prompt
        && revision.hovered == hovered
        && revision.selected == selected
        && !existing.is_empty()
    {
        return;
    }
    revision.seq = seq;
    revision.prompt.clone_from(&prompt);
    revision.hovered = hovered;
    revision.selected.clone_from(&selected);

    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let Some(board) = duel.board.as_ref() else {
        return;
    };

    let root = commands
        .spawn((
            HudRoot,
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            // The overlay must never eat clicks meant for the table.
            Pickable::IGNORE,
        ))
        .id();

    // ---- top: opponents, one compact row each
    let top = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(8),
                padding: UiRect::all(px(10)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for pod in board.pods.iter().filter(|p| !p.is_local) {
        let name = duel.statics.as_ref().map_or_else(
            || format!("Seat {}", pod.player),
            |s| s.seat_name(pod.player).to_string(),
        );
        let seat = spawn_seat_bar(&mut commands, pod, &name, duel.focus == Some(pod.player));
        commands.entity(top).add_child(seat);
    }
    commands.entity(root).add_child(top);

    // ---- turn indicator: one slim centered bar (turn · active · phase)
    if let Some(view) = duel.view.as_ref() {
        let bar = spawn_turn_bar(&mut commands, view, duel.statics.as_ref(), duel.seat());
        commands.entity(root).add_child(bar);
    }

    // ---- bottom: prompt, then the local seat line
    let bottom = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                padding: UiRect::all(px(10)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    if let Some(text) = prompt {
        let waiting = !duel.is_my_turn_to_act();
        let bar = commands
            .spawn((
                Node {
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
        commands.entity(bottom).add_child(bar);
    }

    if let Some(pod) = board.pods.iter().find(|p| p.is_local) {
        let name = duel.statics.as_ref().map_or_else(
            || format!("Seat {}", pod.player),
            |s| s.seat_name(pod.player).to_string(),
        );
        let seat = spawn_seat_bar(&mut commands, pod, &name, false);
        commands.entity(bottom).add_child(seat);
    }

    let hand = spawn_hand_strip(&mut commands, board, hovered, &selected);
    commands.entity(bottom).add_child(hand);
    commands.entity(root).add_child(bottom);

    if !board.stack.is_empty() {
        let stack = spawn_stack_panel(&mut commands, board);
        commands.entity(root).add_child(stack);
    }
}

/// The slim centered turn bar: turn number, whose turn it is, the current
/// phase/step, and a priority marker. One line, always in the same place —
/// the piece of game state a player checks most often.
fn spawn_turn_bar(
    commands: &mut Commands,
    view: &PlayerView,
    statics: Option<&baylee_view::GameStatic>,
    local: Option<baylee_core::ids::PlayerId>,
) -> Entity {
    let yours = local == Some(view.active);
    let active_name = statics.map_or_else(
        || format!("Seat {}", view.active),
        |s| s.seat_name(view.active).to_string(),
    );
    let priority_marker = match (view.priority, local) {
        (Some(p), Some(l)) if p == l => " · ▶ you",
        _ => "",
    };
    let label = format!(
        "T{} · {} · {}{}",
        view.turn,
        active_name,
        phase_label(view),
        priority_marker,
    );

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(10),
                left: percent(50),
                // Slim fixed-width bar, pulled back by half its width to
                // center it; content is one short line.
                width: px(280),
                margin: UiRect::left(px(-140)),
                padding: UiRect::axes(px(10), px(4)),
                flex_direction: FlexDirection::Row,
                column_gap: px(8),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(palette::PANEL),
            Pickable::IGNORE,
            children![
                (
                    Node {
                        width: px(8),
                        height: px(8),
                        border_radius: BorderRadius::all(px(4)),
                        ..default()
                    },
                    BackgroundColor(if yours {
                        palette::ACCENT
                    } else {
                        palette::ACTIVE
                    }),
                ),
                (
                    Text::new(label),
                    TextFont::from_font_size(13.0),
                    TextColor(if yours { palette::ACCENT } else { palette::INK }),
                ),
            ],
        ))
        .id()
}

/// A compact, player-facing name for the current phase/step.
#[must_use]
pub fn phase_label(view: &PlayerView) -> &'static str {
    match view.phase {
        Phase::Beginning => match view.step {
            Step::Untap => "Untap",
            Step::Upkeep => "Upkeep",
            Step::Draw => "Draw",
            _ => "Beginning",
        },
        Phase::FirstMain => "Main 1",
        Phase::Combat => match view.step {
            Step::CombatBegin => "Combat · Begin",
            Step::DeclareAttackers => "Attackers",
            Step::DeclareBlockers => "Blockers",
            Step::CombatDamageFirst | Step::CombatDamage => "Damage",
            Step::CombatEnd => "Combat · End",
            _ => "Combat",
        },
        Phase::SecondMain => "Main 2",
        Phase::Ending => match view.step {
            Step::Cleanup => "Cleanup",
            _ => "End Step",
        },
    }
}

/// One seat's line: life, the threat read, and its token chips.
fn spawn_seat_bar(commands: &mut Commands, pod: &SeatPod, name: &str, focused: bool) -> Entity {
    let border = if pod.has_priority {
        palette::ACCENT
    } else if pod.is_active {
        palette::ACTIVE
    } else {
        palette::PANEL
    };

    let life_colour = if pod.has_lost || pod.life <= 5 {
        palette::DANGER
    } else {
        palette::INK
    };

    let bar = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                padding: UiRect::axes(px(10), px(6)),
                border: UiRect::all(px(if focused { 2.0 } else { 1.0 })),
                min_width: px(190),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            BorderColor::all(border),
            Pickable::IGNORE,
        ))
        .id();

    let headline = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(10),
                ..default()
            },
            children![
                (
                    Text::new(name.to_string()),
                    TextFont::from_font_size(14.0),
                    TextColor(palette::MUTED),
                ),
                (
                    Text::new(format!("{} life", pod.life)),
                    TextFont::from_font_size(16.0),
                    TextColor(life_colour),
                ),
            ],
        ))
        .id();
    commands.entity(bar).add_child(headline);

    let threat = commands
        .spawn((
            Text::new(threat_line(&pod.threat)),
            TextFont::from_font_size(12.0),
            TextColor(palette::MUTED),
        ))
        .id();
    commands.entity(bar).add_child(threat);

    // Token chips: the compact answer to a board too wide to read card by card.
    for chip in pod.tokens.iter().take(3) {
        let entity = commands
            .spawn((
                Text::new(chip.label()),
                TextFont::from_font_size(12.0),
                TextColor(palette::INK),
            ))
            .id();
        commands.entity(bar).add_child(entity);
    }

    bar
}

/// The one-line threat read shown under every seat.
#[must_use]
pub fn threat_line(threat: &ThreatSummary) -> String {
    format!(
        "{} power ready · {} blockers · {} open · {} in hand",
        threat.attack_power, threat.potential_blockers, threat.open_mana, threat.cards_in_hand
    )
}

/// The local hand, playable cards first. Cards are interactive: clicking
/// (or hovering + E) plays a playable card or selects it for the pending
/// choice, exactly like a card on the table.
fn spawn_hand_strip(
    commands: &mut Commands,
    board: &baylee_client_core::BoardModel,
    hovered: Option<baylee_core::ids::ObjectId>,
    selected: &[baylee_core::ids::ObjectId],
) -> Entity {
    let strip = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(6),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    for card in &board.hand {
        let is_selected = selected.contains(&card.id);
        let is_hovered = hovered == Some(card.id);
        let (border, border_px, background) = if is_selected {
            (palette::ACCENT, 2.0, palette::HOVER_BG)
        } else if is_hovered {
            (palette::ACCENT, 1.0, palette::HOVER_BG)
        } else if card.playable {
            (palette::ACCENT, 1.0, palette::PANEL)
        } else {
            (palette::PANEL, 1.0, palette::PANEL)
        };
        let entity = commands
            .spawn((
                HandCardVisual { object: card.id },
                Node {
                    padding: UiRect::axes(px(8), px(5)),
                    border: UiRect::all(px(border_px)),
                    ..default()
                },
                BackgroundColor(background),
                BorderColor::all(border),
                children![(
                    Text::new(card.name.clone()),
                    TextFont::from_font_size(13.0),
                    TextColor(if card.playable || is_hovered || is_selected {
                        palette::INK
                    } else {
                        palette::MUTED
                    }),
                )],
            ))
            .id();
        commands.entity(strip).add_child(entity);
    }
    strip
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
    fn the_threat_line_reports_what_a_player_needs_before_deciding() {
        let threat = ThreatSummary {
            attack_power: 12,
            potential_attackers: 4,
            potential_blockers: 5,
            open_mana: 3,
            cards_in_hand: 2,
            air_defence: 1,
        };
        let line = threat_line(&threat);
        assert!(line.contains("12 power ready"));
        assert!(line.contains("5 blockers"));
        assert!(line.contains("3 open"));
        assert!(line.contains("2 in hand"));
    }
}

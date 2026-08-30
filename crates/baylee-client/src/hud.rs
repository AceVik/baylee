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
use bevy::prelude::*;

/// Root of the overlay.
#[derive(Component)]
pub struct HudRoot;

/// Which snapshot the overlay currently shows.
#[derive(Resource, Default)]
pub struct HudRevision {
    seq: Option<u64>,
    prompt: Option<String>,
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
}

/// Rebuilds the overlay when the snapshot changes.
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

    if revision.seq == seq && revision.prompt == prompt && !existing.is_empty() {
        return;
    }
    revision.seq = seq;
    revision.prompt.clone_from(&prompt);

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
        let seat = spawn_seat_bar(&mut commands, pod, duel.focus == Some(pod.player));
        commands.entity(top).add_child(seat);
    }
    commands.entity(root).add_child(top);

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
        let seat = spawn_seat_bar(&mut commands, pod, false);
        commands.entity(bottom).add_child(seat);
    }

    let hand = spawn_hand_strip(&mut commands, board);
    commands.entity(bottom).add_child(hand);
    commands.entity(root).add_child(bottom);

    if !board.stack.is_empty() {
        let stack = spawn_stack_panel(&mut commands, board);
        commands.entity(root).add_child(stack);
    }
}

/// One seat's line: life, the threat read, and its token chips.
fn spawn_seat_bar(commands: &mut Commands, pod: &SeatPod, focused: bool) -> Entity {
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
                    Text::new(format!("Seat {}", pod.player)),
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

/// The local hand, playable cards first.
fn spawn_hand_strip(commands: &mut Commands, board: &baylee_client_core::BoardModel) -> Entity {
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
        let entity = commands
            .spawn((
                Node {
                    padding: UiRect::axes(px(8), px(5)),
                    ..default()
                },
                BackgroundColor(palette::PANEL),
                BorderColor::all(if card.playable {
                    palette::ACCENT
                } else {
                    palette::PANEL
                }),
                children![(
                    Text::new(card.name.clone()),
                    TextFont::from_font_size(13.0),
                    TextColor(if card.playable {
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

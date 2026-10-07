//! The states every screen defines (the shell design, §2.5): an empty state,
//! skeleton rows while a list is read again (no veil, §10 #7), and an error
//! line with a retry.

use super::controls::Kit;
use super::metrics::px_fixed;
use super::role::Role;
use super::surfaces::prose;
use super::tokens::{self, RADIUS_CONTROL};
use crate::hud::icon_tf;
use bevy::prelude::*;

/// An empty state: a glyph, one sentence, one action.
pub fn empty(
    commands: &mut Commands,
    kit: Kit,
    glyph: char,
    sentence: &str,
    action: Option<Entity>,
) -> Entity {
    let column = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px_fixed(kit.m.gap),
                padding: UiRect::all(kit.m.px(24.0)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let mark = commands
        .spawn((
            Text::new(glyph.to_string()),
            icon_tf(kit.fonts, kit.m.h1),
            TextColor(tokens::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    let words = prose(commands, kit, sentence, false);
    commands.entity(column).add_children(&[mark, words]);
    if let Some(action) = action {
        commands.entity(column).add_child(action);
    }
    column
}

/// `rows` skeleton rows at the list's own pitch, where the list stands while
/// it is read again.
pub fn skeleton(commands: &mut Commands, kit: Kit, rows: usize) -> Entity {
    let column = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                width: Val::Percent(100.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for i in 0..rows {
        let line = commands
            .spawn((
                Node {
                    min_height: px_fixed(kit.m.row),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    row_gap: kit.m.px(6.0),
                    padding: UiRect::axes(kit.m.px(12.0), px_fixed(0.0)),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        // Bars of two lengths, so a list of them reads as rows of text.
        let long = if i % 2 == 0 { 46.0 } else { 38.0 };
        for (width, height) in [(long, 12.0), (long * 0.6, 9.0)] {
            let bar = commands
                .spawn((
                    Role::Skeleton,
                    Node {
                        width: Val::Percent(width),
                        height: kit.m.px(height),
                        border_radius: BorderRadius::all(px_fixed(RADIUS_CONTROL / 2.0)),
                        ..default()
                    },
                    BackgroundColor(tokens::SKELETON),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(line).add_child(bar);
        }
        commands.entity(column).add_child(line);
    }
    column
}

/// An error line: the sentence (a gateway's refusal verbatim) and a retry.
pub fn error_line(commands: &mut Commands, kit: Kit, sentence: &str, retry: Entity) -> Entity {
    let line = commands
        .spawn((
            Role::Error,
            Node {
                align_items: AlignItems::Center,
                column_gap: px_fixed(kit.m.gap),
                padding: UiRect::axes(kit.m.px(12.0), kit.m.px(6.0)),
                border: UiRect::left(px_fixed(3.0)),
                ..default()
            },
            BorderColor::all(tokens::DANGER),
            Pickable::IGNORE,
        ))
        .id();
    let mark = commands
        .spawn((
            Text::new("\u{f071}"),
            icon_tf(kit.fonts, kit.m.small),
            TextColor(tokens::DANGER),
            Pickable::IGNORE,
        ))
        .id();
    let words = prose(commands, kit, sentence, false);
    commands.entity(words).insert(Node {
        flex_shrink: 1.0,
        ..default()
    });
    commands.entity(line).add_children(&[mark, words, retry]);
    line
}

//! The Daily and Weekly pages' layout pieces, built from the shell kit.
//!
//! One vocabulary for both pages: a section is the kit's panel, a block
//! inside it is an inset on the control ground, a caption names a group, body
//! copy is body size, and a number stands in a stat cell rather than a pill.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::challenge_art::Scene;
use crate::shellkit::{Frame, surfaces};

/// Whether two sections stand side by side: a desktop window, not a tablet's.
pub(super) fn wide(kit: Kit) -> bool {
    matches!(kit.m.frame, Frame::Wide | Frame::Vast)
}

/// Whether the window is the narrowest desktop frame or a phone's.
pub(super) fn tight(kit: Kit) -> bool {
    matches!(kit.m.frame, Frame::Phone | Frame::Compact)
}

/// A section of the page: the kit's panel, the ground the lobby's own
/// activity tiles stand on, so the page reads as their continuation.
pub(super) fn section(commands: &mut Commands, root: Entity, kit: Kit) -> Entity {
    let panel = surfaces::panel(commands, kit, percent(100));
    commands.entity(panel).insert(Pickable::IGNORE);
    commands.entity(panel).entry::<Node>().and_modify(|mut n| {
        n.flex_shrink = 0.0;
        n.min_width = px_fixed(0.0);
    });
    commands.entity(root).add_child(panel);
    panel
}

/// A block inside a section: the control ground under a hairline.
pub(super) fn inset(commands: &mut Commands, root: Entity, kit: Kit) -> Entity {
    let block = commands
        .spawn((
            Node {
                width: percent(100),
                min_width: px_fixed(0.0),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.space(1),
                padding: UiRect::all(kit.m.space(2)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL.with_alpha(0.45)),
            BorderColor::all(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(block);
    block
}

/// Two or more children side by side, or stacked where the frame is narrow.
pub(super) fn split(commands: &mut Commands, root: Entity, kit: Kit, stacked: bool) -> Entity {
    let line = commands
        .spawn((
            Node {
                width: percent(100),
                min_width: px_fixed(0.0),
                flex_shrink: 0.0,
                flex_direction: if stacked {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                },
                align_items: AlignItems::Stretch,
                column_gap: px_fixed(kit.m.gap),
                row_gap: px_fixed(kit.m.gap),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(line);
    line
}

/// Gives a child of a [`split`] an equal share of the line.
pub(super) fn share(commands: &mut Commands, child: Entity, stacked: bool, weight: f32) {
    commands
        .entity(child)
        .entry::<Node>()
        .and_modify(move |mut n| {
            if stacked {
                n.width = percent(100);
            } else {
                n.width = Val::Auto;
                n.flex_grow = weight;
                n.flex_basis = px_fixed(0.0);
            }
            n.min_width = px_fixed(0.0);
        });
}

/// A group's caption: small capitals in muted ink.
pub(super) fn caption(commands: &mut Commands, root: Entity, kit: Kit, text: &str) {
    let caption = crate::lobby::parts::caption(commands, kit, text);
    commands.entity(root).add_child(caption);
}

/// Body copy: body size, wrapping in its column.
pub(super) fn prose(commands: &mut Commands, root: Entity, kit: Kit, text: &str, ink: Color) {
    let size = if ink == tokens::MUTED {
        kit.m.small
    } else {
        kit.m.text
    };
    let copy = commands
        .spawn((
            Text::new(text),
            tf(kit.fonts, surfaces::whole_pixels(size)),
            TextColor(ink),
            Node {
                width: percent(100),
                min_width: px_fixed(0.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(copy);
}

/// The page's or a card's title: one line of the given size.
pub(super) fn title(commands: &mut Commands, root: Entity, kit: Kit, text: &str, size: f32) {
    let title = commands
        .spawn((
            Text::new(text),
            tf(kit.fonts, size),
            TextColor(tokens::INK),
            Node {
                max_width: percent(100),
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(title);
}

/// A landscape across the top of a card, cropped to its band.
pub(super) fn banner(commands: &mut Commands, root: Entity, kit: Kit, art: Scene, height: f32) {
    let image = commands
        .spawn((
            art,
            Node {
                width: percent(100),
                height: kit.m.px(height),
                min_width: px_fixed(0.0),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                overflow: Overflow::clip(),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(image);
}

/// A hairline between two groups of a section.
pub(super) fn rule(commands: &mut Commands, root: Entity) {
    let line = commands
        .spawn((
            Node {
                width: percent(100),
                height: px_fixed(1.0),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(line);
}

/// A line of state under the page's head: loading, a failure, a pending
/// result. It stands on its own plate, never bare on the painting.
pub(super) fn notice(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    icon: char,
    text: &str,
    ink: Color,
) {
    let plate = commands
        .spawn((
            Node {
                width: percent(100),
                min_width: px_fixed(0.0),
                flex_shrink: 0.0,
                padding: UiRect::axes(px_fixed(kit.m.pad), kit.m.space(1)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::MIST),
            BorderColor::all(ink.with_alpha(0.45)),
            Pickable::IGNORE,
        ))
        .id();
    let line = crate::lobby::parts::glyph_line(commands, kit, icon, text, ink);
    commands.entity(plate).add_child(line);
    commands.entity(root).add_child(plate);
}

/// The Daily's sun and gold, the Weekly's moon and accent.
pub(super) fn kind_look(kind: api::Kind) -> (char, Phrase, Color) {
    if kind == api::Kind::Daily {
        (crate::hud::glyph::SUN, Phrase::ChallengeDaily, tokens::GOLD)
    } else {
        (
            crate::hud::glyph::MOON,
            Phrase::ChallengeWeekly,
            tokens::ACCENT,
        )
    }
}

/// The kind's badge: its glyph and name in its colour.
pub(super) fn kind_badge(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    lang: Lang,
    kind: api::Kind,
) {
    let (icon, name, ink) = kind_look(kind);
    let badge = crate::lobby::parts::badge_with(commands, kit, icon, name.text(lang), ink);
    commands.entity(root).add_child(badge);
}

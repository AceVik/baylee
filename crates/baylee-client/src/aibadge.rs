//! The one badge an AI player wears wherever its name is drawn (owner,
//! 26.11.2026: "AI players always labelled"; `docs/proposals/ai-players.md`
//! §"Who plays"): the mark of its kind and its label, said aloud as "AI
//! player: …".
//!
//! Two forms of one thing. [`pill`] is the mark and the label on the
//! shell's kit, for the desk, the ladder and the profile; [`disc`] is the
//! mark alone in a round slot where a person's portrait would stand, with
//! the same accessible words. The table draws the mark in its plate's own
//! mark slot ([`glyph`]). What a kind wears and what it is called is decided
//! in `client_core::aiplayers`; the marks come from the icon face the client
//! already ships (Font Awesome Free, `docs/legal.md`), so nothing new is
//! licensed for them.

use crate::hud::{glyph as marks, icon_tf, tf_bold};
use crate::shellkit::{controls::Kit, px_fixed, tokens};
use baylee_client_core::aiplayers::{self, Mark};
use baylee_client_core::aiplayers::{AiBadge, AiKind};
use bevy::prelude::*;

/// The glyph a kind's mark is drawn with.
#[must_use]
pub(crate) const fn glyph(kind: AiKind) -> char {
    match Mark::of(kind) {
        Mark::House => marks::HOUSE,
        Mark::Model => marks::AI_MODEL,
        Mark::Net => marks::AI_NET,
        Mark::Other => marks::AI_OTHER,
    }
}

/// The ink a kind's badge wears: the house the interface's own accent, a
/// language model a quiet violet, Baylee AI the candle's gold, an unknown
/// kind muted. Each passes on the shell's panel ground.
#[must_use]
pub(crate) fn ink(kind: AiKind) -> Color {
    match Mark::of(kind) {
        Mark::House => tokens::ACCENT,
        Mark::Model => MODEL_INK,
        Mark::Net => tokens::GOLD,
        Mark::Other => tokens::MUTED,
    }
}

/// A language model's ink.
const MODEL_INK: Color = Color::srgb(0.74, 0.66, 0.96);

/// The words assistive technology reads for a badge.
fn spoken(badge: &AiBadge, kit: Kit) -> bevy::a11y::AccessibilityNode {
    let mut node = accesskit::Node::new(accesskit::Role::Label);
    node.set_label(aiplayers::spoken(badge, kit.lang()));
    bevy::a11y::AccessibilityNode(node)
}

/// The badge: the kind's mark and the label in a pill of the kind's ink, at
/// the text size `size`. `hinted` gives it the hover hint that says what it
/// is; leave it out inside a control the pointer already answers to (a
/// ladder row), where the row's own press must win.
pub(crate) fn pill(
    commands: &mut Commands,
    kit: Kit,
    badge: &AiBadge,
    size: f32,
    hinted: bool,
) -> Entity {
    let lang = kit.lang();
    let tint = ink(badge.kind);
    let pill = commands
        .spawn((
            AiPill,
            Node {
                padding: UiRect::axes(kit.m.px(8.0), kit.m.px(2.0)),
                column_gap: kit.m.px(5.0),
                align_items: AlignItems::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                flex_shrink: 1.0,
                min_width: px_fixed(0.0),
                overflow: Overflow::clip(),
                ..default()
            },
            BorderColor::all(tint.with_alpha(0.55)),
            BackgroundColor(tint.with_alpha(0.12)),
            spoken(badge, kit),
        ))
        .id();
    if hinted {
        commands.entity(pill).insert((
            Pickable::default(),
            crate::lobby::HoverHint(aiplayers::hint(badge, lang)),
        ));
    } else {
        commands.entity(pill).insert(Pickable::IGNORE);
    }
    let mark = commands
        .spawn((
            Text::new(glyph(badge.kind).to_string()),
            icon_tf(kit.fonts, size * 0.88),
            TextColor(tint),
            Pickable::IGNORE,
        ))
        .id();
    let words = commands
        .spawn((
            Text::new(aiplayers::label(badge, lang)),
            tf_bold(kit.fonts, size),
            TextColor(tokens::INK),
            TextLayout::no_wrap(),
            Node {
                flex_shrink: 1.0,
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(pill).add_children(&[mark, words]);
    pill
}

/// The kind's mark alone, in a round slot of `diameter` logical pixels
/// where a person's portrait would stand.
pub(crate) fn disc(commands: &mut Commands, kit: Kit, badge: &AiBadge, diameter: f32) -> Entity {
    let tint = ink(badge.kind);
    let disc = commands
        .spawn((
            AiDisc,
            Node {
                width: px_fixed(diameter),
                height: px_fixed(diameter),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.5)),
                border_radius: BorderRadius::all(px_fixed(diameter)),
                ..default()
            },
            BorderColor::all(tint.with_alpha(0.65)),
            BackgroundColor(tint.with_alpha(0.14)),
            spoken(badge, kit),
            Pickable::IGNORE,
        ))
        .id();
    let mark = commands
        .spawn((
            Text::new(glyph(badge.kind).to_string()),
            icon_tf(kit.fonts, diameter * 0.34),
            TextColor(tint),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(disc).add_child(mark);
    disc
}

/// Marks a badge's pill, for the window tests and `/state`.
#[derive(Component)]
pub(crate) struct AiPill;

/// Marks a badge's disc.
#[derive(Component)]
pub(crate) struct AiDisc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_its_own_mark_and_ink() {
        let kinds = [AiKind::House, AiKind::Model, AiKind::Net, AiKind::Other];
        let glyphs: std::collections::HashSet<char> = kinds.into_iter().map(glyph).collect();
        assert_eq!(glyphs.len(), kinds.len());
        // The house keeps the robot a house chair already wears at the table.
        assert_eq!(glyph(AiKind::House), marks::HOUSE);
        for (a, b) in [(AiKind::House, AiKind::Model), (AiKind::Model, AiKind::Net)] {
            assert_ne!(ink(a), ink(b));
        }
    }
}

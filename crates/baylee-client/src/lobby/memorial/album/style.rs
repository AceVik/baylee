//! Ink and brass controls: only colour moves, never their hit geometry.
use crate::lobby::Press;
use crate::shellkit::{
    controls::{self, Kit, Live},
    focus::Stop,
    metrics::px_fixed,
    role::Role,
};
use bevy::prelude::*;

pub(super) const INK: Color = Color::srgb(0.24, 0.16, 0.105);
pub(super) const PAPER: Color = Color::srgb(0.96, 0.91, 0.80);
pub(super) const GOLD: Color = Color::srgb(0.58, 0.40, 0.19);

pub(super) fn label(c: &mut Commands, kit: Kit, text: &str, small: bool) -> Entity {
    let e = super::pages::serif(
        c,
        kit,
        text,
        if small { kit.m.small } else { kit.m.text },
        false,
    );
    c.entity(e).insert(TextLayout::no_wrap());
    e
}

pub(super) fn button(
    c: &mut Commands,
    kit: Kit,
    text: &str,
    live: Live,
    selected: bool,
    action: (Press, Stop),
) -> Entity {
    let base = if selected {
        Color::srgb(0.84, 0.73, 0.51)
    } else {
        Color::srgba(0.95, 0.88, 0.73, 0.70)
    };
    let face = c
        .spawn((
            Role::Button,
            Node {
                min_height: px_fixed(kit.m.control),
                min_width: px_fixed(36.0),
                padding: UiRect::axes(kit.m.px(12.0), kit.m.px(3.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(6.0)),
                ..default()
            },
            BackgroundColor(base),
            BorderColor::all(GOLD.with_alpha(if selected { 0.85 } else { 0.45 })),
            crate::ambience::Feel::tinting_to(
                base,
                if selected {
                    Color::srgb(0.92, 0.79, 0.53)
                } else {
                    Color::srgb(1.0, 0.94, 0.79)
                },
            ),
        ))
        .id();
    let words = label(c, kit, text, false);
    let icon = match text {
        "←" => Some("\u{f060}"),
        "→" => Some("\u{f061}"),
        "×" => Some("\u{f00d}"),
        "+" => Some("\u{f067}"),
        "−" => Some("\u{f068}"),
        _ => None,
    };
    if let Some(icon) = icon {
        c.entity(words)
            .insert((Text::new(icon), crate::hud::icon_tf(kit.fonts, kit.m.small)));
    }
    c.entity(face).add_child(words);
    let mut access = accesskit::Node::new(accesskit::Role::Button);
    access.set_label(text);
    let hit = controls::hit(
        c,
        kit,
        face,
        (action, bevy::a11y::AccessibilityNode(access)),
    );
    if let Live::No(reason) = live {
        c.entity(hit).insert(controls::Disabled);
        c.entity(face).remove::<crate::ambience::Feel>();
        c.entity(words).insert(TextColor(INK.with_alpha(0.42)));
        let tip = controls::tooltip(c, kit, reason);
        c.entity(hit).add_child(tip).insert(controls::Tooltip(tip));
    }
    hit
}

pub(super) fn rule(c: &mut Commands, kit: Kit) -> Entity {
    c.spawn((
        Node {
            width: Val::Percent(100.0),
            height: kit.m.px(1.0),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(GOLD.with_alpha(0.3)),
        Pickable::IGNORE,
    ))
    .id()
}

/// Two soft layers distinguish the paper, photograph and controls without scaling.
pub(super) fn elevation(height: f32) -> BoxShadow {
    BoxShadow(vec![
        ShadowStyle {
            color: Color::srgba(0.16, 0.10, 0.04, 0.12),
            x_offset: Val::Px(0.0),
            y_offset: Val::Px(height * 0.25),
            blur_radius: Val::Px(height * 0.65),
            spread_radius: Val::Px(0.0),
        },
        ShadowStyle {
            color: Color::srgba(0.12, 0.08, 0.04, 0.22),
            x_offset: Val::Px(0.0),
            y_offset: Val::Px(height),
            blur_radius: Val::Px(height * 2.5),
            spread_radius: Val::Px(0.0),
        },
    ])
}

//! Smoked obsidian-brass tokens and unified `shellkit` controls for Baylee's memorial.
use crate::lobby::Press;
use crate::shellkit::{
    controls::{self, Kit, Live},
    focus::Stop,
    metrics::px_fixed,
    role::Role,
    surfaces, tokens,
};
use bevy::prelude::*;

/// Warm antique-brass accent for chapter ornaments and rules.
pub(super) const BRASS: Color = Color::srgb(0.80, 0.66, 0.40);
/// Luminous warm gold for Baylee's dedication line and active chapter rims.
pub(super) const BRASS_LIGHT: Color = Color::srgb(0.92, 0.80, 0.56);
/// Subtle antique-brass border used on the memorial dialog, reading well, and photo mat.
pub(super) const BRASS_BORDER: Color = Color::srgba(0.78, 0.64, 0.38, 0.48);
/// Active chapter bookmark ground.
pub(super) const BRASS_ACTIVE_BG: Color = Color::srgba(0.24, 0.19, 0.11, 0.94);
/// Hovered active chapter bookmark ground.
pub(super) const BRASS_ACTIVE_HOT: Color = Color::srgba(0.32, 0.25, 0.14, 0.98);
/// Recessed smoked obsidian-vellum ground of the chapter reading well.
pub(super) const READING_WELL: Color = Color::srgba(0.032, 0.058, 0.088, 0.96);
/// Outer gallery matting around Baylee's framed photographs.
pub(super) const PHOTO_MAT: Color = Color::srgba(0.075, 0.098, 0.138, 0.96);
/// Hovered outer gallery matting around Baylee's framed photographs.
pub(super) const PHOTO_MAT_HOT: Color = Color::srgba(0.115, 0.148, 0.205, 0.99);
/// Inner dark bevel liner framing the photograph print.
pub(super) const PHOTO_LINER: Color = Color::srgb(0.022, 0.034, 0.052);
/// Warm high-contrast ivory ink for long-form chapter paragraphs.
pub(super) const STORY_INK: Color = Color::srgb(0.94, 0.92, 0.88);
/// Warm antique gold-ivory ink for chapter headings.
pub(super) const TITLE_INK: Color = Color::srgb(0.96, 0.88, 0.72);

pub(super) fn label(c: &mut Commands, kit: Kit, text: &str, small: bool) -> Entity {
    c.spawn((
        Text::new(text),
        if small {
            crate::hud::tf(kit.fonts, kit.m.small)
        } else {
            crate::hud::tf_bold(kit.fonts, kit.m.text)
        },
        TextColor(if small { tokens::MUTED } else { tokens::INK }),
        TextLayout::no_wrap(),
        Pickable::IGNORE,
    ))
    .id()
}

pub(super) fn pill_counter(c: &mut Commands, kit: Kit, text: &str) -> Entity {
    let pill = c
        .spawn((
            Role::Count,
            Node {
                min_height: kit.m.px(32.0),
                padding: UiRect::axes(kit.m.px(12.0), px_fixed(0.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_shrink: 0.0,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.07, 0.11, 0.88)),
            BorderColor::all(BRASS_BORDER.with_alpha(0.38)),
            Pickable::IGNORE,
        ))
        .id();
    let words = c
        .spawn((
            Text::new(text),
            crate::hud::tf_bold(kit.fonts, kit.m.small),
            TextColor(BRASS_LIGHT),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    c.entity(pill).add_child(words);
    pill
}

fn spawn_step_face(
    c: &mut Commands,
    kit: Kit,
    lead_icon: Option<char>,
    text: &str,
    trail_icon: Option<char>,
    dead: bool,
    selected: bool,
) -> (Entity, Color, Color) {
    let base = if dead {
        tokens::CONTROL.with_alpha(0.55)
    } else if selected {
        BRASS_ACTIVE_BG
    } else {
        tokens::CONTROL
    };
    let rim = if dead {
        tokens::BORDER.with_alpha(0.35)
    } else if selected {
        BRASS_LIGHT.with_alpha(0.78)
    } else {
        tokens::BORDER
    };
    let ink = if dead {
        tokens::DISABLED
    } else if selected {
        BRASS_LIGHT
    } else {
        tokens::INK
    };
    let pad_x = if text.is_empty() { 10.0 } else { 12.0 };
    let face = c
        .spawn((
            Role::Button,
            Node {
                min_height: px_fixed(kit.m.control * 0.88),
                min_width: px_fixed(34.0),
                flex_grow: 1.0,
                column_gap: kit.m.px(7.0),
                padding: UiRect::axes(kit.m.px(pad_x), px_fixed(0.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(base),
            BorderColor::all(rim),
        ))
        .id();
    let icon_ink = if dead { tokens::DISABLED } else { BRASS_LIGHT };
    if let Some(glyph) = lead_icon {
        let icon_node = c
            .spawn((
                Text::new(glyph.to_string()),
                crate::hud::icon_tf(kit.fonts, kit.m.small * 0.92),
                TextColor(icon_ink),
                Pickable::IGNORE,
            ))
            .id();
        c.entity(face).add_child(icon_node);
    }
    if !text.is_empty() {
        let words = c
            .spawn((
                Text::new(text),
                crate::hud::tf_bold(kit.fonts, kit.m.small),
                TextColor(ink),
                TextLayout::no_wrap(),
                Pickable::IGNORE,
            ))
            .id();
        c.entity(face).add_child(words);
    }
    if let Some(glyph) = trail_icon {
        let icon_node = c
            .spawn((
                Text::new(glyph.to_string()),
                crate::hud::icon_tf(kit.fonts, kit.m.small * 0.92),
                TextColor(icon_ink),
                Pickable::IGNORE,
            ))
            .id();
        c.entity(face).add_child(icon_node);
    }
    (face, base, rim)
}

#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
pub(super) fn step_button(
    c: &mut Commands,
    kit: Kit,
    lead_icon: Option<char>,
    text: &str,
    trail_icon: Option<char>,
    live: Live,
    selected: bool,
    stretch: bool,
    a11y_label: &str,
    action: (Press, Stop),
) -> Entity {
    let dead = matches!(live, Live::No(_));
    let (face, base, rim) = spawn_step_face(c, kit, lead_icon, text, trail_icon, dead, selected);
    let mut access = accesskit::Node::new(accesskit::Role::Button);
    access.set_label(a11y_label);
    let hit = if dead {
        c.entity(face)
            .insert(surfaces::elevation_control_disabled());
        let wrapper = controls::hit(
            c,
            kit,
            face,
            (
                action,
                controls::Disabled,
                bevy::a11y::AccessibilityNode(access),
            ),
        );
        if let Live::No(reason) = live {
            controls::attach_tooltip(c, wrapper, kit, reason);
        }
        wrapper
    } else {
        let hot = if selected {
            BRASS_ACTIVE_HOT
        } else {
            tokens::SELECTED
        };
        c.entity(face).insert((
            surfaces::elevation_control(),
            crate::ambience::Feel::tinting_to(base, hot),
            crate::ambience::FeelBorder {
                rest: rim,
                hot: BRASS_LIGHT,
                press: BRASS,
            },
            crate::ambience::FeelShadow,
        ));
        controls::hit(
            c,
            kit,
            face,
            (action, bevy::a11y::AccessibilityNode(access)),
        )
    };
    if stretch {
        c.entity(hit).entry::<Node>().and_modify(|mut node| {
            node.flex_grow = 1.0;
            node.flex_basis = px_fixed(0.0);
            node.min_width = px_fixed(0.0);
        });
    }
    hit
}

/// Crisp multi-layer gallery drop shadow under Baylee's matted photograph frame.
pub(super) fn photo_shadow() -> BoxShadow {
    BoxShadow(vec![
        ShadowStyle {
            color: Color::srgba(0.0, 0.0, 0.0, 0.46),
            x_offset: px_fixed(0.0),
            y_offset: px_fixed(3.0),
            blur_radius: px_fixed(8.0),
            spread_radius: px_fixed(0.0),
        },
        ShadowStyle {
            color: Color::srgba(0.0, 0.0, 0.0, 0.62),
            x_offset: px_fixed(0.0),
            y_offset: px_fixed(12.0),
            blur_radius: px_fixed(28.0),
            spread_radius: px_fixed(-2.0),
        },
    ])
}

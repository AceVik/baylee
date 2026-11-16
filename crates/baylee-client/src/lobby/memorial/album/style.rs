//! Smoked obsidian-brass tokens and symmetric `shellkit` controls for Baylee's memorial.
use crate::lobby::Press;
use crate::shellkit::{
    controls::{self, Kit, Live},
    focus::{Current, Stop},
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
pub(super) const BRASS_BORDER: Color = Color::srgba(0.78, 0.64, 0.38, 0.46);
/// Active chapter bookmark ground.
pub(super) const BRASS_ACTIVE_BG: Color = Color::srgba(0.26, 0.20, 0.11, 0.92);
/// Hovered active chapter bookmark ground.
pub(super) const BRASS_ACTIVE_HOT: Color = Color::srgba(0.34, 0.26, 0.14, 0.98);
/// Recessed smoked obsidian-vellum ground of the chapter story card.
pub(super) const READING_WELL: Color = Color::srgba(0.026, 0.048, 0.078, 0.72);
/// Outer gallery matting around Baylee's framed photographs.
pub(super) const PHOTO_MAT: Color = Color::srgba(0.062, 0.088, 0.128, 0.88);
/// Hovered outer gallery matting around Baylee's framed photographs.
pub(super) const PHOTO_MAT_HOT: Color = Color::srgba(0.105, 0.142, 0.198, 0.96);
/// Inner dark bevel liner framing the photograph print.
pub(super) const PHOTO_LINER: Color = Color::srgb(0.020, 0.032, 0.050);
/// Warm high-contrast ivory ink for long-form chapter paragraphs.
pub(super) const STORY_INK: Color = Color::srgb(0.94, 0.92, 0.88);
/// Warm antique gold-ivory ink for chapter headings.
pub(super) const TITLE_INK: Color = Color::srgb(0.97, 0.89, 0.73);

/// Single-quad 3-stop obsidian gallery gradient for the memorial dialog & lightbox.
pub(super) fn dialog_gradient() -> BackgroundGradient {
    use bevy::ui::{ColorStop, InterpolationColorSpace, LinearGradient};
    BackgroundGradient::from(
        LinearGradient::to_bottom(vec![
            ColorStop::percent(Color::srgba(0.068, 0.102, 0.152, 0.93), 0.0),
            ColorStop::percent(Color::srgba(0.036, 0.060, 0.096, 0.95), 50.0),
            ColorStop::percent(Color::srgba(0.024, 0.044, 0.074, 0.97), 100.0),
        ])
        .in_color_space(InterpolationColorSpace::Srgba),
    )
}

/// Directional antique-brass specular rim for the memorial dialog & lightbox.
pub(super) const fn dialog_border() -> BorderColor {
    BorderColor {
        top: Color::srgba(0.92, 0.80, 0.56, 0.52),
        right: Color::srgba(0.78, 0.64, 0.38, 0.34),
        bottom: Color::srgba(0.68, 0.55, 0.32, 0.38),
        left: Color::srgba(0.78, 0.64, 0.38, 0.34),
    }
}

pub(super) fn pill_counter(c: &mut Commands, kit: Kit, text: &str) -> Entity {
    let pill = c
        .spawn((
            Role::Count,
            Node {
                min_height: kit.m.px(30.0),
                padding: UiRect::axes(kit.m.px(13.0), px_fixed(0.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_shrink: 0.0,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.028, 0.052, 0.086, 0.72)),
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

/// Compact equal-width chapter step pill (`1`..`8`) for the centered chapter ribbon.
pub(super) fn chapter_pill(
    c: &mut Commands,
    kit: Kit,
    number: &str,
    selected: bool,
    chapter_title: &str,
    action: (Press, Stop),
) -> Entity {
    let base = if selected {
        BRASS_ACTIVE_BG
    } else {
        Color::srgba(0.045, 0.078, 0.122, 0.66)
    };
    let hot = if selected {
        BRASS_ACTIVE_HOT
    } else {
        Color::srgba(0.095, 0.155, 0.230, 0.88)
    };
    let rim = if selected {
        BRASS_LIGHT.with_alpha(0.82)
    } else {
        tokens::BORDER.with_alpha(0.62)
    };
    let ink = if selected { BRASS_LIGHT } else { tokens::INK };
    let face = c
        .spawn((
            Role::Button,
            Node {
                min_width: kit.m.px(34.0),
                min_height: kit.m.px(28.0),
                padding: UiRect::axes(kit.m.px(8.0), px_fixed(0.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(base),
            BorderColor::all(rim),
            surfaces::elevation_control(),
            crate::ambience::Feel::tinting_to(base, hot),
            crate::ambience::FeelBorder {
                rest: rim,
                hot: BRASS_LIGHT,
                press: BRASS,
            },
            crate::ambience::FeelShadow,
        ))
        .id();
    let words = c
        .spawn((
            Text::new(number),
            crate::hud::tf_bold(kit.fonts, kit.m.small),
            TextColor(ink),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    c.entity(face).add_child(words);
    let mut access = accesskit::Node::new(accesskit::Role::Button);
    access.set_label(chapter_title);
    let wrapper = controls::hit(
        c,
        kit,
        face,
        (
            action,
            Current(selected),
            crate::lobby::HoverHint(chapter_title.to_owned()),
            bevy::a11y::AccessibilityNode(access),
        ),
    );
    c.entity(wrapper).entry::<Node>().and_modify(|mut node| {
        node.min_height = px_fixed(34.0);
    });
    wrapper
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
        Color::srgba(0.035, 0.060, 0.095, 0.38)
    } else if selected {
        BRASS_ACTIVE_BG
    } else {
        Color::srgba(0.048, 0.084, 0.132, 0.72)
    };
    let rim = if dead {
        tokens::BORDER.with_alpha(0.28)
    } else if selected {
        BRASS_LIGHT.with_alpha(0.78)
    } else {
        tokens::BORDER.with_alpha(0.68)
    };
    let ink = if dead {
        tokens::DISABLED
    } else if selected {
        BRASS_LIGHT
    } else {
        tokens::INK
    };
    let pad_x = if text.is_empty() { 10.0 } else { 14.0 };
    let face = c
        .spawn((
            Role::Button,
            Node {
                min_height: px_fixed(kit.m.control * 0.86),
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
                crate::hud::icon_tf(kit.fonts, kit.m.small * 0.90),
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
                crate::hud::icon_tf(kit.fonts, kit.m.small * 0.90),
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
            Color::srgba(0.10, 0.17, 0.25, 0.92)
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

/// Symmetric antique-brass rule (`── ◆ ──`) separating the framed photograph from the story.
pub(super) fn divider_ornament(c: &mut Commands, kit: Kit) -> Entity {
    let row = c
        .spawn((
            Node {
                width: Val::Percent(100.0),
                max_width: kit.m.px(320.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: kit.m.px(12.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let wing = |c: &mut Commands| {
        c.spawn((
            Node {
                flex_grow: 1.0,
                height: px_fixed(1.0),
                ..default()
            },
            BackgroundColor(BRASS_BORDER.with_alpha(0.36)),
            Pickable::IGNORE,
        ))
        .id()
    };
    let left = wing(c);
    let gem = c
        .spawn((
            Node {
                width: kit.m.px(5.0),
                height: kit.m.px(5.0),
                flex_shrink: 0.0,
                ..default()
            },
            UiTransform::from_rotation(Rot2::radians(std::f32::consts::FRAC_PI_4)),
            BackgroundColor(BRASS.with_alpha(0.85)),
            Pickable::IGNORE,
        ))
        .id();
    let right = wing(c);
    c.entity(row).add_children(&[left, gem, right]);
    row
}

/// Three small antique-brass diamonds closing the bottom of a chapter's text.
pub(super) fn closing_ornament(c: &mut Commands, kit: Kit) -> Entity {
    let ornament = c
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: kit.m.px(20.0),
                column_gap: kit.m.px(10.0),
                margin: UiRect::top(kit.m.px(4.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for size in [4.0, 6.0, 4.0] {
        let diamond = c
            .spawn((
                Node {
                    width: kit.m.px(size),
                    height: kit.m.px(size),
                    ..default()
                },
                UiTransform::from_rotation(Rot2::radians(std::f32::consts::FRAC_PI_4)),
                BackgroundColor(BRASS),
                Pickable::IGNORE,
            ))
            .id();
        c.entity(ornament).add_child(diamond);
    }
    ornament
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

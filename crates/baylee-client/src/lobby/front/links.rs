//! Styled hyperlinks for the front door's colophon and the About sheet:
//! leading Font Awesome icon, distinct sky-cerulean ink, underline shown only
//! on hover, and a stationary optical click flash that never shifts size or
//! position.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::lobby::Press;
use crate::shellkit::controls::{self, Kit};
use crate::shellkit::metrics::px_fixed;
use crate::shellkit::role::Role;
use crate::shellkit::{surfaces, tokens};
use bevy::picking::hover::PickingInteraction;
use bevy::prelude::*;
use bevy::text::TextBackgroundColor;

use super::FrontPress;

/// Resting colour of every hyperlink icon and label: distinct from plain `INK`
/// and `MUTED`, with high readability against smoked glass and dialog surfaces.
pub(crate) const LINK_REST: Color = Color::srgb(0.44, 0.82, 0.96);

/// Hovered colour of hyperlink icons, labels, and hover underlines.
pub(crate) const LINK_HOT: Color = Color::srgb(0.78, 0.95, 1.0);

/// Peak click-flash colour of hyperlink icons and labels.
pub(crate) const LINK_FLASH: Color = Color::srgb(1.0, 0.93, 0.68);

/// Standard external-link glyph (`fa-up-right-from-square`, `U+F35D`).
pub(crate) const EXTERNAL_ICON: char = '\u{f35d}';

static NEXT_LINK_GROUP: AtomicU32 = AtomicU32::new(1);

fn next_group_id() -> u32 {
    NEXT_LINK_GROUP.fetch_add(1, Ordering::Relaxed)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SpanRole {
    Icon,
    Label,
}

/// Marker on standalone hyperlink child texts (`Icon` or `Label`) driven by
/// their parent face's hover and click state.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct HyperlinkPart {
    role: SpanRole,
}

/// Marker on the visual face of a standalone [`hyperlink`] control.
#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct HyperlinkFace {
    warmth: f32,
    pulse: f32,
    was_pressed: bool,
    painted: bool,
}

/// Marker on an inline `TextSpan` belonging to an interactive hyperlink inside
/// a wrapped prose sentence (`Fan Content Policy`, `Scryfall`).
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct InlineHyperlink {
    pub(crate) group: u32,
    pub(crate) role: SpanRole,
    warmth: f32,
    pulse: f32,
    was_pressed: bool,
    painted: bool,
}

/// Spawns a standalone styled hyperlink control: leading icon, sky-cerulean
/// label, no underline at rest (underlined on hover), subtle hover pill tint,
/// and stationary click flash.
pub(crate) fn hyperlink(
    commands: &mut Commands,
    kit: Kit,
    icon: char,
    text: &str,
    compact: bool,
    action: impl Bundle,
) -> Entity {
    let face = commands
        .spawn((
            HyperlinkFace::default(),
            Node {
                padding: UiRect::axes(
                    kit.m.px(if compact { 7.0 } else { 10.0 }),
                    kit.m.px(if compact { 2.0 } else { 3.0 }),
                ),
                min_height: if compact {
                    px_fixed(24.0)
                } else {
                    px_fixed(kit.m.control * 0.76)
                },
                column_gap: kit.m.px(5.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(Color::NONE),
            Pickable::IGNORE,
        ))
        .id();
    let mark = commands
        .spawn((
            HyperlinkPart {
                role: SpanRole::Icon,
            },
            Text::new(icon.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.82),
            TextColor(LINK_REST),
            Pickable::IGNORE,
        ))
        .id();
    let words = commands
        .spawn((
            HyperlinkPart {
                role: SpanRole::Label,
            },
            Text::new(text),
            crate::hud::tf(kit.fonts, surfaces::whole_pixels(kit.m.small)),
            TextColor(LINK_REST),
            Underline,
            UnderlineColor(Color::NONE),
            bevy::ui::LayoutConfig {
                use_rounding: false,
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(face).add_children(&[mark, words]);
    if compact && !kit.m.touch() {
        let wrapper = commands
            .spawn((
                Role::Hit,
                PickingInteraction::default(),
                Pickable::default(),
                Node {
                    min_height: px_fixed(26.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_shrink: 0.0,
                    ..default()
                },
                action,
            ))
            .id();
        commands.entity(wrapper).add_child(face);
        wrapper
    } else {
        controls::hit(commands, kit, face, action)
    }
}

/// Spawns a wrapped prose paragraph whose `targets` substrings are rendered
/// inline as interactive hyperlinks (`[icon] Target`), not underlined at rest
/// and underlined on hover, with hover and stationary click animations.
pub(crate) fn prose_with_inline_links(
    commands: &mut Commands,
    kit: Kit,
    sentence: &str,
    targets: &[(&str, FrontPress)],
    justify: Justify,
    small: bool,
) -> Entity {
    let size = surfaces::whole_pixels(if small { kit.m.small } else { kit.m.text });
    let segments = split_links(sentence, targets);
    let mut iter = segments.into_iter();
    let first_plain = match iter.next() {
        Some(Segment::Plain(s)) => s,
        Some(Segment::Link(label, press)) => {
            // Sentence opens directly with a link: anchor on an empty root run.
            let root = spawn_link_root(commands, kit, "", size, justify);
            attach_inline_link(commands, root, kit, size, label, press);
            for seg in iter {
                match seg {
                    Segment::Plain(text) => attach_plain_span(commands, root, kit, size, text),
                    Segment::Link(l, p) => attach_inline_link(commands, root, kit, size, l, p),
                }
            }
            return root;
        }
        None => "",
    };
    let root = spawn_link_root(commands, kit, first_plain, size, justify);
    for seg in iter {
        match seg {
            Segment::Plain(text) => attach_plain_span(commands, root, kit, size, text),
            Segment::Link(label, press) => {
                attach_inline_link(commands, root, kit, size, label, press);
            }
        }
    }
    root
}

enum Segment<'a> {
    Plain(&'a str),
    Link(&'a str, FrontPress),
}

fn split_links<'a>(sentence: &'a str, targets: &[(&'a str, FrontPress)]) -> Vec<Segment<'a>> {
    let mut out = Vec::new();
    let mut rest = sentence;
    while !rest.is_empty() {
        let earliest = targets
            .iter()
            .filter(|(needle, _)| !needle.is_empty())
            .filter_map(|&(needle, press)| rest.find(needle).map(|idx| (idx, needle, press)))
            .min_by_key(|&(idx, _, _)| idx);
        let Some((idx, needle, press)) = earliest else {
            out.push(Segment::Plain(rest));
            break;
        };
        if idx > 0 {
            out.push(Segment::Plain(&rest[..idx]));
        }
        out.push(Segment::Link(needle, press));
        rest = &rest[idx + needle.len()..];
    }
    out
}

fn spawn_link_root(
    commands: &mut Commands,
    kit: Kit,
    first: &str,
    size: f32,
    justify: Justify,
) -> Entity {
    commands
        .spawn((
            Text::new(first),
            crate::hud::tf(kit.fonts, size),
            TextColor(tokens::INK),
            TextLayout::justify(justify),
            bevy::ui::LayoutConfig {
                use_rounding: false,
            },
            Node {
                width: Val::Percent(100.0),
                ..default()
            },
            // Hoverable without blocking wheel/drag scroll on the parent sheet.
            Pickable {
                should_block_lower: false,
                is_hoverable: true,
            },
        ))
        .id()
}

fn attach_plain_span(commands: &mut Commands, root: Entity, kit: Kit, size: f32, text: &str) {
    if text.is_empty() {
        return;
    }
    let span = commands
        .spawn((
            TextSpan::new(text),
            crate::hud::tf(kit.fonts, size),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(span);
}

fn attach_inline_link(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    size: f32,
    label: &str,
    press: FrontPress,
) {
    let group = next_group_id();
    let icon_span = commands
        .spawn((
            InlineHyperlink {
                group,
                role: SpanRole::Icon,
                warmth: 0.0,
                pulse: 0.0,
                was_pressed: false,
                painted: false,
            },
            TextSpan::new(format!("{EXTERNAL_ICON}\u{2009}")),
            crate::hud::icon_tf(kit.fonts, size * 0.80),
            TextColor(LINK_REST),
            Pickable::default(),
            PickingInteraction::default(),
            Press::Front(press),
        ))
        .id();
    let word_span = commands
        .spawn((
            InlineHyperlink {
                group,
                role: SpanRole::Label,
                warmth: 0.0,
                pulse: 0.0,
                was_pressed: false,
                painted: false,
            },
            TextSpan::new(label),
            crate::hud::tf_bold(kit.fonts, size),
            TextColor(LINK_REST),
            Underline,
            UnderlineColor(Color::NONE),
            TextBackgroundColor(Color::NONE),
            Pickable::default(),
            PickingInteraction::default(),
            Press::Front(press),
        ))
        .id();
    commands.entity(root).add_children(&[icon_span, word_span]);
}

fn step_towards(current: f32, target: f32, step: f32, still: bool) -> f32 {
    if still || (current - target).abs() < 0.002 {
        target
    } else {
        let next = current + (target - current) * step;
        if (next - target).abs() < 0.002 {
            target
        } else {
            next
        }
    }
}

/// Animates hover colour, hover-only underline, and stationary click flash on
/// both standalone [`HyperlinkFace`] controls and inline [`InlineHyperlink`]
/// text spans, remaining 100% quiet at rest.
#[allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::too_many_lines
)]
pub(crate) fn animate_hyperlinks(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    focus: Option<Res<bevy::input_focus::InputFocus>>,
    focus_ring: Option<Res<bevy::input_focus::InputFocusVisible>>,
    clicks: Option<Res<Messages<bevy::picking::events::PointerClick>>>,
    mut click_cursor: Local<bevy::ecs::message::MessageCursor<bevy::picking::events::PointerClick>>,
    held: Query<&PickingInteraction>,
    mut faces: Query<(
        Entity,
        &mut HyperlinkFace,
        Option<&ChildOf>,
        &Children,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut parts: Query<
        (&HyperlinkPart, &mut TextColor, Option<&mut UnderlineColor>),
        Without<InlineHyperlink>,
    >,
    mut inlines: Query<(
        Entity,
        &mut InlineHyperlink,
        &PickingInteraction,
        &mut TextColor,
        Option<&mut UnderlineColor>,
        Option<&mut TextBackgroundColor>,
    )>,
) {
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let raw_dt = time.delta_secs();
    let dt = if raw_dt.is_finite() {
        raw_dt.clamp(0.0, 0.1)
    } else {
        0.0
    };
    let hover_step = 1.0 - (-18.0 * dt).exp();
    let decay = (-11.0 * dt).exp();
    let focused = focus
        .as_deref()
        .filter(|_| focus_ring.as_ref().is_some_and(|r| r.0))
        .and_then(bevy::input_focus::InputFocus::get);

    let mut clicked_entities: Vec<Entity> = Vec::new();
    if let Some(clicks) = clicks.as_deref() {
        for ev in click_cursor.read(clicks) {
            if ev.button == PointerButton::Primary {
                clicked_entities.push(ev.entity);
            }
        }
    }

    // 1. Standalone hyperlink chips/rows.
    for (entity, mut face, parent, children, mut bg, mut border) in &mut faces {
        let hit = parent.map_or(entity, ChildOf::parent);
        let pick = held.get(hit).ok();
        let pressed = matches!(pick, Some(PickingInteraction::Pressed));
        let hovered =
            pressed || matches!(pick, Some(PickingInteraction::Hovered)) || focused == Some(hit);
        let just_clicked = clicked_entities.contains(&hit) || clicked_entities.contains(&entity);

        let want_warmth = if hovered { 1.0 } else { 0.0 };
        let next_warmth = step_towards(face.warmth, want_warmth, hover_step, still);
        let next_pulse = if still {
            if pressed { 1.0 } else { 0.0 }
        } else if (pressed && !face.was_pressed) || just_clicked {
            1.0
        } else if pressed {
            face.pulse.max(0.75)
        } else if face.pulse > 0.0 {
            let p = face.pulse * decay;
            if p < 0.004 { 0.0 } else { p }
        } else {
            0.0
        };

        if face.painted
            && face.was_pressed == pressed
            && face.warmth.to_bits() == next_warmth.to_bits()
            && face.pulse.to_bits() == next_pulse.to_bits()
        {
            continue;
        }
        face.painted = true;
        face.was_pressed = pressed;
        face.warmth = next_warmth;
        face.pulse = next_pulse;

        let hover_bg = Color::srgba(0.14, 0.34, 0.46, 0.24 * next_warmth);
        let flash_bg = Color::srgba(0.28, 0.62, 0.78, 0.44);
        bg.set_if_neq(BackgroundColor(crate::ambience::blend(
            hover_bg, flash_bg, next_pulse,
        )));
        let hover_rim = LINK_REST.with_alpha(0.32 * next_warmth);
        let flash_rim = LINK_FLASH.with_alpha(0.72);
        border.set_if_neq(BorderColor::all(crate::ambience::blend(
            hover_rim, flash_rim, next_pulse,
        )));

        let base_ink = crate::ambience::blend(LINK_REST, LINK_HOT, next_warmth);
        let ink = crate::ambience::blend(base_ink, LINK_FLASH, next_pulse);
        let underline = ink.with_alpha(next_warmth.max(next_pulse));
        for child in children.iter() {
            if let Ok((part, mut tc, ul)) = parts.get_mut(child) {
                tc.set_if_neq(TextColor(ink));
                if part.role == SpanRole::Label
                    && let Some(mut ul) = ul
                {
                    ul.set_if_neq(UnderlineColor(underline));
                }
            }
        }
    }

    // 2. Inline prose hyperlink span groups (`[icon] Phrase`).
    if inlines.is_empty() {
        return;
    }
    let mut hot_groups: Vec<u32> = Vec::new();
    let mut pressed_groups: Vec<u32> = Vec::new();
    let mut clicked_groups: Vec<u32> = Vec::new();
    for (entity, link, pick, _, _, _) in &inlines {
        if *pick == PickingInteraction::Pressed {
            if !pressed_groups.contains(&link.group) {
                pressed_groups.push(link.group);
            }
            if !hot_groups.contains(&link.group) {
                hot_groups.push(link.group);
            }
        } else if *pick == PickingInteraction::Hovered && !hot_groups.contains(&link.group) {
            hot_groups.push(link.group);
        }
        if clicked_entities.contains(&entity) && !clicked_groups.contains(&link.group) {
            clicked_groups.push(link.group);
        }
    }

    for (_, mut link, _, mut tc, ul, tbg) in &mut inlines {
        let pressed = pressed_groups.contains(&link.group);
        let hovered = hot_groups.contains(&link.group);
        let just_clicked = clicked_groups.contains(&link.group);

        let want_warmth = if hovered { 1.0 } else { 0.0 };
        let next_warmth = step_towards(link.warmth, want_warmth, hover_step, still);
        let next_pulse = if still {
            if pressed { 1.0 } else { 0.0 }
        } else if (pressed && !link.was_pressed) || just_clicked {
            1.0
        } else if pressed {
            link.pulse.max(0.75)
        } else if link.pulse > 0.0 {
            let p = link.pulse * decay;
            if p < 0.004 { 0.0 } else { p }
        } else {
            0.0
        };

        if link.painted
            && link.was_pressed == pressed
            && link.warmth.to_bits() == next_warmth.to_bits()
            && link.pulse.to_bits() == next_pulse.to_bits()
        {
            continue;
        }
        link.painted = true;
        link.was_pressed = pressed;
        link.warmth = next_warmth;
        link.pulse = next_pulse;

        let base_ink = crate::ambience::blend(LINK_REST, LINK_HOT, next_warmth);
        let ink = crate::ambience::blend(base_ink, LINK_FLASH, next_pulse);
        tc.set_if_neq(TextColor(ink));
        if link.role == SpanRole::Label {
            let underline_alpha = next_warmth.max(next_pulse);
            if let Some(mut ul) = ul {
                ul.set_if_neq(UnderlineColor(ink.with_alpha(underline_alpha)));
            }
            if let Some(mut tbg) = tbg {
                let highlight_alpha = 0.14 * next_warmth + 0.24 * next_pulse;
                tbg.set_if_neq(TextBackgroundColor(if highlight_alpha > 0.002 {
                    Color::srgba(0.24, 0.62, 0.80, highlight_alpha.min(0.42))
                } else {
                    Color::NONE
                }));
            }
        }
    }
}

//! Styled hyperlinks for the front door's colophon and the About sheet:
//! leading single-character Font Awesome icon (verified against bundled
//! `fa-solid-900.ttf`), distinct sky-cerulean ink, underline shown only on
//! hover, stationary optical click flash, and real `Role::Hit` hit boxes so
//! every click always registers on the first press.

use crate::lobby::Press;
use crate::shellkit::controls::{self, Kit};
use crate::shellkit::metrics::px_fixed;
use crate::shellkit::role::Role;
use crate::shellkit::{surfaces, tokens};
use bevy::picking::hover::PickingInteraction;
use bevy::prelude::*;

use super::FrontPress;

/// Resting colour of every hyperlink icon and label: distinct from plain `INK`
/// and `MUTED`, with high readability against smoked glass and dialog surfaces.
pub(crate) const LINK_REST: Color = Color::srgb(0.44, 0.82, 0.96);

/// Hovered colour of hyperlink icons, labels, and hover underlines.
pub(crate) const LINK_HOT: Color = Color::srgb(0.78, 0.95, 1.0);

/// Peak click-flash colour of hyperlink icons and labels.
pub(crate) const LINK_FLASH: Color = Color::srgb(1.0, 0.93, 0.68);

/// Standard external-link glyph (`fa-up-right-from-square`, `U+F35D`),
/// verified non-`.notdef` in `assets/fonts/fa-solid-900.ttf`.
pub(crate) const EXTERNAL_ICON: char = '\u{f35d}';

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SpanRole {
    Icon,
    Label,
}

/// Marker on hyperlink child `Text` nodes (`Icon` or `Label`) driven by their
/// parent [`HyperlinkFace`]'s hover and click state.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct HyperlinkPart {
    pub(crate) role: SpanRole,
}

/// Marker on the visual face of every [`hyperlink`] control.
#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct HyperlinkFace {
    warmth: f32,
    pulse: f32,
    was_pressed: bool,
    painted: bool,
}

/// Marks a flex-wrapped prose sentence containing real [`hyperlink`] controls
/// so layout wraps words naturally and tests can inspect the full sentence.
#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct LinkedSentence;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Density {
    Regular,
    Compact,
    Inline,
    InlineTightRight,
}

/// Spawns a standalone styled hyperlink control: leading single-char icon in
/// `fa-solid-900.ttf`, sky-cerulean label in `Alegreya`, no underline at rest
/// (underlined only on hover), subtle hover pill tint, and stationary click
/// flash.
pub(crate) fn hyperlink(
    commands: &mut Commands,
    kit: Kit,
    icon: char,
    text: &str,
    compact: bool,
    action: impl Bundle,
) -> Entity {
    let size = surfaces::whole_pixels(kit.m.small);
    spawn_hyperlink(
        commands,
        kit,
        icon,
        text,
        size,
        if compact {
            Density::Compact
        } else {
            Density::Regular
        },
        action,
    )
}

fn spawn_hyperlink(
    commands: &mut Commands,
    kit: Kit,
    icon: char,
    text: &str,
    font_size: f32,
    density: Density,
    action: impl Bundle,
) -> Entity {
    let (padding, min_h, gap) = match density {
        Density::Regular => (
            UiRect::axes(kit.m.px(10.0), kit.m.px(3.0)),
            kit.m.control * 0.76,
            5.0,
        ),
        Density::Compact => (UiRect::axes(kit.m.px(7.0), kit.m.px(2.0)), 24.0, 5.0),
        Density::Inline => (UiRect::axes(kit.m.px(3.0), px_fixed(1.0)), 20.0, 4.0),
        Density::InlineTightRight => (
            UiRect {
                left: kit.m.px(3.0),
                right: px_fixed(1.0),
                top: px_fixed(1.0),
                bottom: px_fixed(1.0),
            },
            20.0,
            4.0,
        ),
    };
    let face = commands
        .spawn((
            HyperlinkFace::default(),
            Node {
                padding,
                min_height: px_fixed(min_h),
                column_gap: kit.m.px(gap),
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
    // Strictly a single icon codepoint in `icon_tf` (`fa-solid-900.ttf`) —
    // never attach spaces or punctuation here, as `fa-solid` maps no spaces
    // outside ASCII and would draw a `.notdef` tofu box (`☒`).
    let mark = commands
        .spawn((
            HyperlinkPart {
                role: SpanRole::Icon,
            },
            Text::new(icon.to_string()),
            crate::hud::icon_tf(kit.fonts, font_size * 0.80),
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
            crate::hud::tf_bold(kit.fonts, font_size),
            TextColor(LINK_REST),
            Underline,
            UnderlineColor(Color::NONE),
            TextLayout::no_wrap(),
            bevy::ui::LayoutConfig {
                use_rounding: false,
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(face).add_children(&[mark, words]);

    if density != Density::Regular && !kit.m.touch() {
        let hit_h = match density {
            Density::Compact => 26.0,
            Density::Inline | Density::InlineTightRight => 22.0,
            Density::Regular => kit.m.hit,
        };
        let wrapper = commands
            .spawn((
                Role::Hit,
                PickingInteraction::default(),
                Pickable::default(),
                Node {
                    min_height: px_fixed(hit_h),
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

/// Spawns a flex-wrapped prose sentence whose `targets` substrings are real
/// `Role::Hit` [`hyperlink`] controls (`[icon] Target`) — zero `TextSpan`
/// font-mixing, zero `.notdef` boxes, and reliable single-click activation
/// even when a text field currently holds keyboard focus.
pub(crate) fn prose_with_inline_links(
    commands: &mut Commands,
    kit: Kit,
    sentence: &str,
    targets: &[(&str, FrontPress)],
    justify: Justify,
    small: bool,
) -> Entity {
    let size = surfaces::whole_pixels(if small { kit.m.small } else { kit.m.text });
    let justify_content = match justify {
        Justify::Center => JustifyContent::Center,
        Justify::Right => JustifyContent::FlexEnd,
        _ => JustifyContent::FlexStart,
    };
    let row = commands
        .spawn((
            LinkedSentence,
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
                justify_content,
                row_gap: kit.m.px(2.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    let segments = split_links(sentence, targets);
    for seg in segments {
        match seg {
            Segment::Plain(text) => {
                for word_chunk in split_keeping_spaces(text) {
                    let node = commands
                        .spawn((
                            Text::new(word_chunk),
                            crate::hud::tf(kit.fonts, size),
                            TextColor(tokens::INK),
                            TextLayout::no_wrap(),
                            bevy::ui::LayoutConfig {
                                use_rounding: false,
                            },
                            Pickable::IGNORE,
                        ))
                        .id();
                    commands.entity(row).add_child(node);
                }
            }
            Segment::Link {
                label,
                press,
                trailing_punct,
            } => {
                let link_node = spawn_hyperlink(
                    commands,
                    kit,
                    EXTERNAL_ICON,
                    label,
                    size,
                    if trailing_punct.is_empty() {
                        Density::Inline
                    } else {
                        Density::InlineTightRight
                    },
                    Press::Front(press),
                );
                if trailing_punct.is_empty() {
                    commands.entity(row).add_child(link_node);
                } else {
                    // Keep trailing punctuation (e.g. `. `) glued to the link
                    // so a period never wraps onto the next line alone.
                    let glue = commands
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                flex_shrink: 0.0,
                                ..default()
                            },
                            Pickable::IGNORE,
                        ))
                        .id();
                    let punct = commands
                        .spawn((
                            Text::new(trailing_punct),
                            crate::hud::tf(kit.fonts, size),
                            TextColor(tokens::INK),
                            TextLayout::no_wrap(),
                            bevy::ui::LayoutConfig {
                                use_rounding: false,
                            },
                            Pickable::IGNORE,
                        ))
                        .id();
                    commands.entity(glue).add_children(&[link_node, punct]);
                    commands.entity(row).add_child(glue);
                }
            }
        }
    }
    row
}

enum Segment<'a> {
    Plain(&'a str),
    Link {
        label: &'a str,
        press: FrontPress,
        trailing_punct: &'a str,
    },
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
        let after = &rest[idx + needle.len()..];
        let mut punct_bytes = 0;
        for ch in after.chars() {
            if matches!(ch, '.' | ',' | ';' | ':' | '!' | '?') {
                punct_bytes += ch.len_utf8();
            } else if ch == ' ' && punct_bytes > 0 {
                punct_bytes += 1;
                break;
            } else {
                break;
            }
        }
        out.push(Segment::Link {
            label: needle,
            press,
            trailing_punct: &after[..punct_bytes],
        });
        rest = &after[punct_bytes..];
    }
    out
}

/// Splits plain text into word-and-trailing-space tokens so a flex-wrap row
/// wraps cleanly at word boundaries while preserving exact characters.
fn split_keeping_spaces(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        while i < bytes.len() && bytes[i] == b' ' {
            i += 1;
        }
        while i < bytes.len() && bytes[i] != b' ' {
            i += 1;
        }
        while i < bytes.len() && bytes[i] == b' ' {
            i += 1;
        }
        out.push(&text[start..i]);
    }
    out
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
/// every [`HyperlinkFace`] control, remaining 100% quiet at rest.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
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
    mut parts: Query<(&HyperlinkPart, &mut TextColor, Option<&mut UnderlineColor>)>,
) {
    if faces.is_empty() {
        return;
    }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use swash::FontRef;

    #[test]
    fn every_front_door_and_about_icon_maps_to_a_real_glyph_in_fontawesome() {
        let bytes = include_bytes!("../../../assets/fonts/fa-solid-900.ttf");
        let font = FontRef::from_index(bytes, 0).expect("valid fa-solid-900.ttf");
        let charmap = font.charmap();
        for glyph in [
            EXTERNAL_ICON,
            '\u{f05a}', // circle-info
            '\u{f126}', // code-branch
            '\u{f02b}', // tag
            '\u{f2f6}', // right-to-bracket (Sign in)
            '\u{f234}', // user-plus
            '\u{f007}', // user
            '\u{f084}', // key
            '\u{f24e}', // scale-balanced
            '\u{f121}', // code
            '\u{f1b3}', // cubes
            '\u{f031}', // font
            '\u{f53f}', // palette
            '\u{f5fd}', // layer-group
            '\u{f1b2}', // cube
            '\u{f0ec}', // right-left
            '\u{f3ed}', // shield-halved
            '\u{f001}', // music
            '\u{f1eb}', // wifi / network
            '\u{f252}', // hourglass-half
            '\u{f054}', // chevron-right
            '\u{f00c}', // check
            '\u{f00d}', // xmark
        ] {
            assert_ne!(
                charmap.map(glyph),
                0,
                "Font Awesome glyph U+{:04X} ({glyph:?}) is missing (.notdef tofu box!)",
                glyph as u32
            );
        }
        // Confirm why U+2009 caused `☒`: `fa-solid-900.ttf` has no Thin Space glyph!
        assert_eq!(charmap.map('\u{2009}'), 0);
    }
}

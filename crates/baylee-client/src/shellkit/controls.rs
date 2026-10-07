//! The shell's controls (the shell design, §2.4): buttons, the hit area,
//! pills, chips, tabs, segmented controls, toggles, steppers, sliders, the
//! search field's look and key caps.
//!
//! Every maker takes the action as a bundle (`impl Bundle`): the screen puts
//! its own `Press` on the control, the gallery puts nothing. The action
//! always lands on the **hit wrapper**, never on the visual inside it: under
//! Touch the wrapper is 44 × 44 at the least, whatever the visual measures
//! (a chip 40, a pip 36, a checkbox 28; S4-2), and it is the wrapper the
//! 44-px acceptance measures (`Role::Hit`).

use super::metrics::{ShellMetrics, px_fixed};
use super::role::Role;
use super::tokens::{self, RADIUS_CONTROL, RADIUS_PILL};
use crate::hud::{UiFonts, icon_tf, tf, tf_bold};
use bevy::picking::hover::PickingInteraction;
use bevy::prelude::*;

/// What every maker needs: the fonts, the sizes, and whether the labels are
/// German (which takes 10 px of side padding instead of 12, §2.3).
#[derive(Clone, Copy)]
pub struct Kit<'a> {
    /// The interface's faces.
    pub fonts: &'a UiFonts,
    /// The sizes.
    pub m: ShellMetrics,
    /// Whether the interface speaks German.
    pub german: bool,
}

/// A button's weight (§2.4): the blue face for the one primary action, gold
/// only for Return to your game.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Weight {
    /// The screen's one primary action: the blue face.
    Primary,
    /// Return to your game, and nothing else.
    Gold,
    /// Every other action.
    Secondary,
    /// Text only, for the least of actions.
    Ghost,
    /// Delete, and Leave while hosting.
    Danger,
}

impl Weight {
    fn ground(self) -> Color {
        match self {
            Self::Primary => tokens::PRIMARY,
            Self::Gold => tokens::GOLD,
            Self::Secondary => tokens::CONTROL,
            Self::Ghost => Color::NONE,
            Self::Danger => tokens::DANGER,
        }
    }

    fn rim(self) -> Color {
        match self {
            Self::Primary => tokens::PRIMARY_EDGE,
            Self::Gold | Self::Danger => self.ground(),
            Self::Secondary => tokens::BORDER,
            Self::Ghost => Color::NONE,
        }
    }

    fn ink(self) -> Color {
        match self {
            // Dark ink on the two light faces.
            Self::Gold | Self::Danger => Color::srgb(0.06, 0.05, 0.04),
            Self::Primary | Self::Secondary | Self::Ghost => tokens::INK,
        }
    }
}

/// Whether a control works, and if not, why not (§2.4: a disabled control
/// always says why — a reason line under Touch, a tooltip under a pointer).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Live<'a> {
    /// It works.
    Yes,
    /// It does not, for this reason.
    No(&'a str),
}

/// A control that is drawn but off: it takes focus and shows its reason,
/// and nothing acts on it.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Disabled;

/// The hover text of a disabled control under a pointer.
#[derive(Component)]
pub struct Tooltip(pub Entity);

/// Wraps `visual` in a hit area at least [`ShellMetrics::hit`] square, and
/// puts the action on the wrapper.
pub fn hit(commands: &mut Commands, kit: Kit, visual: Entity, action: impl Bundle) -> Entity {
    let wrapper = commands
        .spawn((
            Role::Hit,
            Node {
                min_width: px_fixed(kit.m.hit),
                min_height: px_fixed(kit.m.hit),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_shrink: 0.0,
                ..default()
            },
            action,
        ))
        .id();
    commands.entity(visual).insert(Pickable::IGNORE);
    commands.entity(wrapper).add_child(visual);
    wrapper
}

/// A text label, `Pickable::IGNORE` like every label inside a control.
pub fn label(commands: &mut Commands, kit: Kit, text: &str, size: f32, ink: Color) -> Entity {
    commands
        .spawn((
            Text::new(text),
            tf(kit.fonts, size),
            TextColor(ink),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id()
}

/// A key cap, drawn only under a pointer (`KEYBOARD.md` §4.2: no key caps
/// under Touch). `None` under Touch.
pub fn key_cap(commands: &mut Commands, kit: Kit, keys: &str) -> Option<Entity> {
    if kit.m.touch() {
        return None;
    }
    let text = label(commands, kit, keys, kit.m.small, tokens::INK);
    let cap = commands
        .spawn((
            Role::KeyCap,
            Node {
                padding: UiRect::axes(kit.m.px(5.0), px_fixed(1.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(4.0)),
                margin: UiRect::left(kit.m.px(8.0)),
                ..default()
            },
            BorderColor::all(tokens::BORDER),
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.04)),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(cap).add_child(text);
    Some(cap)
}

/// A button: label, optional key cap, a disabled state that says why.
///
/// Returns the outermost node: the hit wrapper, or under Touch a column of
/// the wrapper and its reason line when disabled.
pub fn button(
    commands: &mut Commands,
    kit: Kit,
    text: &str,
    weight: Weight,
    live: Live,
    keys: Option<&str>,
    action: impl Bundle,
) -> Entity {
    shaped(commands, kit, text, weight, live, keys, action, false)
}

/// A [`button`] as wide as its row: a form's submit (the front door's Sign
/// in), its face filling the hit wrapper.
pub fn wide_button(
    commands: &mut Commands,
    kit: Kit,
    text: &str,
    weight: Weight,
    live: Live,
    keys: Option<&str>,
    action: impl Bundle,
) -> Entity {
    shaped(commands, kit, text, weight, live, keys, action, true)
}

#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)] // the two makers' one body
fn shaped(
    commands: &mut Commands,
    kit: Kit,
    text: &str,
    weight: Weight,
    live: Live,
    keys: Option<&str>,
    action: impl Bundle,
    wide: bool,
) -> Entity {
    let side = if kit.german { 10.0 } else { 12.0 };
    let dead = matches!(live, Live::No(_));
    let ink = if dead { tokens::DISABLED } else { weight.ink() };
    let ground = if dead {
        tokens::CONTROL.with_alpha(0.6)
    } else {
        weight.ground()
    };
    let face = commands
        .spawn((
            Role::Button,
            Node {
                min_height: px_fixed(kit.m.control),
                // The face fills its hit area: as wide as its words in a
                // wrapper of its own size, the whole width where a screen
                // stretches the wrapper (a full-width primary).
                flex_grow: 1.0,
                padding: UiRect::axes(kit.m.px(side), px_fixed(0.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(ground),
            BorderColor::all(if dead { tokens::BORDER } else { weight.rim() }),
        ))
        .id();
    let words = commands
        .spawn((
            Text::new(text),
            tf_bold(kit.fonts, kit.m.text),
            TextColor(ink),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(face).add_child(words);
    if !dead
        && let Some(keys) = keys
        && let Some(cap) = key_cap(commands, kit, keys)
    {
        commands.entity(face).add_child(cap);
    }
    let widen = |commands: &mut Commands, wrapper: Entity| {
        if wide {
            commands
                .entity(wrapper)
                .entry::<Node>()
                .and_modify(|mut node| node.width = Val::Percent(100.0));
        }
        wrapper
    };
    if !dead {
        commands
            .entity(face)
            .insert(crate::ambience::Feel::new(ground));
        let wrapper = hit(commands, kit, face, action);
        return widen(commands, wrapper);
    }
    // A dead button keeps its action bundle, so it stays a focus stop: a
    // disabled control is focusable so its reason can be read (APG). What
    // reads an action skips a wrapper marked [`Disabled`].
    let wrapper = hit(commands, kit, face, (action, Disabled));
    let wrapper = widen(commands, wrapper);
    let Live::No(reason) = live else {
        return wrapper;
    };
    if kit.m.touch() {
        let column = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Start,
                    row_gap: kit.m.px(2.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let why = commands
            .spawn((
                Text::new(reason),
                tf(kit.fonts, kit.m.small),
                TextColor(tokens::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(column).add_children(&[wrapper, why]);
        return column;
    }
    let tip = tooltip(commands, kit, reason);
    commands.entity(wrapper).add_child(tip).insert(Tooltip(tip));
    wrapper
}

/// A tooltip's box, hidden until the pointer rests on its owner.
fn tooltip(commands: &mut Commands, kit: Kit, text: &str) -> Entity {
    let words = label(commands, kit, text, kit.m.small, tokens::INK);
    let tip = commands
        .spawn((
            Role::Opaque,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(100.0),
                left: px_fixed(0.0),
                margin: UiRect::top(kit.m.px(4.0)),
                padding: UiRect::axes(kit.m.px(8.0), kit.m.px(4.0)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::OPAQUE),
            GlobalZIndex(tokens::z::POPOVER),
            Visibility::Hidden,
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(tip).add_child(words);
    tip
}

/// Shows a tooltip while the pointer rests on its owner.
fn show_tooltips(
    owners: Query<(&Tooltip, &PickingInteraction), Changed<PickingInteraction>>,
    mut tips: Query<&mut Visibility>,
) {
    for (tip, interaction) in &owners {
        if let Ok(mut shown) = tips.get_mut(tip.0) {
            shown.set_if_neq(if *interaction == PickingInteraction::None {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            });
        }
    }
}

/// A header pill: a dot, a line of text, a caret (§2.4).
pub fn pill(
    commands: &mut Commands,
    kit: Kit,
    dot: Option<Color>,
    text: &str,
    caret: bool,
    action: impl Bundle,
) -> Entity {
    let face = commands
        .spawn((
            Role::Pill,
            Node {
                min_height: kit.m.px(36.0),
                padding: UiRect::axes(kit.m.px(12.0), px_fixed(0.0)),
                column_gap: kit.m.px(8.0),
                align_items: AlignItems::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL),
            BorderColor::all(tokens::BORDER),
        ))
        .id();
    if let Some(colour) = dot {
        let disc = commands
            .spawn((
                Node {
                    width: kit.m.px(8.0),
                    height: kit.m.px(8.0),
                    border_radius: BorderRadius::all(px_fixed(RADIUS_PILL)),
                    ..default()
                },
                BackgroundColor(colour),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(face).add_child(disc);
    }
    let words = label(commands, kit, text, kit.m.small, tokens::INK);
    commands.entity(face).add_child(words);
    if caret {
        let mark = commands
            .spawn((
                Text::new("\u{f078}"),
                icon_tf(kit.fonts, kit.m.small * 0.7),
                TextColor(tokens::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(face).add_child(mark);
    }
    hit(commands, kit, face, action)
}

/// A navigation pill in the header: filled when it is the screen shown.
pub fn nav(
    commands: &mut Commands,
    kit: Kit,
    text: &str,
    active: bool,
    action: impl Bundle,
) -> Entity {
    let face = commands
        .spawn((
            Role::Nav,
            Node {
                min_height: kit.m.px(34.0),
                padding: UiRect::axes(kit.m.px(14.0), px_fixed(0.0)),
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(px_fixed(RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(if active {
                tokens::SELECTED
            } else {
                Color::NONE
            }),
        ))
        .id();
    let words = label(
        commands,
        kit,
        text,
        kit.m.text,
        if active { tokens::INK } else { tokens::MUTED },
    );
    commands.entity(face).add_child(words);
    hit(commands, kit, face, action)
}

/// A filter chip: on or off, an optional count, an optional `×`.
pub fn chip(
    commands: &mut Commands,
    kit: Kit,
    text: &str,
    on: bool,
    count: Option<u32>,
    removable: bool,
    action: impl Bundle,
) -> Entity {
    // The visual is 40 under a finger and 32 under a pointer (S4-2); the
    // wrapper is 44 either way under Touch.
    let height = if kit.m.touch() {
        40.0
    } else {
        kit.m.scaled(32.0)
    };
    let face = commands
        .spawn((
            Role::Chip,
            Node {
                min_height: px_fixed(height),
                padding: UiRect::axes(kit.m.px(12.0), px_fixed(0.0)),
                column_gap: kit.m.px(6.0),
                align_items: AlignItems::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(if on {
                tokens::SELECTED
            } else {
                tokens::CONTROL
            }),
            BorderColor::all(if on { tokens::ACCENT } else { tokens::BORDER }),
        ))
        .id();
    let words = label(commands, kit, text, kit.m.small, tokens::INK);
    commands.entity(face).add_child(words);
    if let Some(n) = count {
        let badge = label(commands, kit, &n.to_string(), kit.m.small, tokens::MUTED);
        commands.entity(badge).insert(Role::Count);
        commands.entity(face).add_child(badge);
    }
    if removable {
        let mark = commands
            .spawn((
                Text::new("\u{f00d}"),
                icon_tf(kit.fonts, kit.m.small * 0.8),
                TextColor(tokens::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(face).add_child(mark);
    }
    hit(commands, kit, face, action)
}

/// A row of tabs with an underline under the selected one; counts after
/// the label. `action(i)` is tab `i`'s action.
pub fn tabs<B: Bundle>(
    commands: &mut Commands,
    kit: Kit,
    items: &[(&str, Option<u32>)],
    selected: usize,
    action: impl Fn(usize) -> B,
) -> Entity {
    let bar = commands
        .spawn((
            Node {
                column_gap: kit.m.px(16.0),
                border: UiRect::bottom(px_fixed(1.0)),
                ..default()
            },
            BorderColor::all(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id();
    for (i, (text, count)) in items.iter().enumerate() {
        let on = i == selected;
        let face = commands
            .spawn((
                Role::Tab,
                Node {
                    min_height: px_fixed(kit.m.control),
                    column_gap: kit.m.px(6.0),
                    align_items: AlignItems::Center,
                    border: UiRect::bottom(px_fixed(2.0)),
                    ..default()
                },
                BorderColor::all(if on { tokens::ACCENT } else { Color::NONE }),
            ))
            .id();
        let words = label(
            commands,
            kit,
            text,
            kit.m.text,
            if on { tokens::INK } else { tokens::MUTED },
        );
        commands.entity(face).add_child(words);
        if let Some(n) = count {
            let badge = label(commands, kit, &n.to_string(), kit.m.small, tokens::MUTED);
            commands.entity(badge).insert(Role::Count);
            commands.entity(face).add_child(badge);
        }
        let wrapper = hit(commands, kit, face, action(i));
        commands.entity(bar).add_child(wrapper);
    }
    bar
}

/// A segmented control of two to eight options; the chosen one filled.
pub fn segmented<B: Bundle>(
    commands: &mut Commands,
    kit: Kit,
    items: &[&str],
    selected: usize,
    action: impl Fn(usize) -> B,
) -> Entity {
    let group = commands
        .spawn((
            Node {
                padding: UiRect::all(px_fixed(2.0)),
                column_gap: px_fixed(2.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL),
            BorderColor::all(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id();
    for (i, text) in items.iter().enumerate() {
        let on = i == selected;
        let face = commands
            .spawn((
                Role::Segment,
                Node {
                    min_height: px_fixed(if kit.m.touch() {
                        34.0
                    } else {
                        kit.m.scaled(30.0)
                    }),
                    padding: UiRect::axes(kit.m.px(12.0), px_fixed(0.0)),
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::all(px_fixed(RADIUS_CONTROL - 2.0)),
                    ..default()
                },
                BackgroundColor(if on { tokens::SELECTED } else { Color::NONE }),
            ))
            .id();
        let words = label(
            commands,
            kit,
            text,
            kit.m.small,
            if on { tokens::INK } else { tokens::MUTED },
        );
        commands.entity(face).add_child(words);
        let wrapper = hit(commands, kit, face, action(i));
        commands.entity(group).add_child(wrapper);
    }
    group
}

/// A toggle, 44 × 24 (§2.4).
pub fn toggle(commands: &mut Commands, kit: Kit, on: bool, action: impl Bundle) -> Entity {
    let track = commands
        .spawn((
            Role::Toggle,
            Node {
                width: px_fixed(44.0),
                height: px_fixed(24.0),
                padding: UiRect::all(px_fixed(3.0)),
                justify_content: if on {
                    JustifyContent::FlexEnd
                } else {
                    JustifyContent::FlexStart
                },
                border_radius: BorderRadius::all(px_fixed(RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(if on { tokens::ACCENT } else { tokens::CONTROL }),
            BorderColor::all(tokens::BORDER),
        ))
        .id();
    let knob = commands
        .spawn((
            Node {
                width: px_fixed(18.0),
                height: px_fixed(18.0),
                border_radius: BorderRadius::all(px_fixed(RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(track).add_child(knob);
    hit(commands, kit, track, action)
}

/// A stepper: `− n +`. On a phone it stands in for an eight-way segmented
/// control (Players, §5).
pub fn stepper(
    commands: &mut Commands,
    kit: Kit,
    value: &str,
    less: impl Bundle,
    more: impl Bundle,
) -> Entity {
    let group = commands
        .spawn((
            Role::Stepper,
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(4.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let minus = button(
        commands,
        kit,
        "\u{2212}",
        Weight::Secondary,
        Live::Yes,
        None,
        less,
    );
    let shown = commands
        .spawn((
            Text::new(value),
            tf_bold(kit.fonts, kit.m.text),
            TextColor(tokens::INK),
            Node {
                min_width: kit.m.px(32.0),
                ..default()
            },
            TextLayout::justify(Justify::Center),
            Pickable::IGNORE,
        ))
        .id();
    let plus = button(commands, kit, "+", Weight::Secondary, Live::Yes, None, more);
    commands.entity(group).add_children(&[minus, shown, plus]);
    group
}

/// A slider's state: 0 to 100. A press or a drag on the track sets it;
/// the screen reads it back (`Changed<Slider>`).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Slider {
    /// The value, 0 to 100.
    pub value: u8,
    fill: Entity,
    knob: Entity,
    readout: Entity,
}

/// A slider: 0–100, a value label, a 44-tall track and a 20-px knob (§2.4).
pub fn slider(commands: &mut Commands, kit: Kit, value: u8, action: impl Bundle) -> Entity {
    let value = value.min(100);
    let row = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(12.0),
                width: Val::Percent(100.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let rail = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: px_fixed(4.0),
                border_radius: BorderRadius::all(px_fixed(RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL),
            Pickable::IGNORE,
        ))
        .id();
    let fill = commands
        .spawn((
            Node {
                width: Val::Percent(f32::from(value)),
                height: Val::Percent(100.0),
                border_radius: BorderRadius::all(px_fixed(RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(tokens::ACCENT),
            Pickable::IGNORE,
        ))
        .id();
    let knob = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(f32::from(value)),
                width: px_fixed(20.0),
                height: px_fixed(20.0),
                margin: UiRect::left(px_fixed(-10.0)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(rail).add_child(fill);
    let readout = label(
        commands,
        kit,
        &format!("{value} %"),
        kit.m.small,
        tokens::INK,
    );
    commands.entity(readout).insert(Node {
        min_width: kit.m.px(44.0),
        ..default()
    });
    let track = commands
        .spawn((
            Role::Slider,
            Node {
                flex_grow: 1.0,
                min_height: px_fixed(44.0),
                align_items: AlignItems::Center,
                ..default()
            },
            Slider {
                value,
                fill,
                knob,
                readout,
            },
            action,
        ))
        .id();
    commands.entity(track).add_children(&[rail, knob]);
    commands
        .entity(track)
        .observe(slide_on_press)
        .observe(slide_on_drag);
    commands.entity(row).add_children(&[track, readout]);
    row
}

/// The value a pointer at `x` (logical) means on a track spanning
/// `left..left + width`: 0 to 100, clamped.
#[must_use]
pub fn slider_value_at(x: f32, left: f32, width: f32) -> u8 {
    if width <= 0.0 {
        return 0;
    }
    let share = ((x - left) / width).clamp(0.0, 1.0);
    // In 0..=100 by the clamp above.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let value = (share * 100.0).round() as u8;
    value
}

fn slide_to(
    entity: Entity,
    x: f32,
    sliders: &mut Query<(&mut Slider, &ComputedNode, &UiGlobalTransform)>,
) {
    let Ok((mut slider, node, place)) = sliders.get_mut(entity) else {
        return;
    };
    let scale = node.inverse_scale_factor;
    let width = node.size().x * scale;
    let left = place.translation.x * scale - width / 2.0;
    let value = slider_value_at(x, left, width);
    if slider.value != value {
        slider.value = value;
    }
}

fn slide_on_press(
    press: On<Pointer<Press>>,
    mut sliders: Query<(&mut Slider, &ComputedNode, &UiGlobalTransform)>,
) {
    slide_to(
        press.entity,
        press.pointer_location.position.x,
        &mut sliders,
    );
}

fn slide_on_drag(
    drag: On<Pointer<Drag>>,
    mut sliders: Query<(&mut Slider, &ComputedNode, &UiGlobalTransform)>,
) {
    slide_to(drag.entity, drag.pointer_location.position.x, &mut sliders);
}

/// Draws a slider's value where it changed: the fill, the knob, the readout.
fn draw_sliders(
    sliders: Query<&Slider, Changed<Slider>>,
    mut nodes: Query<&mut Node>,
    mut texts: Query<&mut Text>,
) {
    for slider in &sliders {
        let share = Val::Percent(f32::from(slider.value));
        if let Ok(mut fill) = nodes.get_mut(slider.fill) {
            fill.width = share;
        }
        if let Ok(mut knob) = nodes.get_mut(slider.knob) {
            knob.left = share;
        }
        if let Ok(mut text) = texts.get_mut(slider.readout) {
            text.0 = format!("{} %", slider.value);
        }
    }
}

/// A search field's look: the magnifier, the text or its hint, the `/` cap
/// under a pointer, a clear `×` once something is typed. The typing itself
/// is the lobby's field (`lobby::text_field`); this is the shell's face.
pub fn search(
    commands: &mut Commands,
    kit: Kit,
    text: &str,
    hint: &str,
    action: impl Bundle,
) -> Entity {
    let face = commands
        .spawn((
            Role::Field,
            Node {
                min_height: px_fixed(kit.m.control),
                width: Val::Percent(100.0),
                padding: UiRect::axes(kit.m.px(12.0), px_fixed(0.0)),
                column_gap: kit.m.px(8.0),
                align_items: AlignItems::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_CONTROL)),
                overflow: Overflow::clip_x(),
                ..default()
            },
            BackgroundColor(tokens::CONTROL),
            BorderColor::all(tokens::BORDER),
            action,
        ))
        .id();
    let glass = commands
        .spawn((
            Text::new("\u{f002}"),
            icon_tf(kit.fonts, kit.m.small),
            TextColor(tokens::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    let (shown, ink) = if text.is_empty() {
        (hint, tokens::MUTED)
    } else {
        (text, tokens::INK)
    };
    let words = label(commands, kit, shown, kit.m.text, ink);
    commands.entity(words).insert((
        super::focus::FieldWords,
        Node {
            flex_grow: 1.0,
            min_width: px_fixed(0.0),
            ..default()
        },
    ));
    commands.entity(face).add_children(&[glass, words]);
    if text.is_empty() {
        if let Some(cap) = key_cap(commands, kit, "/") {
            commands.entity(face).add_child(cap);
        }
    } else {
        let clear = commands
            .spawn((
                Text::new("\u{f00d}"),
                icon_tf(kit.fonts, kit.m.small),
                TextColor(tokens::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(face).add_child(clear);
    }
    face
}

/// The controls' own systems.
pub(super) fn install(app: &mut App) {
    app.add_systems(Update, (show_tooltips, draw_sliders));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slider_reads_the_pointer_across_its_track_and_clamps() {
        assert_eq!(slider_value_at(100.0, 100.0, 200.0), 0);
        assert_eq!(slider_value_at(200.0, 100.0, 200.0), 50);
        assert_eq!(slider_value_at(300.0, 100.0, 200.0), 100);
        assert_eq!(slider_value_at(-50.0, 100.0, 200.0), 0);
        assert_eq!(slider_value_at(999.0, 100.0, 200.0), 100);
        assert_eq!(slider_value_at(5.0, 0.0, 0.0), 0);
    }
}

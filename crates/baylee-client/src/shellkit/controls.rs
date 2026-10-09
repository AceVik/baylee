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
            Self::Gold | Self::Danger => tokens::INK_ON_LIGHT,
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

/// The air between a label and the key cap after it: ten pixels at the
/// default step, never under eight (owner, 09.10.2026: a cap that touches
/// its word reads as part of the word).
#[must_use]
pub fn cap_gap(kit: Kit) -> f32 {
    kit.m.scaled(10.0).max(8.0)
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
                margin: UiRect::left(px_fixed(cap_gap(kit))),
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
        // A ghost has no ground to lighten: it rises out of nothing to the
        // hover ground, as a tab does.
        let feel = if ground.alpha() < 0.01 {
            crate::ambience::Feel::rising_to(ground, tokens::HOVER)
        } else {
            crate::ambience::Feel::new(ground)
        };
        commands.entity(face).insert(feel);
        if weight == Weight::Primary {
            commands.entity(face).insert(super::sheen::Sheen);
        }
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
            crate::ambience::Feel::new(tokens::CONTROL),
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
            if active {
                crate::ambience::Feel::new(tokens::SELECTED)
            } else {
                crate::ambience::Feel::rising_to(Color::NONE, tokens::HOVER)
            },
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
            crate::ambience::Feel::new(if on {
                tokens::SELECTED
            } else {
                tokens::CONTROL
            }),
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
                BackgroundColor(Color::NONE),
                crate::ambience::Feel::tinting_to(Color::NONE, tokens::HOVER),
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

/// How much a segment grows under the pointer and gives under a press:
/// less than a button's, as it stands shoulder to shoulder with others.
const SEGMENT_LIFT: f32 = 0.03;

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
                // Wider than its row (four German camera choices at 640 px,
                // beta.6 QA), its segments wrap rather than leave the window.
                flex_wrap: FlexWrap::Wrap,
                row_gap: px_fixed(2.0),
                max_width: Val::Percent(100.0),
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
                    padding: UiRect::axes(kit.m.px(14.0), px_fixed(0.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: BorderRadius::all(px_fixed(RADIUS_CONTROL - 2.0)),
                    ..default()
                },
                BackgroundColor(if on { tokens::SELECTED } else { Color::NONE }),
                // A segment answers the pointer as a button does: it lights,
                // and gives a little under a press (owner, 09.10.2026).
                if on {
                    crate::ambience::Feel::lifting(
                        tokens::SELECTED,
                        crate::ambience::lighter(tokens::SELECTED, 0.12),
                        SEGMENT_LIFT,
                    )
                } else {
                    crate::ambience::Feel::lifting(Color::NONE, tokens::HOVER, SEGMENT_LIFT)
                },
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

/// A toggle's size: the track, the knob, and the air round the knob.
const TRACK_W: f32 = 44.0;
const TRACK_H: f32 = 24.0;
const KNOB: f32 = 18.0;
const KNOB_AIR: f32 = 3.0;

/// How fast a toggle's knob crosses and its track changes colour: about
/// 180 ms to settle, eased out (owner, 09.10.2026: "a nice on/off switching
/// animation").
pub(crate) const TOGGLE_RATE: f32 = 22.0;

/// A toggle on its way between off and on.
///
/// The screen is rebuilt from the setting, so a press draws a **new**
/// toggle already standing at its end. What the toggle showed is therefore
/// kept by its focus stop ([`ToggleShown`]), and a toggle drawn again under
/// the same stop starts where the last one stood and slides on from there.
#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub struct ToggleMotion {
    /// What the setting says.
    pub on: bool,
    /// Where the knob stands, 0 (off) to 1 (on); `None` until the first frame
    /// has read the stop's memory.
    pub shown: Option<f32>,
    /// How far the pointer warms the track, -1 (pressed) to 1 (hovered).
    pub warmth: f32,
    knob: Entity,
}

/// Where each stop's toggle last stood, so a rebuilt one carries on.
#[derive(Resource, Default, Debug)]
pub struct ToggleShown(pub std::collections::HashMap<super::focus::Stop, f32>);

/// The knob's left edge at `shown` (0 off, 1 on), inside the track's border.
fn knob_left(shown: f32) -> f32 {
    KNOB_AIR + (TRACK_W - 2.0 - KNOB - 2.0 * KNOB_AIR) * shown
}

/// The track's ground at `shown`, warmed by the pointer.
fn track_ground(shown: f32, warmth: f32) -> Color {
    let ground = crate::ambience::blend(tokens::TRACK_OFF, tokens::ACCENT, shown);
    if warmth >= 0.0 {
        crate::ambience::lighter(ground, 0.14 * warmth)
    } else {
        crate::ambience::lighter(ground, 0.08 * warmth)
    }
}

/// A toggle, 44 × 24 (§2.4): a knob that slides and a track whose colour
/// eases between off and on, both at once ([`animate_toggles`]).
pub fn toggle(commands: &mut Commands, kit: Kit, on: bool, action: impl Bundle) -> Entity {
    let at = f32::from(u8::from(on));
    let track = commands
        .spawn((
            Role::Toggle,
            Node {
                width: px_fixed(TRACK_W),
                height: px_fixed(TRACK_H),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(track_ground(at, 0.0)),
            BorderColor::all(tokens::BORDER),
        ))
        .id();
    let knob = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px_fixed(knob_left(at)),
                top: px_fixed(KNOB_AIR - 1.0),
                width: px_fixed(KNOB),
                height: px_fixed(KNOB),
                border_radius: BorderRadius::all(px_fixed(RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(tokens::INK),
            BoxShadow(vec![ShadowStyle {
                color: Color::srgba(0.0, 0.0, 0.0, 0.35),
                x_offset: px_fixed(0.0),
                y_offset: px_fixed(1.0),
                spread_radius: px_fixed(0.0),
                blur_radius: px_fixed(2.0),
            }]),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(track).add_child(knob).insert(ToggleMotion {
        on,
        shown: None,
        warmth: 0.0,
        knob,
    });
    hit(commands, kit, track, action)
}

/// Slides each toggle's knob towards its setting and eases its track's
/// colour, from where the toggle under the same stop last stood; warms the
/// track under the pointer. Writes nothing once a toggle is at rest.
#[allow(clippy::type_complexity)] // one toggle, its hit area's stop and interaction
pub(crate) fn animate_toggles(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut kept: ResMut<ToggleShown>,
    mut toggles: Query<(&mut ToggleMotion, &mut BackgroundColor, Option<&ChildOf>)>,
    wrappers: Query<(Option<&super::focus::Stop>, Option<&PickingInteraction>)>,
    mut nodes: Query<&mut Node>,
) {
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let step = if still {
        1.0
    } else {
        1.0 - (-TOGGLE_RATE * time.delta_secs()).exp()
    };
    for (mut motion, mut ground, parent) in &mut toggles {
        let (stop, interaction) = parent
            .and_then(|p| wrappers.get(p.parent()).ok())
            .unwrap_or((None, None));
        let target = f32::from(u8::from(motion.on));
        let warm_to = match interaction {
            Some(PickingInteraction::Pressed) => -1.0,
            Some(PickingInteraction::Hovered) => 1.0,
            _ => 0.0,
        };
        let from = motion
            .shown
            .unwrap_or_else(|| stop.and_then(|s| kept.0.get(s).copied()).unwrap_or(target));
        if motion.shown.is_some()
            && (from - target).abs() < f32::EPSILON
            && (motion.warmth - warm_to).abs() < f32::EPSILON
        {
            continue;
        }
        let mut shown = from + (target - from) * step;
        if (shown - target).abs() < 0.002 {
            shown = target;
        }
        let mut warmth = motion.warmth + (warm_to - motion.warmth) * step;
        if (warmth - warm_to).abs() < 0.002 {
            warmth = warm_to;
        }
        motion.shown = Some(shown);
        motion.warmth = warmth;
        if let Some(stop) = stop
            && kept
                .0
                .get(stop)
                .is_none_or(|was| (was - shown).abs() > f32::EPSILON)
        {
            kept.0.insert(*stop, shown);
        }
        let colour = track_ground(shown, warmth);
        if ground.0 != colour {
            ground.0 = colour;
        }
        if let Ok(mut knob) = nodes.get_mut(motion.knob) {
            let left = px_fixed(knob_left(shown));
            if knob.left != left {
                knob.left = left;
            }
        }
    }
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
    stepper_room(commands, kit, value, value, less, more)
}

/// The width a bold value of `chars` characters takes at the body size,
/// roughly (Alegreya Sans Bold averages about half an em): what a stepper
/// keeps for its value so the `+` does not move as the value changes.
fn value_room(kit: Kit, chars: usize) -> f32 {
    #[allow(clippy::cast_precision_loss)] // a value is a few characters
    let chars = chars as f32;
    (chars * kit.m.text * crate::hud::UI_SCALE * 0.52).max(kit.m.scaled(32.0))
}

/// A [`stepper`] that keeps room for its widest value, `widest`, whatever
/// value it shows (an arrangement's name, "Default"): the value never wraps
/// or is cut, and the `+` stands still (owner, 09.10.2026: "Defa").
pub fn stepper_room(
    commands: &mut Commands,
    kit: Kit,
    value: &str,
    widest: &str,
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
    let room = value_room(kit, widest.chars().count().max(value.chars().count()));
    // The value centred in the room kept for the widest: a box of that width
    // round words that never wrap.
    let shown = commands
        .spawn((
            Node {
                min_width: px_fixed(room),
                flex_shrink: 0.0,
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let words = commands
        .spawn((
            Text::new(value),
            tf_bold(kit.fonts, kit.m.text),
            TextColor(tokens::INK),
            TextLayout::new(Justify::Center, LineBreak::NoWrap),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(shown).add_child(words);
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
    // On a phone it stands beside its row's words where they leave room, so
    // three rows still show in the 390-px height (§2.7, M4-6); a narrower
    // row wraps it under them, where it grows to the full width.
    let width = if kit.m.frame == super::size::Frame::Phone {
        Node {
            flex_grow: 1.0,
            flex_basis: kit.m.px(240.0),
            ..default()
        }
    } else {
        Node {
            width: Val::Percent(100.0),
            ..default()
        }
    };
    let row = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(12.0),
                ..width
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
    (share * 100.0).round() as u8
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
    app.init_resource::<ToggleShown>()
        .add_systems(Update, (show_tooltips, draw_sliders, animate_toggles));
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

    fn fonts() -> UiFonts {
        UiFonts {
            text: Handle::default(),
            medium: Handle::default(),
            bold: Handle::default(),
            italic: Handle::default(),
            medium_italic: Handle::default(),
            serif: Handle::default(),
            serif_italic: Handle::default(),
            icons: Handle::default(),
            mana: Handle::default(),
        }
    }

    /// An app running what moves the kit's controls: the pointer's feel and
    /// the toggles, with motion allowed or not.
    fn moving(still: bool) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            // Sixty frames a second, whatever the machine running the test.
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_micros(16_667),
            ))
            .init_resource::<ToggleShown>()
            .add_systems(Update, (crate::ambience::feel, animate_toggles));
        if still {
            let mut prefs = crate::prefs::Prefs::default();
            prefs.edit().reduce_motion = true;
            app.insert_resource(prefs);
        }
        app
    }

    /// Makes something with the kit, at the default step on a desktop.
    fn make<T>(app: &mut App, build: impl FnOnce(&mut Commands, Kit) -> T) -> T {
        let fonts = fonts();
        let kit = Kit {
            fonts: &fonts,
            m: ShellMetrics::of(
                super::super::size::Viewport::desktop(1920.0, 1080.0),
                super::super::size::TextSize::M,
            ),
            german: false,
        };
        let made = {
            let mut commands = app.world_mut().commands();
            build(&mut commands, kit)
        };
        app.world_mut().flush();
        made
    }

    fn face_of(app: &App, wrapper: Entity) -> Entity {
        app.world().get::<Children>(wrapper).expect("a face")[0]
    }

    fn point(app: &mut App, wrapper: Entity, how: PickingInteraction) {
        app.world_mut().entity_mut(wrapper).insert(how);
    }

    fn scale(app: &App, face: Entity) -> f32 {
        app.world()
            .get::<UiTransform>(face)
            .expect("a transform")
            .scale
            .x
    }

    fn ground(app: &App, face: Entity) -> Color {
        app.world()
            .get::<BackgroundColor>(face)
            .expect("a ground")
            .0
    }

    /// Every pressable control the kit draws answers the pointer on its face
    /// though the pointer is over its hit area: it lightens and grows under a
    /// hover, gives and darkens under a press, and gets there over frames,
    /// not at once (owner, 09.10.2026: "no hover and click animations").
    #[test]
    fn the_kit_s_controls_answer_the_pointer_over_frames() {
        type Maker = fn(&mut Commands, Kit) -> Entity;
        let makers: [(&str, Maker); 6] = [
            ("primary", |c, k| {
                button(c, k, "Go", Weight::Primary, Live::Yes, Some("Enter"), ())
            }),
            ("secondary", |c, k| {
                button(c, k, "Back", Weight::Secondary, Live::Yes, None, ())
            }),
            ("ghost", |c, k| {
                button(c, k, "Later", Weight::Ghost, Live::Yes, None, ())
            }),
            ("chip", |c, k| chip(c, k, "Red", false, None, false, ())),
            ("pill", |c, k| pill(c, k, None, "Gateway", true, ())),
            ("nav", |c, k| nav(c, k, "Play", false, ())),
        ];
        for (what, maker) in makers {
            let mut app = moving(false);
            let wrapper = make(&mut app, maker);
            let face = face_of(&app, wrapper);
            app.update();
            let rest = ground(&app, face);
            assert!((scale(&app, face) - 1.0).abs() < 1e-6, "{what} at rest");
            point(&mut app, wrapper, PickingInteraction::Hovered);
            app.update();
            app.update();
            let early = scale(&app, face);
            for _ in 0..40 {
                app.update();
            }
            let lit = ground(&app, face);
            assert_ne!(lit, rest, "{what} lights under a hover");
            assert!(
                lit.to_srgba().red > rest.to_srgba().red || lit.alpha() > rest.alpha(),
                "{what}"
            );
            let hovered = scale(&app, face);
            assert!(hovered > 1.0, "{what} lifts under a hover");
            assert!(
                early < hovered,
                "{what} eases into the hover, it does not jump"
            );
            point(&mut app, wrapper, PickingInteraction::Pressed);
            for _ in 0..40 {
                app.update();
            }
            assert!(scale(&app, face) < 1.0, "{what} gives under a press");
            point(&mut app, wrapper, PickingInteraction::None);
            for _ in 0..60 {
                app.update();
            }
            assert!((scale(&app, face) - 1.0).abs() < 1e-3, "{what} comes back");
        }
    }

    /// The segments of a segmented control answer the pointer as buttons do.
    #[test]
    fn a_segment_lights_and_gives() {
        let mut app = moving(false);
        let group = make(&mut app, |c, k| segmented(c, k, &["One", "Two"], 0, |_| ()));
        let wrapper = app.world().get::<Children>(group).expect("segments")[1];
        let face = face_of(&app, wrapper);
        app.update();
        let rest = ground(&app, face);
        point(&mut app, wrapper, PickingInteraction::Hovered);
        for _ in 0..40 {
            app.update();
        }
        assert_ne!(ground(&app, face), rest, "an unchosen segment lights");
        assert!(scale(&app, face) > 1.0);
        point(&mut app, wrapper, PickingInteraction::Pressed);
        for _ in 0..40 {
            app.update();
        }
        assert!(scale(&app, face) < 1.0, "and gives under a press");
    }

    /// Under `reduce_motion` the states change at once: the first frame of a
    /// hover is the whole of it.
    #[test]
    fn reduce_motion_changes_the_state_at_once() {
        let mut app = moving(true);
        let wrapper = make(&mut app, |c, k| {
            button(c, k, "Go", Weight::Primary, Live::Yes, None, ())
        });
        let face = face_of(&app, wrapper);
        app.update();
        point(&mut app, wrapper, PickingInteraction::Hovered);
        app.update();
        let first = scale(&app, face);
        app.update();
        assert!(first > 1.0 && (scale(&app, face) - first).abs() < 1e-6);
    }

    fn knob_left_of(app: &App, track: Entity) -> f32 {
        let knob = app.world().get::<Children>(track).expect("a knob")[0];
        match app.world().get::<Node>(knob).expect("a node").left {
            Val::Px(px) => px,
            other => panic!("{other:?}"),
        }
    }

    /// A toggle pressed on is drawn again by the screen already on; the new
    /// one starts where the old one stood (its stop's memory) and slides
    /// over frames to the end, its track easing to the accent, and then
    /// writes nothing.
    #[test]
    fn a_toggle_slides_to_its_end_state_through_a_rebuild() {
        let stop = super::super::focus::Stop::new("settings", "text-face");
        let mut app = moving(false);
        let off = make(&mut app, |c, k| toggle(c, k, false, stop));
        for _ in 0..3 {
            app.update();
        }
        let track = face_of(&app, off);
        let at_off = knob_left_of(&app, track);
        // The press: the screen rebuilds with the toggle on.
        app.world_mut().entity_mut(off).despawn();
        let on = make(&mut app, |c, k| toggle(c, k, true, stop));
        let track = face_of(&app, on);
        app.update();
        let first = knob_left_of(&app, track);
        assert!(
            first < knob_left(1.0) - 1.0,
            "it starts from where it was, not at the end"
        );
        assert!(first >= at_off, "and moves on from there");
        for _ in 0..80 {
            app.update();
        }
        assert!(
            (knob_left_of(&app, track) - knob_left(1.0)).abs() < 1e-3,
            "it reaches the end"
        );
        assert_eq!(
            app.world()
                .get::<BackgroundColor>(track)
                .expect("a track")
                .0,
            track_ground(1.0, 0.0),
            "the track is the accent"
        );
        let tick = app
            .world()
            .entity(track)
            .get_ref::<BackgroundColor>()
            .map(|r| r.last_changed());
        app.update();
        app.update();
        let again = app
            .world()
            .entity(track)
            .get_ref::<BackgroundColor>()
            .map(|r| r.last_changed());
        assert_eq!(tick, again, "a toggle at rest writes nothing");
    }

    /// Under `reduce_motion` a pressed toggle stands at its end at once.
    #[test]
    fn under_reduce_motion_a_toggle_jumps() {
        let stop = super::super::focus::Stop::new("settings", "text-face");
        let mut app = moving(true);
        let off = make(&mut app, |c, k| toggle(c, k, false, stop));
        app.update();
        app.world_mut().entity_mut(off).despawn();
        let on = make(&mut app, |c, k| toggle(c, k, true, stop));
        let track = face_of(&app, on);
        app.update();
        assert!((knob_left_of(&app, track) - knob_left(1.0)).abs() < 1e-3);
    }
}

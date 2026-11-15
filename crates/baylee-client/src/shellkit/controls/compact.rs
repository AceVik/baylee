//! Compact pill buttons, compact segmented controls, and their explanatory tooltips.

#[allow(clippy::wildcard_imports)] // the controls module's shared vocabulary
use super::*;

/// Visual track height of compact toolbar controls (`button_compact`, `segmented_compact`).
const COMPACT_TRACK_H: f32 = 28.0;

/// A compact button for contextual strips (such as the seated-at-table bar):
/// smaller visual height and tighter padding than a primary screen button,
/// while preserving hover, press, and shader sheen animations.
pub fn button_compact(
    commands: &mut Commands,
    kit: Kit,
    icon: Option<char>,
    text: &str,
    weight: Weight,
    live: Live,
    action: impl Bundle,
) -> Entity {
    button_compact_with(commands, kit, icon, text, weight, live, &[], action)
}

/// A compact button with caller-provided decorations after its label, such
/// as the builder's unsaved mark and keyboard shortcut.
#[allow(clippy::too_many_arguments)] // same control plus optional trailing decorations
pub fn button_compact_with(
    commands: &mut Commands,
    kit: Kit,
    icon: Option<char>,
    text: &str,
    weight: Weight,
    live: Live,
    trailing: &[Entity],
    action: impl Bundle,
) -> Entity {
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
                min_height: kit.m.px(COMPACT_TRACK_H),
                padding: UiRect::axes(kit.m.px(10.0), px_fixed(0.0)),
                column_gap: kit.m.px(6.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_PILL)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(ground),
            BorderColor::all(if dead { tokens::BORDER } else { weight.rim() }),
        ))
        .id();
    if let Some(glyph) = icon {
        let mark = commands
            .spawn((
                Text::new(glyph.to_string()),
                icon_tf(kit.fonts, kit.m.small * 0.9),
                TextColor(ink),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(face).add_child(mark);
    }
    let words = commands
        .spawn((
            Text::new(text),
            tf_bold(kit.fonts, kit.m.small),
            TextColor(ink),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands
        .entity(face)
        .add_child(words)
        .add_children(trailing);
    if !dead {
        super::attach_live_feel(commands, face, ground, weight);
        return hit(commands, kit, face, action);
    }
    let wrapper = hit(commands, kit, face, (action, Disabled));
    if let Live::No(reason) = live {
        let tip = tooltip(commands, kit, reason);
        commands.entity(wrapper).add_child(tip).insert(Tooltip(tip));
    }
    wrapper
}

/// A compact segmented control matching [`button_compact`]'s 28 px visual track
/// while keeping the full [`ShellMetrics::hit`] touch/pointer target height.
pub fn segmented_compact<B: Bundle>(
    commands: &mut Commands,
    kit: Kit,
    items: &[&str],
    selected: usize,
    action: impl Fn(usize) -> B,
) -> Entity {
    let track_h = kit.m.scaled(COMPACT_TRACK_H).max(24.0);
    let outer_h = kit.m.hit.max(track_h);
    let inner_h = (track_h - 6.0).max(18.0);
    let v_margin = (track_h - 2.0 - outer_h) * 0.5;
    let group = commands
        .spawn((
            Node {
                min_height: px_fixed(track_h),
                padding: UiRect::horizontal(px_fixed(2.0)),
                column_gap: px_fixed(2.0),
                align_items: AlignItems::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL),
            BorderColor::all(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id();
    for (i, text) in items.iter().enumerate() {
        let on = i == selected;
        let face = super::segment_face(commands, inner_h, kit.m.px(10.0), None, RADIUS_PILL, on);
        let words = label(
            commands,
            kit,
            text,
            kit.m.small,
            if on { tokens::INK } else { tokens::MUTED },
        );
        commands.entity(face).add_child(words);
        let wrapper = hit(commands, kit, face, action(i));
        commands
            .entity(wrapper)
            .entry::<Node>()
            .and_modify(move |mut node| {
                node.margin = UiRect::vertical(px_fixed(v_margin));
            });
        commands.entity(group).add_child(wrapper);
    }
    group
}

/// A compact icon-only segmented control matching [`button_compact`]'s 28 px
/// visual track while keeping [`ShellMetrics::hit`] target compliance.
pub fn segmented_icons_compact<B: Bundle>(
    commands: &mut Commands,
    kit: Kit,
    items: &[(char, &str)],
    selected: usize,
    action: impl Fn(usize) -> B,
) -> Entity {
    let track_h = kit.m.scaled(COMPACT_TRACK_H).max(24.0);
    let outer_h = kit.m.hit.max(track_h);
    let inner_h = (track_h - 6.0).max(18.0);
    let v_margin = (track_h - 2.0 - outer_h) * 0.5;
    let touch = kit.m.touch();
    let min_w = (inner_h + 4.0).max(24.0);
    let group = commands
        .spawn((
            Node {
                min_height: px_fixed(track_h),
                padding: UiRect::horizontal(px_fixed(2.0)),
                column_gap: px_fixed(2.0),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL),
            BorderColor::all(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id();
    for (i, (glyph, tip)) in items.iter().enumerate() {
        let on = i == selected;
        let face = super::segment_face(
            commands,
            inner_h,
            px_fixed(0.0),
            Some(px_fixed(min_w)),
            RADIUS_PILL,
            on,
        );
        let icon_node = commands
            .spawn((
                Text::new(glyph.to_string()),
                icon_tf(kit.fonts, kit.m.small * 0.9),
                TextColor(if on { tokens::INK } else { tokens::MUTED }),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(face).add_child(icon_node);
        let wrapper = hit(
            commands,
            kit,
            face,
            (action(i), HoverHint((*tip).to_owned())),
        );
        commands
            .entity(wrapper)
            .entry::<Node>()
            .and_modify(move |mut node| {
                if !touch {
                    node.min_width = px_fixed(min_w);
                }
                node.margin = UiRect::vertical(px_fixed(v_margin));
            });
        commands.entity(group).add_child(wrapper);
    }
    group
}

/// A tooltip's box, hidden until the pointer rests on its owner.
pub(crate) fn tooltip(commands: &mut Commands, kit: Kit, text: &str) -> Entity {
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
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::OPAQUE),
            BorderColor::all(tokens::BORDER),
            crate::shellkit::surfaces::popover_shadow(),
            GlobalZIndex(tokens::z::POPOVER),
            Visibility::Hidden,
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(tip).add_child(words);
    tip
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Component)]
    struct Action;

    fn test_kit(fonts: &UiFonts) -> Kit<'_> {
        Kit {
            fonts,
            m: ShellMetrics::of(
                super::super::super::Viewport::desktop(1280.0, 800.0),
                crate::shellkit::TextSize::default(),
            ),
            german: false,
        }
    }

    #[test]
    fn compact_disabled_actions_keep_their_reason_and_trailing_shortcut() {
        let mut world = World::new();
        let fonts = UiFonts::default();
        let kit = test_kit(&fonts);
        let mut commands = world.commands();
        let key = commands.spawn(Text::new("Cmd+S")).id();
        let button = button_compact_with(
            &mut commands,
            kit,
            Some('\u{f0c7}'),
            "Save",
            Weight::Primary,
            Live::No("Add a name first"),
            &[key],
            Action,
        );
        world.flush();
        assert!(world.get::<Action>(button).is_some());
        assert!(world.get::<Disabled>(button).is_some());
        let tip = world
            .get::<Tooltip>(button)
            .expect("the disabled reason stays reachable")
            .0;
        let words = world.get::<Children>(tip).unwrap()[0];
        assert_eq!(world.get::<Text>(words).unwrap().0, "Add a name first");
        let face = world.get::<Children>(button).unwrap()[0];
        assert!(world.get::<Children>(face).unwrap().contains(&key));
    }

    #[test]
    fn compact_live_buttons_align_sheen_and_feel_border_by_weight() {
        let mut world = World::new();
        let fonts = UiFonts::default();
        let kit = test_kit(&fonts);
        let weights = [
            (Weight::Primary, true),
            (Weight::Gold, true),
            (Weight::Danger, true),
            (Weight::Secondary, false),
            (Weight::Ghost, false),
        ];
        let mut spawned = Vec::new();
        {
            let mut commands = world.commands();
            for (weight, expect_sheen) in weights {
                let btn = button_compact(&mut commands, kit, None, "Act", weight, Live::Yes, ());
                spawned.push((weight, expect_sheen, btn));
            }
        }
        world.flush();
        for (weight, expect_sheen, wrapper) in spawned {
            let face = world.get::<Children>(wrapper).unwrap()[0];
            assert!(
                world.get::<crate::ambience::Feel>(face).is_some(),
                "{weight:?} compact button has Feel"
            );
            assert!(
                world.get::<crate::ambience::FeelBorder>(face).is_some(),
                "{weight:?} compact button has FeelBorder"
            );
            assert_eq!(
                world.get::<crate::shellkit::sheen::Sheen>(face).is_some(),
                expect_sheen,
                "{weight:?} compact Sheen mismatch"
            );
        }
    }
}

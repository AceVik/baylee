//! Compact pill buttons and their explanatory tooltips.

#[allow(clippy::wildcard_imports)] // the controls module's shared vocabulary
use super::*;

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
                min_height: kit.m.px(28.0),
                padding: UiRect::axes(kit.m.px(10.0), px_fixed(0.0)),
                column_gap: kit.m.px(6.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_PILL)),
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
        let feel = if ground.alpha() < 0.01 {
            crate::ambience::Feel::rising_to(ground, tokens::HOVER)
        } else {
            crate::ambience::Feel::new(ground)
        };
        commands
            .entity(face)
            .insert((feel, crate::shellkit::sheen::Sheen));
        return hit(commands, kit, face, action);
    }
    let wrapper = hit(commands, kit, face, (action, Disabled));
    if let Live::No(reason) = live {
        let tip = tooltip(commands, kit, reason);
        commands.entity(wrapper).add_child(tip).insert(Tooltip(tip));
    }
    wrapper
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Component)]
    struct Action;

    #[test]
    fn compact_disabled_actions_keep_their_reason_and_trailing_shortcut() {
        let mut world = World::new();
        let fonts = UiFonts::default();
        let kit = Kit {
            fonts: &fonts,
            m: ShellMetrics::of(
                super::super::super::Viewport::desktop(1280.0, 800.0),
                crate::shellkit::TextSize::default(),
            ),
            german: false,
        };
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
}

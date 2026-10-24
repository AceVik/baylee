//! Quantity controls use exactly the row menu's square face and hit area.

#[allow(clippy::wildcard_imports)] // the builder module's shared vocabulary
use super::*;

/// Minus, quantity, plus: shared by deck rows and the card window.
pub(crate) fn stepper(
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
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let minus = icon_button(commands, kit, '\u{f068}', false, less);
    let shown = commands
        .spawn((
            Node {
                min_width: kit.m.px(32.0),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let number = commands
        .spawn((
            Text::new(value),
            tf_bold(kit.fonts, kit.m.text),
            TextColor(tokens::INK),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(shown).add_child(number);
    let plus = icon_button(commands, kit, '\u{f067}', false, more);
    commands.entity(group).add_children(&[minus, shown, plus]);
    group
}

#[cfg(all(
    test,
    not(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))
))]
mod tests {
    use super::*;
    use crate::shellkit::header::tests::{bounds, layout_app};
    use crate::shellkit::{InputClass, ShellMetrics, TextSize, Viewport};

    #[test]
    fn quantity_buttons_and_overflow_have_equal_rendered_faces_and_hit_areas() {
        let (mut app, fonts) = layout_app();
        for input in [InputClass::Pointer, InputClass::Touch] {
            for (step, value) in TextSize::ALL
                .into_iter()
                .flat_map(|step| ["1", "12", "100"].map(|value| (step, value)))
            {
                let mut view = Viewport::desktop(1280.0, 800.0);
                view.input = input;
                let kit = Kit {
                    fonts: &fonts,
                    m: ShellMetrics::of(view, step),
                    german: false,
                };
                let mut commands = app.world_mut().commands();
                let root = commands
                    .spawn(Node {
                        width: px_fixed(1000.0),
                        height: px_fixed(100.0),
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .id();
                let quantities = stepper(&mut commands, kit, value, (), ());
                let overflow = icon_button(&mut commands, kit, mark::MORE, false, ());
                let pool_plus = icon_button(&mut commands, kit, '\u{f067}', false, ());
                commands
                    .entity(root)
                    .add_children(&[quantities, overflow, pool_plus]);
                app.world_mut().flush();
                app.update();
                app.update();
                let children = app.world().get::<Children>(quantities).unwrap();
                let number = app.world().get::<Children>(children[1]).unwrap()[0];
                let middle = (bounds(app.world(), children[0]).center()
                    + bounds(app.world(), children[2]).center())
                    * 0.5;
                let centre = bounds(app.world(), number).center();
                assert!(
                    (centre - middle).abs().max_element() <= 0.5,
                    "{input:?} {step:?} {value}: count {centre:?} != midpoint {middle:?}"
                );
                let controls = [children[0], children[2], pool_plus];
                let reference_hit = bounds(app.world(), overflow).size();
                let reference_face = app.world().get::<Children>(overflow).unwrap()[0];
                let reference_face = bounds(app.world(), reference_face).size();
                assert!(
                    (reference_face.x - reference_face.y).abs() <= 1.0,
                    "{input:?} {step:?}: square face {reference_face:?}"
                );
                for control in controls {
                    let hit = bounds(app.world(), control).size();
                    let face = app.world().get::<Children>(control).unwrap()[0];
                    let face = bounds(app.world(), face).size();
                    assert!(
                        (hit - reference_hit).abs().max_element() <= 1.0,
                        "{input:?} {step:?}: hit {hit:?} != {reference_hit:?}"
                    );
                    assert!(
                        (face - reference_face).abs().max_element() <= 1.0,
                        "{input:?} {step:?}: face {face:?} != {reference_face:?}"
                    );
                    if input == InputClass::Touch {
                        assert!(hit.min_element() >= 44.0 - 0.5);
                    }
                }
                app.world_mut().entity_mut(root).despawn();
            }
        }
    }
}

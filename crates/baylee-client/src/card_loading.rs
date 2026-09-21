//! Retained image placeholders. Loading never rebuilds the card or its parent list.
use crate::cardmat::CardUiMaterial;
use bevy::prelude::*;
use bevy::ui::px;

/// A small spinner, also used by the printing catalog refresh action.
#[derive(Component)]
pub(crate) struct Spinner;

#[derive(Component)]
pub(crate) struct AwaitingImage {
    overlay: Entity,
    indicator: Entity,
    failed: bool,
}

/// Eight dots avoid depending on a font being ready before an image can load.
pub(crate) fn spinner(commands: &mut Commands, size: f32) -> Entity {
    let root = commands
        .spawn((
            Spinner,
            Node {
                width: px(size),
                height: px(size),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for i in 0..8_u8 {
        let angle = f32::from(i) * std::f32::consts::TAU / 8.0;
        let dot = size * 0.13;
        commands.entity(root).with_child((
            Node {
                position_type: PositionType::Absolute,
                left: px(size * (0.5 + angle.cos() * 0.36) - dot * 0.5),
                top: px(size * (0.5 + angle.sin() * 0.36) - dot * 0.5),
                width: px(dot),
                height: px(dot),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(Color::srgba(0.65, 0.83, 0.88, 0.2 + f32::from(i) * 0.1)),
            Pickable::IGNORE,
        ));
    }
    root
}

pub(crate) fn spin(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut nodes: Query<&mut UiTransform, With<Spinner>>,
) {
    let angle = if prefs.is_some_and(|p| p.all().reduce_motion) {
        0.0
    } else {
        time.elapsed_secs() * 2.5
    };
    for mut node in &mut nodes {
        node.rotation = Rot2::radians(angle);
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn images(
    mut commands: Commands,
    images: Res<Assets<Image>>,
    materials: Res<Assets<CardUiMaterial>>,
    server: Res<AssetServer>,
    nodes: Query<
        (
            Entity,
            Option<&MaterialNode<CardUiMaterial>>,
            Option<&ImageNode>,
            Option<&AwaitingImage>,
        ),
        Or<(With<MaterialNode<CardUiMaterial>>, With<ImageNode>)>,
    >,
) {
    for (entity, material, plain, waiting) in &nodes {
        let art = material
            .and_then(|m| materials.get(&m.0))
            .and_then(|m| m.art.as_ref())
            .or_else(|| plain.map(|p| &p.image));
        let pending = art.is_some_and(|h| !images.contains(h.id()));
        if !pending {
            if let Some(waiting) = waiting {
                commands.entity(waiting.overlay).despawn();
                commands.entity(entity).remove::<AwaitingImage>();
            }
            continue;
        }
        let failed = art.is_some_and(|h| {
            matches!(
                server.get_load_state(h.id()),
                Some(bevy::asset::LoadState::Failed(_))
            )
        });
        if let Some(waiting) = waiting {
            if failed && !waiting.failed {
                commands.entity(waiting.indicator).despawn();
                for radians in [-0.78, 0.78] {
                    commands.entity(waiting.overlay).with_child((
                        Node {
                            position_type: PositionType::Absolute,
                            width: px(18),
                            height: px(2),
                            ..default()
                        },
                        UiTransform::from_rotation(Rot2::radians(radians)),
                        BackgroundColor(Color::srgb(0.65, 0.73, 0.78)),
                        Pickable::IGNORE,
                    ));
                }
                commands.entity(entity).insert(AwaitingImage {
                    overlay: waiting.overlay,
                    indicator: waiting.overlay,
                    failed: true,
                });
            }
            continue;
        }
        let overlay = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    right: px(0),
                    top: px(0),
                    bottom: px(0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: BorderRadius::all(px(8)),
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.055, 0.085, 0.12)),
                BorderColor::all(Color::srgb(0.23, 0.34, 0.39)),
                Pickable::IGNORE,
            ))
            .id();
        let spinner = spinner(&mut commands, 22.0);
        commands.entity(overlay).add_child(spinner);
        commands
            .entity(entity)
            .add_child(overlay)
            .insert(AwaitingImage {
                overlay,
                indicator: spinner,
                failed: false,
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_loading_card_keeps_its_placeholder_until_the_image_arrives() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<CardUiMaterial>()
            .add_systems(Update, images);
        let handle = app.world().resource::<Assets<Image>>().reserve_handle();
        let card = app.world_mut().spawn(ImageNode::new(handle.clone())).id();
        app.update();
        let overlay = app.world().get::<AwaitingImage>(card).unwrap().overlay;
        app.update();
        assert_eq!(
            app.world().get::<AwaitingImage>(card).unwrap().overlay,
            overlay
        );
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .insert(handle.id(), Image::default())
            .unwrap();
        app.update();
        assert!(app.world().get::<AwaitingImage>(card).is_none());
        assert!(app.world().get_entity(overlay).is_err());
        assert!(app.world().get_entity(card).is_ok());
    }
}

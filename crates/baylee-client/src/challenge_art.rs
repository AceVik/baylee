//! Original generated landscapes, loaded once and shared across activity surfaces.
use bevy::prelude::*;

#[derive(Component, Clone, Copy)]
pub(crate) enum Scene {
    Day,
    Night,
    Tower,
    Victory,
    Defeat,
    Ranked,
}

impl Scene {
    const fn index(self) -> usize {
        match self {
            Self::Day => 0,
            Self::Night => 1,
            Self::Tower => 2,
            Self::Victory => 3,
            Self::Defeat => 4,
            Self::Ranked => 5,
        }
    }
}

const PATHS: [&str; 6] = [
    "tower/scenes/daily.png",
    "tower/scenes/weekly.png",
    "tower/scenes/tower.png",
    "tower/run-victory.png",
    "tower/run-defeat.png",
    "tower/scenes/ranked-arena.png",
];

#[derive(Resource, Default)]
struct Landscapes([Option<Handle<Image>>; PATHS.len()]);

fn dress(
    mut commands: Commands,
    scenes: Query<(Entity, &Scene), Changed<Scene>>,
    mut cache: ResMut<Landscapes>,
    server: Res<AssetServer>,
) {
    for (entity, scene) in &scenes {
        let index = scene.index();
        let material = cache.0[index]
            .get_or_insert_with(|| server.load(PATHS[index]))
            .clone();
        commands.entity(entity).try_insert(ImageNode {
            image_mode: bevy::ui::widget::NodeImageMode::Stretch,
            ..ImageNode::new(material)
        });
    }
}

pub(crate) fn install(app: &mut App) {
    if !app.world().contains_resource::<AssetServer>()
        || app.get_sub_app(bevy::render::RenderApp).is_none()
    {
        return;
    }
    let cache = {
        let server = app.world().resource::<AssetServer>();
        Landscapes(PATHS.map(|path| Some(server.load(path))))
    };
    app.insert_resource(cache)
        .add_systems(
            PostUpdate,
            dress
                .before(bevy::ui::UiSystems::Prepare)
                .before(bevy::ui::UiSystems::Content)
                .before(bevy::ui::UiSystems::Layout)
                .before(bevy::ui::UiSystems::Stack),
        )
        .add_systems(
            PostUpdate,
            cover
                .after(bevy::ui::UiSystems::Layout)
                .before(bevy::ui::UiSystems::PostLayout),
        );
}

/// Crop the source, never stretch the landscape or let its intrinsic size reflow a tile.
fn cover(
    mut scenes: Query<(&ComputedNode, &mut ImageNode), With<Scene>>,
    images: Res<Assets<Image>>,
) {
    for (node, mut image) in &mut scenes {
        let Some(source) = images.get(&image.image) else {
            continue;
        };
        let size = source.size().as_vec2();
        let target = node.size();
        if target.x <= 0.0 || target.y <= 0.0 {
            continue;
        }
        let scale = (target.x / size.x).max(target.y / size.y);
        let visible = target / scale;
        let margin = (size - visible) * 0.5;
        let rect = Some(Rect::from_corners(margin, size - margin));
        if image.rect != rect {
            image.rect = rect;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_landscapes_validate_without_a_clock_or_external_texture() {
        crate::cardmat::tests::check_wgsl(
            include_str!("shaders/challenge_art.wesl"),
            "struct UiVertexOutput { @location(0) uv: vec2<f32>, @builtin(position) position: vec4<f32> };",
        );
    }

    #[test]
    fn scene_attaches_and_crops_in_the_same_post_update_frame() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>();
        let handle = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::new_fill(
                bevy::render::render_resource::Extent3d {
                    width: 800,
                    height: 400,
                    depth_or_array_layers: 1,
                },
                bevy::render::render_resource::TextureDimension::D2,
                &[0, 0, 0, 255],
                bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                bevy::asset::RenderAssetUsages::default(),
            ));
        let mut cache = Landscapes::default();
        cache.0[Scene::Tower.index()] = Some(handle.clone());
        app.insert_resource(cache)
            .configure_sets(
                PostUpdate,
                (
                    bevy::ui::UiSystems::Prepare,
                    bevy::ui::UiSystems::Content,
                    bevy::ui::UiSystems::Layout,
                    bevy::ui::UiSystems::PostLayout,
                )
                    .chain_weak(),
            )
            .add_systems(
                PostUpdate,
                dress
                    .before(bevy::ui::UiSystems::Prepare)
                    .before(bevy::ui::UiSystems::Content)
                    .before(bevy::ui::UiSystems::Layout)
                    .before(bevy::ui::UiSystems::Stack),
            )
            .add_systems(
                PostUpdate,
                (|mut nodes: Query<&mut ComputedNode, With<Scene>>| {
                    for mut node in &mut nodes {
                        node.size = Vec2::new(200.0, 200.0);
                    }
                })
                .in_set(bevy::ui::UiSystems::Layout),
            )
            .add_systems(
                PostUpdate,
                cover
                    .after(bevy::ui::UiSystems::Layout)
                    .before(bevy::ui::UiSystems::PostLayout),
            );
        let tile = app
            .world_mut()
            .spawn((Scene::Tower, ComputedNode::default()))
            .id();
        app.update();
        let image = app
            .world()
            .get::<ImageNode>(tile)
            .expect("ImageNode attached before render on Frame N");
        assert_eq!(image.image, handle);
        assert_eq!(
            image.rect,
            Some(Rect::from_corners(
                Vec2::new(200.0, 0.0),
                Vec2::new(600.0, 400.0)
            )),
            "landscape cropped in the exact same frame as layout"
        );
    }
}

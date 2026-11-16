//! Original generated landscapes, loaded once and shared across activity surfaces.
use bevy::prelude::*;

#[derive(Component, Clone, Copy)]
pub(crate) enum Scene {
    Day,
    Night,
    Tower,
    Victory,
    Defeat,
}

impl Scene {
    const fn index(self) -> usize {
        match self {
            Self::Day => 0,
            Self::Night => 1,
            Self::Tower => 2,
            Self::Victory => 3,
            Self::Defeat => 4,
        }
    }
}

#[derive(Resource, Default)]
struct Landscapes([Option<Handle<Image>>; 5]);

fn dress(
    mut commands: Commands,
    scenes: Query<(Entity, &Scene), Changed<Scene>>,
    mut cache: ResMut<Landscapes>,
    server: Res<AssetServer>,
) {
    for (entity, scene) in &scenes {
        let index = scene.index();
        let material = cache.0[index].get_or_insert_with(|| {
            server.load(
                [
                    "tower/scenes/daily.png",
                    "tower/scenes/weekly.png",
                    "tower/scenes/tower.png",
                    "tower/run-victory.png",
                    "tower/run-defeat.png",
                ][index],
            )
        });
        commands.entity(entity).insert(ImageNode {
            image_mode: bevy::ui::widget::NodeImageMode::Stretch,
            ..ImageNode::new(material.clone())
        });
    }
}

pub(crate) fn install(app: &mut App) {
    if !app.world().contains_resource::<AssetServer>()
        || app.get_sub_app(bevy::render::RenderApp).is_none()
    {
        return;
    }
    app.init_resource::<Landscapes>()
        .add_systems(PostUpdate, (dress, cover).chain());
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
    #[test]
    fn the_three_landscapes_validate_without_a_clock_or_external_texture() {
        crate::cardmat::tests::check_wgsl(
            include_str!("shaders/challenge_art.wesl"),
            "struct UiVertexOutput { @location(0) uv: vec2<f32>, @builtin(position) position: vec4<f32> };",
        );
    }
}

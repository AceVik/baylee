//! Original, static landscape vignettes; three shared materials, no downloaded art.
use bevy::{
    asset::embedded_asset, prelude::*, render::render_resource::AsBindGroup, shader::ShaderRef,
};

#[derive(Component, Clone, Copy)]
pub(crate) enum Scene {
    Day,
    Night,
    Tower,
}

impl Scene {
    const fn index(self) -> usize {
        match self {
            Self::Day => 0,
            Self::Night => 1,
            Self::Tower => 2,
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
struct Landscape {
    #[uniform(0)]
    scene: Vec4,
}

impl UiMaterial for Landscape {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/challenge_art.wesl".into()
    }
}

#[derive(Resource, Default)]
struct Landscapes([Option<Handle<Landscape>>; 3]);

fn dress(
    mut commands: Commands,
    scenes: Query<(Entity, &Scene), Changed<Scene>>,
    mut cache: ResMut<Landscapes>,
    mut assets: ResMut<Assets<Landscape>>,
) {
    for (entity, scene) in &scenes {
        let index = scene.index();
        let material = cache.0[index].get_or_insert_with(|| {
            assets.add(Landscape {
                scene: Vec4::new(index as f32, 0.0, 0.0, 0.0),
            })
        });
        commands
            .entity(entity)
            .insert(MaterialNode(material.clone()));
    }
}

pub(crate) fn install(app: &mut App) {
    if !app.world().contains_resource::<AssetServer>()
        || app.get_sub_app(bevy::render::RenderApp).is_none()
    {
        return;
    }
    embedded_asset!(app, "shaders/challenge_art.wesl");
    app.init_resource::<Landscapes>()
        .add_plugins(UiMaterialPlugin::<Landscape>::default())
        .add_systems(PostUpdate, dress);
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

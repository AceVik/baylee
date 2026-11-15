//! Bounded shared table materials. Idle frames perform no asset writes.
use super::{PICTURES, Params};
use bevy::{
    asset::embedded_asset, prelude::*, render::render_resource::AsBindGroup, shader::ShaderRef,
};

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(super) struct Portrait {
    pub goods: super::Portrait,
    pub veil: u8,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
struct Material {
    #[uniform(0)]
    params: Params,
    #[texture(1)]
    #[sampler(2)]
    portrait: Option<Handle<Image>>,
}

impl bevy::pbr::Material for Material {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/itemart/world.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

#[derive(Resource, Default)]
struct Cache {
    materials: [[[Option<Handle<Material>>; 8]; 4]; PICTURES.len() + 1],
    mesh: Option<Handle<Mesh>>,
}

#[allow(clippy::too_many_arguments)] // ECS queries/resources remain independently borrowable
fn dress(
    mut commands: Commands,
    portraits: Query<(Entity, &Portrait), Changed<Portrait>>,
    mut cache: ResMut<Cache>,
    mut assets: ResMut<Assets<Material>>,
    mut meshes: ResMut<Assets<Mesh>>,
    server: Res<AssetServer>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    quality: Option<Res<crate::quality::InUse>>,
) {
    let energy = if crate::quality::ambient_still(
        prefs.as_ref().is_some_and(|p| p.all().reduce_motion),
        quality.as_deref(),
    ) {
        0.0
    } else {
        1.0
    };
    if prefs.as_ref().is_some_and(DetectChanges::is_changed)
        || quality.as_ref().is_some_and(DetectChanges::is_changed)
    {
        for handle in cache.materials.iter().flatten().flatten().flatten() {
            if assets
                .get(handle)
                .is_some_and(|material| (material.params.shape.z - energy).abs() > f32::EPSILON)
                && let Some(mut material) = assets.get_mut(handle)
            {
                material.params.shape.z = energy;
            }
        }
    }
    for (entity, portrait) in &portraits {
        let (kind, tier) = portrait.goods.indices();
        let veil = portrait.veil & 7;
        let handle = cache.materials[kind][tier][usize::from(veil)]
            .get_or_insert_with(|| {
                let mut params = super::params(kind, tier, energy);
                params.shape.w = f32::from(veil);
                assets.add(Material {
                    params,
                    portrait: kind
                        .checked_sub(1)
                        .map(|index| server.load(PICTURES[index])),
                })
            })
            .clone();
        let mesh = cache
            .mesh
            .get_or_insert_with(|| meshes.add(Rectangle::new(1.0, 1.0)))
            .clone();
        commands.entity(entity).try_insert((
            Mesh3d(mesh),
            MeshMaterial3d(handle),
            bevy::light::NotShadowCaster,
        ));
    }
}

pub(super) fn install(app: &mut App) {
    embedded_asset!(app, "world.wesl");
    app.init_resource::<Cache>()
        .add_plugins(MaterialPlugin::<Material>::default())
        .add_systems(PostUpdate, dress);
}

#[cfg(test)]
mod tests;

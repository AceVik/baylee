//! Original raster item portraits in rarity settings. Shared images, GPU-clock animation.
pub(crate) mod faces;
pub(crate) mod identity;
mod world;

use baylee_client_core::echoes::domain::v2::{
    catalog::{Definition, Family, Potion, Rarity},
    shop::Goods,
};
use bevy::{
    asset::embedded_asset,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Portrait(pub Goods);

impl Portrait {
    fn indices(self) -> (usize, usize) {
        match self.0 {
            Goods::Card(_) => (0, 0),
            Goods::Relic(Definition { family, rarity }) => (
                match family {
                    Family::Heartwood => 1,
                    Family::Satchel => 2,
                    Family::Compass => 3,
                    Family::Grove => 4,
                    Family::SpringChalice => 8,
                    Family::Hearthstone => 9,
                    Family::BloodAmber => 10,
                    Family::LifeCoil => 11,
                    Family::SeedCradle => 12,
                    Family::SwarmBanner => 13,
                    Family::HarvestWheel => 14,
                    Family::RootReservoir => 15,
                    Family::EmberLantern => 16,
                    Family::TideglassLens => 17,
                    Family::GraveBell => 18,
                    Family::StormSpindle => 19,
                    Family::DawnAegis => 20,
                    Family::GildedAcorn => 21,
                },
                match rarity {
                    Rarity::Common => 0,
                    Rarity::Uncommon => 1,
                    Rarity::Rare => 2,
                    Rarity::MythicRare => 3,
                },
            ),
            Goods::Potion(Potion::Healing) => (5, 0),
            Goods::Potion(Potion::Mana) => (6, 0),
            Goods::Mythic => (7, 3),
        }
    }
}

#[derive(Clone, Copy, ShaderType, Debug)]
struct Params {
    metal: Vec4,
    gem: Vec4,
    // motif, rarity, animation energy, passing-state bits; no per-frame writes.
    shape: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
struct Material {
    #[uniform(0)]
    params: Params,
    #[texture(1)]
    #[sampler(2)]
    portrait: Option<Handle<Image>>,
}

// Loaded on first use, once per family across all rarities and rebuilt panels.
const PICTURES: [&str; 21] = [
    "tower/portraits/heartwood.png",
    "tower/portraits/satchel.png",
    "tower/portraits/compass.png",
    "tower/portraits/grove.png",
    "tower/portraits/healing.png",
    "tower/portraits/mana.png",
    "tower/portraits/mythic.png",
    "tower/portraits/spring-chalice.png",
    "tower/portraits/hearthstone.png",
    "tower/portraits/blood-amber.png",
    "tower/portraits/life-coil.png",
    "tower/portraits/seed-cradle.png",
    "tower/portraits/swarm-banner.png",
    "tower/portraits/harvest-wheel.png",
    "tower/portraits/root-reservoir.png",
    "tower/portraits/ember-lantern.png",
    "tower/portraits/tideglass-lens.png",
    "tower/portraits/grave-bell.png",
    "tower/portraits/storm-spindle.png",
    "tower/portraits/dawn-aegis.png",
    "tower/portraits/gilded-acorn.png",
];

impl UiMaterial for Material {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/itemart/ui.wesl".into()
    }
}

#[derive(Resource, Default)]
struct Cache([[Option<Handle<Material>>; 4]; PICTURES.len() + 1]);

fn linear(rgb: [f32; 3]) -> Vec4 {
    let color = Color::srgb(rgb[0], rgb[1], rgb[2]).to_linear();
    Vec4::new(color.red, color.green, color.blue, 1.0)
}

fn params(kind: usize, tier: usize, energy: f32) -> Params {
    Params {
        metal: linear(
            [
                [0.66, 0.52, 0.36],
                [0.76, 0.85, 0.89],
                [0.97, 0.74, 0.34],
                [1.0, 0.62, 0.34],
            ][tier],
        ),
        gem: linear(
            [
                [0.71, 0.76, 0.90],
                [0.83, 0.43, 0.29],
                [0.56, 0.74, 0.88],
                [0.30, 0.84, 0.83],
                [0.42, 0.85, 0.48],
                [0.93, 0.31, 0.42],
                [0.40, 0.56, 1.0],
                [0.86, 0.45, 1.0],
                [0.36, 0.88, 0.79],
                [1.0, 0.64, 0.29],
                [0.95, 0.30, 0.23],
                [0.36, 0.78, 0.72],
                [0.62, 0.91, 0.35],
                [0.35, 0.78, 0.49],
                [0.98, 0.73, 0.35],
                [0.37, 0.93, 0.63],
                [1.0, 0.42, 0.16],
                [0.30, 0.75, 1.0],
                [0.74, 0.45, 0.94],
                [0.55, 0.70, 1.0],
                [1.0, 0.91, 0.56],
                [0.97, 0.70, 0.24],
            ][kind],
        ),
        shape: Vec4::new(kind as f32, tier as f32, energy, 0.0),
    }
}

fn dress(
    mut commands: Commands,
    portraits: Query<(Entity, &Portrait), Changed<Portrait>>,
    mut cache: ResMut<Cache>,
    mut assets: ResMut<Assets<Material>>,
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
        for handle in cache.0.iter().flatten().flatten() {
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
        let (kind, tier) = portrait.indices();
        let handle = cache.0[kind][tier]
            .get_or_insert_with(|| {
                assets.add(Material {
                    params: params(kind, tier, energy),
                    portrait: kind
                        .checked_sub(1)
                        .map(|index| server.load(PICTURES[index])),
                })
            })
            .clone();
        commands.entity(entity).try_insert(MaterialNode(handle));
    }
}

pub(crate) fn install(app: &mut App) {
    if !app.world().contains_resource::<AssetServer>()
        || app.get_sub_app(bevy::render::RenderApp).is_none()
    {
        return;
    }
    embedded_asset!(app, "ui.wesl");
    bevy::shader::load_shader_library!(app, "common.wesl");
    world::install(app);
    app.init_resource::<Cache>()
        .add_plugins(UiMaterialPlugin::<Material>::default())
        .add_systems(PostUpdate, dress);
}

#[cfg(test)]
mod tests;

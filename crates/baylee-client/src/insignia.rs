//! Original, procedural earned medals. Forty-eight shared materials bound the cache.
//! The GPU clock animates them; idle frames never upload material uniforms.
use baylee_client_core::challenges::api::{BadgeKind, Tier};
use bevy::{
    asset::embedded_asset,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};

/// Stable motif and earned tier; progress and labels remain ordinary accessible UI.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Insignia {
    pub kind: Motif,
    pub tier: Option<Tier>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Motif {
    Challenge(BadgeKind),
    Tower(baylee_client_core::tower::api::BadgeKind),
}

impl Insignia {
    fn indices(self) -> (usize, usize) {
        let kind = match self.kind {
            Motif::Tower(baylee_client_core::tower::api::BadgeKind::Pathfinder) => 5,
            Motif::Tower(baylee_client_core::tower::api::BadgeKind::Summit) => 6,
            Motif::Tower(baylee_client_core::tower::api::BadgeKind::Unbroken) => 7,
            Motif::Challenge(kind) => match kind {
                BadgeKind::Daily => 0,
                BadgeKind::Weekly => 1,
                BadgeKind::Tactician => 2,
                BadgeKind::FirstTry => 3,
                BadgeKind::Regular => 4,
            },
        };
        let tier = match self.tier {
            None => 0,
            Some(Tier::Bronze) => 1,
            Some(Tier::Silver) => 2,
            Some(Tier::Gold) => 3,
            Some(Tier::Platinum) => 4,
            Some(Tier::Obsidian) => 5,
        };
        (kind, tier)
    }
}

#[derive(Clone, Copy, ShaderType, PartialEq, Debug)]
struct Params {
    metal: Vec4,
    gem: Vec4,
    // motif, tier, animation energy, reserved. All vectors aligned to 16 bytes.
    shape: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
struct Medal {
    #[uniform(0)]
    params: Params,
}

impl UiMaterial for Medal {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/insignia.wesl".into()
    }
}

#[derive(Resource, Default)]
struct Medals([[Option<Handle<Medal>>; 6]; 8]);

fn linear(rgb: [f32; 3]) -> Vec4 {
    let c = Color::srgb(rgb[0], rgb[1], rgb[2]).to_linear();
    Vec4::new(c.red, c.green, c.blue, 1.0)
}

fn params(kind: usize, tier: usize, energy: f32) -> Params {
    let metal = [
        [0.35, 0.39, 0.42],
        [0.79, 0.48, 0.26],
        [0.77, 0.84, 0.90],
        [0.96, 0.74, 0.32],
        [0.78, 0.94, 0.96],
        [0.60, 0.50, 0.86],
    ][tier];
    let gem = [
        [1.0, 0.68, 0.25],
        [0.48, 0.73, 1.0],
        [0.84, 0.50, 0.90],
        [0.98, 0.48, 0.33],
        [0.36, 0.86, 0.62],
        [0.40, 0.86, 0.76],
        [0.96, 0.72, 0.28],
        [0.64, 0.58, 0.98],
    ][kind];
    Params {
        metal: linear(metal),
        gem: linear(gem),
        shape: Vec4::new(kind as f32, tier as f32, energy, 0.0),
    }
}

fn dress(
    mut commands: Commands,
    icons: Query<(Entity, &Insignia), Changed<Insignia>>,
    mut cache: ResMut<Medals>,
    mut assets: ResMut<Assets<Medal>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    quality: Option<Res<crate::quality::InUse>>,
) {
    let still = crate::quality::ambient_still(
        prefs.as_ref().is_some_and(|p| p.all().reduce_motion),
        quality.as_deref(),
    );
    let energy = if still { 0.0 } else { 1.0 };
    // Only the preference changes an existing material; time is never a CPU write.
    if prefs.as_ref().is_some_and(DetectChanges::is_changed)
        || quality.as_ref().is_some_and(DetectChanges::is_changed)
    {
        for handle in cache.0.iter().flatten().flatten() {
            if assets
                .get(handle)
                .is_some_and(|m| (m.params.shape.z - energy).abs() > f32::EPSILON)
                && let Some(mut medal) = assets.get_mut(handle)
            {
                medal.params.shape.z = energy;
            }
        }
    }
    for (entity, icon) in &icons {
        let (kind, tier) = icon.indices();
        let handle = cache.0[kind][tier]
            .get_or_insert_with(|| {
                assets.add(Medal {
                    params: params(kind, tier, energy),
                })
            })
            .clone();
        commands.entity(entity).insert(MaterialNode(handle));
    }
}

pub(crate) fn install(app: &mut App) {
    if !app.world().contains_resource::<AssetServer>()
        || app.get_sub_app(bevy::render::RenderApp).is_none()
    {
        return;
    }
    embedded_asset!(app, "shaders/insignia.wesl");
    app.init_resource::<Medals>()
        .add_plugins(UiMaterialPlugin::<Medal>::default())
        .add_systems(PostUpdate, dress);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lobby_rebuilds_reuse_materials_and_idle_frames_do_not_upload_them() {
        let mut app = App::new();
        app.init_resource::<Assets<Medal>>()
            .init_resource::<Medals>()
            .add_systems(Update, dress);
        for _ in 0..3 {
            let icons: Vec<_> = BadgeKind::ALL
                .into_iter()
                .map(|kind| {
                    app.world_mut()
                        .spawn(Insignia {
                            kind: Motif::Challenge(kind),
                            tier: Some(Tier::Gold),
                        })
                        .id()
                })
                .collect();
            app.update();
            assert_eq!(app.world().resource::<Assets<Medal>>().len(), 5);
            for entity in icons {
                app.world_mut().despawn(entity);
            }
        }
        app.update();
        let tick = app.world().read_change_tick();
        app.update();
        assert!(
            !app.world()
                .resource_ref::<Assets<Medal>>()
                .last_changed()
                .is_newer_than(tick, app.world().read_change_tick())
        );
    }

    #[test]
    fn medal_shader_validates_with_native_frontend() {
        crate::cardmat::tests::check_wgsl(
            include_str!("shaders/insignia.wesl"),
            "struct UiVertexOutput { @location(0) uv: vec2<f32>, @builtin(position) position: vec4<f32> }; struct Globals { time: f32 };",
        );
    }

    #[test]
    fn every_motif_and_tier_has_a_unique_bounded_cache_slot() {
        let mut slots = std::collections::BTreeSet::new();
        for kind in BadgeKind::ALL.into_iter().map(Motif::Challenge).chain(
            baylee_client_core::tower::api::BadgeKind::ALL
                .into_iter()
                .map(Motif::Tower),
        ) {
            for tier in [
                None,
                Some(Tier::Bronze),
                Some(Tier::Silver),
                Some(Tier::Gold),
                Some(Tier::Platinum),
                Some(Tier::Obsidian),
            ] {
                let slot = Insignia { kind, tier }.indices();
                assert!(slot.0 < 8 && slot.1 < 6 && slots.insert(slot));
                let p = params(slot.0, slot.1, 0.0);
                assert!(p.metal.is_finite() && p.gem.is_finite());
                assert_eq!(p.shape.z, 0.0);
            }
        }
        assert_eq!(slots.len(), 48);
    }
}

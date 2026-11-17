//! Shared GPU-clock finishes for modular wooden cabinets and frozen shop shelves,
//! with bounded cached materials and zero idle frame uploads.
use super::Decor;
use bevy::{
    asset::embedded_asset, prelude::*, render::render_resource::AsBindGroup, shader::ShaderRef,
};
#[cfg(test)]
mod tests;

/// Marks a frozen-shelf frost overlay node so `dress` attaches the shared
/// crystalline rime-frost `UiMaterial` before layout/extraction on Frame 0.
#[derive(Component, Clone, Copy)]
pub(crate) struct FrostSurface;

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
struct Finish {
    #[uniform(0)]
    motion: Vec4,
}

impl UiMaterial for Finish {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/towerui/scenery/glint.wesl".into()
    }
}

#[derive(Resource, Default)]
struct Cached {
    warm: Option<Handle<Finish>>,
    frost: Option<Handle<Finish>>,
}

pub(super) fn install(app: &mut App) {
    if app.get_sub_app(bevy::render::RenderApp).is_none() {
        return;
    }
    embedded_asset!(app, "glint.wesl");
    app.init_resource::<Cached>()
        .add_plugins(UiMaterialPlugin::<Finish>::default())
        .add_systems(
            PostUpdate,
            (dress, ApplyDeferred)
                .chain()
                .before(bevy::ui::UiSystems::Prepare)
                .before(bevy::ui::UiSystems::Content)
                .before(bevy::ui::UiSystems::Layout)
                .before(bevy::ui::UiSystems::Stack)
                .before(bevy::asset::AssetEventSystems),
        );
}

fn dress(
    mut commands: Commands,
    frames: Query<(Entity, &Decor), Added<Decor>>,
    frosts: Query<Entity, Added<FrostSurface>>,
    mut cache: ResMut<Cached>,
    mut assets: ResMut<Assets<Finish>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    quality: Option<Res<crate::quality::InUse>>,
) {
    let energy = if crate::quality::ambient_still(
        prefs
            .as_ref()
            .is_some_and(|prefs| prefs.all().reduce_motion),
        quality.as_deref(),
    ) {
        0.0
    } else {
        1.0
    };
    if prefs.as_ref().is_some_and(DetectChanges::is_changed)
        || quality.as_ref().is_some_and(DetectChanges::is_changed)
    {
        for handle in [cache.warm.as_ref(), cache.frost.as_ref()]
            .into_iter()
            .flatten()
        {
            if assets
                .get(handle)
                .is_some_and(|finish| (finish.motion.x - energy).abs() > f32::EPSILON)
                && let Some(mut finish) = assets.get_mut(handle)
            {
                finish.motion.x = energy;
            }
        }
    }
    for (entity, decor) in &frames {
        if !matches!(decor, Decor::Frame) {
            continue;
        }
        let handle = cache
            .warm
            .get_or_insert_with(|| {
                assets.add(Finish {
                    motion: Vec4::new(energy, 0.0, 0.0, 0.0),
                })
            })
            .clone();
        commands.spawn((
            MaterialNode(handle),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                left: px(0),
                top: px(0),
                ..default()
            },
            ChildOf(entity),
            Pickable::IGNORE,
        ));
    }
    for entity in &frosts {
        let handle = cache
            .frost
            .get_or_insert_with(|| {
                assets.add(Finish {
                    motion: Vec4::new(energy, 1.0, 0.0, 0.0),
                })
            })
            .clone();
        commands.entity(entity).try_insert(MaterialNode(handle));
    }
}

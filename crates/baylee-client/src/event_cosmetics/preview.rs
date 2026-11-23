//! A preview occupies its caller's existing UI node. Changing equipment or
//! motion preference replaces a material handle without moving that node.
use super::Paint;
use bevy::{
    platform::collections::HashMap, prelude::*, render::render_resource::AsBindGroup,
    shader::ShaderRef,
};

/// A server-authorized image displayed in a slot of the inventory or on a card.
#[derive(Component, Clone)]
pub(crate) struct CosmeticPreview(pub(super) Paint);

impl CosmeticPreview {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn new(
        image: Handle<Image>,
        _kind: baylee_account::advent::Kind,
        finish: baylee_account::advent::Finish,
    ) -> Self {
        Self(Paint {
            image,
            tier: super::admission::tier(finish),
        })
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub(super) struct Lacquer {
    #[uniform(0)]
    settings: Vec4,
    #[texture(1)]
    #[sampler(2)]
    image: Handle<Image>,
}

impl UiMaterial for Lacquer {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/event_preview.wesl".into()
    }
}

type Surfaces = HashMap<(AssetId<Image>, u8, bool), AssetId<Lacquer>>;
pub(super) fn install(app: &mut App) {
    app.add_plugins(UiMaterialPlugin::<Lacquer>::default())
        .add_systems(Update, show);
}

fn show(
    mut commands: Commands,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut assets: ResMut<Assets<Lacquer>>,
    mut shared: Local<Surfaces>,
    previews: Query<(Entity, Ref<CosmeticPreview>, Option<&MaterialNode<Lacquer>>)>,
    mut removed: RemovedComponents<CosmeticPreview>,
    mut was_still: Local<bool>,
) {
    let still = prefs.is_some_and(|value| value.all().reduce_motion);
    let changed = *was_still != still;
    *was_still = still;
    for entity in removed.read() {
        if !previews.contains(entity)
            && let Ok(mut node) = commands.get_entity(entity)
        {
            node.remove::<MaterialNode<Lacquer>>();
        }
    }
    for (entity, preview, current) in &previews {
        if !changed && !preview.is_changed() {
            continue;
        }
        let p = &preview.0;
        let key = (p.image.id(), p.tier, still);
        let handle = shared
            .get(&key)
            .and_then(|id| assets.get_strong_handle(*id))
            .unwrap_or_else(|| {
                shared.retain(|_, id| assets.contains(*id));
                let mat = assets.add(Lacquer {
                    settings: Vec4::new(
                        f32::from(p.tier),
                        crate::cardmat::motion_of(still),
                        0.0,
                        0.0,
                    ),
                    image: p.image.clone(),
                });
                shared.insert(key, mat.id());
                mat
            });
        if current.is_none_or(|node| node.0 != handle) {
            commands.entity(entity).insert(MaterialNode(handle));
        }
    }
}

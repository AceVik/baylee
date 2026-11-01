//! Cached preview materials. Cropping and decoration never touch the full photos.
use baylee_client_core::memorial::PHOTOS;
use bevy::{prelude::*, render::render_resource::AsBindGroup, shader::ShaderRef};

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub(crate) struct PreviewMaterial {
    #[uniform(0)]
    focal: Vec4,
    #[texture(1)]
    #[sampler(2)]
    photo: Handle<Image>,
}
impl UiMaterial for PreviewMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/memorial_preview.wesl".into()
    }
}
#[derive(Resource, Default)]
struct Previews([Option<Handle<PreviewMaterial>>; PHOTOS]);
pub(super) struct PreviewPlugin;
impl Plugin for PreviewPlugin {
    fn build(&self, app: &mut App) {
        if app.world().contains_resource::<AssetServer>() {
            app.add_plugins(UiMaterialPlugin::<PreviewMaterial>::default())
                .init_resource::<Previews>();
        }
    }
}
// The same object-position anchors are used by the website. Keep eyes and ears in frame.
const FOCAL: [(f32, f32); PHOTOS] = [
    (0.5, 0.57),
    (0.5, 0.6),
    (0.47, 0.53),
    (0.48, 0.5),
    (0.5, 0.56),
    (0.45, 0.5),
    (0.61, 0.54),
    (0.43, 0.64),
    (0.5, 0.58),
    (0.5, 0.54),
];
pub(super) fn attach(c: &mut Commands, entity: Entity, photo: usize, available: bool) {
    if !available {
        return;
    }
    c.queue(move |world: &mut World| {
        let cached = world.resource::<Previews>().0[photo].clone();
        let material = cached.unwrap_or_else(|| {
            let image = world
                .resource::<AssetServer>()
                .load(format!("memorial/baylee-{}.jpg", photo + 1));
            let (x, y) = FOCAL[photo];
            let handle = world
                .resource_mut::<Assets<PreviewMaterial>>()
                .add(PreviewMaterial {
                    focal: Vec4::new(x, y, 12.0, 0.15),
                    photo: image,
                });
            world.resource_mut::<Previews>().0[photo] = Some(handle.clone());
            handle
        });
        if let Ok(mut node) = world.get_entity_mut(entity) {
            node.insert(MaterialNode(material));
        }
    });
}

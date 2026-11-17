//! Original modular art. Fixed corner proportions, stretchable wooden rails.
use bevy::{
    prelude::*,
    sprite::{BorderRect, TextureSlicer},
    ui::widget::NodeImageMode,
};
pub(crate) mod glint;

#[derive(Component, Clone, Copy)]
pub(crate) enum Decor {
    Frame,
    Shelf,
}

#[derive(Resource, Default)]
struct Art([Option<Handle<Image>>; 2]);

const PATHS: [&str; 2] = ["tower/shop-frame.png", "tower/shop-shelf.png"];

pub(crate) fn install(app: &mut App) {
    crate::sound::tower::install(app);
    if app.world().contains_resource::<AssetServer>() && !app.world().contains_resource::<Art>() {
        let preloaded = {
            let server = app.world().resource::<AssetServer>();
            Art(PATHS.map(|path| Some(server.load(path))))
        };
        app.insert_resource(preloaded).add_systems(
            PostUpdate,
            (dress, ApplyDeferred)
                .chain()
                .before(bevy::ui::UiSystems::Prepare)
                .before(bevy::ui::UiSystems::Content)
                .before(bevy::ui::UiSystems::Layout)
                .before(bevy::ui::UiSystems::Stack),
        );
        glint::install(app);
    }
}

fn dress(
    mut commands: Commands,
    added: Query<(Entity, &Decor), Added<Decor>>,
    mut art: ResMut<Art>,
    server: Res<AssetServer>,
) {
    for (entity, decor) in &added {
        let (index, border, rect, max_corner_scale) = match decor {
            Decor::Frame => (0, BorderRect::axes(280.0, 230.0), None, 0.135),
            Decor::Shelf => (
                1,
                BorderRect::axes(130.0, 18.0),
                Some(Rect::new(0.0, 270.0, 2172.0, 460.0)),
                0.25,
            ),
        };
        let image = art.0[index]
            .get_or_insert_with(|| server.load(PATHS[index]))
            .clone();
        commands.entity(entity).try_insert(ImageNode {
            image,
            rect,
            image_mode: NodeImageMode::Sliced(TextureSlicer {
                border,
                max_corner_scale,
                ..default()
            }),
            ..default()
        });
    }
}

//! Original modular art. Fixed corner proportions, stretchable wooden rails.
use bevy::{
    prelude::*,
    sprite::{BorderRect, TextureSlicer},
    ui::widget::NodeImageMode,
};

#[derive(Component, Clone, Copy)]
pub(crate) enum Decor {
    Frame,
    Shelf,
}

#[derive(Resource, Default)]
struct Art([Option<Handle<Image>>; 2]);

pub(crate) fn install(app: &mut App) {
    if app.world().contains_resource::<AssetServer>() {
        app.init_resource::<Art>().add_systems(PostUpdate, dress);
    }
}

fn dress(
    mut commands: Commands,
    added: Query<(Entity, &Decor), Added<Decor>>,
    mut art: ResMut<Art>,
    server: Res<AssetServer>,
) {
    for (entity, decor) in &added {
        let (index, path, border, rect) = match decor {
            Decor::Frame => (
                0,
                "tower/shop-frame.png",
                BorderRect::axes(280.0, 230.0),
                None,
            ),
            Decor::Shelf => (
                1,
                "tower/shop-shelf.png",
                BorderRect::axes(130.0, 18.0),
                Some(Rect::new(0.0, 270.0, 2172.0, 460.0)),
            ),
        };
        let image = art.0[index]
            .get_or_insert_with(|| server.load(path))
            .clone();
        commands.entity(entity).try_insert(ImageNode {
            image,
            rect,
            image_mode: NodeImageMode::Sliced(TextureSlicer {
                border,
                max_corner_scale: 0.25,
                ..default()
            }),
            ..default()
        });
    }
}

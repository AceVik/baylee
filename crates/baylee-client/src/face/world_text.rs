//! Render Bevy's shaped glyphs on the 3D table. Text2d itself belongs to the
//! Transparent2d phase and cannot draw through the table's Camera3d.
//! Layout, fallback fonts and spans still belong to Bevy's text pipeline.
use std::collections::HashMap;

mod shape;

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    sprite::{Anchor, update_text2d_layout},
    text::{ComputedTextBlock, PositionedGlyph, TextBounds, TextLayoutInfo},
};

#[derive(Component, Default)]
struct Drawn(Vec<Entity>);

#[derive(Component)]
struct MeshRoot(Entity);

fn prepare(
    mut commands: Commands,
    roots: Query<Entity, (With<super::WorldFace>, Without<MeshRoot>)>,
) {
    for root in &roots {
        // Keep glyph geometry beneath a stable child: changing the text root's
        // direct Children invalidates Bevy's layout and would cause a redraw
        // feedback loop. Install this before its first change detection pass.
        let surface = commands
            .spawn((Transform::IDENTITY, Visibility::Inherited, ChildOf(root)))
            .id();
        commands.entity(root).insert(MeshRoot(surface));
    }
}

#[derive(Resource, Default)]
struct Materials(HashMap<AssetId<Image>, AssetId<StandardMaterial>>);

#[derive(Default)]
struct Batch {
    positions: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl Batch {
    fn glyph(
        &mut self,
        glyph: &PositionedGlyph,
        image_size: Vec2,
        origin: Vec2,
        scale: f32,
        ink: LinearRgba,
    ) {
        let half = glyph.atlas_info.rect.size() * 0.5;
        let rect = Rect::from_center_half_size(glyph.position, half);
        let first = u32::try_from(self.positions.len()).expect("bounded shaped card text");
        // CCW as seen from above the card. Glyph coordinates grow downwards;
        // the local card's y grows up. Texture coordinates keep their top-left.
        for (position, uv) in [
            (rect.min, glyph.atlas_info.rect.min),
            (
                Vec2::new(rect.min.x, rect.max.y),
                Vec2::new(glyph.atlas_info.rect.min.x, glyph.atlas_info.rect.max.y),
            ),
            (rect.max, glyph.atlas_info.rect.max),
            (
                Vec2::new(rect.max.x, rect.min.y),
                Vec2::new(glyph.atlas_info.rect.max.x, glyph.atlas_info.rect.min.y),
            ),
        ] {
            self.positions.push(
                (origin + position * Vec2::new(scale, -scale))
                    .extend(0.0)
                    .to_array(),
            );
            self.uv.push((uv / image_size).to_array());
            self.colors.push(ink.to_f32_array());
        }
        self.indices
            .extend([first, first + 1, first + 2, first, first + 2, first + 3]);
    }

    fn mesh(self) -> Mesh {
        let normals = vec![[0.0, 0.0, 1.0]; self.positions.len()];
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(Indices::U32(self.indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uv)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
    }
}

#[allow(clippy::type_complexity)] // ECS query keeps one explicit change filter
fn draw(
    mut commands: Commands,
    text: Query<
        (
            Entity,
            &TextLayoutInfo,
            &ComputedTextBlock,
            &TextBounds,
            &Anchor,
            &TextColor,
            &MeshRoot,
            Option<&Drawn>,
        ),
        (
            With<super::WorldFace>,
            Or<(Changed<TextLayoutInfo>, Changed<TextColor>, Changed<Anchor>)>,
        ),
    >,
    colors: Query<&TextColor>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cache: ResMut<Materials>,
) {
    for (entity, layout, block, bounds, anchor, color, surface, drawn) in &text {
        if let Some(drawn) = drawn {
            for &child in &drawn.0 {
                commands.entity(child).despawn();
            }
        }
        let scale = layout.scale_factor.recip();
        if !scale.is_finite() || scale <= 0.0 {
            continue;
        }
        let size = Vec2::new(
            bounds.width.unwrap_or(layout.size.x * scale),
            bounds.height.unwrap_or(layout.size.y * scale),
        );
        let origin = (Anchor::TOP_LEFT.as_vec() - anchor.as_vec()) * size;
        let mut batches: HashMap<AssetId<Image>, Batch> = HashMap::new();
        for glyph in &layout.glyphs {
            let Some(image) = images.get(glyph.atlas_info.texture) else {
                continue;
            };
            let ink = block
                .entities()
                .get(glyph.section_index as usize)
                .and_then(|section| colors.get(section.entity).ok())
                .unwrap_or(color)
                .0
                .to_linear();
            batches.entry(glyph.atlas_info.texture).or_default().glyph(
                glyph,
                image.size_f32(),
                origin,
                scale,
                ink,
            );
        }
        let mut children = Vec::with_capacity(batches.len());
        for (image, batch) in batches {
            // Bound the cache across repeated games/fonts. Live glyph meshes
            // retain their own material handles when the cache is recycled.
            if cache.0.len() >= 128 && !cache.0.contains_key(&image) {
                cache.0.clear();
            }
            let material = cache
                .0
                .get(&image)
                .and_then(|id| materials.get_strong_handle(*id))
                .unwrap_or_else(|| {
                    let handle = materials.add(StandardMaterial {
                        base_color_texture: images.get_strong_handle(image),
                        unlit: true,
                        alpha_mode: AlphaMode::Blend,
                        ..default()
                    });
                    // Cache IDs, not ownership: leaving a table releases its
                    // materials/atlas references with the last glyph mesh.
                    cache.0.insert(image, handle.id());
                    handle
                });
            children.push(
                commands
                    .spawn((
                        Mesh3d(meshes.add(batch.mesh())),
                        MeshMaterial3d(material),
                        Transform::IDENTITY,
                        ChildOf(surface.0),
                        Pickable::IGNORE,
                        bevy::light::NotShadowCaster,
                    ))
                    .id(),
            );
        }
        commands.entity(entity).insert(Drawn(children));
    }
}

pub(crate) fn install(app: &mut App) {
    if app.get_sub_app(bevy::render::RenderApp).is_none() {
        return;
    }
    app.init_resource::<Materials>()
        .add_systems(
            PostUpdate,
            prepare.before(bevy::text::detect_text_needs_rerender),
        )
        .add_systems(
            PostUpdate,
            (shape::shape, draw).chain().after(update_text2d_layout),
        );
}

#[cfg(test)]
mod tests;

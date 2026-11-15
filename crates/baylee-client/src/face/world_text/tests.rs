use super::*;
use bevy::text::GlyphAtlasInfo;

#[test]
fn real_fonts_shape_and_draw_without_any_2d_camera_then_rest() {
    use bevy::text::{
        FontAtlasSet, FontCx, LayoutCx, RemSize, ScaleCx, TextIterScratch, TextPipeline,
        detect_text_needs_rerender, load_font_assets_into_font_collection,
    };
    let mut app = App::new();
    app.init_resource::<Assets<Font>>()
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<Assets<TextureAtlasLayout>>()
        .init_resource::<Materials>()
        .init_resource::<FontAtlasSet>()
        .init_resource::<FontCx>()
        .init_resource::<LayoutCx>()
        .init_resource::<ScaleCx>()
        .init_resource::<TextIterScratch>()
        .init_resource::<RemSize>()
        .init_resource::<TextPipeline>()
        .add_systems(
            Update,
            (
                load_font_assets_into_font_collection,
                prepare,
                detect_text_needs_rerender,
                shape::shape,
                draw,
            )
                .chain(),
        );
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/fonts/AlegreyaSans-Regular.ttf"),
    )
    .expect("shipped font");
    let font = app
        .world_mut()
        .resource_mut::<Assets<Font>>()
        .add(Font::from_bytes(bytes));
    let root = app
        .world_mut()
        .spawn((
            super::super::WorldFace,
            Text2d::new("Root Reservoir"),
            TextFont {
                font: font.into(),
                font_size: bevy::text::FontSize::Px(20.0),
                ..default()
            },
            bevy::camera::visibility::RenderLayers::none(),
            Anchor::TOP_LEFT,
        ))
        .id();
    app.update();
    let info = app
        .world()
        .get::<TextLayoutInfo>(root)
        .expect("real layout");
    assert!(
        !info.glyphs.is_empty(),
        "the table has no 2D camera to drive layout"
    );
    assert!(info.size.x > 20.0 && info.size.y > 10.0);
    assert!(
        !app.world()
            .get::<Drawn>(root)
            .expect("3D draw")
            .0
            .is_empty()
    );
    let count = app.world().resource::<Assets<Mesh>>().len();
    let tick = app.world().read_change_tick();
    app.update();
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), count);
    assert!(
        !app.world()
            .resource_ref::<Assets<Mesh>>()
            .last_changed()
            .is_newer_than(tick, app.world().read_change_tick())
    );
    assert!(
        !app.world()
            .resource_ref::<Assets<Image>>()
            .last_changed()
            .is_newer_than(tick, app.world().read_change_tick())
    );
}

fn glyph(texture: AssetId<Image>) -> PositionedGlyph {
    PositionedGlyph {
        position: Vec2::new(4.0, 6.0),
        atlas_info: GlyphAtlasInfo {
            texture,
            rect: Rect::new(20.0, 30.0, 28.0, 42.0),
            offset: Vec2::ZERO,
            is_alpha_mask: true,
        },
        section_index: 0,
        line_index: 0,
    }
}

#[test]
fn glyphs_keep_upright_uvs_correct_scale_and_front_facing_triangles() {
    let mut batch = Batch::default();
    batch.glyph(
        &glyph(AssetId::default()),
        Vec2::splat(100.0),
        Vec2::new(-2.0, 3.0),
        0.5,
        LinearRgba::BLACK,
    );
    assert_eq!(
        batch.positions,
        [
            [-2.0, 3.0, 0.0],
            [-2.0, -3.0, 0.0],
            [2.0, -3.0, 0.0],
            [2.0, 3.0, 0.0]
        ]
    );
    assert_eq!(
        batch.uv,
        [[0.2, 0.3], [0.2, 0.42], [0.28, 0.42], [0.28, 0.3]]
    );
    for triangle in batch.indices.as_chunks::<3>().0 {
        let point = |i: u32| Vec3::from_array(batch.positions[i as usize]);
        assert!(
            (point(triangle[1]) - point(triangle[0]))
                .cross(point(triangle[2]) - point(triangle[0]))
                .z
                > 0.0
        );
    }
    for color in &batch.colors {
        assert_eq!(*color, LinearRgba::BLACK.to_f32_array());
    }
    let mesh = batch.mesh();
    assert_eq!(mesh.count_vertices(), 4);
}

#[test]
fn shaped_text_shares_atlases_updates_on_changes_and_removes_old_geometry() {
    let mut app = App::new();
    app.init_resource::<Assets<Image>>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<Materials>()
        .add_systems(Update, (prepare, draw).chain());
    let image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    let mut roots = Vec::new();
    for _ in 0..2 {
        roots.push(
            app.world_mut()
                .spawn((
                    super::super::WorldFace,
                    TextLayoutInfo {
                        glyphs: vec![glyph(image.id())],
                        size: Vec2::new(8.0, 12.0),
                        ..default()
                    },
                    ComputedTextBlock::default(),
                    TextBounds::default(),
                    Anchor::TOP_LEFT,
                    TextColor(Color::BLACK),
                ))
                .id(),
        );
    }
    app.update();
    assert_eq!(app.world().resource::<Assets<StandardMaterial>>().len(), 1);
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 2);
    let first = app.world().get::<Drawn>(roots[0]).expect("drawn").0[0];
    assert_eq!(
        *app.world()
            .get::<Pickable>(first)
            .expect("not an input target"),
        Pickable::IGNORE
    );
    assert_eq!(
        app.world()
            .get::<ChildOf>(first)
            .expect("text parent")
            .parent(),
        app.world()
            .get::<MeshRoot>(roots[0])
            .expect("stable mesh parent")
            .0
    );
    let tick = app.world().read_change_tick();
    app.update();
    assert!(
        !app.world()
            .resource_ref::<Assets<Mesh>>()
            .last_changed()
            .is_newer_than(tick, app.world().read_change_tick())
    );
    assert!(
        !app.world()
            .resource_ref::<Assets<StandardMaterial>>()
            .last_changed()
            .is_newer_than(tick, app.world().read_change_tick())
    );
    app.world_mut()
        .get_mut::<TextLayoutInfo>(roots[0])
        .expect("layout")
        .glyphs
        .clear();
    app.update();
    assert!(app.world().get_entity(first).is_err());
    assert!(
        app.world()
            .get::<Drawn>(roots[0])
            .expect("cleared")
            .0
            .is_empty()
    );
    let other = app.world().get::<Drawn>(roots[1]).expect("kept").0[0];
    app.world_mut().despawn(roots[1]);
    assert!(
        app.world().get_entity(other).is_err(),
        "face cleanup removes its mesh too"
    );
}

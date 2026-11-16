use super::*;

#[test]
fn rebuilt_portraits_share_materials_and_idle_frames_never_upload() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_resource::<Assets<Material>>()
        .init_resource::<Cache>()
        .add_systems(Update, dress);
    for _ in 0..3 {
        let entities: Vec<_> = Family::ALL
            .into_iter()
            .flat_map(|family| {
                [
                    Rarity::Common,
                    Rarity::Uncommon,
                    Rarity::Rare,
                    Rarity::MythicRare,
                ]
                .map(|rarity| Portrait(Goods::Relic(Definition { family, rarity })))
            })
            .map(|portrait| app.world_mut().spawn(portrait).id())
            .collect();
        app.update();
        assert_eq!(
            app.world().resource::<Assets<Material>>().len(),
            Family::ALL.len() * 4
        );
        let image_ids: std::collections::HashSet<_> = app
            .world()
            .resource::<Assets<Material>>()
            .iter()
            .filter_map(|(_, material)| {
                material.portrait.as_ref().map(|image| {
                    (
                        image.id(),
                        material.params.atlas.to_array().map(f32::to_bits),
                    )
                })
            })
            .collect();
        assert_eq!(
            image_ids.len(),
            Family::ALL.len(),
            "all four tiers share one unique painting/atlas region per family"
        );
        for entity in entities {
            assert!(app.world().get::<MaterialNode<Material>>(entity).is_some());
            app.world_mut().despawn(entity);
        }
    }
    app.update();
    let tick = app.world().read_change_tick();
    app.update();
    assert!(
        !app.world()
            .resource_ref::<Assets<Material>>()
            .last_changed()
            .is_newer_than(tick, app.world().read_change_tick())
    );
}

#[test]
fn item_shader_validates_with_native_frontend() {
    crate::cardmat::tests::check_wgsl(
        include_str!("ui.wesl"),
        &format!(
            "struct UiVertexOutput {{ @location(0) uv: vec2<f32>, @builtin(position) position: vec4<f32> }}; struct Globals {{ time: f32 }}; {}",
            include_str!("common.wesl")
        ),
    );
}

#[test]
fn table_shader_validates_with_native_frontend() {
    crate::cardmat::tests::check_wgsl(
        include_str!("world.wesl"),
        &format!(
            "struct VertexOutput {{ @location(2) uv: vec2<f32>, @builtin(position) position: vec4<f32> }}; struct Globals {{ time: f32 }}; @group(0) @binding(11) var<uniform> globals: Globals; {}",
            include_str!("common.wesl")
        ),
    );
}

#[test]
fn reduced_motion_updates_cached_portraits_once_and_covers_new_ones() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_resource::<Assets<Material>>()
        .init_resource::<Cache>()
        .init_resource::<crate::prefs::Prefs>()
        .add_systems(Update, dress);
    app.world_mut()
        .spawn(Portrait(Goods::Potion(Potion::Healing)));
    app.update();
    assert!(
        app.world()
            .resource::<Assets<Material>>()
            .iter()
            .all(|(_, material)| (material.params.shape.z - 1.0).abs() < f32::EPSILON)
    );
    app.world_mut()
        .resource_mut::<crate::prefs::Prefs>()
        .edit()
        .reduce_motion = true;
    app.update();
    app.world_mut().spawn(Portrait(Goods::Mythic));
    app.update();
    assert_eq!(app.world().resource::<Assets<Material>>().len(), 2);
    assert!(
        app.world()
            .resource::<Assets<Material>>()
            .iter()
            .all(|(_, material)| material.params.shape.z == 0.0)
    );
    let tick = app.world().read_change_tick();
    app.update();
    assert!(
        !app.world()
            .resource_ref::<Assets<Material>>()
            .last_changed()
            .is_newer_than(tick, app.world().read_change_tick())
    );
    app.world_mut()
        .resource_mut::<crate::prefs::Prefs>()
        .edit()
        .reduce_motion = false;
    app.update();
    assert!(
        app.world()
            .resource::<Assets<Material>>()
            .iter()
            .all(|(_, material)| (material.params.shape.z - 1.0).abs() < f32::EPSILON)
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn shipped_item_images_have_transparency_and_bounded_dimensions() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    for path in PICTURES.into_iter().chain(ATLASES) {
        let image = image::open(root.join(path)).expect("shipped original portrait");
        assert!(
            image.color().has_alpha(),
            "{path} must keep its generated alpha"
        );
        assert_eq!(image.width(), image.height(), "{path} is square");
        assert!(image.width() <= 1280, "bounded texture allocation");
        let rgba = image.into_rgba8();
        let mut transparent = 0;
        let mut opaque = 0;
        for pixel in rgba.pixels() {
            transparent += usize::from(pixel[3] == 0);
            opaque += usize::from(pixel[3] >= 250);
        }
        let area = rgba.width() as usize * rgba.height() as usize;
        assert!(transparent > area / 4, "{path} is genuinely freestanding");
        if PICTURES.contains(&path) {
            assert!(opaque > area / 10, "{path} has a visible object");
        }
        // Atlas visibility is checked per sampled cell below. Transparent
        // gutters and the unused outer apron are deliberately not object area.
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn each_runic_recipe_samples_a_visible_independent_atlas_cell() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    for (vessel, path) in ATLASES.into_iter().enumerate() {
        let rgba = image::open(root.join(path)).unwrap().into_rgba8();
        for sigil in 0..16 {
            let uv = params(22 + sigil * 10 + vessel, 0, 0.0).atlas;
            let x = (uv.x * rgba.width() as f32).round() as u32;
            let y = (uv.y * rgba.height() as f32).round() as u32;
            let w = (uv.z * rgba.width() as f32).floor() as u32;
            let h = (uv.w * rgba.height() as f32).floor() as u32;
            assert!(x + w <= rgba.width() && y + h <= rgba.height());
            let pixels = image::GenericImageView::view(&rgba, x, y, w, h);
            let opaque = image::GenericImageView::pixels(&*pixels)
                .filter(|(_, _, pixel)| pixel[3] >= 250)
                .count();
            assert!(opaque > (w * h) as usize / 50, "{path} cell {sigil}");
        }
    }
}

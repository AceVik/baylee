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
fn frozen_emphasis_caches_distinct_rime_material_without_idle_writes() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_resource::<Assets<Material>>()
        .init_resource::<Cache>()
        .add_systems(Update, dress);
    let goods = Goods::Relic(Definition {
        family: Family::GildedAcorn,
        rarity: Rarity::Rare,
    });
    let normal = app
        .world_mut()
        .spawn((Portrait(goods), Emphasis::None))
        .id();
    let frozen = app
        .world_mut()
        .spawn((Portrait(goods), Emphasis::None.frozen_if(true)))
        .id();
    app.update();
    assert_eq!(app.world().resource::<Assets<Material>>().len(), 2);
    let normal_mat = app
        .world()
        .resource::<Assets<Material>>()
        .get(&app.world().get::<MaterialNode<Material>>(normal).unwrap().0)
        .unwrap();
    let frozen_mat = app
        .world()
        .resource::<Assets<Material>>()
        .get(&app.world().get::<MaterialNode<Material>>(frozen).unwrap().0)
        .unwrap();
    assert!(normal_mat.params.shape.w.abs() < f32::EPSILON);
    assert!((frozen_mat.params.shape.w - 128.0).abs() < f32::EPSILON);
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
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
#[allow(clippy::many_single_char_names)] // Image coordinates (u, v, w, h, a).
fn every_item_portrait_is_optically_centered_with_balanced_margins_and_disjoint_cells() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");

    let check_centered = |rgba: &image::RgbaImage, kind: usize, label: &str| {
        let p = params(kind, 0, 0.0);
        assert!(
            (p.atlas.z - p.atlas.w).abs() < 1e-6,
            "{label} window must be strictly 1:1 square so aspect ratio never distorts"
        );
        let w = rgba.width() as f32;
        let h = rgba.height() as f32;
        let x0 = (p.clip.x.max(p.atlas.x) * w).floor().max(0.0) as u32;
        let y0 = (p.clip.y.max(p.atlas.y) * h).floor().max(0.0) as u32;
        let x1 = ((p.clip.z.min(p.atlas.x + p.atlas.z)) * w).ceil().min(w) as u32;
        let y1 = ((p.clip.w.min(p.atlas.y + p.atlas.w)) * h).ceil().min(h) as u32;
        assert!(x0 < x1 && y0 < y1, "{label} empty clipped window");

        let mut min_u = 1.0_f32;
        let mut max_u = 0.0_f32;
        let mut min_v = 1.0_f32;
        let mut max_v = 0.0_f32;
        let mut opaque = 0usize;
        for py in y0..y1 {
            for px in x0..x1 {
                let a = rgba.get_pixel(px, py)[3];
                if a >= 250 {
                    opaque += 1;
                }
                if a >= 20 {
                    let u = ((px as f32 + 0.5) / w - p.atlas.x) / p.atlas.z;
                    let v = ((py as f32 + 0.5) / h - p.atlas.y) / p.atlas.w;
                    min_u = min_u.min(u);
                    max_u = max_u.max(u);
                    min_v = min_v.min(v);
                    max_v = max_v.max(v);
                }
            }
        }
        assert!(
            opaque > ((x1 - x0) * (y1 - y0)) as usize / 60,
            "{label} lacks visible object area"
        );
        // Bounding-box midpoint in `image_uv` [0, 1] sits near 0.5 (allowing up
        // to 0.06 optical pull toward alpha center-of-mass for top-heavy scepters/horns),
        // with two-sided breathing room on all four edges.
        let mid_u = f32::midpoint(min_u, max_u);
        let mid_v = f32::midpoint(min_v, max_v);
        assert!(
            (mid_u - 0.5).abs() < 0.065 && (mid_v - 0.5).abs() < 0.065,
            "{label} off-center in image_uv: u=[{min_u:.3}, {max_u:.3}] mid={mid_u:.3}, v=[{min_v:.3}, {max_v:.3}] mid={mid_v:.3}"
        );
        assert!(
            min_u > 0.03 && max_u < 0.97 && min_v > 0.03 && max_v < 0.97,
            "{label} crowds frame edge: u=[{min_u:.3}, {max_u:.3}], v=[{min_v:.3}, {max_v:.3}]"
        );
        let max_span = (max_u - min_u).max(max_v - min_v);
        assert!(
            (0.72..=0.94).contains(&max_span),
            "{label} inconsistent visual fill {max_span:.3}"
        );
    };

    for (idx, path) in PICTURES.into_iter().enumerate() {
        let rgba = image::open(root.join(path)).unwrap().into_rgba8();
        check_centered(&rgba, idx + 1, path);
    }
    for (vessel, path) in ATLASES.into_iter().enumerate() {
        let rgba = image::open(root.join(path)).unwrap().into_rgba8();
        for sigil in 0..16 {
            check_centered(
                &rgba,
                22 + sigil * 10 + vessel,
                &format!("{path} sigil {sigil}"),
            );
        }
    }
}

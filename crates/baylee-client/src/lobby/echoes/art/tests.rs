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
        let entities: Vec<_> = [
            Family::Heartwood,
            Family::Satchel,
            Family::Compass,
            Family::Grove,
        ]
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
        assert_eq!(app.world().resource::<Assets<Material>>().len(), 16);
        let image_ids: std::collections::HashSet<_> = app
            .world()
            .resource::<Assets<Material>>()
            .iter()
            .filter_map(|(_, material)| material.portrait.as_ref().map(Handle::id))
            .collect();
        assert_eq!(
            image_ids.len(),
            4,
            "all four tiers share their family's image"
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
        include_str!("../art.wesl"),
        "struct UiVertexOutput { @location(0) uv: vec2<f32>, @builtin(position) position: vec4<f32> }; struct Globals { time: f32 };",
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
    for path in PICTURES {
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
        assert!(opaque > area / 10, "{path} has a visible object");
    }
}

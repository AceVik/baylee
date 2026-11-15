use super::super::{Definition, Family, Goods, Rarity};
use super::*;

#[test]
fn table_materials_share_images_meshes_and_do_not_upload_idle_frames() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_resource::<Assets<Material>>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Cache>()
        .init_resource::<crate::prefs::Prefs>()
        .add_systems(Update, dress);
    for _ in 0..3 {
        let mut entities = Vec::new();
        for rarity in Rarity::ALL {
            for veil in 0..8 {
                entities.push(
                    app.world_mut()
                        .spawn(Portrait {
                            goods: super::super::Portrait(Goods::Relic(Definition {
                                family: Family::RootReservoir,
                                rarity,
                            })),
                            veil,
                        })
                        .id(),
                );
            }
        }
        app.update();
        assert_eq!(app.world().resource::<Assets<Material>>().len(), 32);
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 1);
        let images: std::collections::HashSet<_> = app
            .world()
            .resource::<Assets<Material>>()
            .iter()
            .filter_map(|(_, m)| m.portrait.as_ref().map(Handle::id))
            .collect();
        assert_eq!(images.len(), 1);
        for entity in entities {
            let p = app.world().get::<Portrait>(entity).expect("portrait");
            let handle = &app
                .world()
                .get::<MeshMaterial3d<Material>>(entity)
                .expect("dressed")
                .0;
            let material = app
                .world()
                .resource::<Assets<Material>>()
                .get(handle)
                .expect("material");
            assert!((material.params.shape.w - f32::from(p.veil)).abs() < f32::EPSILON);
            app.world_mut().despawn(entity);
        }
    }
    app.world_mut()
        .resource_mut::<crate::prefs::Prefs>()
        .edit()
        .reduce_motion = true;
    app.update();
    assert!(
        app.world()
            .resource::<Assets<Material>>()
            .iter()
            .all(|(_, m)| m.params.shape.z == 0.0)
    );
    let tick = app.world().read_change_tick();
    app.update();
    assert!(
        !app.world()
            .resource_ref::<Assets<Material>>()
            .last_changed()
            .is_newer_than(tick, app.world().read_change_tick())
    );
    assert!(
        !app.world()
            .resource_ref::<Assets<Mesh>>()
            .last_changed()
            .is_newer_than(tick, app.world().read_change_tick())
    );
}

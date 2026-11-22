use super::*;
use crate::rank_art::promotion::Step;
use std::collections::HashSet;

#[test]
fn every_approved_painting_has_one_portable_asset_and_cache_key() {
    let mut paths = HashSet::new();
    let mut keys = HashSet::new();
    for style in &catalog::STYLES[..usize::from(catalog::UNRANKED)] {
        for deck in [false, true] {
            for division in 0..=style.divisions {
                let art = RankArt::badge(style.english, Some(division), deck, None).unwrap();
                assert_eq!(
                    art,
                    RankArt::badge(style.slug, Some(division), deck, None).unwrap()
                );
                let path = art.path();
                let path_on_disk = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../art/baylee/ranks")
                    .join(
                        path.strip_prefix("embedded://baylee_client/ranks/")
                            .unwrap(),
                    );
                assert!(path_on_disk.is_file(), "{}", path_on_disk.display());
                assert!(paths.insert(path));
                assert!(keys.insert(art));
            }
        }
    }
    for place in [1, 2, 3] {
        let art = RankArt::badge("champion", None, false, Some(place)).unwrap();
        assert!(paths.insert(art.path()));
        assert!(keys.insert(art));
        assert_eq!(
            RankArt::badge("champion", None, true, Some(place))
                .unwrap()
                .podium,
            0
        );
    }
    assert_eq!((paths.len(), keys.len()), (73, 73));
    assert!(RankArt::badge("../../other", None, false, None).is_none());
    assert_eq!(
        RankArt::frame("diamond", Some(1), Some(1)).unwrap().podium,
        0
    );
    assert_eq!(RankArt::frame("iron", Some(250), None).unwrap().division, 0);
    assert_ne!(
        RankArt::deck_frame("unranked", None),
        RankArt::frame("unranked", None, None)
    );
}

#[test]
fn only_confirmed_climbs_and_new_podium_places_are_promotions() {
    let art = |tier: &str, div, place| RankArt::badge(tier, div, false, place).unwrap();
    let step = |before, after| {
        Promotion {
            before,
            after,
            label: String::new(),
        }
        .step()
    };
    let gold = art("gold", Some(4), None);
    let gold_iii = art("gold", Some(3), None);
    let bronze = art("bronze", Some(1), None);
    let neutral = art("unranked", None, None);
    let champ = |place| art("champion", None, Some(place));
    assert_eq!(step(neutral, bronze), Some(Step::League));
    assert_eq!(step(bronze, gold), Some(Step::League));
    assert_eq!(step(gold, gold_iii), Some(Step::Division));
    assert_eq!(step(gold_iii, gold), None);
    assert_eq!(step(gold, gold), None);
    assert_eq!(step(gold, neutral), None);
    assert_eq!(step(gold, art("master", None, None)), Some(Step::Elite));
    assert_eq!(step(champ(17), champ(4)), None);
    assert_eq!(step(champ(4), champ(3)), Some(Step::Elite));
    assert_eq!(step(champ(3), champ(2)), Some(Step::Elite));
    assert_eq!(step(champ(2), champ(1)), Some(Step::Elite));
    assert_eq!(step(champ(1), champ(2)), None);
}

#[test]
fn rank_shader_entry_points_validate_on_native_frontend() {
    let ui = "struct UiVertexOutput { @location(0) uv: vec2<f32>, @builtin(position) position: vec4<f32> }; struct Globals { time: f32 };";
    crate::cardmat::tests::check_wgsl(include_str!("corona.wesl"), ui);
    // The real WESL linker strips module prefixes. Use distinct names here for the
    // standalone gallery entry points and the portrait library's private helpers.
    let gallery = include_str!("../../../../art/baylee/ranks/finish.wgsl");
    let portrait = include_str!("portrait.wesl").replace("TAU", "PORTRAIT_TAU");
    let ui = format!("{ui}\n{gallery}\n{portrait}\n");
    crate::cardmat::tests::check_wgsl(include_str!("surface.wesl"), &ui);
}

#[test]
fn profile_rows_share_frame_material_and_standing_idle_uploads_nothing() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Material>()
        .init_resource::<Paints>()
        .add_systems(Update, dress);
    let gold = RankArt::frame("gold", Some(4), None).unwrap();
    let first = app.world_mut().spawn(gold).id();
    let second = app.world_mut().spawn(gold).id();
    app.update();
    let handle = |app: &App, entity| {
        app.world()
            .get::<MaterialNode<Material>>(entity)
            .unwrap()
            .0
            .id()
    };
    assert_eq!(handle(&app, first), handle(&app, second));
    app.world_mut()
        .entity_mut(first)
        .insert(RankArt::frame("gold", Some(3), None).unwrap());
    app.update();
    assert_ne!(handle(&app, first), handle(&app, second));
    assert_eq!(app.world().resource::<Assets<Material>>().len(), 2);
    let tick = app.world().read_change_tick();
    app.update();
    assert!(
        !app.world()
            .resource_ref::<Assets<Material>>()
            .last_changed()
            .is_newer_than(tick, app.world().read_change_tick())
    );
}

//! Run the actual scene system: cached content must still follow new public views.

use super::*;
use baylee_client_core::test_support::{ViewBuilder, token};

fn app() -> App {
    bevy::tasks::IoTaskPool::get_or_init(Default::default);
    let mut app = App::new();
    app.add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<Assets<CardMaterial>>()
        .init_resource::<Assets<MarksMaterial>>()
        .init_resource::<Assets<BadgeMaterial>>()
        .init_resource::<Assets<PlateMaterial>>()
        .init_resource::<Assets<FloorMaterial>>()
        .init_resource::<Assets<ShellMaterial>>()
        .init_resource::<Time>()
        .init_resource::<ZoneWatch>()
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::settings::ClientSettings>()
        .init_resource::<crate::face::FaceMode>()
        .init_resource::<crate::cardtext::CardTexts>()
        .init_resource::<crate::sheen::Sheen>()
        .init_resource::<ShownRig>();
    let textures = CardTextures::new(
        &mut app.world_mut().resource_mut::<Assets<Image>>(),
        1 << 20,
    );
    let mut duel =
        crate::table::flying_tests::duel(vec![crate::table::flying_tests::creature(1, Vec::new())]);
    duel.statics = Some(baylee_view::GameStatic {
        view_version: baylee_view::VIEW_VERSION,
        game_id: "scene-cache-test".into(),
        your_seat: PlayerId::new(0),
        seats: Vec::new(),
        prints: Vec::new(),
        decision_secs: None,
        reconnect_secs: None,
    });
    duel.receive_view(
        ViewBuilder::new(2)
            .with_battlefield(0, [token(1, 0, "First face", 2, 2)])
            .build(),
    );
    app.insert_resource(duel)
        .insert_resource(textures)
        .insert_resource(SceneIndex {
            quad: Some(Handle::default()),
            ..default()
        })
        .add_systems(Update, sync_scene);
    app
}

fn words(app: &mut App) -> Vec<String> {
    app.world_mut()
        .query::<&Text2d>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect()
}

#[test]
fn a_face_arrives_with_fonts_then_refreshes_on_a_same_sequence_replay_view() {
    let mut app = app();
    app.update();
    assert!(words(&mut app).is_empty(), "fonts have not arrived yet");
    app.init_resource::<crate::hud::UiFonts>();
    app.update();
    assert!(words(&mut app).iter().any(|s| s.contains("First face")));
    let first_card = app.world().resource::<SceneIndex>().cards[&ObjectId::new(1, 0)];
    let first_text = app.world().resource::<SceneIndex>().faces[&ObjectId::new(1, 0)]
        .texts
        .clone();
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(
        app.world().resource::<SceneIndex>().faces[&ObjectId::new(1, 0)].texts,
        first_text,
        "an unchanged frame must preserve text entities"
    );

    // Same sequence and object handle, different authorized replay perspective.
    let seq = app.world().resource::<Duel>().view.as_ref().unwrap().seq;
    let mut view = ViewBuilder::new(2)
        .with_battlefield(0, [token(1, 0, "Other visible face", 3, 3)])
        .build();
    view.seq = seq;
    app.world_mut().resource_mut::<Duel>().receive_view(view);
    app.update();
    let drawn = words(&mut app);
    assert!(
        drawn.iter().any(|s| s.contains("Other visible face")),
        "{drawn:?}"
    );
    assert!(
        !drawn.iter().any(|s| s.contains("First face")),
        "stale text must leave with the old perspective"
    );
    assert_eq!(
        app.world().resource::<SceneIndex>().cards[&ObjectId::new(1, 0)],
        first_card
    );

    // Losing visibility of the object's identity must not retain its face.
    app.world_mut()
        .resource_mut::<Duel>()
        .receive_view(ViewBuilder::new(2).build());
    app.update();
    assert!(words(&mut app).is_empty());
}

#[test]
fn hovering_a_cached_card_updates_its_target_without_rebuilding_its_words() {
    let mut app = app();
    app.init_resource::<crate::hud::UiFonts>();
    app.update();
    app.update();
    let id = ObjectId::new(1, 0);
    let entity = app.world().resource::<SceneIndex>().cards[&id];
    let text = app.world().resource::<SceneIndex>().faces[&id]
        .texts
        .clone();
    let resting = app.world().get::<Motion>(entity).unwrap().target;
    app.world_mut().resource_mut::<Duel>().hovered = Some(id);
    app.update();
    let hovered = app.world().get::<Motion>(entity).unwrap().target;
    assert!(hovered.translation.y > resting.translation.y);
    assert_eq!(app.world().resource::<SceneIndex>().faces[&id].texts, text);
    app.world_mut().resource_mut::<Duel>().hovered = None;
    app.update();
    assert_eq!(app.world().get::<Motion>(entity).unwrap().target, resting);
}

#[test]
fn a_late_keyword_mesh_fills_in_a_cached_card_without_a_new_view() {
    let mut app = app();
    app.init_resource::<crate::hud::UiFonts>();
    // This fixture supplies its board directly; give its visible creature a
    // nonempty strip while deliberately leaving the scene's mesh absent.
    app.world_mut()
        .resource_mut::<Duel>()
        .board
        .as_mut()
        .unwrap()
        .pods[0]
        .lanes[0]
        .groups[0]
        .badges
        .push(KeywordBadge::Flying);
    for _ in 0..3 {
        app.update();
    }
    let id = ObjectId::new(1, 0);
    let index = app.world().resource::<SceneIndex>();
    let card = index.cards[&id];
    let words = index.faces[&id].texts.clone();
    assert!(!index.marks.contains_key(&id));
    let seq = app.world().resource::<Duel>().view.as_ref().unwrap().seq;

    app.world_mut().resource_mut::<SceneIndex>().marks_quad = Some(Handle::default());
    app.update();
    let index = app.world().resource::<SceneIndex>();
    let strip = index
        .marks
        .get(&id)
        .expect("the late mesh must be retried")
        .2;
    assert!(app.world().get::<KeywordStrip>(strip).is_some());
    assert_eq!(app.world().get::<ChildOf>(strip).unwrap().parent(), card);
    assert_eq!(index.cards[&id], card);
    assert_eq!(index.faces[&id].texts, words);
    assert_eq!(
        app.world().resource::<Duel>().view.as_ref().unwrap().seq,
        seq
    );

    // Readiness becoming stable must not recreate the decoration each frame.
    app.update();
    assert_eq!(app.world().resource::<SceneIndex>().marks[&id].2, strip);
}

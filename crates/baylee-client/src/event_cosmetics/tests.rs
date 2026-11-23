use super::*;
use crate::{
    cardmat::{CardLook, CardMaterial},
    matmat::{MatMaterial, MatParams},
};
use baylee_client_core::images::FinishTreatment;

#[test]
fn hidden_appearances_use_the_owner_even_under_a_control_change() {
    let mut first = baylee_client_core::test_support::printed(17, 1, "Visible card", 42);
    first.owner = PlayerId::new(0);
    assert!(concealed(&first).is_none());
    first.status = baylee_view::ObjectStatus::FACE_DOWN;
    assert!(concealed(&first).is_none());
    first.card = None;
    let mut other = baylee_client_core::test_support::token(111, 0, "Face-down", 2, 2);
    other.status = baylee_view::ObjectStatus::FACE_DOWN;
    assert_eq!(concealed(&first), Some(PlayerId::new(0)));
    assert_eq!(concealed(&first), concealed(&other));
}

fn card(
    images: &mut Assets<Image>,
    cards: &mut Assets<CardMaterial>,
) -> (Paint, Handle<CardMaterial>) {
    let paint = Paint {
        image: images.add(Image::default()),
        tier: 2,
    };
    let material = crate::cardmat::material(
        CardLook::back(FinishTreatment::Plain),
        Some(paint.image.clone()),
        Color::WHITE,
        0.0,
    );
    (paint, cards.add(material))
}

#[test]
fn identical_sleeves_per_owner_and_mats_follow_their_own_seat_then_restore() {
    let mut app = App::new();
    let mut images = Assets::<Image>::default();
    let mut cards = Assets::<CardMaterial>::default();
    let (_, resident) = card(&mut images, &mut cards);
    let (paint, sleeve) = card(&mut images, &mut cards);
    let mut worn = Worn::default();
    worn.0[0].sleeve = Some((paint.clone(), sleeve.clone()));
    worn.0[1].mat = Some(MatPaint {
        paint: paint.clone(),
        pixels: Vec2::new(1600., 900.),
        framing: framing::Framing::default(),
    });
    let mut mats = Assets::<MatMaterial>::default();
    let ground = || MatMaterial {
        params: MatParams {
            accent: Vec4::ONE,
            size: Vec2::ONE,
            corner: 0.1,
            rim: 0.1,
            on_turn: 0.0,
            awaited: 0.0,
            held: 0.0,
            motion: 0.0,
            ledge_outer: 0.0,
        },
        artwork: None,
        decoration: UVec4::ZERO,
        viewport: Vec4::ZERO,
    };
    let handles = [mats.add(ground()), mats.add(ground())];
    for (id, handle) in (0u8..).zip(&handles) {
        app.world_mut().spawn((
            crate::table::SeatMat(PlayerId::new(id)),
            MeshMaterial3d(handle.clone()),
        ));
    }
    let slabs = [0, 0, 1].map(|seat| {
        app.world_mut()
            .spawn((
                LibrarySleeve(PlayerId::new(seat), resident.clone()),
                MeshMaterial3d(resident.clone()),
            ))
            .id()
    });
    app.insert_resource(worn)
        .insert_resource(mats)
        .add_systems(Update, apply_surfaces);
    app.update();
    let read = |app: &App, entity: Entity| {
        app.world()
            .get::<MeshMaterial3d<CardMaterial>>(entity)
            .unwrap()
            .0
            .id()
    };
    assert_eq!(
        slabs.map(|s| read(&app, s)),
        [sleeve.id(), sleeve.id(), resident.id()]
    );
    let mats = app.world().resource::<Assets<MatMaterial>>();
    assert!(mats.get(&handles[0]).unwrap().artwork.is_none());
    assert_eq!(
        mats.get(&handles[1]).unwrap().artwork.as_ref(),
        Some(&paint.image)
    );
    assert_eq!(mats.get(&handles[1]).unwrap().decoration.x, 3);
    #[cfg(feature = "dev-control")]
    {
        assert!(mats.get(&handles[1]).unwrap().has_loaded_print(&images));
        assert!(!mats.get(&handles[0]).unwrap().has_loaded_print(&images));
        assert!(
            !mats
                .get(&handles[1])
                .unwrap()
                .has_loaded_print(&Assets::default())
        );
    }
    app.insert_resource(Worn::default());
    app.update();
    assert_eq!(slabs.map(|s| read(&app, s)), [resident.id(); 3]);
    assert!(
        app.world()
            .resource::<Assets<MatMaterial>>()
            .get(&handles[1])
            .unwrap()
            .artwork
            .is_none()
    );
}

#[test]
fn the_reward_and_deck_preview_shader_validates() {
    let prelude = "struct Globals {time:f32}; struct UiVertexOutput { @builtin(position) position:vec4<f32>, @location(0) uv:vec2<f32>, @location(1) size:vec2<f32>, @location(2) border_radius_x:vec4<f32> };";
    let common = include_str!("../shaders/card_common.wesl");
    crate::cardmat::tests::check_wgsl(
        include_str!("../shaders/event_preview.wesl"),
        &format!("{prelude}{common}"),
    );
}

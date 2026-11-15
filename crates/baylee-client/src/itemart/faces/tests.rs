use super::*;
use baylee_cards::tokens;
use baylee_client_core::{images::ArtSize, textface::Regions};

#[test]
fn portraits_fit_the_band_clear_of_names_types_and_rules() {
    for lines in [1, 2] {
        let regions = Regions::table(lines);
        let rect = square(regions.band);
        assert!(rect.min.x > regions.band[0] && rect.max.x < regions.band[2]);
        assert!(rect.min.y > regions.name_bar[3]);
        assert!(rect.max.y < regions.type_bar[1]);
        assert!(rect.max.y < regions.text_box[1]);
        assert!(rect.width() > 0.1);
        assert!((rect.width() - rect.height()).abs() < 1e-6);
    }
}

#[test]
fn portraits_are_unpickable_children_and_leave_with_their_face() {
    let mut scene = World::new();
    let card = scene.spawn_empty().id();
    let key = Some(ImageKey::token(
        tokens::token_id(&tokens::TOWER_ROOTS_MYTHIC),
        ArtSize::Normal,
    ));
    let band = Regions::table(1).band;
    let portrait = world(
        &mut scene.commands(),
        card,
        key,
        band,
        crate::cardmat::veil::PHASED,
    )
    .expect("original item");
    ui(&mut scene.commands(), card, key, Some(band), 400.0);
    scene.flush();
    assert_eq!(
        scene.get::<ChildOf>(portrait).expect("parent").parent(),
        card
    );
    assert_eq!(
        *scene.get::<Pickable>(portrait).expect("pickable"),
        Pickable::IGNORE
    );
    let state = scene
        .get::<super::super::world::Portrait>(portrait)
        .expect("state");
    assert_eq!(u32::from(state.veil), crate::cardmat::veil::PHASED);
    let pose = scene.get::<Transform>(portrait).expect("pose");
    assert!(
        pose.translation.z > crate::table::CARD_THICKNESS,
        "the picture must stand above the slab's surface"
    );
    let rect = square(band);
    assert!(
        (pose.translation.x - (rect.center().x - 0.5) * baylee_client_core::layout::CARD_WIDTH)
            .abs()
            < 1e-6
    );
    assert!((pose.scale.x - pose.scale.y).abs() < 1e-6);
    let children = scene.get::<Children>(card).expect("two portraits").to_vec();
    assert_eq!(children.len(), 2);
    scene.despawn(card);
    assert!(
        children
            .into_iter()
            .all(|child| scene.get_entity(child).is_err())
    );
}

#[test]
fn hidden_or_unrecognized_identities_and_occupied_bands_get_no_portrait() {
    let mut scene = World::new();
    let card = scene.spawn_empty().id();
    let band = Regions::table(1).band;
    for key in [
        None,
        Some(ImageKey::card_back(ArtSize::Normal)),
        Some(ImageKey::token(u16::MAX, ArtSize::Normal)),
        Some(ImageKey::token(
            tokens::token_id(&tokens::TOWER_SEEDLING),
            ArtSize::Normal,
        )),
    ] {
        assert!(world(&mut scene.commands(), card, key, band, 0).is_none());
        ui(&mut scene.commands(), card, key, Some(band), 400.0);
    }
    let key = Some(ImageKey::token(
        tokens::token_id(&tokens::TOWER_ROOTS_COMMON),
        ArtSize::Normal,
    ));
    ui(&mut scene.commands(), card, key, None, 400.0);
    scene.flush();
    assert!(scene.get::<Children>(card).is_none());
}

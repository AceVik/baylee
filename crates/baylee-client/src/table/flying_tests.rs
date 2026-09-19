use super::*;
use baylee_client_core::board::Provenance;

fn obj(slot: u32) -> ObjectId {
    ObjectId::new(slot, 0)
}

/// A placement standing for one creature, which flies or does not.
fn card(slot: u32, flying: bool) -> Placement {
    Placement {
        object: obj(slot),
        // `float_of` reads only the two fields below; the slot is here
        // because a `Placement` is a whole card and comes from a table.
        slot: *TableLayout::new(&[PlayerId::new(0)], 16.0 / 9.0, None)
            .slot(PlayerId::new(0))
            .expect("a one-seat table seats its one player"),
        position: Vec2::ZERO,
        lift: 0.0,
        tapped: false,
        flying,
        count: 1,
        art: None,
        offer: crate::cardmat::Offer::default(),
        corner: baylee_client_core::cardplate::Corner::default(),
        selected: false,
        fan: None,
    }
}

/// One creature, with whatever keywords the caller wants drawn on it.
fn creature(slot: u32, badges: Vec<KeywordBadge>) -> CardGroup {
    CardGroup {
        representative: obj(slot),
        members: vec![obj(slot)],
        name: format!("Creature {slot}"),
        power: Some(2),
        toughness: Some(2),
        base_power: Some(2),
        base_toughness: Some(2),
        damage: 0,
        loyalty: None,
        status: baylee_view::ObjectStatus::NONE,
        counters: Vec::new(),
        badges,
        art: None,
        provenance: Provenance::Token,
        original: None,
        summoning_sick: false,
        activatable: false,
        commander: false,
        individual: None,
    }
}

/// A one-seat table with those creatures standing in the creature lane.
fn duel(groups: Vec<CardGroup>) -> Duel {
    use baylee_client_core::board::{BoardModel, Lane, SeatPod};
    use baylee_client_core::layout::LaneKind;
    Duel {
        board: Some(BoardModel {
            seq: 1,
            local: PlayerId::new(0),
            turn: 1,
            step: baylee_view::Step::Main,
            pods: vec![SeatPod {
                player: PlayerId::new(0),
                life: 20,
                poison: 0,
                energy: 0,
                hand_count: 0,
                library_count: 40,
                graveyard_count: 0,
                has_lost: false,
                is_local: true,
                is_active: true,
                is_awaited: true,
                role: baylee_client_core::board::SeatRole::Present,
                lanes: vec![Lane {
                    kind: LaneKind::Creatures,
                    groups,
                    overflowing: false,
                }],
                piles: baylee_client_core::PileKind::ALL
                    .into_iter()
                    .map(baylee_client_core::ZonePile::empty)
                    .collect(),
                tokens: Vec::new(),
                threat: baylee_client_core::ThreatSummary::default(),
            }],
            stack: Vec::new(),
            hand: Vec::new(),
        }),
        layout: Some(TableLayout::new(&[PlayerId::new(0)], 16.0 / 9.0, None)),
        ..Duel::default()
    }
}

#[test]
fn the_keyword_on_the_card_is_what_puts_it_in_the_air() {
    let placed = placements(&duel(vec![
        creature(1, vec![KeywordBadge::Flying]),
        creature(2, vec![KeywordBadge::Trample, KeywordBadge::Reach]),
    ]));
    let flying: Vec<(u32, bool)> = placed.iter().map(|p| (p.object.slot(), p.flying)).collect();
    assert!(
        flying.contains(&(1, true)),
        "the creature with flying is not in the air: {flying:?}"
    );
    assert!(
        flying.contains(&(2, false)),
        "reach is not flying, and a trampler is on the ground: {flying:?}"
    );
}

#[test]
fn the_keyword_survives_the_trip_from_the_view_to_the_table() {
    // The hand-built group above says the renderer reads the badge. This
    // says the badge is there to read: a view with the flying bit set,
    // through the real `BoardModel::from_view`, and out the other side as
    // a card that is off the felt. Both halves, or a bit that never
    // reaches `CardGroup::badges` would pass the first test forever.
    use baylee_client_core::board::{BoardModel, Openings, keyword_bits};
    use baylee_client_core::test_support::{ViewBuilder, token};

    let mut drake = token(1, 1, "Gilded Drake", 3, 3);
    drake.keywords = keyword_bits::FLYING;
    let view = ViewBuilder::new(2)
        .with_battlefield(1, vec![drake])
        .with_battlefield(0, vec![token(2, 0, "Ogre", 3, 3)])
        .build();
    let board = BoardModel::from_view(
        &view,
        Openings::none(),
        |_| 12.0,
        &[],
        crate::cardart::registry(),
    );
    let duel = Duel {
        board: Some(board),
        layout: Some(TableLayout::new(
            &[PlayerId::new(0), PlayerId::new(1)],
            16.0 / 9.0,
            None,
        )),
        ..Duel::default()
    };
    let flying: Vec<(u32, bool)> = placements(&duel)
        .iter()
        .map(|p| (p.object.slot(), p.flying))
        .collect();
    assert!(
        flying.contains(&(1, true)) && flying.contains(&(2, false)),
        "the flying bit did not reach the table: {flying:?}"
    );
}

#[test]
fn a_creature_that_does_not_fly_is_never_lifted() {
    for t in [0.0, 0.7, 3.3, 11.9] {
        assert!(
            float_of(&card(1, false), false, t).abs() < f32::EPSILON,
            "a ground creature left the felt at {t} seconds"
        );
    }
}

#[test]
fn a_flier_stands_off_the_felt_and_is_never_still() {
    let one = card(1, true);
    let first = float_of(&one, false, 0.0);
    let later = float_of(&one, false, 0.9);
    assert!(
        (first - later).abs() > 1e-4,
        "a flier that does not move is a card somebody forgot to put down"
    );
    for h in [first, later] {
        assert!(
            (airborne::RESTING - airborne::SWAY..=airborne::RESTING + airborne::SWAY).contains(&h),
            "the bob left its band: {h}"
        );
    }
}

#[test]
fn a_flier_the_pointer_is_on_holds_still_without_coming_down() {
    let one = card(1, true);
    for t in [0.0, 0.9, 4.4] {
        let held = float_of(&one, true, t);
        assert!(
            (held - airborne::RESTING).abs() < f32::EPSILON,
            "a held flier moved, or dropped, at {t} seconds: {held}"
        );
    }
}

/// A card standing `height` above the felt, with its contact shadow
/// underneath it exactly as `sync_scene` spawns one.
fn a_card_in_the_air(app: &mut App, height: f32, floating: bool) -> Entity {
    let mut card = app.world_mut().spawn((
        CardVisual {
            object: obj(1),
            count: 1,
        },
        Transform::from_xyz(0.0, TABLE_Y + CARD_LIFT + height, 0.0)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    if floating {
        card.insert(Floating);
    }
    let card = card.id();
    app.world_mut().spawn((
        CardShadow,
        Transform::from_xyz(0.0, 0.0, -(CARD_LIFT * 0.5)),
        ChildOf(card),
    ));
    card
}

fn shadow_of(app: &mut App) -> Transform {
    let mut q = app
        .world_mut()
        .query_filtered::<&Transform, With<CardShadow>>();
    *q.iter(app.world()).next().expect("the shadow is spawned")
}

#[test]
fn a_flier_leaves_its_shadow_on_the_table() {
    let mut app = App::new();
    app.add_systems(Update, ground_the_shadows);
    let card = a_card_in_the_air(&mut app, airborne::RESTING, true);
    app.update();

    let shadow = shadow_of(&mut app);
    let card = *app.world().entity(card).get::<Transform>().unwrap();
    let on_the_felt = card.to_matrix().transform_point3(shadow.translation).y;
    assert!(
        (on_the_felt - (TABLE_Y + CARD_LIFT * 0.5)).abs() < 1e-5,
        "the shadow is {on_the_felt} above the felt and not {}",
        TABLE_Y + CARD_LIFT * 0.5
    );
    assert!(
        shadow.scale.x > 1.0 + airborne::RESTING * 0.5,
        "the shadow did not spread as the card climbed: {}",
        shadow.scale.x
    );
}

#[test]
fn a_card_lying_on_the_felt_keeps_the_shadow_it_was_given() {
    // The counter-test, and the one that says the system is doing
    // something rather than everything: a card that does not fly is not
    // in the query at all, so its shadow is the one `sync_scene` spawned.
    let mut app = App::new();
    app.add_systems(Update, ground_the_shadows);
    a_card_in_the_air(&mut app, 0.0, false);
    app.update();

    let shadow = shadow_of(&mut app);
    assert!(
        (shadow.translation.z + CARD_LIFT * 0.5).abs() < f32::EPSILON
            && (shadow.scale.x - 1.0).abs() < f32::EPSILON,
        "a grounded card's shadow was moved: {shadow:?}"
    );
}

#[test]
fn the_spread_stops_however_high_the_card_is_carried() {
    // A flier the pointer is resting on is at `RESTING` plus the hover
    // lift, and the height read here is the whole of that. The cap is
    // what keeps the sum from putting a lane-sized pool under one card,
    // so it is measured at the largest rise anything on this table ever
    // takes: the one `retire` throws a departing card into.
    let mut app = App::new();
    app.add_systems(Update, ground_the_shadows);
    a_card_in_the_air(&mut app, BOUNCE_RISE, true);
    app.update();

    let spread = shadow_of(&mut app).scale.x;
    assert!(
        spread <= 1.0 + FLOAT_SHADOW_CAP * DECK_SHADOW_SPREAD + 1e-5,
        "the spread ran past its cap: {spread}×"
    );
}

#[test]
fn a_shadow_does_not_follow_a_card_out_of_the_game() {
    // The other half, and the one the cap cannot do: a card `retire` has
    // taken over is no longer a `CardVisual`, so `Aloft` does not match
    // it and its shadow is left exactly where the bounce carries it —
    // glued underneath, which is what a card flying off to a hand wants.
    let mut app = App::new();
    app.add_systems(Update, ground_the_shadows);
    let card = app.world_mut().spawn((
        Floating,
        Transform::from_xyz(0.0, TABLE_Y + CARD_LIFT + BOUNCE_RISE, 0.0),
    ));
    let card = card.id();
    let was = Transform::from_xyz(0.0, 0.0, -(CARD_LIFT * 0.5));
    app.world_mut().spawn((CardShadow, was, ChildOf(card)));
    app.update();

    assert_eq!(
        shadow_of(&mut app),
        was,
        "a card that has left the table was still being grounded"
    );
}

#[test]
fn banking_shadows_stay_flat_but_follow_a_tapped_creatures_heading() {
    let mut app = App::new();
    app.add_systems(Update, ground_the_shadows);
    let entity = a_card_in_the_air(&mut app, airborne::RESTING, true);
    let card = Transform::from_xyz(0.0, CARD_LIFT + airborne::RESTING, 0.0).with_rotation(
        Quat::from_rotation_y(1.5)
            * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
            * Quat::from_rotation_y(0.02),
    );
    app.world_mut().entity_mut(entity).insert(card);
    app.update();
    let shadow = shadow_of(&mut app);
    let world = card.to_matrix() * shadow.to_matrix();
    assert!(world.transform_vector3(Vec3::Z).normalize().dot(Vec3::Y) > 0.9999);
    let right = card.rotation * Vec3::X;
    let heading = Vec3::new(right.x, 0.0, right.z).normalize();
    assert!(world.transform_vector3(Vec3::X).normalize().dot(heading) > 0.9999);
    assert!((world.transform_point3(Vec3::ZERO).y - CARD_LIFT * 0.5).abs() < 1e-5);
}

#[test]
fn losing_flying_restores_the_original_contact_shadow() {
    let mut app = App::new();
    app.add_systems(Update, ground_the_shadows);
    let entity = a_card_in_the_air(&mut app, airborne::RESTING, true);
    let original = shadow_of(&mut app);
    app.update();
    assert_ne!(shadow_of(&mut app), original);
    app.world_mut().entity_mut(entity).remove::<Floating>();
    app.update();
    assert_eq!(shadow_of(&mut app), original);
}

use super::*;
use baylee_client_core::{BoardModel, Lane, LaneKind, PileKind, SeatPod, ZonePile};

/// One seat's pod with these cards in its three counted piles.
fn pod(player: PlayerId, library: u32, graveyard: u32, exile: u32) -> SeatPod {
    SeatPod {
        player,
        life: 20,
        poison: 0,
        energy: 0,
        hand_count: 0,
        library_count: library,
        graveyard_count: graveyard,
        has_lost: false,
        is_local: player == PlayerId::new(0),
        is_active: true,
        is_awaited: true,
        role: baylee_client_core::board::SeatRole::Present,
        lanes: vec![Lane {
            kind: LaneKind::Creatures,
            groups: Vec::new(),
        }],
        piles: PileKind::ALL
            .into_iter()
            .map(|kind| {
                let mut pile = ZonePile::empty(kind);
                pile.count = match kind {
                    PileKind::Library => library,
                    PileKind::Graveyard => graveyard,
                    PileKind::Exile => exile,
                    _ => 0,
                };
                pile
            })
            .collect(),
        tokens: Vec::new(),
        threat: baylee_client_core::ThreatSummary::default(),
    }
}

/// Whether the last frame wrote the scene's index, or a badge's material.
#[derive(Resource, Default)]
struct Wrote {
    index: bool,
    materials: bool,
}

fn watch(index: Res<SceneIndex>, materials: Res<Assets<BadgeMaterial>>, mut wrote: ResMut<Wrote>) {
    wrote.index = index.is_changed();
    wrote.materials = materials.is_changed();
}

/// An app with `seats` at the table, each with a zone already built where
/// its slot is (which is `sync_zones`' work and needs a renderer's assets),
/// and the one handle the system refuses to draw without.
fn running(seats: u8, piles: [u32; 3]) -> App {
    let players: Vec<PlayerId> = (0..seats).map(PlayerId::new).collect();
    let [library, graveyard, exile] = piles;
    let pods: Vec<SeatPod> = players
        .iter()
        .map(|player| pod(*player, library, graveyard, exile))
        .collect();
    let duel = Duel {
        board: Some(BoardModel {
            seq: 1,
            local: PlayerId::new(0),
            turn: 1,
            step: baylee_view::Step::Main,
            pods: pods.clone(),
            stack: Vec::new(),
            hand: Vec::new(),
        }),
        layout: Some(TableLayout::new(&players, 1.78, None)),
        ..Duel::default()
    };
    let mut index = SceneIndex {
        badge_quad: Some(Handle::default()),
        ..SceneIndex::default()
    };
    for pod in &pods {
        let slot = *duel.drawn_slot(pod.player).expect("every seat is drawn");
        index.zones.insert(
            pod.player,
            Zone {
                slot,
                raised: 0.0,
                mat: Entity::PLACEHOLDER,
                glow: Entity::PLACEHOLDER,
                mat_material: Handle::default(),
                glow_material: Handle::default(),
                piles: Vec::new(),
                counts: Vec::new(),
                library_shown: 0,
                mood: Mood::of(pod),
                accent: seat_accent(&slot),
            },
        );
    }
    let mut app = App::new();
    app.insert_resource(duel)
        .insert_resource(index)
        .init_resource::<ShownRig>()
        .init_resource::<Assets<BadgeMaterial>>()
        .init_resource::<Wrote>()
        .add_systems(Update, (sync_pile_counts, watch).chain());
    app.update();
    app
}

/// What the numbers on the table say, by pile.
fn said(app: &mut App) -> Vec<(PileKind, u32)> {
    let mut out: Vec<(PileKind, u32)> = app
        .world_mut()
        .query::<(&PileCount, &MeshMaterial3d<BadgeMaterial>)>()
        .iter(app.world())
        .map(|(count, material)| {
            let material = app
                .world()
                .resource::<Assets<BadgeMaterial>>()
                .get(&material.0)
                .expect("a badge has its material");
            (count.kind, material.params.count)
        })
        .collect();
    out.sort_by_key(|(kind, _)| PileKind::ALL.iter().position(|k| k == kind));
    out
}

fn set_piles(app: &mut App, piles: [u32; 3]) {
    let mut duel = app.world_mut().resource_mut::<Duel>();
    let board = duel.board.as_mut().expect("a board");
    for old in &mut board.pods {
        *old = pod(old.player, piles[0], piles[1], piles[2]);
    }
}

/// A library says how many from nought, since an empty one is the fact that
/// loses a game; a graveyard and an exile pile say it from their first card
/// and say nothing while they are bare table.
#[test]
fn a_library_always_wears_its_number_and_the_others_from_their_first_card() {
    let mut app = running(1, [0, 0, 0]);
    assert_eq!(
        said(&mut app),
        [(PileKind::Library, cardplate::pile_word(0))]
    );

    set_piles(&mut app, [53, 2, 1]);
    app.update();
    assert_eq!(
        said(&mut app),
        [
            (PileKind::Library, cardplate::pile_word(53)),
            (PileKind::Graveyard, cardplate::pile_word(2)),
            (PileKind::Exile, cardplate::pile_word(1)),
        ]
    );

    // A graveyard emptied again is bare table again.
    set_piles(&mut app, [52, 0, 1]);
    app.update();
    assert_eq!(
        said(&mut app),
        [
            (PileKind::Library, cardplate::pile_word(52)),
            (PileKind::Exile, cardplate::pile_word(1)),
        ]
    );
}

/// At rest nothing is written: not the index, not a material. The first
/// frame is the counter-arm, since a watch that never saw a write would
/// pass the rest of this.
#[test]
fn a_table_at_rest_rewrites_no_count() {
    let mut app = running(4, [40, 3, 0]);
    {
        let wrote = app.world().resource::<Wrote>();
        assert!(wrote.index && wrote.materials, "laying the counts wrote nothing");
    }
    for _ in 0..3 {
        app.update();
        let wrote = app.world().resource::<Wrote>();
        assert!(!wrote.index, "a frame at rest wrote the scene's index");
        assert!(!wrote.materials, "a frame at rest wrote a badge's material");
    }
    // And one card drawn is one write, of the material it changes.
    set_piles(&mut app, [39, 3, 0]);
    app.update();
    let wrote = app.world().resource::<Wrote>();
    assert!(wrote.index && wrote.materials);
}

/// The corners of the body of a number laid at `number`, in the world.
fn body_corners(number: &Transform, cards: u32) -> [Vec3; 4] {
    let [x0, y0, x1, y1] = cardplate::pile_badge_rect(cardplate::pile_word(cards));
    let quad = on_card(0.0, cardplate::pile_badge_quad_rect()).translation;
    [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].map(|(x, y)| {
        let corner = on_card(0.0, [x, y, x, y]).translation - quad;
        number.transform_point(corner.truncate().extend(0.0))
    })
}

/// Every seat's numbers lie beside its piles on its own board's side of
/// them, clear of the pile's card by the same air and short of where its
/// cards are played, at their least size and at their most.
#[test]
fn every_seats_numbers_lie_between_its_piles_and_its_board() {
    let app = running(4, [40, 3, 2]);
    let index = app.world().resource::<SceneIndex>();
    assert_eq!(index.zones.len(), 4);
    let turn = upright(&CameraRig::default().eye());
    for zone in index.zones.values() {
        let slot = &zone.slot;
        let seat = card_transform(slot, slot.center, false, 0.0);
        // The seat's own right, and its own far side, on the table.
        let (right, far) = (seat.rotation * Vec3::X, seat.rotation * Vec3::Y);
        for kind in [PileKind::Library, PileKind::Graveyard, PileKind::Exile] {
            let pile = card_transform(slot, slot.pile_center(kind), false, 0.0);
            for (cards, scale) in [
                (40, cardplate::PILE_BADGE_SCALE),
                (40, cardplate::PILE_BADGE_MOST),
                (cardplate::COUNT_MAX, cardplate::PILE_BADGE_MOST),
            ] {
                let number = pile_count_transform(slot, kind, cards, turn, scale);
                let corners = body_corners(&number, cards).map(|corner| {
                    let off = (corner - pile.translation) / slot.scale;
                    // Towards the mat, and along the pile's own row.
                    (-off.dot(right) * kind.side(), off.dot(far))
                });
                let nearest = corners.iter().map(|c| c.0).fold(f32::MAX, f32::min);
                let furthest = corners.iter().map(|c| c.0).fold(f32::MIN, f32::max);
                let along = corners.iter().map(|c| c.1.abs()).fold(0.0, f32::max);
                let air = 0.5 * CARD_WIDTH + cardplate::PILE_BADGE_AIR;
                assert!(
                    (nearest - air).abs() < 1e-3,
                    "{kind:?} ×{scale}: the number begins {nearest} from its pile's middle, not {air}"
                );
                assert!(
                    furthest < baylee_client_core::layout::PILE_REACH,
                    "{kind:?} ×{scale}: the number reaches {furthest}, into the rows"
                );
                assert!(
                    along <= CARD_HEIGHT * 0.5,
                    "{kind:?} ×{scale}: the number stands {along} off its pile's row"
                );
                assert!((number.scale.x / slot.scale - scale).abs() < 1e-4);
            }
        }
    }
}

/// A number is written for the one looking: at every seat of a ring it
/// stands the same way up, its tops up the screen and its figures running
/// to the screen's right, also once the camera has gone round the table.
#[test]
fn every_seats_number_stands_upright_for_the_viewer() {
    let app = running(4, [40, 3, 2]);
    let index = app.world().resource::<SceneIndex>();
    for yaw in [0.0, 1.1, std::f32::consts::PI, -2.0] {
        let eye = CameraRig {
            yaw,
            ..CameraRig::default()
        }
        .eye();
        let turn = upright(&eye);
        let (screen_right, screen_up) = (eye.rotation * Vec3::X, eye.rotation * Vec3::Y);
        for zone in index.zones.values() {
            let number = pile_count_transform(&zone.slot, PileKind::Library, 40, turn, 2.0);
            let (runs, tops) = (number.rotation * Vec3::X, number.rotation * Vec3::Y);
            assert!(
                runs.dot(screen_right) > 0.999,
                "yaw {yaw}: the figures run {runs}, the screen's right is {screen_right}"
            );
            assert!(tops.dot(screen_up) > 0.0, "yaw {yaw}: the tops point down the screen");
            assert!(
                (number.rotation * Vec3::Z).dot(Vec3::Y) > 0.999,
                "yaw {yaw}: the number does not lie flat"
            );
        }
    }
    // The four seats face four ways, or this proved nothing about turning.
    let facings: Vec<f32> = index.zones.values().map(|zone| zone.slot.facing).collect();
    assert!(
        facings
            .iter()
            .any(|a| (a - facings[0]).rem_euclid(std::f32::consts::TAU) > 1.0)
    );
}

/// Through a window, a pile's figures are the height the interface asks for
/// at every seat, near or far, wherever the strip leaves the room for it.
#[test]
fn the_figures_are_as_tall_on_the_screen_at_every_seat() {
    let window = Vec2::new(1708.0, 1032.0);
    let players: Vec<PlayerId> = (0..4).map(PlayerId::new).collect();
    let layout = TableLayout::new(&players, window.x / window.y, None);
    let rig = CameraRig::home(&layout, Canvas::hud(window));
    let lens = Lens::new(rig, window);
    let reader = Reader {
        upright: upright(&rig.eye()),
        lens: Some(lens),
    };
    let lens = Lens::new(rig, window);
    let mut asked = 0;
    for player in &players {
        let slot = *layout.slot(*player).expect("every seat has a place");
        for kind in [PileKind::Library, PileKind::Graveyard, PileKind::Exile] {
            let number = reader.lay(&(slot, 0.0), kind, 40);
            let scale = number.scale.x / slot.scale;
            // The figures' height in the world, drawn from the body's middle.
            let cap = cardplate::badge_cap(cardplate::pile_word(40)) * CARD_WIDTH * number.scale.x;
            let corners = body_corners(&number, 40);
            let middle = (corners[0] + corners[2]) * 0.5;
            let tall = lens
                .project_world(middle)
                .zip(lens.project_world(middle + number.rotation * Vec3::Y * cap))
                .map(|(a, b)| a.distance(b))
                .expect("the table is in front of the eye");
            if (scale - cardplate::PILE_BADGE_MOST).abs() < 1e-4 {
                assert!(tall <= cardplate::PILE_FIGURE_PX + 0.5, "{kind:?}: {tall} px at the most");
            } else if (scale - cardplate::PILE_BADGE_SCALE).abs() < 1e-4 {
                assert!(tall >= cardplate::PILE_FIGURE_PX - 0.5, "{kind:?}: {tall} px at the least");
            } else {
                asked += 1;
                assert!(
                    (tall - cardplate::PILE_FIGURE_PX).abs() < 0.5,
                    "{kind:?} at seat {player:?}: figures {tall} px tall"
                );
            }
        }
    }
    // A table of four on a laptop's window is the case this was built for:
    // the size is the asked one there, not a bound's.
    assert!(asked >= 6, "only {asked} of twelve numbers are the asked size");
}

/// A camera going round re-lays the numbers and leaves the scene's index
/// alone: the index is what every other table system watches.
#[test]
fn a_camera_going_round_moves_the_numbers_and_writes_no_index() {
    let mut app = running(4, [40, 3, 0]);
    let before: Vec<Transform> = app
        .world_mut()
        .query_filtered::<&Transform, With<PileCount>>()
        .iter(app.world())
        .copied()
        .collect();
    assert_eq!(before.len(), 8);
    app.insert_resource(ShownRig::standing(CameraRig {
        yaw: 1.2,
        ..CameraRig::default()
    }));
    app.update();
    let wrote = app.world().resource::<Wrote>();
    assert!(!wrote.index, "a turn of the camera wrote the scene's index");
    assert!(!wrote.materials, "a turn of the camera wrote a badge's material");
    let after: Vec<Transform> = app
        .world_mut()
        .query_filtered::<&Transform, With<PileCount>>()
        .iter(app.world())
        .copied()
        .collect();
    assert!(
        before.iter().zip(&after).all(|(a, b)| a.rotation.angle_between(b.rotation) > 1.0),
        "the numbers did not turn with the eye"
    );
}

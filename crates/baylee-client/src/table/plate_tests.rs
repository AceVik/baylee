//! The plate on the table (the owner, 25.09): where its quad lies on the
//! card, that it stays upright under a tapped card, and that it comes and
//! goes with what the print cannot say.

use super::flying_tests::{creature, duel};
use super::*;
use crate::badgemat::BadgeMaterial;
use crate::platemat::quad_size;
use baylee_client_core::cardplate::{
    Corner, PLATE_TURNED, Plate, PlateRoom, plate_quad, plate_rect,
};
use bevy::ecs::world::CommandQueue;

/// A 3/3's plate, awake.
fn words() -> PlateWords {
    let corner = Corner {
        plate: Plate::Fight {
            power: 3,
            toughness: 3,
            damage: 0,
        },
        ..Corner::default()
    };
    PlateWords::of(corner, false)
}

/// One creature on a table of its own, placed.
fn one() -> Placement {
    let mut placed = placements(&duel(vec![creature(1, Vec::new())]));
    assert_eq!(placed.len(), 1);
    placed.remove(0)
}

/// Syncs `placement`'s plate onto `card`, saying `words`.
fn sync(
    world: &mut World,
    index: &mut SceneIndex,
    materials: &mut Assets<PlateMaterial>,
    card: Entity,
    placement: &Placement,
    words: Option<PlateWords>,
) {
    let mut queue = CommandQueue::default();
    let mut commands = Commands::new(&mut queue, world);
    sync_plate(
        &mut commands,
        index,
        materials,
        card,
        placement,
        words,
        &home(),
    );
    queue.apply(world);
}

fn index() -> SceneIndex {
    SceneIndex {
        plate_quad: Some(Handle::default()),
        ..default()
    }
}

/// The camera the table opens on, as far as which way up anything reads.
pub(super) fn home() -> Transform {
    CameraRig::default().eye()
}

/// The plate's quad in the untapped card's frame, `[x0, y0, x1, y1]` in
/// card widths from its top-left corner, read back out of the plate's
/// transform in the card's frame and the mesh's size rather than restated:
/// a flipped axis — a card's `+y` is its top, and its UV's is its bottom —
/// stands the plate at another corner and the tests below go red.
fn quad_on_card(local: Transform) -> [f32; 4] {
    let size = quad_size();
    let (w, h) = (size.x * CARD_WIDTH, size.y * DOWN_THE_CARD);
    let at = local.translation;
    let x0 = (at.x - w * 0.5) / CARD_WIDTH + 0.5;
    let y0 = (CARD_HEIGHT * 0.5 - (at.y + h * 0.5)) / DOWN_THE_CARD;
    [x0, y0, x0 + size.x, y0 + size.y]
}

/// The plate lies where `cardplate` puts it, above its own card's face and
/// below the face of the card laid over it, at every place it stands.
#[test]
fn the_plate_lies_where_cardplate_puts_it() {
    let rooms = [
        PlateRoom::OPEN,
        PlateRoom {
            right: 0.6,
            below: [f32::NEG_INFINITY, 0.6],
        },
    ];
    for tapped in [false, true] {
        for room in rooms {
            let mut placement = one();
            placement.tapped = tapped;
            placement.room = room;
            let (body, _) = plate_rect(cardplate::KIND_FIGHT, tapped, room, None).expect("room");
            let at = on_card(placement.rung, plate_quad(body));
            let read = quad_on_card(at);
            for (a, b) in read.iter().zip(plate_quad(body)) {
                assert!((a - b).abs() < 1e-5, "tapped {tapped}, {room:?}: {read:?}");
            }
            let face = CARD_THICKNESS;
            assert!(
                at.translation.z > face,
                "the plate is under its card's face"
            );
            assert!(
                at.translation.z < face + placement.rung.max(1e-3),
                "the plate is over the next card's face"
            );
        }
    }
}

/// Under a tapped card the plate stands where `plate_rect` puts it in the
/// untapped card's frame, upright, and stays so all the way round as the
/// card turns.
#[test]
fn a_plate_stands_upright_under_a_tapped_card() {
    use bevy::ecs::system::RunSystemOnce;
    let mut card = one();
    let pose = |tapped| card_transform(&card.slot, card.position, tapped, card.lift);
    let (rest, tapped) = (pose(false), pose(true));
    let mut world = World::new();
    let mut index = index();
    let mut materials = Assets::<PlateMaterial>::default();
    let entity = world.spawn(rest).id();
    let plate_at = |world: &mut World| {
        let local = *world
            .query_filtered::<&Transform, With<CardPlate>>()
            .single(world)
            .expect("one plate");
        let card = *world.get::<Transform>(entity).expect("the card");
        card.mul_transform(local)
    };
    let same = |a: Transform, b: Transform| {
        a.translation.distance(b.translation) < 1e-4
            && a.rotation.dot(b.rotation).abs() > 1.0 - 1e-6
    };

    card.tapped = true;
    sync(
        &mut world,
        &mut index,
        &mut materials,
        entity,
        &card,
        Some(words()),
    );
    // Where it stands: the untapped card's frame, laid at the tapped body.
    let (body, spot) =
        plate_rect(cardplate::KIND_FIGHT, true, card.room, None).expect("room under the card");
    assert_eq!(spot, cardplate::PlateSpot::Below);
    let upright = rest.mul_transform(on_card(card.rung, plate_quad(body)));

    *world.get_mut::<Transform>(entity).expect("the card") = tapped;
    assert!(
        same(plate_at(&mut world), upright),
        "tapped, the plate turned"
    );
    for share in [0.0, 0.25, 0.5, 0.9] {
        world
            .get_mut::<Transform>(entity)
            .expect("the card")
            .rotation = rest.rotation.slerp(tapped.rotation, share);
        world
            .run_system_once(keep_upright)
            .expect("the system runs");
        assert!(
            same(plate_at(&mut world), upright),
            "{share} of the way round, the plate turned"
        );
    }
}

/// The plate goes on where the print cannot say the numbers, moves when its
/// card taps, comes off when the print can say them again and where a
/// tapped card's neighbours leave it no air; and a card that did not change
/// is not given a second one.
#[test]
fn a_plate_comes_and_goes() {
    let mut world = World::new();
    let mut index = index();
    let mut materials = Assets::<PlateMaterial>::default();
    let card = world.spawn(Transform::default()).id();
    let plates = |world: &mut World| {
        world
            .query_filtered::<&ChildOf, With<CardPlate>>()
            .iter(world)
            .count()
    };
    let mut placement = one();

    sync(
        &mut world,
        &mut index,
        &mut materials,
        card,
        &placement,
        None,
    );
    assert_eq!(plates(&mut world), 0, "a plate the print says anyway");

    sync(
        &mut world,
        &mut index,
        &mut materials,
        card,
        &placement,
        Some(words()),
    );
    assert_eq!(plates(&mut world), 1);
    let (key, entity) = index.plates[&placement.object];
    sync(
        &mut world,
        &mut index,
        &mut materials,
        card,
        &placement,
        Some(words()),
    );
    assert_eq!(
        index.plates[&placement.object],
        (key, entity),
        "nothing moved"
    );
    assert_eq!(plates(&mut world), 1);

    placement.tapped = true;
    sync(
        &mut world,
        &mut index,
        &mut materials,
        card,
        &placement,
        Some(words()),
    );
    let (moved, same) = index.plates[&placement.object];
    assert_eq!(
        same, entity,
        "a tap moves the plate, it does not make another"
    );
    assert_ne!(moved, key);
    assert_eq!(plates(&mut world), 1);

    // Squeezed between two untapped cards in the tightest fan.
    placement.room = PlateRoom {
        right: 0.33,
        below: [0.67, 0.33],
    };
    sync(
        &mut world,
        &mut index,
        &mut materials,
        card,
        &placement,
        Some(words()),
    );
    world.flush();
    assert_eq!(plates(&mut world), 0, "no air, no plate");
    assert!(!index.plates.contains_key(&placement.object));

    placement.room = PlateRoom::OPEN;
    placement.tapped = false;
    sync(
        &mut world,
        &mut index,
        &mut materials,
        card,
        &placement,
        Some(words()),
    );
    sync(
        &mut world,
        &mut index,
        &mut materials,
        card,
        &placement,
        None,
    );
    assert_eq!(plates(&mut world), 0, "the print says it again");
    assert_eq!(
        index.plate_materials.len(),
        1,
        "one thing said, one material"
    );
}

/// Every plate and every count badge reads upright to the one looking (the
/// PO, 25.09: a `6/1` upside down reads `1/9`, and power and toughness
/// decide combat). At every table from two seats to eight, on every seat's
/// cards, tapped and not, from the shot the table opens on and from the
/// camera framing each seat, the tops of its numbers — its quad's `+y` as
/// laid, turned back round where its material says it draws turned — never
/// point down the screen. A side seat's may lie square to it.
///
/// An opponent's plate laid as its card lies and drawn unturned points its
/// tops straight down, and so does one turned twice, by its material and by
/// its transform; so both kinds of plate are asserted met, turned and not.
#[test]
fn every_plate_and_badge_reads_upright_to_the_eye() {
    let window = Vec2::new(1728.0, 1052.0);
    let mut card = one();
    card.badge = 2;
    let mut seen = [0usize; 2];
    for seats in 2..=8u8 {
        let players: Vec<PlayerId> = (0..seats).map(PlayerId::new).collect();
        let layout = TableLayout::new(&players, 16.0 / 9.0, None);
        let mut eyes = vec![CameraRig::home(&layout, Canvas::hud(window))];
        for slot in &layout.slots {
            eyes.push(CameraRig::framing(
                slot,
                Vec2::new(slot.center.x, -slot.center.y),
            ));
        }
        for rig in eyes {
            let eye = rig.eye();
            let screen_up = eye.rotation * Vec3::Y;
            for slot in &layout.slots {
                for tapped in [false, true] {
                    card.slot = *slot;
                    card.tapped = tapped;
                    let rest = card_transform(&card.slot, card.position, tapped, card.lift);
                    let mut world = World::new();
                    let mut index = SceneIndex {
                        plate_quad: Some(Handle::default()),
                        badge_quad: Some(Handle::default()),
                        ..default()
                    };
                    let mut plates = Assets::<PlateMaterial>::default();
                    let mut badges = Assets::<BadgeMaterial>::default();
                    let entity = world.spawn(rest).id();
                    let mut queue = CommandQueue::default();
                    let mut commands = Commands::new(&mut queue, &world);
                    sync_plate(
                        &mut commands,
                        &mut index,
                        &mut plates,
                        entity,
                        &card,
                        Some(words()),
                        &eye,
                    );
                    sync_badge(&mut commands, &mut index, &mut badges, entity, &card, &eye);
                    queue.apply(&mut world);

                    let mut read = |laid: Transform, turned: bool, what: &str| {
                        let tops = rest.mul_transform(laid).rotation * Vec3::Y;
                        let tops = if turned { -tops } else { tops };
                        assert!(
                            tops.dot(screen_up) > -1e-3,
                            "{seats} seats, seat {} facing {}, camera yaw {}, tapped \
                             {tapped}: the {what} reads upside down (turned {turned})",
                            slot.ring_index,
                            slot.facing,
                            rig.yaw,
                        );
                        seen[usize::from(turned)] += 1;
                    };
                    let (_, plate) = index.plates[&card.object];
                    let material = world
                        .get::<MeshMaterial3d<PlateMaterial>>(plate)
                        .expect("a plate material");
                    let ink = plates.get(&material.0).expect("made").params.ink;
                    let laid = *world.get::<Transform>(plate).expect("laid");
                    read(laid, ink & PLATE_TURNED != 0, "plate");
                    let (_, badge) = index.badges[&card.object];
                    let material = world
                        .get::<MeshMaterial3d<BadgeMaterial>>(badge)
                        .expect("a badge material");
                    let turned = badges.get(&material.0).expect("made").params.turned;
                    let laid = *world.get::<Transform>(badge).expect("laid");
                    read(laid, turned != 0, "badge");
                }
            }
        }
    }
    assert!(
        seen.iter().all(|&n| n > 0),
        "turned and unturned both met: {seen:?}"
    );
}

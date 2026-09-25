//! The count badge on the table (#261): where its quad lies on the card, what
//! it lies over in a fanned row, and that it comes and goes with the count.

use super::flying_tests::{creature, duel};
use super::*;
use bevy::ecs::world::CommandQueue;

/// The badge's quad is where `cardplate` says, in the card's own space: its
/// left edge off the card's left edge, its top edge as far down the card as
/// [`cardplate::badge_quad_rect`] puts it.
///
/// Read back out of the transform and the mesh's size rather than restated,
/// so a flipped axis — a card's `+y` is its top, and its UV's is its bottom —
/// hangs the badge off the other corner and this goes red.
#[test]
fn the_badge_lies_where_cardplate_puts_it() {
    let at = badge_transform(0.0).translation;
    let size = crate::badgemat::quad_size();
    let (w, h) = (size.x * CARD_WIDTH, size.y * DOWN_THE_CARD);
    let top = (CARD_HEIGHT * 0.5 - (at.y + h * 0.5)) / DOWN_THE_CARD;
    let left = (at.x - w * 0.5) / CARD_WIDTH + 0.5;
    let [x0, y0, ..] = cardplate::badge_quad_rect();
    assert!(
        (top - y0).abs() < 1e-5,
        "the badge's quad starts {top} card widths down the card, not at {y0}"
    );
    assert!(
        (left - x0).abs() < 1e-5,
        "the badge's quad starts {left} across the card, not at {x0}"
    );
    assert!(left < 0.0, "the badge does not hang off the card");
}

/// A row of merged creatures, `n` of them, each standing for two.
fn merged_row(n: usize) -> Vec<Placement> {
    let groups = (0..n)
        .map(|i| {
            let slot = u32::try_from(i + 1).expect("a small row");
            let mut group = creature(slot, Vec::new());
            group.members.push(obj(slot + 1000));
            group
        })
        .collect();
    let mut placed = placements(&duel(groups));
    assert_eq!(placed.len(), n);
    placed.sort_by(|a, b| a.lift.total_cmp(&b.lift));
    placed
}

fn obj(n: u32) -> ObjectId {
    ObjectId::new(n, 0)
}

/// Every badge in a fanned row lies above its own card's face and below the
/// face of the card laid over it.
///
/// Above its own face is also what lays the overhang over the card *before*
/// it, which the row puts lower still: a badge under its own face would be
/// cut off by its neighbour wherever it hangs over it, and its count cut in
/// half. Below the next face keeps it in the strip's place in the transparent
/// pass, however long the row.
#[test]
fn a_badge_lies_on_its_card_and_under_the_next_one() {
    for n in [1usize, 2, 5, 17, 40] {
        let placed = merged_row(n);
        for (i, card) in placed.iter().enumerate() {
            assert_eq!(card.badge, 2, "the placement carries the count");
            let face = card.lift + CARD_THICKNESS;
            let badge = card.lift + badge_transform(card.rung).translation.z;
            assert!(
                badge > face,
                "in a row of {n}, badge {i} is not above its card"
            );
            if let Some(next) = placed.get(i + 1) {
                let over = next.lift + CARD_THICKNESS;
                assert!(
                    badge < over,
                    "in a row of {n}, badge {i} at {badge} is over the next card's face at {over}"
                );
            }
        }
    }
}

/// The badge goes on with a count, changes with it, comes off when the
/// group is one card again, and a count is one material however many cards
/// say it.
#[test]
fn a_badge_comes_and_goes_with_the_count() {
    let mut world = World::new();
    let mut materials = Assets::<BadgeMaterial>::default();
    let mut index = SceneIndex {
        badge_quad: Some(Handle::default()),
        ..default()
    };
    let mut placed = merged_row(2);
    let cards = [world.spawn_empty().id(), world.spawn_empty().id()];

    let mut sync = |world: &mut World, index: &mut SceneIndex, placement: &Placement, card| {
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, world);
        sync_badge(&mut commands, index, &mut materials, card, placement);
        queue.apply(world);
    };
    let badges = |world: &mut World| {
        world
            .query_filtered::<&ChildOf, With<CountBadge>>()
            .iter(world)
            .map(ChildOf::parent)
            .collect::<Vec<_>>()
    };

    for (placement, card) in placed.iter().zip(cards) {
        sync(&mut world, &mut index, placement, card);
    }
    assert_eq!(
        badges(&mut world),
        cards.to_vec(),
        "one badge on each merged card"
    );
    assert_eq!(
        index.badge_materials.len(),
        1,
        "two cards saying ×2 are one material"
    );

    // Nothing moved: nothing is spawned again.
    sync(&mut world, &mut index, &placed[0], cards[0]);
    assert_eq!(
        badges(&mut world).len(),
        2,
        "a card that did not change got a second badge"
    );

    // A third creature joins the first group.
    placed[0].badge = 3;
    sync(&mut world, &mut index, &placed[0], cards[0]);
    assert_eq!(
        badges(&mut world).len(),
        2,
        "a new count spawned a second badge on the card"
    );
    assert_eq!(index.badge_materials.len(), 2);

    // And the second group is one card again.
    placed[1].badge = 0;
    sync(&mut world, &mut index, &placed[1], cards[1]);
    assert_eq!(
        badges(&mut world),
        vec![cards[0]],
        "a lone card kept its badge"
    );
    assert!(!index.badges.contains_key(&placed[1].object));
}

/// A table of `seats` chairs, every lane of every seat holding `n` merged
/// creatures of two each, tapped where `tapped` says by their place in the
/// row.
fn crowded(seats: u8, n: usize, tapped: fn(usize) -> bool) -> Duel {
    use baylee_client_core::board::{BoardModel, Lane, SeatPod};
    use baylee_client_core::layout::LaneKind;
    let players: Vec<PlayerId> = (0..seats).map(PlayerId::new).collect();
    let mut slot = 0;
    let mut group = |i: usize| {
        slot += 1;
        let mut group = creature(slot, Vec::new());
        group.members.push(obj(slot + 100_000));
        if tapped(i) {
            group.status = baylee_view::ObjectStatus::TAPPED;
        }
        group
    };
    let pods = players
        .iter()
        .map(|&player| SeatPod {
            player,
            life: 20,
            poison: 0,
            energy: 0,
            hand_count: 0,
            library_count: 40,
            graveyard_count: 0,
            has_lost: false,
            is_local: player == PlayerId::new(0),
            is_active: player == PlayerId::new(0),
            is_awaited: player == PlayerId::new(0),
            role: baylee_client_core::board::SeatRole::Present,
            lanes: LaneKind::ALL
                .iter()
                .map(|&kind| Lane {
                    kind,
                    groups: (0..n).map(&mut group).collect(),
                    overflowing: false,
                })
                .collect(),
            piles: baylee_client_core::PileKind::ALL
                .into_iter()
                .map(baylee_client_core::ZonePile::empty)
                .collect(),
            tokens: Vec::new(),
            threat: baylee_client_core::ThreatSummary::default(),
        })
        .collect();
    Duel {
        board: Some(BoardModel {
            seq: 1,
            local: PlayerId::new(0),
            turn: 1,
            step: baylee_view::Step::Main,
            pods,
            stack: Vec::new(),
            hand: Vec::new(),
        }),
        layout: Some(TableLayout::new(&players, 16.0 / 9.0, None)),
        ..Duel::default()
    }
}

/// A rectangle on the card, `[x0, y0, x1, y1]` in card widths from its
/// top-left corner, `y` down the card, as four corners on the table.
fn on_table(card: &Transform, [x0, y0, x1, y1]: [f32; 4]) -> [Vec2; 4] {
    [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].map(|(x, y)| {
        let local = Vec3::new(
            (x - 0.5) * CARD_WIDTH,
            CARD_HEIGHT * 0.5 - y * DOWN_THE_CARD,
            CARD_THICKNESS,
        );
        let world = card.transform_point(local);
        Vec2::new(world.x, -world.z)
    })
}

/// Whether two convex quads on the table overlap by more than a hair: no
/// edge of either separates them.
fn overlap(a: &[Vec2; 4], b: &[Vec2; 4]) -> bool {
    let reach = |quad: &[Vec2; 4], axis: Vec2| {
        quad.iter()
            .map(|p| p.dot(axis))
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), d| {
                (lo.min(d), hi.max(d))
            })
    };
    [a, b].into_iter().all(|quad| {
        (0..4).all(|i| {
            let edge = quad[(i + 1) % 4] - quad[i];
            let axis = Vec2::new(-edge.y, edge.x).normalize();
            let (a_lo, a_hi) = reach(a, axis);
            let (b_lo, b_hi) = reach(b, axis);
            a_hi > b_lo + 1e-4 && b_hi > a_lo + 1e-4
        })
    })
}

/// How a test row is tapped: named, and by a card's place in its row.
type Tap = (&'static str, fn(usize) -> bool);

/// A badge's quad on the table, with the card it lies on.
type Laid = (Entity, [Vec2; 4]);

/// Every card of `placed` at its resting pose with its badge on it, as the
/// scene spawns them, and [`hold_badges_upright`] run over them: each card's
/// pose and entity, and each badge's quad on the table.
fn laid_badges(placed: &[Placement]) -> (Vec<Transform>, Vec<Entity>, Vec<Laid>) {
    use bevy::ecs::system::RunSystemOnce;
    let mut world = World::new();
    let mut index = SceneIndex {
        badge_quad: Some(Handle::default()),
        ..default()
    };
    let mut materials = Assets::<BadgeMaterial>::default();
    let poses: Vec<Transform> = placed
        .iter()
        .map(|p| card_transform(&p.slot, p.position, p.tapped, p.lift))
        .collect();
    let mut cards = Vec::new();
    for (placement, pose) in placed.iter().zip(&poses) {
        let card = world.spawn(*pose).id();
        cards.push(card);
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        sync_badge(&mut commands, &mut index, &mut materials, card, placement);
        queue.apply(&mut world);
    }
    world
        .run_system_once(hold_badges_upright)
        .expect("the system runs");
    let [x0, y0, x1, y1] = cardplate::badge_quad_rect();
    let half = Vec2::new((x1 - x0) * CARD_WIDTH, (y1 - y0) * DOWN_THE_CARD) * 0.5;
    let laid = world
        .query_filtered::<(&ChildOf, &Transform), With<CountBadge>>()
        .iter(&world)
        .map(|(parent, local)| {
            // The quad in the badge's own space, laid on the card by the
            // badge's transform and not by `badge_rect`.
            let card = world.get::<Transform>(parent.parent()).expect("a card");
            let at = card.mul_transform(*local);
            let corners = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(sx, sy)| {
                let w = at.transform_point((half * Vec2::new(sx, sy)).extend(0.0));
                Vec2::new(w.x, -w.z)
            });
            (parent.parent(), corners)
        })
        .collect();
    (poses, cards, laid)
}

/// No count badge lies on another card's print (#274), wherever the table
/// puts the cards.
///
/// Every badge is spawned as the scene spawns it, on its card at the card's
/// resting pose, [`hold_badges_upright`] is run over them, and each badge's
/// quad, shadow and all, is laid against every other card's print window on
/// the table. A duel and a ring of eight; rows of two to forty in all three
/// lanes, so the tightest fan (0.26 of a card, asserted) and the lane before
/// and behind are all there; untapped, all tapped and every other one
/// tapped; and a creature staged into combat beside a merged one, half a
/// card forward of its row, which is where a badge above the top edge
/// failed.
#[test]
fn no_badge_lies_on_another_card_s_print() {
    use baylee_client_core::layout::{MIN_VISIBLE_FRACTION, STAGE_STEP};
    let taps: [Tap; 3] = [
        ("untapped", |_| false),
        ("tapped", |_| true),
        ("every other tapped", |i| i % 2 == 1),
    ];
    let (mut badges, mut pairs, mut tightest) = (0, 0, f32::INFINITY);
    for seats in [2u8, 8] {
        for n in [2usize, 3, 4, 5, 6, 8, 10, 13, 17, 24, 32, 40] {
            for (tap, tapped) in taps {
                for staged in [false, true] {
                    let mut placed = placements(&crowded(seats, n, tapped));
                    // The first two cards of the local seat's creature row.
                    let row: Vec<usize> = (1..=2)
                        .filter_map(|slot| placed.iter().position(|p| p.object == obj(slot)))
                        .collect();
                    if let [first, second] = row[..] {
                        tightest =
                            tightest.min(placed[first].position.distance(placed[second].position));
                        if staged {
                            // Staged out of its group, as the board model does
                            // for a declared attacker, and forward of the row.
                            let forward = placed[first].slot.forward();
                            placed[first].position += forward * STAGE_STEP;
                            placed[first].badge = 0;
                        }
                    }
                    let (poses, cards, laid) = laid_badges(&placed);
                    let windows: Vec<[Vec2; 4]> = poses
                        .iter()
                        .map(|pose| on_table(pose, baylee_client_core::cardframe::window()))
                        .collect();
                    for (card, badge) in &laid {
                        badges += 1;
                        let own = cards.iter().position(|c| c == card);
                        for (other, window) in windows.iter().enumerate() {
                            if own == Some(other) {
                                continue;
                            }
                            pairs += 1;
                            assert!(
                                !overlap(badge, window),
                                "{seats} seats, rows of {n}, {tap}{}: a badge lies on the print of {:?}",
                                if staged { ", one staged" } else { "" },
                                placed[other].object
                            );
                        }
                    }
                }
            }
        }
    }
    assert!(
        (tightest - CARD_WIDTH * MIN_VISIBLE_FRACTION).abs() < 1e-4,
        "the rows never reach the tightest fan: {tightest}"
    );
    assert!(
        badges > 10_000 && pairs > badges,
        "{badges} badges, {pairs} pairs"
    );
}

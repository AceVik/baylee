//! The count badge on the table (#261): where its quad lies on the card, what
//! it lies over in a fanned row, and that it comes and goes with the count.

use super::flying_tests::{creature, duel};
use super::*;
use bevy::ecs::world::CommandQueue;

/// The badge's quad is where `cardplate` says, in the card's own space: its
/// left edge off the card's left edge, its top edge on the card's top edge.
///
/// Read back out of the transform and the mesh's size rather than restated,
/// so a flipped axis — a card's `+y` is its top, and its UV's is its bottom —
/// hangs the badge off the bottom corner and this goes red.
#[test]
fn the_badge_hangs_off_the_top_left_corner() {
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

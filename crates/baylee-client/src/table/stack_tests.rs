//! What stands under a card on the table (#261): the slabs of its deck,
//! jogged which way, and that the deck follows the count.

use super::flying_tests::{creature, duel};
use super::*;
use bevy::ecs::world::CommandQueue;

/// A table holding one merged card standing for `count`.
fn merged(count: usize) -> Placement {
    let mut group = creature(1, Vec::new());
    group.members = (1..=u32::try_from(count).expect("a small group"))
        .map(|n| ObjectId::new(n, 0))
        .collect();
    let mut placed = placements(&duel(vec![group]));
    assert_eq!(placed.len(), 1);
    placed.remove(0)
}

/// An index holding the handles `sync_stack` refuses to build without.
/// Dangling: it asks whether it has them and never what is in them.
fn index() -> SceneIndex {
    SceneIndex {
        quad: Some(Handle::default()),
        shadow_quad: Some(Handle::default()),
        shadow_material: Some(Handle::default()),
        ..default()
    }
}

fn sync(
    world: &mut World,
    index: &mut SceneIndex,
    materials: &mut Assets<CardMaterial>,
    card: Entity,
    placement: &Placement,
) {
    let mut queue = CommandQueue::default();
    let mut commands = Commands::new(&mut queue, world);
    sync_stack(&mut commands, index, materials, card, placement, MOVING);
    queue.apply(world);
}

/// The slabs under `card`, top first, as where each hangs and what it wears.
fn slabs(world: &mut World, card: Entity) -> Vec<(Vec3, Handle<CardMaterial>)> {
    let mut out: Vec<_> = world
        .query_filtered::<(&ChildOf, &Transform, &MeshMaterial3d<CardMaterial>), With<StackSlab>>()
        .iter(world)
        .filter(|(parent, ..)| parent.parent() == card)
        .map(|(_, at, material)| (at.translation, material.0.clone()))
        .collect();
    out.sort_by(|a, b| b.0.z.total_cmp(&a.0.z));
    out
}

/// The contact shadows under `card`, as where each lies.
fn shadows(world: &mut World, card: Entity) -> Vec<(Entity, Transform)> {
    world
        .query_filtered::<(Entity, &ChildOf, &Transform), With<CardShadow>>()
        .iter(world)
        .filter(|(_, parent, _)| parent.parent() == card)
        .map(|(entity, _, at)| (entity, *at))
        .collect()
}

/// The slabs hang between the card's face and the foot of its deck, one per
/// card under it up to the cap, and jog out right and left in turn — by the
/// jog and no further. And no slab carries a print.
#[test]
fn the_slabs_hang_under_the_card_and_jog_in_turn() {
    for count in [2usize, 3, 8, 15, 40] {
        let mut world = World::new();
        let mut materials = Assets::<CardMaterial>::default();
        let mut index = index();
        let card = world.spawn_empty().id();
        sync(&mut world, &mut index, &mut materials, card, &merged(count));
        let under = slabs(&mut world, card);
        assert_eq!(under.len(), stack_layers(count - 1), "{count} cards");
        let deck = stack_rise(count - 1);
        for (i, (at, handle)) in under.iter().enumerate() {
            let made = materials.get(handle).expect("a slab's material");
            assert!(
                made.art.is_none(),
                "{count} cards: slab {i} carries a print"
            );
            assert!(
                at.z < 0.0 && at.z >= -deck - 1e-6,
                "{count} cards: slab {i} hangs at {}, outside the deck of {deck}",
                at.z
            );
            assert!(
                (at.x.abs() - PILE_JOG * CARD_WIDTH).abs() < 1e-6,
                "{count} cards: slab {i} stands out {}",
                at.x
            );
            assert!(
                at.y.abs() < 1e-6,
                "{count} cards: slab {i} jogs along the card"
            );
            if let Some((next, _)) = under.get(i + 1) {
                assert!(
                    at.x * next.x < 0.0,
                    "{count} cards: slabs {i} and {} jog the same way",
                    i + 1
                );
            }
        }
    }
}

/// A group grows under the same top card, turn by turn, and its deck grows
/// with it: the slabs and the shadow are rebuilt for the new count. Built
/// once at spawn, a pile of Treasures that grew from two to twelve kept one
/// slab under a card that had risen to stand on eleven.
///
/// The same count again rebuilds nothing: the shadow is what a flier's
/// grounded shadow is remembered by.
#[test]
fn the_deck_follows_the_count() {
    let mut world = World::new();
    let mut materials = Assets::<CardMaterial>::default();
    let mut index = index();
    let card = world.spawn_empty().id();

    for count in [2usize, 12, 1, 12] {
        sync(&mut world, &mut index, &mut materials, card, &merged(count));
        let under = slabs(&mut world, card);
        assert_eq!(under.len(), stack_layers(count - 1), "{count} cards");
        let deck = stack_rise(count - 1);
        if let Some((foot, _)) = under.last() {
            assert!(
                (foot.z + deck).abs() < 1e-6,
                "{count} cards: the deck ends at {} and the card stands {deck} high",
                foot.z
            );
        }
        let shadow = shadows(&mut world, card);
        assert_eq!(shadow.len(), 1, "{count} cards: one shadow");
        let at = shadow[0].1;
        assert!(
            (at.translation.z + CARD_LIFT * 0.5 + deck).abs() < 1e-6,
            "{count} cards: the shadow lies at {} under a deck {deck} high",
            at.translation.z
        );
        assert!(
            (at.scale.x - (1.0 + deck * DECK_SHADOW_SPREAD)).abs() < 1e-6,
            "{count} cards: the shadow is the width of another deck"
        );
    }

    let before = shadows(&mut world, card)[0].0;
    let slabs_before: Vec<_> = slabs(&mut world, card)
        .into_iter()
        .map(|(at, _)| at)
        .collect();
    sync(&mut world, &mut index, &mut materials, card, &merged(12));
    assert_eq!(
        shadows(&mut world, card)[0].0,
        before,
        "the same count rebuilt the shadow"
    );
    assert_eq!(
        slabs(&mut world, card)
            .into_iter()
            .map(|(at, _)| at)
            .collect::<Vec<_>>(),
        slabs_before,
        "the same count moved the slabs"
    );
}

/// A library is face down (CR 401.2), so it is backs all the way down.
#[test]
fn a_library_is_backs_all_the_way_down() {
    let mut world = World::new();
    // A handle of its own, so a slab wearing anything else is told apart.
    let mut materials = Assets::<CardMaterial>::default();
    let blank = materials.add(material(
        CardLook::flat(BACK_COLOR, FinishTreatment::Plain),
        None,
        BACK_COLOR,
        MOVING,
    ));
    let index = SceneIndex {
        quad: Some(Handle::default()),
        blank: Some(blank.clone()),
        ..default()
    };
    let layout = TableLayout::new(&[PlayerId::new(0)], 1.78, None);
    let slot = layout.slot(PlayerId::new(0)).expect("the one seat");
    let mut library = baylee_client_core::ZonePile::empty(baylee_client_core::PileKind::Library);
    library.count = 20;

    let mut queue = CommandQueue::default();
    let mut commands = Commands::new(&mut queue, &world);
    let spawned = spawn_piles(&mut commands, &index, slot, &[library], 20);
    queue.apply(&mut world);

    assert_eq!(
        spawned.len(),
        stack_layers(20),
        "a slab per card up to the cap"
    );
    let first = world
        .get::<Transform>(spawned[0])
        .expect("a slab")
        .translation;
    for slab in spawned {
        let worn = world
            .get::<MeshMaterial3d<CardMaterial>>(slab)
            .expect("a material");
        assert_eq!(worn.0, blank, "a library slab wears something but the back");
        let at = world.get::<Transform>(slab).expect("a slab").translation;
        assert!(
            (at.x - first.x).abs() < 1e-6 && (at.z - first.z).abs() < 1e-6,
            "a library slab stands out of the deck"
        );
    }
}

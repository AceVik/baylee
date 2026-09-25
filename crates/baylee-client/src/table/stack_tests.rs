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

/// A merged pile's cards hang between its top card's face and the foot of
/// its deck, one per card under it up to five (the owner, 25.09: "bis 5
/// reichen aus"), each a further [`PILE_JOG`] out to the **left** of its
/// seat, tapped or not. Read in the seat's frame through the card's own
/// transform, so a tapped pile whose cards stepped towards the next row
/// would fail here. And no slab carries a print.
#[test]
fn a_pile_steps_out_to_the_left_five_cards_at_most() {
    for tapped in [false, true] {
        for count in [2usize, 3, 6, 15, 40] {
            let mut world = World::new();
            let mut materials = Assets::<CardMaterial>::default();
            let mut index = index();
            let card = world.spawn_empty().id();
            let mut placed = merged(count);
            placed.tapped = tapped;
            sync(&mut world, &mut index, &mut materials, card, &placed);
            let under = slabs(&mut world, card);
            assert_eq!(under.len(), (count - 1).min(PILE_SLABS), "{count} cards");
            let deck = stack_rise(under.len());
            let pose = card_transform(&placed.slot, placed.position, tapped, 0.0);
            let along = Vec3::new(placed.slot.facing.cos(), 0.0, placed.slot.facing.sin());
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
                let out = pose.rotation * Vec3::new(at.x, at.y, 0.0);
                #[allow(clippy::cast_precision_loss)] // five at most
                let step = (i + 1) as f32 * PILE_JOG * CARD_WIDTH;
                assert!(
                    (out.dot(along) + step).abs() < 1e-5,
                    "{count} cards, tapped {tapped}: slab {i} stands {} along its seat, not {} to the left",
                    out.dot(along),
                    step
                );
                assert!(
                    (out - along * out.dot(along)).length() < 1e-5,
                    "{count} cards, tapped {tapped}: slab {i} steps across the row"
                );
            }
        }
    }
}

/// A pile laid again when it taps: the same count, turned, is a pile whose
/// cards must still step out to the seat's left.
#[test]
fn a_pile_lays_its_cards_again_when_it_taps() {
    let mut world = World::new();
    let mut materials = Assets::<CardMaterial>::default();
    let mut index = index();
    let card = world.spawn_empty().id();
    let mut placed = merged(4);
    sync(&mut world, &mut index, &mut materials, card, &placed);
    let rest: Vec<Vec3> = slabs(&mut world, card)
        .into_iter()
        .map(|(at, _)| at)
        .collect();
    placed.tapped = true;
    sync(&mut world, &mut index, &mut materials, card, &placed);
    let turned: Vec<Vec3> = slabs(&mut world, card)
        .into_iter()
        .map(|(at, _)| at)
        .collect();
    assert_eq!(rest.len(), turned.len());
    for (a, b) in rest.iter().zip(&turned) {
        assert!(
            (a.x - b.y).abs() < 1e-6 && b.x.abs() < 1e-6,
            "{a} became {b}"
        );
    }
}

/// A pile reads as cards and not as one card on a block (#261, #298).
///
/// Three things make it: the pile stands out far enough to be seen at table
/// size (`PILE_JOG`, eight hundredths of a card), each of its at most
/// `PILE_SLABS` cards stands out further to the left than the one above it
/// so every edge shows and not only the first (#263), and the slabs
/// alternate between the back and a lighter edge, so the layers stripe.
/// Measured live on e72c8980, before any of it: at 0.012 a card the jog was
/// a pixel on a 94-pixel card, every slab was the back at the same offset,
/// and twenty Forests read as one Forest on a dark block.
#[test]
fn a_pile_shows_its_layers() {
    // The edge is a card's stock in shadow: at least twenty display levels
    // over the back, so the stripe is more than the felt's own grain, and
    // well short of the grey the owner took off with the frame.
    let level = |c: Color| {
        let s = c.to_srgba();
        255.0 * (0.2126 * s.red + 0.7152 * s.green + 0.0722 * s.blue)
    };
    let (back, edge) = (level(BACK_COLOR), level(SLAB_EDGE_COLOR));
    assert!(
        edge - back >= 20.0,
        "the edge ({edge:.0}) is only {:.0} levels over the back ({back:.0})",
        edge - back
    );
    assert!(
        edge <= 90.0,
        "the edge reads as grey paper at {edge:.0} of 255"
    );

    let mut world = World::new();
    let mut materials = Assets::<CardMaterial>::default();
    let mut index = index();
    let card = world.spawn_empty().id();
    sync(&mut world, &mut index, &mut materials, card, &merged(9));
    let under = slabs(&mut world, card);
    assert_eq!(under.len(), PILE_SLABS);
    let steps: Vec<f32> = under.iter().map(|(at, _)| -at.x).collect();
    for pair in steps.windows(2) {
        assert!(
            pair[1] > pair[0] + 1e-4,
            "a deeper slab does not stand out past the one above it: {steps:?}"
        );
    }
    let tints: Vec<Vec4> = under
        .iter()
        .map(|(_, handle)| {
            materials
                .get(handle)
                .expect("a slab's material")
                .params
                .tint
        })
        .collect();
    for pair in tints.windows(2) {
        assert_ne!(
            pair[0], pair[1],
            "two slabs in a row wear the same colour: {tints:?}"
        );
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
        assert_eq!(under.len(), (count - 1).min(PILE_SLABS), "{count} cards");
        let deck = stack_rise(under.len());
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

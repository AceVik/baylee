use super::*;
use crate::Duel;
use crate::table::flying_tests::{creature, duel};
use crate::table::{placements, placements_into};
use bevy::prelude::World;

fn input() -> CardInput {
    let board = duel(vec![creature(1, Vec::new())]);
    CardInput {
        decorations: DecorationReady::default(),
        placement: placements(&board).remove(0),
        eye: Transform::IDENTITY,
        still: false,
        finish: FinishTreatment::default(),
        glow: 0,
        veil: crate::cardmat::Veil::NONE,
        sweep: None,
        show_face: false,
        texture_epoch: 0,
        text_epoch: 0,
        seq: 0,
    }
}

#[test]
fn a_replaced_scene_cannot_inherit_a_previous_entities_decorations() {
    let mut world = World::new();
    let first = world.spawn_empty().id();
    let input = input();
    let stamp = CardStamp::new(first, &input, None);
    assert!(stamp.matches(first, &input, None));
    world.despawn(first);
    let replacement = world.spawn_empty().id();
    assert!(!stamp.matches(replacement, &input, None));
}

#[test]
fn late_assets_camera_preferences_and_card_changes_invalidate_the_drawing() {
    let mut world = World::new();
    let entity = world.spawn_empty().id();
    let input = input();
    let stamp = CardStamp::new(entity, &input, None);
    let changes: &[fn(&mut CardInput)] = &[
        |i| i.texture_epoch += 1,
        |i| i.text_epoch += 1,
        |i| i.eye.translation.x += 1.0,
        |i| i.still = !i.still,
        |i| i.show_face = !i.show_face,
        |i| i.placement.tapped = !i.placement.tapped,
        |i| i.placement.selected = !i.placement.selected,
        |i| i.placement.count += 1,
        |i| i.placement.slot.scale *= 0.5,
        |i| i.placement.shown = false,
        |i| i.placement.flying = true,
        |i| i.placement.offer.armed = true,
        |i| i.placement.veil = crate::cardmat::veil::PHASED,
        |i| i.glow = crate::cardmat::glow::OFFERS,
        |i| i.veil = crate::cardmat::Veil::settled(crate::cardmat::veil::ASLEEP),
    ];
    for change in changes {
        let mut changed = input;
        change(&mut changed);
        assert!(!stamp.matches(entity, &changed, None));
    }
    assert!(stamp.matches(entity, &input, None));
}

#[test]
fn a_same_sequence_replay_replacement_refits_the_constructed_face() {
    let mut world = World::new();
    let entity = world.spawn_empty().id();
    let input = CardInput {
        show_face: true,
        seq: 7,
        ..input()
    };
    let mut object = crate::registry_printed(1, 0, "Serra Angel");
    let stamp = CardStamp::new(entity, &input, Some(&object));
    assert!(stamp.matches(entity, &input, Some(&object)));
    object.name = "An authorized different face".into();
    assert!(!stamp.matches(entity, &input, Some(&object)));
    assert!(stamp.face_changed(&input, Some(&object)));
    assert!(
        !stamp.matches(entity, &input, None),
        "losing sight of a card invalidates its words"
    );
    assert!(stamp.face_changed(&input, None));
}

#[test]
fn cached_cards_keep_hover_and_flight_but_reduce_motion_stays_still() {
    let mut placement = input().placement;
    let (rest, _) = super::super::pose_of(&placement, None, false, 0.0);
    let (hover, unhovered) = super::super::pose_of(&placement, Some(placement.object), false, 1.0);
    assert_eq!(rest, unhovered);
    assert!(hover.translation.y > rest.translation.y);
    assert!(hover.scale.x > rest.scale.x);
    placement.flying = true;
    let first = super::super::pose_of(&placement, None, false, 0.0);
    let second = super::super::pose_of(&placement, None, false, 0.6);
    assert_ne!(first, second);
    let still = super::super::pose_of(&placement, None, true, 0.0);
    assert_eq!(still, super::super::pose_of(&placement, None, true, 0.6));
}

#[test]
fn placement_storage_is_reused_and_old_games_are_cleared() {
    let board = duel(vec![creature(1, Vec::new()), creature(2, Vec::new())]);
    let mut retained = Vec::new();
    placements_into(&board, &mut retained);
    let capacity = retained.capacity();
    assert!(capacity >= 2);
    let first = retained.clone();
    placements_into(&board, &mut retained);
    assert!(first == retained);
    assert_eq!(retained.capacity(), capacity);
    placements_into(&Duel::default(), &mut retained);
    assert!(retained.is_empty());
    assert_eq!(retained.capacity(), capacity);
}

#[test]
fn changing_draw_inputs_keeps_the_owned_public_object_buffers() {
    let mut world = World::new();
    let entity = world.spawn_empty().id();
    let mut input = CardInput {
        show_face: true,
        ..input()
    };
    let mut object = crate::registry_printed(1, 0, "Serra Angel");
    object.counters.push(baylee_view::CounterEntry {
        kind: baylee_view::CounterKind::Charge,
        count: 2,
    });
    let mut stamp = CardStamp::new(entity, &input, Some(&object));
    let owned = stamp.face.as_ref().unwrap();
    let name = owned.name.as_ptr();
    let counters = owned.counters.as_ptr();
    assert_ne!(name, object.name.as_ptr(), "the cache owns its snapshot");
    assert_ne!(counters, object.counters.as_ptr());

    // Every step requires drawing work, but neither a camera move, a lane
    // repack nor late assets should allocate another identical snapshot.
    let changes: &[fn(&mut CardInput)] = &[
        |i| i.eye.translation.x += 1.0,
        |i| i.placement.lift += 0.1,
        |i| i.placement.slot.scale *= 0.9,
        |i| i.texture_epoch += 1,
        |i| i.text_epoch += 1,
        |i| i.seq += 1,
    ];
    for change in changes {
        change(&mut input);
        assert!(!stamp.matches(entity, &input, Some(&object)));
        stamp.update(entity, &input, Some(&object));
        assert!(stamp.matches(entity, &input, Some(&object)));
        let owned = stamp.face.as_ref().unwrap();
        assert_eq!(owned.name.as_ptr(), name, "name storage must be retained");
        assert_eq!(
            owned.counters.as_ptr(),
            counters,
            "projected counters must retain their storage too"
        );
        assert_eq!(owned, &object);
    }
}

#[test]
fn retained_snapshots_refresh_at_the_same_sequence_and_forget_hidden_faces() {
    let mut world = World::new();
    let entity = world.spawn_empty().id();
    let mut input = CardInput {
        show_face: true,
        seq: 7,
        ..input()
    };
    let mut object = crate::registry_printed(1, 0, "Serra Angel");
    let mut stamp = CardStamp::new(entity, &input, Some(&object));
    object.name = "A different authorized face".into();
    assert!(stamp.face_changed(&input, Some(&object)));
    stamp.update(entity, &input, Some(&object));
    assert_eq!(stamp.face.as_ref(), Some(&object));
    assert!(!stamp.face_changed(&input, Some(&object)));

    stamp.update(entity, &input, None);
    assert!(
        stamp.face.is_none(),
        "losing public visibility drops the copy"
    );
    assert!(!stamp.matches(entity, &input, Some(&object)));
    stamp.update(entity, &input, Some(&object));
    input.show_face = false;
    stamp.update(entity, &input, Some(&object));
    assert!(stamp.face.is_none(), "art does not retain a text snapshot");
    input.show_face = true;
    assert!(!stamp.matches(entity, &input, Some(&object)));
    stamp.update(entity, &input, Some(&object));
    assert_eq!(stamp.face.as_ref(), Some(&object));
}

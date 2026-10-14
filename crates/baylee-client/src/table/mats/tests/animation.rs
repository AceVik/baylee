use super::*;

#[test]
fn unchanged_target_keeps_easing_without_registry_reads_or_duel_writes() {
    let mut table = Table::new(tower(ManaColor::Green), false);
    let target = baylee_client_core::firewheel::strength([0.0, 0.0, 0.0, 0.0, 1.0], 2)[4];
    let scans = reads();
    table.step(0.1);
    let first = table.shown()[4];
    let changed = table.app.world().resource_ref::<Duel>().last_changed();
    table.step(0.1);
    let second = table.shown()[4];
    table.step(0.1);
    let third = table.shown()[4];
    assert!(0.0 < first && first < second && second < third && third < target);
    assert_eq!(
        reads(),
        scans,
        "three frames must not walk the mana registry"
    );
    assert_eq!(
        table.app.world().resource_ref::<Duel>().last_changed(),
        changed
    );
}

#[test]
fn reduce_motion_snaps_to_each_snapshot_and_rest_stops_material_writes() {
    let mut table = Table::new(tower(ManaColor::Green), true);
    table.step(0.1);
    let expected = baylee_client_core::firewheel::strength([0.0, 0.0, 0.0, 0.0, 1.0], 2);
    assert_eq!(table.shown().map(f32::to_bits), expected.map(f32::to_bits));
    let changed = table
        .app
        .world()
        .resource_ref::<Assets<FeltMaterial>>()
        .last_changed();
    let scans = reads();
    table.step(0.1);
    assert_eq!(
        table
            .app
            .world()
            .resource_ref::<Assets<FeltMaterial>>()
            .last_changed(),
        changed
    );
    assert_eq!(reads(), scans);

    // Same sequence and same layout, but a new public mana projection. Join
    // the real snapshot receiver to the next material update.
    table
        .app
        .world_mut()
        .resource_mut::<Duel>()
        .receive_view(tower(ManaColor::Blue));
    let scans = reads();
    table.step(0.1);
    let expected = baylee_client_core::firewheel::strength([0.0, 1.0, 0.0, 0.0, 0.0], 2);
    assert_eq!(table.shown().map(f32::to_bits), expected.map(f32::to_bits));
    assert_eq!(reads(), scans);
}

#[test]
fn seat_count_and_layout_direction_remain_live_with_the_same_mana_snapshot() {
    let duel = received(tower(ManaColor::Green));
    let mut board = duel.board.as_ref().unwrap().clone();
    let mut layout = duel.settled_layout().unwrap().clone();
    let scans = reads();
    let (flames, tail) = firewheel_of(&duel, &board, &layout);
    board.pods.truncate(1);
    let (_, fewer_seats) = firewheel_of(&duel, &board, &layout);
    assert!(
        fewer_seats.x > tail.x,
        "one source has more weight at one seat"
    );
    let local = layout.slots.iter_mut().find(|slot| slot.is_local).unwrap();
    local.center = Vec2::new(-10.0, 0.0);
    let (turned_flames, turned_tail) = firewheel_of(&duel, &board, &layout);
    assert_eq!(turned_flames, flames);
    assert_eq!(turned_tail.x.to_bits(), fewer_seats.x.to_bits());
    assert_eq!(turned_tail.yz(), Vec2::X);
    assert_ne!(turned_tail.yz(), tail.yz());
    assert_eq!(
        reads(),
        scans,
        "layout changes must not reread mana sources"
    );
}

#[test]
fn cached_target_easing_is_independent_of_frame_partition() {
    let mut coarse = Table::new(tower(ManaColor::Red), false);
    let mut fine = Table::new(tower(ManaColor::Red), false);
    let scans = reads();
    for _ in 0..10 {
        coarse.step(0.1);
        fine.step(0.05);
        fine.step(0.05);
    }
    let a = coarse.shown()[3];
    let b = fine.shown()[3];
    assert!(a > 0.0 && b > 0.0);
    assert!(
        (a - b).abs() < 1e-6,
        "the same second must produce the same easing"
    );
    assert_eq!(
        reads(),
        scans,
        "neither frame rate should recalculate the target"
    );
}

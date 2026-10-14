//! Differential motion contracts and a bounded, opt-in CPU benchmark.

// Snapping must assign the exact goal; tolerance would hide unfinished motion.
#![allow(clippy::float_cmp)]

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_client_core::{combat::LineEnd, strike::Strike};
use bevy::ecs::system::{RunSystemOnce, System};
use std::{hint::black_box, time::Duration};

// Kept independent of advance_card: this is the pre-optimization scan, including
// the exact arithmetic/order and moving-count semantics. PlainMotion also lets
// the benchmark charge summary-tick writes only to the candidate.
trait Goal: Component {
    fn target(&self) -> Transform;
}
impl Goal for Motion {
    fn target(&self) -> Transform {
        self.target
    }
}
#[derive(Component)]
struct PlainMotion(Transform);
impl Goal for PlainMotion {
    fn target(&self) -> Transform {
        self.0
    }
}
fn original<M: Goal>(
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut cards: Query<(
        &M,
        &mut Transform,
        Option<&crate::combatfx::Recoil>,
        Option<&Visibility>,
    )>,
    report: Option<ResMut<GlideReport>>,
    placing: Option<ResMut<Placing>>,
) {
    let placed = placing.is_some_and(|mut placing| placing.frames > 0 && placing.spend());
    let still = prefs.all().reduce_motion || placed;
    let t = 1.0 - (-16.0 * time.delta_secs()).exp();
    let mut moving = 0;
    for (motion, mut transform, recoil, seen) in &mut cards {
        let mut target = motion.target();
        if !still && let Some(recoil) = recoil {
            target.translation += recoil.offset(time.elapsed_secs());
        }
        let there = transform.translation.distance_squared(target.translation) < 0.0008 * 0.0008
            && (transform.rotation.abs_diff_eq(target.rotation, 0.0008)
                || transform.rotation.abs_diff_eq(-target.rotation, 0.0008))
            && transform.scale.distance_squared(target.scale) < 0.0008 * 0.0008;
        let hidden = matches!(seen, Some(Visibility::Hidden));
        if still || there || hidden {
            if *transform != target {
                *transform = target;
            }
            continue;
        }
        moving += 1;
        transform.translation = transform.translation.lerp(target.translation, t);
        transform.rotation = transform.rotation.slerp(target.rotation, t);
        transform.scale = transform.scale.lerp(target.scale, t);
    }
    if let Some(mut report) = report {
        report.note(moving);
    }
}

fn app(reference: bool) -> App {
    let mut app = App::new();
    app.init_resource::<Time>()
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<GlideReport>()
        .init_resource::<Placing>();
    if reference {
        app.add_systems(Update, original::<Motion>);
    } else {
        app.add_systems(Update, glide);
    }
    app
}

#[derive(Debug, PartialEq)]
struct Frame {
    poses: Vec<Option<[u32; 10]>>,
    report: GlideReport,
}
fn step(app: &mut App, cards: &[Entity], seconds: f32) -> Frame {
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs_f32(seconds));
    app.update();
    Frame {
        poses: cards
            .iter()
            .map(|e| {
                app.world().get::<Transform>(*e).map(|t| {
                    [
                        t.translation.x,
                        t.translation.y,
                        t.translation.z,
                        t.rotation.x,
                        t.rotation.y,
                        t.rotation.z,
                        t.rotation.w,
                        t.scale.x,
                        t.scale.y,
                        t.scale.z,
                    ]
                    .map(f32::to_bits)
                })
            })
            .collect(),
        report: *app.world().resource::<GlideReport>(),
    }
}
fn same_as_original(scenario: impl Fn(&mut App) -> Vec<Frame>) {
    assert_eq!(scenario(&mut app(false)), scenario(&mut app(true)));
}
fn card(app: &mut App, target: Transform, at: Transform) -> Entity {
    app.world_mut().spawn((Motion { target }, at)).id()
}
fn retarget(app: &mut App, e: Entity, target: Transform) {
    app.world_mut().get_mut::<Motion>(e).unwrap().target = target;
}
fn at(app: &App, e: Entity) -> Transform {
    *app.world().get::<Transform>(e).unwrap()
}

#[derive(Resource, Default)]
struct WakeProbe {
    active: EntityHashSet,
    inspected: usize,
}
fn probe_targets(targets: MotionTargets, mut probe: ResMut<WakeProbe>) {
    let probe = &mut *probe;
    probe.active.clear();
    probe.inspected = wake_targets(&targets, &mut probe.active);
}

#[test]
fn unchanged_columns_skip_target_ticks_with_a_correct_fallback_without_summary_ticks() {
    let mut app = App::new();
    app.init_resource::<WakeProbe>()
        .add_systems(Update, probe_targets);
    let a: Vec<_> = (0..3)
        .map(|_| card(&mut app, Transform::IDENTITY, Transform::IDENTITY))
        .collect();
    for _ in 0..2 {
        let e = card(&mut app, Transform::IDENTITY, Transform::IDENTITY);
        app.world_mut().entity_mut(e).insert(Floating);
    }
    // No Transform yet: it must not enter the active set until there is a pose.
    app.world_mut().spawn(Motion {
        target: Transform::IDENTITY,
    });
    app.update();
    assert_eq!(app.world().resource::<WakeProbe>().inspected, 5);
    app.update();
    assert_eq!(
        app.world().resource::<WakeProbe>().inspected,
        if Motion::HAS_SUMMARY_TICK { 0 } else { 5 }
    );
    assert!(app.world().resource::<WakeProbe>().active.is_empty());
    retarget(&mut app, a[1], Transform::from_xyz(1.0, 0.0, 0.0));
    app.update();
    let probe = app.world().resource::<WakeProbe>();
    assert_eq!(
        probe.inspected,
        if Motion::HAS_SUMMARY_TICK { 3 } else { 5 },
        "only summary ticks can skip the other column"
    );
    assert_eq!(probe.active.len(), 1);
    assert!(probe.active.contains(&a[1]));
    app.update();
    assert_eq!(
        app.world().resource::<WakeProbe>().inspected,
        if Motion::HAS_SUMMARY_TICK { 0 } else { 5 }
    );
}

#[test]
fn a_glide_continues_without_target_writes_settles_without_writes_and_reactivates() {
    same_as_original(|app| {
        let target = Transform {
            translation: Vec3::new(9.0, 0.4, -2.0),
            rotation: Quat::from_rotation_z(1.3),
            scale: Vec3::splat(1.4),
        };
        let e = card(app, target, Transform::IDENTITY);
        let mut frames = vec![step(app, &[e], 0.0)];
        assert_eq!(at(app, e), Transform::IDENTITY);
        for seconds in [1.0 / 30.0, 1.0 / 144.0, 1.0 / 60.0] {
            let previous = at(app, e);
            frames.push(step(app, &[e], seconds));
            assert_ne!(at(app, e), previous);
            assert_eq!(frames.last().unwrap().report.moving, 1);
        }
        for _ in 0..90 {
            frames.push(step(app, &[e], 1.0 / 60.0));
        }
        assert_eq!(at(app, e), target);
        let tick = app
            .world()
            .entity(e)
            .get_ref::<Transform>()
            .unwrap()
            .last_changed();
        for _ in 0..8 {
            frames.push(step(app, &[e], 1.0 / 60.0));
        }
        assert_eq!(
            app.world()
                .entity(e)
                .get_ref::<Transform>()
                .unwrap()
                .last_changed(),
            tick
        );
        retarget(app, e, Transform::IDENTITY);
        frames.push(step(app, &[e], 1.0 / 60.0));
        assert_ne!(at(app, e), target);
        assert_ne!(at(app, e), Transform::IDENTITY);
        frames.push(step(app, &[e], 1.0 / 60.0));
        assert_eq!(frames.last().unwrap().report.moving, 1);
        frames
    });
}

#[test]
fn external_transforms_hidden_snaps_and_optional_visibility_reactivate_settled_cards() {
    same_as_original(|app| {
        let target = Transform::from_xyz(4.0, 0.0, -2.0);
        let e = card(app, target, target);
        let mut frames = vec![step(app, &[e], 0.016), step(app, &[e], 0.016)];
        *app.world_mut().get_mut::<Transform>(e).unwrap() = Transform::IDENTITY;
        frames.push(step(app, &[e], 0.016));
        assert_eq!(frames.last().unwrap().report.moving, 1);
        app.world_mut().entity_mut(e).insert(Visibility::Hidden);
        frames.push(step(app, &[e], 0.016));
        assert_eq!(at(app, e), target);
        assert_eq!(frames.last().unwrap().report.moving, 0);
        retarget(app, e, Transform::from_xyz(-5.0, 2.0, 3.0));
        frames.push(step(app, &[e], 0.016));
        assert_eq!(at(app, e).translation.x, -5.0);
        app.world_mut().entity_mut(e).remove::<Visibility>();
        retarget(app, e, target);
        frames.push(step(app, &[e], 0.016));
        assert_ne!(at(app, e), target);
        app.world_mut().entity_mut(e).insert(Visibility::Hidden);
        frames.push(step(app, &[e], 0.016));
        *app.world_mut().get_mut::<Transform>(e).unwrap() = Transform::IDENTITY;
        frames.push(step(app, &[e], 0.016));
        assert_eq!(at(app, e), target);
        frames
    });
}

#[test]
fn resting_cards_keep_exact_poses_through_still_placing_and_visibility_only_changes() {
    same_as_original(|app| {
        let target = Transform {
            translation: Vec3::new(4.0, 0.2, -2.0),
            rotation: Quat::from_rotation_z(0.75),
            scale: Vec3::splat(1.2),
        };
        let e = card(app, target, target);
        // Existing cards, but no unfinished motion or recoil in the active set.
        let mut frames = vec![step(app, &[e], 0.016), step(app, &[e], 0.016)];
        let transform_tick = app
            .world()
            .entity(e)
            .get_ref::<Transform>()
            .unwrap()
            .last_changed();
        let motion_tick = app
            .world()
            .entity(e)
            .get_ref::<Motion>()
            .unwrap()
            .last_changed();
        for still in [true, false] {
            app.world_mut()
                .resource_mut::<crate::prefs::Prefs>()
                .edit()
                .reduce_motion = still;
            frames.push(step(app, &[e], 0.016));
            app.world_mut().resource_mut::<Placing>().arm();
            for remaining in [2, 1, 0] {
                frames.push(step(app, &[e], 0.016));
                assert_eq!(app.world().resource::<Placing>().frames, remaining);
            }
        }
        // Removal stands alone: no retarget or transform write can mask it.
        for seen in [
            Some(Visibility::Hidden),
            None,
            Some(Visibility::Visible),
            Some(Visibility::Inherited),
        ] {
            if let Some(seen) = seen {
                app.world_mut().entity_mut(e).insert(seen);
            } else {
                app.world_mut().entity_mut(e).remove::<Visibility>();
            }
            frames.push(step(app, &[e], 0.016));
        }
        assert!(frames.iter().all(|frame| frame.poses == frames[0].poses));
        assert!(frames.iter().all(|frame| frame.report.moving == 0));
        assert_eq!(
            app.world()
                .entity(e)
                .get_ref::<Transform>()
                .unwrap()
                .last_changed(),
            transform_tick
        );
        assert_eq!(
            app.world()
                .entity(e)
                .get_ref::<Motion>()
                .unwrap()
                .last_changed(),
            motion_tick
        );
        frames
    });
}

#[test]
fn settings_and_three_replay_placing_frames_cut_in_flight_glides() {
    same_as_original(|app| {
        let e = card(app, Transform::from_xyz(8.0, 0.0, 0.0), Transform::IDENTITY);
        let mut frames = vec![step(app, &[e], 0.016)];
        app.world_mut()
            .resource_mut::<crate::prefs::Prefs>()
            .edit()
            .reduce_motion = true;
        frames.push(step(app, &[e], 0.016));
        assert_eq!(at(app, e).translation.x, 8.0);
        app.world_mut()
            .resource_mut::<crate::prefs::Prefs>()
            .edit()
            .reduce_motion = false;
        retarget(app, e, Transform::IDENTITY);
        frames.push(step(app, &[e], 0.016));
        assert!(at(app, e).translation.x > 0.0);
        app.world_mut().resource_mut::<Placing>().arm();
        for x in [1.0, 2.0, 3.0] {
            retarget(app, e, Transform::from_xyz(x, 0.0, 0.0));
            frames.push(step(app, &[e], 0.016));
            assert_eq!(at(app, e).translation.x, x);
        }
        assert_eq!(app.world().resource::<Placing>().frames, 0);
        retarget(app, e, Transform::from_xyz(9.0, 0.0, 0.0));
        frames.push(step(app, &[e], 0.016));
        assert!(at(app, e).translation.x < 9.0);
        frames
    });
}

#[test]
fn placing_spends_empty_frames_and_does_not_survive_its_three_frames() {
    same_as_original(|app| {
        app.world_mut().resource_mut::<Placing>().arm();
        let mut frames = vec![step(app, &[], 0.016), step(app, &[], 0.016)];
        let e = card(app, Transform::from_xyz(3.0, 0.0, 0.0), Transform::IDENTITY);
        frames.push(step(app, &[e], 0.016));
        assert_eq!(at(app, e).translation.x, 3.0);
        retarget(app, e, Transform::IDENTITY);
        frames.push(step(app, &[e], 0.016));
        assert!(at(app, e).translation.x > 0.0);
        frames
    });
}

#[test]
fn departures_component_removal_reinsertion_and_reused_entity_indices_match_the_scan() {
    same_as_original(|app| {
        let e = card(app, Transform::from_xyz(8.0, 0.0, 0.0), Transform::IDENTITY);
        app.world_mut().entity_mut(e).insert(CardVisual {
            object: ObjectId::new(0, 0),
            count: 1,
        });
        let mut frames = vec![step(app, &[e], 0.016)];
        app.world_mut()
            .entity_mut(e)
            .remove::<CardVisual>()
            .insert(Departing { left: EXIT_LIFE });
        retarget(app, e, Transform::from_xyz(-8.0, 0.0, 0.0));
        let before = at(app, e).translation.x;
        frames.push(step(app, &[e], 0.016));
        assert!(
            at(app, e).translation.x < before,
            "departing cards still glide"
        );
        app.world_mut().entity_mut(e).remove::<Motion>();
        let before = at(app, e);
        frames.push(step(app, &[e], 0.016));
        assert_eq!(at(app, e), before);
        app.world_mut().entity_mut(e).insert(Motion {
            target: Transform::IDENTITY,
        });
        frames.push(step(app, &[e], 0.016));
        app.world_mut().entity_mut(e).remove::<Transform>();
        frames.push(step(app, &[e], 0.016));
        app.world_mut()
            .entity_mut(e)
            .insert(Transform::from_xyz(5.0, 0.0, 0.0));
        frames.push(step(app, &[e], 0.016));
        assert!(
            at(app, e).translation.x < 5.0,
            "unchanged Motion with a new pose wakes"
        );
        // Bevy batches freed IDs; immediate spawn() need not reuse this slot.
        // Its public no-free/spawn-at pair gives the next generation directly.
        let replacement = app.world_mut().entity_mut(e).despawn_no_free();
        app.world_mut()
            .spawn_at(
                replacement,
                (
                    Motion {
                        target: Transform::IDENTITY,
                    },
                    Transform::IDENTITY,
                ),
            )
            .unwrap();
        assert_eq!(replacement.index(), e.index(), "exercise actual slot reuse");
        assert_ne!(replacement, e);
        frames.push(step(app, &[replacement], 0.016));
        assert_eq!(frames.last().unwrap().report.moving, 0);
        assert_eq!(at(app, replacement), Transform::IDENTITY);
        retarget(app, replacement, Transform::from_xyz(1.0, 0.0, 0.0));
        frames.push(step(app, &[replacement], 0.016));
        assert_eq!(frames.last().unwrap().report.moving, 1);
        app.world_mut().entity_mut(replacement).despawn();
        frames.push(step(app, &[replacement], 0.016));
        assert_eq!(frames.last().unwrap().report.moving, 0);
        assert!(frames.last().unwrap().poses[0].is_none());
        frames
    });
}

fn recoil_cards(app: &mut App) -> [Entity; 2] {
    app.init_resource::<Duel>()
        .init_resource::<ShownRig>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>();
    [0, 1].map(|id| {
        let target = Transform::from_xyz(id as f32 * 4.0, 0.0, 0.0);
        let e = card(app, target, target);
        app.world_mut().entity_mut(e).insert(CardVisual {
            object: ObjectId::new(id, 0),
            count: 1,
        });
        e
    })
}
fn strike(app: &mut App, delayed: bool) {
    let mut strikes = Vec::new();
    if delayed {
        strikes.push(Strike {
            source: ObjectId::new(1, 0),
            target: LineEnd::Object(ObjectId::new(0, 0)),
            first: true,
        });
    }
    strikes.push(Strike {
        source: ObjectId::new(0, 0),
        target: LineEnd::Object(ObjectId::new(1, 0)),
        first: false,
    });
    app.world_mut().resource_mut::<Duel>().strikes = strikes;
    app.world_mut()
        .run_system_once(crate::combatfx::animate)
        .unwrap();
}

#[test]
fn delayed_recoil_keeps_advancing_and_removal_settles_even_with_no_motion_change() {
    same_as_original(|app| {
        let ids = recoil_cards(app);
        let mut frames = vec![step(app, &ids, 0.016), step(app, &ids, 0.016)];
        strike(app, true);
        for _ in 0..10 {
            frames.push(step(app, &ids, 0.016));
            assert_eq!(
                at(app, ids[0]),
                Transform::IDENTITY,
                "strike starts after 0.32 s"
            );
        }
        for _ in 0..20 {
            frames.push(step(app, &ids, 0.016));
        }
        assert!(at(app, ids[0]).translation.x > 0.0);
        // Replacement is also a zero-offset start before its next pulse.
        strike(app, false);
        frames.push(step(app, &ids, 0.016));
        for e in ids {
            app.world_mut()
                .entity_mut(e)
                .remove::<crate::combatfx::Recoil>();
        }
        for _ in 0..90 {
            frames.push(step(app, &ids, 0.016));
        }
        assert_eq!(at(app, ids[0]), Transform::IDENTITY);
        assert_eq!(at(app, ids[1]), Transform::from_xyz(4.0, 0.0, 0.0));
        assert_eq!(frames.last().unwrap().report.moving, 0);
        frames
    });
}

#[test]
fn hidden_recoil_and_mid_pulse_reduce_motion_and_placing_preserve_the_clock() {
    same_as_original(|app| {
        let ids = recoil_cards(app);
        let mut frames = vec![step(app, &ids, 0.016), step(app, &ids, 0.016)];
        strike(app, false);
        app.world_mut()
            .entity_mut(ids[0])
            .insert(Visibility::Hidden);
        frames.push(step(app, &ids, 0.10));
        assert!(
            at(app, ids[0]).translation.x > 0.0,
            "hidden snaps to the recoil goal too"
        );
        assert_eq!(frames.last().unwrap().report.moving, 0);
        app.world_mut()
            .resource_mut::<crate::prefs::Prefs>()
            .edit()
            .reduce_motion = true;
        frames.push(step(app, &ids, 0.02));
        assert_eq!(at(app, ids[0]), Transform::IDENTITY);
        app.world_mut()
            .resource_mut::<crate::prefs::Prefs>()
            .edit()
            .reduce_motion = false;
        frames.push(step(app, &ids, 0.02));
        assert!(at(app, ids[0]).translation.x > 0.0);
        app.world_mut().resource_mut::<Placing>().arm();
        for _ in 0..3 {
            frames.push(step(app, &ids, 0.02));
            assert_eq!(at(app, ids[0]), Transform::IDENTITY);
        }
        frames.push(step(app, &ids, 0.02));
        assert!(at(app, ids[0]).translation.x > 0.0);
        app.world_mut().entity_mut(ids[0]).remove::<Visibility>();
        frames.push(step(app, &ids, 0.02));
        // Aging/removal remains combatfx's job; the glide must return to rest.
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs(1));
        app.world_mut()
            .run_system_once(crate::combatfx::age)
            .unwrap();
        assert!(app.world().get::<crate::combatfx::Recoil>(ids[0]).is_none());
        for _ in 0..90 {
            frames.push(step(app, &ids, 0.016));
        }
        assert_eq!(at(app, ids[0]), Transform::IDENTITY);
        frames
    });
}

#[test]
fn age_removes_recoil_before_glide_on_reduce_motion_and_the_old_pulse_never_resumes() {
    for hidden in [false, true] {
        same_as_original(|app| {
            let ids = recoil_cards(app);
            // Match the production ordering, including deferred Recoil removal.
            app.add_systems(
                Update,
                crate::combatfx::age
                    .before(glide)
                    .before(original::<Motion>),
            );
            if hidden {
                app.world_mut()
                    .entity_mut(ids[0])
                    .insert(Visibility::Hidden);
            }
            let mut frames = vec![step(app, &ids, 0.016), step(app, &ids, 0.016)];
            let motion_tick = app
                .world()
                .entity(ids[0])
                .get_ref::<Motion>()
                .unwrap()
                .last_changed();
            strike(app, false);
            frames.push(step(app, &ids, 0.10));
            assert!(app.world().get::<crate::combatfx::Recoil>(ids[0]).is_some());
            assert!(
                at(app, ids[0]).translation.x > 0.0,
                "a real pulse must be in flight"
            );
            app.world_mut()
                .resource_mut::<crate::prefs::Prefs>()
                .edit()
                .reduce_motion = true;
            frames.push(step(app, &ids, 0.016));
            assert!(app.world().get::<crate::combatfx::Recoil>(ids[0]).is_none());
            assert_eq!(at(app, ids[0]), Transform::IDENTITY);
            assert_eq!(frames.last().unwrap().report.moving, 0);
            let resting_tick = app
                .world()
                .entity(ids[0])
                .get_ref::<Transform>()
                .unwrap()
                .last_changed();
            app.world_mut()
                .resource_mut::<crate::prefs::Prefs>()
                .edit()
                .reduce_motion = false;
            // Still inside the original 0.62 s pulse: a retained recoil would
            // visibly resume now. No new target is written to wake the glide.
            for _ in 0..4 {
                frames.push(step(app, &ids, 0.016));
                assert_eq!(at(app, ids[0]), Transform::IDENTITY);
                assert_eq!(at(app, ids[1]), Transform::from_xyz(4.0, 0.0, 0.0));
                assert_eq!(frames.last().unwrap().report.moving, 0);
            }
            assert_eq!(
                app.world()
                    .entity(ids[0])
                    .get_ref::<Transform>()
                    .unwrap()
                    .last_changed(),
                resting_tick
            );
            assert_eq!(
                app.world()
                    .entity(ids[0])
                    .get_ref::<Motion>()
                    .unwrap()
                    .last_changed(),
                motion_tick
            );
            frames
        });
    }
}

#[derive(Resource, Default)]
struct ModeProbe {
    dense_before: bool,
    active_before: usize,
}

fn traced_glide(
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    cards: GlideCards,
    report: Option<ResMut<GlideReport>>,
    placing: Option<ResMut<Placing>>,
    mut probe: ResMut<ModeProbe>,
) {
    probe.dense_before = *cards.dense;
    probe.active_before = cards.active.len();
    assert!(
        cards.candidates.is_empty(),
        "candidate storage is reused, not retained as work"
    );
    if *cards.dense {
        assert!(
            cards.active.is_empty(),
            "dense frames must not populate the hash set"
        );
    }
    glide(time, prefs, cards, report, placing);
}

fn modes_match_original(scenario: impl Fn(&mut App) -> Vec<Frame>) {
    let mut candidate = App::new();
    candidate
        .init_resource::<Time>()
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<GlideReport>()
        .init_resource::<Placing>()
        .init_resource::<ModeProbe>()
        .add_systems(Update, traced_glide);
    assert_eq!(scenario(&mut candidate), scenario(&mut app(true)));
}

fn expect_mode(app: &App, dense: bool, active: usize) {
    if let Some(probe) = app.world().get_resource::<ModeProbe>() {
        assert_eq!(probe.dense_before, dense);
        assert_eq!(probe.active_before, active);
    }
}

#[test]
fn dense_returns_to_one_flier_and_reenters_for_external_targets_and_component_changes() {
    modes_match_original(|app| {
        let ids: Vec<_> = (0..16)
            .map(|_| card(app, Transform::from_xyz(8.0, 0.0, 0.0), Transform::IDENTITY))
            .collect();
        let mut frames = vec![step(app, &ids, 0.016), step(app, &ids, 0.016)];
        expect_mode(app, true, 0);
        assert_eq!(frames.last().unwrap().report.moving, 16);
        retarget(app, ids[0], Transform::from_xyz(-8.0, 0.0, 0.0));
        *app.world_mut().get_mut::<Transform>(ids[1]).unwrap() = Transform::IDENTITY;
        app.world_mut()
            .entity_mut(ids[2])
            .insert(Visibility::Hidden);
        app.world_mut().entity_mut(ids[3]).remove::<Motion>();
        app.world_mut().entity_mut(ids[4]).remove::<Transform>();
        app.world_mut().entity_mut(ids[5]).despawn();
        frames.push(step(app, &ids, 0.016));
        expect_mode(app, true, 0);
        app.world_mut().entity_mut(ids[3]).insert(Motion {
            target: Transform::IDENTITY,
        });
        app.world_mut()
            .entity_mut(ids[4])
            .insert(Transform::IDENTITY);
        // One continuously changing flying target must not pin all cards to
        // the dense path after the others finish their entry animations.
        for n in 0..90_usize {
            let x = if n.is_multiple_of(2) { 9.0 } else { -9.0 };
            retarget(app, ids[0], Transform::from_xyz(x, 0.0, 0.0));
            frames.push(step(app, &ids, 0.016));
        }
        expect_mode(app, false, 1);
        assert_eq!(frames.last().unwrap().report.moving, 1);
        // Wake more than a quarter while sparse, including an external pose
        // change. The next frame must use a dense scan without target writes.
        for &e in &ids[6..10] {
            retarget(app, e, Transform::from_xyz(-8.0, 0.0, 0.0));
        }
        *app.world_mut().get_mut::<Transform>(ids[10]).unwrap() = Transform::IDENTITY;
        app.world_mut().entity_mut(ids[2]).remove::<Visibility>();
        frames.push(step(app, &ids, 0.016));
        expect_mode(app, false, 1);
        frames.push(step(app, &ids, 0.016));
        expect_mode(app, true, 0);
        app.world_mut().resource_mut::<Placing>().arm();
        frames.push(step(app, &ids, 0.016));
        assert_eq!(frames.last().unwrap().report.moving, 0);
        for _ in 0..3 {
            frames.push(step(app, &ids, 0.016));
            expect_mode(app, false, 0);
        }
        assert_eq!(app.world().resource::<Placing>().frames, 0);
        frames
    });
}

#[test]
fn dense_carries_delayed_recoil_into_sparse_and_settles_after_removal_in_dense() {
    modes_match_original(|app| {
        let recoil = recoil_cards(app);
        let mut ids = recoil.to_vec();
        ids.extend(
            (0..14).map(|_| card(app, Transform::from_xyz(8.0, 0.0, 0.0), Transform::IDENTITY)),
        );
        let mut frames = vec![step(app, &ids, 0.016), step(app, &ids, 0.016)];
        expect_mode(app, true, 0);
        strike(app, true);
        for &e in &ids[2..] {
            app.world_mut().entity_mut(e).insert(Visibility::Hidden);
        }
        frames.push(step(app, &ids, 0.10));
        expect_mode(app, true, 0);
        frames.push(step(app, &ids, 0.016));
        expect_mode(app, false, 2);
        assert_eq!(
            at(app, recoil[0]),
            Transform::IDENTITY,
            "delayed recoil is retained at zero offset"
        );
        for &e in &ids[2..7] {
            app.world_mut().entity_mut(e).remove::<Visibility>();
            retarget(app, e, Transform::from_xyz(-8.0, 0.0, 0.0));
        }
        frames.push(step(app, &ids, 0.016));
        expect_mode(app, false, 2);
        frames.push(step(app, &ids, 0.25));
        expect_mode(app, true, 0);
        assert!(at(app, recoil[0]).translation.x > 0.0);
        assert!(at(app, recoil[1]).translation.x < 4.0);
        for &e in &recoil {
            app.world_mut()
                .entity_mut(e)
                .remove::<crate::combatfx::Recoil>();
        }
        for &e in &ids[2..] {
            app.world_mut().entity_mut(e).insert(Visibility::Hidden);
        }
        frames.push(step(app, &ids, 0.016));
        expect_mode(app, true, 0);
        frames.push(step(app, &ids, 0.016));
        expect_mode(app, false, 2);
        for _ in 0..90 {
            frames.push(step(app, &ids, 0.016));
        }
        expect_mode(app, false, 0);
        assert_eq!(at(app, recoil[0]), Transform::IDENTITY);
        assert_eq!(at(app, recoil[1]), Transform::from_xyz(4.0, 0.0, 0.0));
        frames
    });
}

fn benchmark_leg(reference: bool, count: usize, changed: usize, target_period: usize) -> Duration {
    let mut world = World::new();
    world.init_resource::<Time>();
    world.init_resource::<crate::prefs::Prefs>();
    world.init_resource::<GlideReport>();
    let ids: Vec<_> = (0..count)
        .map(|n| {
            let pose = Transform::from_xyz(n as f32 * 0.1, 0.0, 0.0);
            if reference {
                world.spawn((PlainMotion(pose), pose)).id()
            } else {
                world.spawn((Motion { target: pose }, pose)).id()
            }
        })
        .collect();
    let mut system: Box<dyn System<In = (), Out = ()>> = if reference {
        Box::new(IntoSystem::into_system(original::<PlainMotion>))
    } else {
        Box::new(IntoSystem::into_system(glide))
    };
    system.initialize(&mut world);
    let mut frame = |n: usize| {
        // Include writer cost, especially Motion's additional summary tick.
        if n.is_multiple_of(target_period) {
            let y = if (n / target_period).is_multiple_of(2) {
                1.0
            } else {
                -1.0
            };
            for &e in &ids[..changed] {
                if reference {
                    world.get_mut::<PlainMotion>(e).unwrap().0.translation.y = y;
                } else {
                    world.get_mut::<Motion>(e).unwrap().target.translation.y = y;
                }
            }
        }
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(1.0 / 60.0));
        system.run((), &mut world).unwrap();
        black_box(world.resource::<GlideReport>().moving);
    };
    for n in 0..64 {
        frame(n);
    }
    let start = std::time::Instant::now();
    for n in 0..6_000 {
        frame(n);
    }
    let elapsed = start.elapsed();
    // Also prove that the periodically retargeted case has not become an idle
    // benchmark. Sixteen frames leave ample distance above SETTLED.
    assert_eq!(world.resource::<GlideReport>().moving, changed);
    elapsed
}

/// Run only in release under the global Cargo lock, without another client or
/// measurement. Two table sizes, idle/one/eighth/quarter/half/all rewritten goals,
/// 6,000 frames per leg, three A/B/A cycles. Baseline has no summary-tick writer tax.
/// Repeat the same build without `Motion`'s `summary_tick` annotation for the third
/// variant; the candidate algorithm and its fallback stay unchanged.
/// This measures only these systems plus target writes, not whole-frame/GPU cost.
#[test]
#[ignore = "bounded release CPU measurement; parent owns the global Cargo lock"]
fn benchmark_glide_scan_vs_active() {
    let summary_tick = Motion::HAS_SUMMARY_TICK;
    for count in [40, 400] {
        for changed in [0, 1, count / 8, count / 4, count / 2, count] {
            for cycle in 0..3 {
                let a1 = benchmark_leg(true, count, changed, 1);
                let b = benchmark_leg(false, count, changed, 1);
                let a2 = benchmark_leg(true, count, changed, 1);
                eprintln!(
                    "glide summary_tick={summary_tick} cards={count} changed={changed} cycle={cycle} A1={a1:?} B={b:?} A2={a2:?}"
                );
            }
        }
    }
}

/// Same isolated A/B/A, but each target is held for sixteen animation frames.
/// This separates sustained motion from writing every target on every frame.
#[test]
#[ignore = "bounded release CPU measurement; parent owns the global Cargo lock"]
fn benchmark_glide_in_flight_without_continuous_target_writes() {
    let summary_tick = Motion::HAS_SUMMARY_TICK;
    for count in [40, 400] {
        for changed in [1, count] {
            for cycle in 0..3 {
                let a1 = benchmark_leg(true, count, changed, 16);
                let b = benchmark_leg(false, count, changed, 16);
                let a2 = benchmark_leg(true, count, changed, 16);
                eprintln!(
                    "glide summary_tick={summary_tick} cards={count} changed={changed} target_period=16 cycle={cycle} A1={a1:?} B={b:?} A2={a2:?}"
                );
            }
        }
    }
}

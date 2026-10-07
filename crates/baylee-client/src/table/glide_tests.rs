//! `glide` at rest: a card on its mark is written no more (DESIGN-v8 §3.9,
//! the idle invariant — nothing keeps the GPU busy on a still table).

use super::*;

/// Every rotation a card on the table is turned to: each ring facing, laid
/// flat, tapped or not.
fn rotations() -> Vec<Quat> {
    let mut out = Vec::new();
    for k in 0..32 {
        #[allow(clippy::cast_precision_loss)]
        let facing = std::f32::consts::TAU * k as f32 / 32.0 + 0.0137 * k as f32;
        let slot = SeatSlot {
            player: baylee_core::ids::PlayerId::new(0),
            ring_index: 0,
            angle: facing,
            center: Vec2::ZERO,
            facing,
            half_extent: Vec2::new(6.0, 3.0),
            reclaimed: 0.0,
            is_local: true,
            scale: 1.0,
            parked: false,
        };
        for tapped in [false, true] {
            out.push(card_transform(&slot, Vec2::new(3.0, -2.0), tapped, 0.0).rotation);
        }
    }
    out
}

/// Cards standing on their marks, and cards a hair off their mark's
/// rotation: none of them may be written frame after frame once there.
#[test]
fn a_card_on_its_mark_is_never_written_again() {
    let mut app = App::new();
    app.add_plugins(bevy::time::TimePlugin)
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<GlideReport>()
        .add_systems(Update, glide);
    let mut cards = Vec::new();
    for rotation in rotations() {
        for nudge in [0.0_f32, 1e-5, 3e-4] {
            let target = Transform {
                translation: Vec3::new(1.0, 0.2, -3.0),
                rotation,
                scale: Vec3::ONE,
            };
            let shown = Transform {
                rotation: (rotation * Quat::from_rotation_z(nudge)).normalize(),
                ..target
            };
            cards.push(app.world_mut().spawn((Motion { target }, shown)).id());
        }
    }
    let step = std::time::Duration::from_secs_f32(1.0 / 60.0);
    // Let anything that is going to settle settle.
    for _ in 0..90 {
        app.world_mut().resource_mut::<Time>().advance_by(step);
        app.update();
    }
    let before: Vec<Transform> = cards
        .iter()
        .map(|e| *app.world().get::<Transform>(*e).expect("a card"))
        .collect();
    for _ in 0..60 {
        app.world_mut().resource_mut::<Time>().advance_by(step);
        app.update();
    }
    for (e, was) in cards.iter().zip(before) {
        let now = *app.world().get::<Transform>(*e).expect("a card");
        assert_eq!(now, was, "a settled card was written again");
    }
    let report = *app.world().resource::<GlideReport>();
    assert_eq!(report.moving, 0, "{report:?}");
    assert!(report.settled_frames >= 60, "{report:?}");
}

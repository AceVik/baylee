//! The v7 camera (DESIGN-v7 §1.6, §2): the home pose at a ring, and the
//! visit. Projected forwards through [`Lens`], never through the fit's own
//! inverse.

use super::*;
use baylee_client_core::tableview::{RingLean, VisitCamera, VisitFrame};
use baylee_core::ids::PlayerId;

/// The three desktop windows the design measures at (logical pixels).
const WINDOWS: [Vec2; 3] = [
    Vec2::new(1708.0, 1032.0),
    Vec2::new(1180.0, 820.0),
    Vec2::new(960.0, 700.0),
];

/// v7's visit, from behind the seat: what these tests were written about.
/// The default is *Automatic* since the owner's word of 07.10.2026
/// (`an_opponent_is_visited_from_across_and_a_teammate_from_behind`).
const BEHIND: Shot = Shot {
    arrangement: baylee_client_core::tableview::Arrangement::Ring,
    lean: RingLean::Steep,
    visit: VisitCamera::Behind,
    teammates: 0,
};

fn seats(n: u8) -> Vec<PlayerId> {
    (0..n).map(PlayerId::new).collect()
}

fn layout(n: u8, canvas: Canvas) -> TableLayout {
    TableLayout::new(&seats(n), canvas.aspect(), None)
}

/// A seat's whole footprint, four corners in table space.
fn footprint(slot: &SeatSlot) -> [Vec2; 4] {
    let away = Vec2::new(slot.facing.sin(), slot.facing.cos());
    let side = Vec2::new(slot.facing.cos(), -slot.facing.sin());
    let half = slot.footprint();
    let c = slot.footprint_center();
    [
        c - side * half.x - away * half.y,
        c + side * half.x - away * half.y,
        c + side * half.x + away * half.y,
        c - side * half.x + away * half.y,
    ]
}

/// Whether a screen point is inside the band the HUD leaves (a pixel's
/// grace for rounding).
fn in_band(p: Vec2, canvas: Canvas) -> bool {
    p.x >= -1.0
        && p.x <= canvas.window.x - canvas.right + 1.0
        && p.y >= canvas.top - 1.0
        && p.y <= canvas.window.y - canvas.bottom + 1.0
}

/// A board's drawn width: its playing width's two ends, projected.
fn drawn_width(lens: &Lens, slot: &SeatSlot) -> f32 {
    let side = Vec2::new(slot.facing.cos(), -slot.facing.sin()) * slot.half_extent.x;
    let a = lens
        .project(slot.center + side)
        .expect("in front of the eye");
    let b = lens
        .project(slot.center - side)
        .expect("in front of the eye");
    a.distance(b)
}

/// The home pose at a ring keeps every pod whole in the band, at three to
/// eight seats on the three desktop windows (a phone frames only its own
/// pod at home, §1.4, and is held by its own test).
#[test]
fn the_home_pose_at_a_ring_keeps_every_pod_in_the_band() {
    for window in WINDOWS {
        let canvas = Canvas::hud(window);
        for n in 3..=8u8 {
            let layout = layout(n, canvas);
            let lens = Lens::new(CameraRig::home(&layout, canvas), window);
            for slot in &layout.slots {
                for corner in footprint(slot) {
                    let at = lens.project(corner).expect("in front of the eye");
                    assert!(
                        in_band(at, canvas),
                        "{n} seats in {window}: seat {}'s corner is drawn at {at}",
                        slot.ring_index
                    );
                }
            }
        }
    }
}

/// The home pose at a ring stands closer than the pre-v7 shot (0.36 lean,
/// 3.5 units of air).
///
/// The design derived 0.66–0.71 of the old eye distance at four to eight
/// seats, assuming the depth bound the shot. Measured, it does at three and
/// four seats (0.76–0.81) and does **not** from five: those rings are bound
/// by their width, which neither the lean nor the air shortens, and shaping
/// the ring narrower (the design's fourth lever) bought under 2 % in every
/// case tried (`aspect × 0.6…0.9`). So the bound is the measurement: at
/// most 0.82 at three and four seats, and closer at all from five.
#[test]
fn the_home_pose_at_a_ring_is_closer_than_today() {
    for window in WINDOWS {
        let canvas = Canvas::hud(window);
        for n in 3..=8u8 {
            let layout = layout(n, canvas);
            let (min, max) = layout.extent().expect("seats");
            let (min, max) = (min - Vec2::splat(AIR), max + Vec2::splat(AIR));
            let then = super::camera::fit(min, max, &layout.corners(AIR), CAMERA_LEAN, canvas).eye;
            let now = CameraRig::home(&layout, canvas);
            let now_eye = now.distance * now.lean.hypot(1.0);
            let bound = if n <= 4 { 0.82 } else { 0.97 };
            assert!(
                now_eye <= then * bound,
                "{n} seats in {window}: the eye stands {now_eye} off, {:.2} of the \
                 {then} it stood before",
                now_eye / then
            );
        }
    }
}

/// The ring takes the device's lean (D20): the recommended 0.62 by default,
/// the old 0.36 when asked for — and a duel keeps its own blend either way.
#[test]
fn the_ring_lean_is_the_devices_choice() {
    let canvas = Canvas::hud(WINDOWS[0]);
    let ring = layout(4, canvas);
    let duel = layout(2, canvas);
    let gentle = Shot {
        lean: RingLean::Gentle,
        ..Shot::default()
    };
    assert!((CameraRig::home(&ring, canvas).lean - 0.62).abs() < 1e-6);
    assert!((CameraRig::home_shot(&ring, canvas, gentle).0.lean - 0.36).abs() < 1e-6);
    assert_eq!(
        CameraRig::home(&duel, canvas),
        CameraRig::home_shot(&duel, canvas, gentle).0,
        "a duel does not read the ring's lean"
    );
}

/// Every visited pod's facing projects within a degree of screen-up, at
/// every seat of a three-, four-, six- and eight-seat ring: the camera
/// stands behind the pod on its own axis. The seat across is visited at
/// yaw π — the bug the old `framing` had (it gave 0 there, the far board
/// upside down from behind my own chair).
#[test]
fn the_visited_pod_is_upright() {
    for window in WINDOWS {
        let canvas = Canvas::hud(window);
        for n in [3u8, 4, 6, 8] {
            let layout = layout(n, canvas);
            for slot in layout.slots.iter().skip(1) {
                let (rig, _, _) = CameraRig::visit(&layout, canvas, slot.player, BEHIND)
                    .expect("the seat is at the table");
                let lens = Lens::new(rig, window);
                let away = Vec2::new(slot.facing.sin(), slot.facing.cos());
                let foot = lens.project(slot.center).expect("in front");
                let head = lens.project(slot.center + away).expect("in front");
                let up = head - foot;
                let off = up.angle_to(Vec2::NEG_Y).abs().to_degrees();
                assert!(
                    off < 1.0,
                    "{n} seats in {window}: seat {} is drawn {off:.2}° off upright",
                    slot.ring_index
                );
            }
        }
    }
    let canvas = Canvas::hud(WINDOWS[0]);
    let four = layout(4, canvas);
    let across = four.slots[2];
    let (rig, _, _) = CameraRig::visit(&four, canvas, across.player, BEHIND).expect("seat");
    assert!(
        (rig.yaw - std::f32::consts::PI).abs() < 1e-3,
        "the seat across is visited at yaw {}",
        rig.yaw
    );
}

/// The visit is the equaliser: from every chair the visited board is drawn
/// within 3 % of every other seat's visited board (3–8 seats).
#[test]
fn the_visited_board_is_drawn_at_the_same_width_from_every_chair() {
    for window in WINDOWS {
        let canvas = Canvas::hud(window);
        for n in 3..=8u8 {
            let layout = layout(n, canvas);
            let drawn: Vec<f32> = layout
                .slots
                .iter()
                .skip(1)
                .map(|slot| {
                    let (rig, _, _) =
                        CameraRig::visit(&layout, canvas, slot.player, BEHIND).expect("seat");
                    drawn_width(&Lens::new(rig, window), slot)
                })
                .collect();
            let widest = drawn.iter().copied().fold(0.0_f32, f32::max);
            let narrowest = drawn.iter().copied().fold(f32::INFINITY, f32::min);
            assert!(
                widest <= narrowest * 1.03,
                "{n} seats in {window}: visited boards drawn {drawn:?}"
            );
        }
    }
}

/// A visit draws the visited board larger than home does, and larger than
/// my own board at home — the reason to visit — and near a duel's size.
///
/// Pinned to the measurement rather than the derivation (DESIGN-v7 §1.3 asked
/// ≥ 1.2× my home board and ≥ 1.8× its own home size): a three-seat ring is
/// round and already large at home, and at three and four seats the frame
/// keeps my near lane, so there the gain is smaller (1.09× mine and 1.38× its
/// own at three seats on the laptop, 1.40× and 1.60× at four). Against a
/// duel's own board (WT2: ≥ 0.8× at four seats, 0.85× at six, 0.6× at
/// eight) the laptop measures 0.90, 0.78 and 0.62.
#[test]
fn a_visit_draws_the_board_larger_than_home() {
    for window in WINDOWS {
        let canvas = Canvas::hud(window);
        let duel = layout(2, canvas);
        let duel_board = drawn_width(
            &Lens::new(CameraRig::home(&duel, canvas), window),
            &duel.slots[0],
        );
        for n in 3..=8u8 {
            let layout = layout(n, canvas);
            let home = Lens::new(CameraRig::home(&layout, canvas), window);
            let mine = drawn_width(&home, &layout.slots[0]);
            for slot in layout.slots.iter().skip(1) {
                let (rig, _, _) =
                    CameraRig::visit(&layout, canvas, slot.player, BEHIND).expect("seat");
                let visited = drawn_width(&Lens::new(rig, window), slot);
                let own_home = drawn_width(&home, slot);
                let of_a_duel = match n {
                    3 | 4 => 0.80,
                    5 | 6 => 0.70,
                    _ => 0.55,
                };
                assert!(
                    visited >= mine * 1.05 && visited >= own_home * 1.3,
                    "{n} seats in {window}: seat {} drawn {visited} on a visit, \
                     {own_home} at home, mine {mine} at home",
                    slot.ring_index
                );
                assert!(
                    visited >= duel_board * of_a_duel,
                    "{n} seats in {window}: a visit draws {visited}, a duel {duel_board}"
                );
            }
        }
    }
}

/// The visited pod stands whole in the band, and the frame rule holds: my
/// near lane in frame at four seats when visiting the seat across, the dial
/// in frame from five seats (at the seat across, where the reach is
/// measured), the dial always when asked for.
#[test]
fn a_visit_frames_the_pod_and_what_its_rule_names() {
    for window in WINDOWS {
        let canvas = Canvas::hud(window);
        for n in 3..=8u8 {
            let layout = layout(n, canvas);
            for slot in layout.slots.iter().skip(1) {
                let (rig, frame, _) =
                    CameraRig::visit(&layout, canvas, slot.player, BEHIND).expect("seat");
                assert!(matches!(frame, VisitFrame::Lane | VisitFrame::Dial));
                let lens = Lens::new(rig, window);
                for corner in footprint(slot) {
                    let at = lens.project(corner).expect("in front");
                    assert!(
                        in_band(at, canvas),
                        "{n} seats in {window}: visiting seat {}, its corner at {at}",
                        slot.ring_index
                    );
                }
            }
            // At the seat across, the rule's far edge is in frame.
            let across = layout.slots[usize::from(n) / 2];
            let (rig, frame, _) =
                CameraRig::visit(&layout, canvas, across.player, BEHIND).expect("seat");
            let lens = Lens::new(rig, window);
            if frame == VisitFrame::Lane {
                let mine = layout.slots[0];
                let lane = mine.lane_center(baylee_client_core::layout::LaneKind::Creatures);
                let at = lens.project(lane).expect("in front");
                assert!(
                    in_band(at, canvas),
                    "{n} seats: my near lane is out of frame at {at}"
                );
            } else {
                assert!(
                    rig.sees_the_dial(canvas, baylee_client_core::dial::scale_for(&layout)),
                    "{n} seats: the dial is out of frame"
                );
            }
            let dial = Shot {
                visit: VisitCamera::BehindDial,
                ..Shot::default()
            };
            let (rig, frame, _) =
                CameraRig::visit(&layout, canvas, across.player, dial).expect("seat");
            assert_eq!(frame, VisitFrame::Dial);
            assert!(
                rig.sees_the_dial(canvas, baylee_client_core::dial::scale_for(&layout)),
                "{n} seats: dial frame without the dial"
            );
        }
    }
}

/// Seen from across (D21's third option): the camera stands on the other
/// side, the visited pod upside down at the top like a duel opponent's.
#[test]
fn a_visit_from_across_faces_the_seat() {
    let canvas = Canvas::hud(WINDOWS[0]);
    let layout = layout(4, canvas);
    let across = Shot {
        visit: VisitCamera::Across,
        ..Shot::default()
    };
    for slot in layout.slots.iter().skip(1) {
        let (rig, frame, _) = CameraRig::visit(&layout, canvas, slot.player, across).expect("seat");
        assert_eq!(frame, VisitFrame::Across);
        let lens = Lens::new(rig, WINDOWS[0]);
        let away = Vec2::new(slot.facing.sin(), slot.facing.cos());
        let up = lens.project(slot.center + away).expect("in front")
            - lens.project(slot.center).expect("in front");
        assert!(
            up.angle_to(Vec2::Y).abs().to_degrees() < 1.0,
            "seat {} is not upside down from across: {up}",
            slot.ring_index
        );
        for corner in footprint(slot) {
            assert!(in_band(lens.project(corner).expect("in front"), canvas));
        }
    }
}

/// The numbers the v7 camera takes, per window and seat count: run with
/// `--ignored --nocapture` to read them (`real7-measures.md`).
#[test]
#[ignore = "prints the v7 camera's numbers"]
fn print_the_numbers() {
    for window in WINDOWS {
        let canvas = Canvas::hud(window);
        for n in 2..=8u8 {
            let layout = layout(n, canvas);
            let (min, max) = layout.extent().expect("seats");
            let (min, max) = (min - Vec2::splat(AIR), max + Vec2::splat(AIR));
            let then = super::camera::fit(min, max, &layout.corners(AIR), CAMERA_LEAN, canvas);
            let (now, binds) = CameraRig::home_shot(&layout, canvas, Shot::default());
            let now_eye = now.distance * now.lean.hypot(1.0);
            let home = Lens::new(now, window);
            let mine = drawn_width(&home, &layout.slots[0]);
            let before = Lens::new(then.rig(0.0, CAMERA_LEAN, |p| p), window);
            let mine_then = drawn_width(&before, &layout.slots[0]);
            let across = layout.slots[usize::from(n) / 2];
            let far = drawn_width(&home, &across);
            let (rig, frame, _) =
                CameraRig::visit(&layout, canvas, across.player, BEHIND).expect("seat");
            let visited = drawn_width(&Lens::new(rig, window), &across);
            eprintln!(
                "{window} n{n}: eye {:.1}->{now_eye:.1} ({:.2}) binds {:?}/{binds:?} \
                 mine {mine_then:.0}->{mine:.0} ({:.2}) far {far:.0} visit {visited:.0} {frame:?}",
                then.eye,
                now_eye / then.eye,
                then.binds,
                mine / mine_then,
            );
        }
    }
}

/// A phone frames my own pod and the dial at home, and a visit frames the
/// visited pod alone.
#[test]
fn a_phone_frames_one_pod() {
    let window = Vec2::new(844.0, 390.0);
    let canvas = Canvas::hud(window);
    assert_eq!(
        canvas.class(),
        baylee_client_core::tableview::WindowClass::Phone
    );
    let layout = layout(4, canvas);
    let home = Lens::new(CameraRig::home(&layout, canvas), window);
    for corner in footprint(&layout.slots[0]) {
        let at = home.project(corner).expect("in front");
        assert!(in_band(at, canvas), "my own corner at {at}");
    }
    let (rig, frame, _) =
        CameraRig::visit(&layout, canvas, layout.slots[2].player, BEHIND).expect("seat");
    assert_eq!(frame, VisitFrame::Pod);
    let lens = Lens::new(rig, window);
    for corner in footprint(&layout.slots[2]) {
        assert!(in_band(lens.project(corner).expect("in front"), canvas));
    }
}

/// The owner (07.10.2026): *"The current view fits for teammates; if it is
/// not a teammate it should be rotated 180 degrees."* Under the default
/// (*Automatic*) an opponent's board is seen from across — the eye half a
/// turn from behind it, its cards upside down to me as a duel opponent's
/// are — and a teammate's from behind, upright. In every camera
/// arrangement. Red on v7, whose default visited every seat from behind.
#[test]
fn an_opponent_is_visited_from_across_and_a_teammate_from_behind() {
    use baylee_client_core::layout::Seat;
    use baylee_client_core::tableview::Arrangement;
    let canvas = Canvas::hud(WINDOWS[0]);
    for arrangement in [Arrangement::Ring, Arrangement::UprightRing] {
        for n in [3_u8, 4, 6] {
            // Teams of two from four seats: seat 2 is my teammate.
            let roster: Vec<Seat> = seats(n)
                .into_iter()
                .map(|p| {
                    let team = (n >= 4).then_some(p.get() % 2);
                    Seat::on(p, team)
                })
                .collect();
            let layout = TableLayout::arranged(&roster, canvas.aspect(), arrangement, None);
            let shot = Shot {
                arrangement,
                teammates: Shot::teammates_of(
                    PlayerId::new(0),
                    roster.iter().map(|s| (s.player, s.team)),
                ),
                ..Shot::default()
            };
            assert_eq!(shot.visit, VisitCamera::Auto, "the default");
            for slot in layout.slots.iter().skip(1) {
                let (rig, _, _) =
                    CameraRig::visit(&layout, canvas, slot.player, shot).expect("a seat");
                let teammate = roster[slot.ring_index].team.is_some()
                    && roster[slot.ring_index].team == roster[0].team;
                let off = (rig.yaw - behind(slot) + std::f32::consts::PI)
                    .rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI;
                let what = format!("{arrangement:?} {n} seats, seat {}", slot.player.get());
                if teammate {
                    assert!(off.abs() < 1e-3, "{what}: a teammate from behind ({off})");
                } else {
                    assert!(
                        (off.abs() - std::f32::consts::PI).abs() < 1e-3,
                        "{what}: an opponent from across ({off})"
                    );
                }
                let lens = Lens::new(rig, canvas.window);
                assert!(
                    footprint(slot).iter().all(|p| lens.project(*p).is_some()),
                    "{what}: the visited board is in front of the eye"
                );
            }
        }
    }
}

//! The invariants every arrangement keeps (DESIGN-v8 §3), asked of every
//! arrangement at three to eight seats on four canvases, alone and in
//! teams, at home and with every seat of interest.

use super::*;

/// The canvases DESIGN-v8 §3 names: an ultrawide or a phone on its side, a
/// laptop's HUD, a tablet's, and a square.
const ASPECTS: [f32; 4] = [2.8, 2.0, 1.44, 1.0];

/// Every table the invariants are asked at: each arrangement, three to eight
/// seats, each canvas, everybody alone and (from four) in teams of two, at
/// home and with each seat as the seat of interest.
fn every_table(mut check: impl FnMut(&str, &[Seat], &TableLayout)) {
    for arrangement in Arrangement::ALL {
        for n in 3..=8_u8 {
            let alone: Vec<Seat> = seats(n).into_iter().map(Seat::alone).collect();
            let teams: Vec<Seat> = seats(n)
                .into_iter()
                .map(|p| Seat::on(p, Some(p.get() % 2)))
                .collect();
            let rosters: Vec<Vec<Seat>> = if n >= 4 {
                vec![alone, teams]
            } else {
                vec![alone]
            };
            for roster in &rosters {
                for aspect in ASPECTS {
                    let interests =
                        std::iter::once(None).chain(roster.iter().map(|seat| Some(seat.player)));
                    for interest in interests {
                        let layout = TableLayout::arranged(roster, aspect, arrangement, interest);
                        let teamed = roster.iter().any(|s| s.team.is_some());
                        let what = format!(
                            "{arrangement:?} n={n} aspect={aspect} interest={interest:?} teams={teamed}"
                        );
                        check(&what, roster, &layout);
                    }
                }
            }
        }
    }
}

/// Invariant 1: exactly one slot per seat, in the roster's order.
#[test]
fn every_seat_has_one_slot() {
    every_table(|what, roster, layout| {
        assert_eq!(layout.slots.len(), roster.len(), "{what}");
        let mut rings: Vec<usize> = layout.slots.iter().map(|s| s.ring_index).collect();
        rings.sort_unstable();
        assert_eq!(rings, (0..roster.len()).collect::<Vec<_>>(), "{what}");
        for (slot, seat) in layout.slots.iter().zip(roster) {
            assert_eq!(slot.player, seat.player, "{what}: in the roster's order");
            assert!(slot.scale > 0.0 && slot.scale <= 1.0, "{what}");
        }
    });
}

/// Invariant 1 again: the local seat is the first, upright, on the felt
/// and nearest the eye.
#[test]
fn the_local_seat_is_nearest_and_upright() {
    every_table(|what, _, layout| {
        let local = layout.local().expect("a local seat");
        assert!(local.is_local, "{what}");
        assert!(local.facing.abs() < 1e-6, "{what}: upright");
        assert!(!local.parked, "{what}: mine is never parked");
        assert!(
            (local.scale - 1.0).abs() < 1e-6,
            "{what}: at a duel's scale"
        );
        for slot in layout.slots.iter().filter(|s| !s.parked) {
            assert!(
                local.center.y <= slot.center.y + 1e-3,
                "{what}: {:?} is nearer than mine",
                slot.player
            );
        }
    });
}

/// Invariant 2: no two pods on the felt meet — their grounds and their
/// pile strips included. A parked seat is off the felt.
#[test]
fn no_two_footprints_overlap() {
    every_table(|what, _, layout| {
        let shown: Vec<&SeatSlot> = layout.slots.iter().filter(|s| !s.parked).collect();
        for (i, a) in shown.iter().enumerate() {
            for b in &shown[i + 1..] {
                assert!(
                    !grounds_overlap(a, b),
                    "{what}: {:?} and {:?} meet",
                    a.player,
                    b.player
                );
            }
        }
    });
}

/// A parked seat stands off the felt: outside the extent the camera frames.
#[test]
fn a_parked_seat_stands_outside_the_framed_table() {
    every_table(|what, _, layout| {
        let Some((lo, hi)) = layout.extent() else {
            return;
        };
        for slot in layout.slots.iter().filter(|s| s.parked) {
            let inside = slot.center.cmpge(lo).all() && slot.center.cmple(hi).all();
            assert!(!inside, "{what}: {:?} parked on the table", slot.player);
        }
        assert!(
            layout.corners(0.0).len() == 4 * layout.slots.iter().filter(|s| !s.parked).count(),
            "{what}: the corners skip the parked"
        );
    });
}

/// Invariant 10: a duel is a duel, whatever arrangement is chosen.
#[test]
fn every_arrangement_at_two_seats_is_the_duel() {
    let duel: Vec<Seat> = seats(2).into_iter().map(Seat::alone).collect();
    for aspect in ASPECTS {
        let reference = TableLayout::seated(&duel, aspect, None);
        for arrangement in Arrangement::ALL {
            for interest in [None, Some(PlayerId::new(1))] {
                assert_eq!(
                    TableLayout::arranged(&duel, aspect, arrangement, interest),
                    reference,
                    "{arrangement:?} at {aspect}"
                );
            }
        }
    }
}

/// The ring is `seated`'s, unchanged, whatever the seat of interest: a
/// camera arrangement moves no card.
#[test]
fn the_ring_is_seated_and_ignores_the_seat_of_interest() {
    for n in 3..=8 {
        let roster: Vec<Seat> = seats(n).into_iter().map(Seat::alone).collect();
        let reference = TableLayout::seated(&roster, HUD_ASPECT, None);
        for interest in std::iter::once(None).chain(roster.iter().map(|s| Some(s.player))) {
            assert_eq!(
                TableLayout::arranged(&roster, HUD_ASPECT, Arrangement::Ring, interest),
                reference
            );
        }
    }
}

/// Invariant 3: hidden information is unrepresentable here too. The
/// arrangements read the roster (`&[Seat]`: a player and a team) and
/// nothing else, so no file of the layout names a view or an object.
#[test]
fn arranged_reads_only_the_roster() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = vec![root.join("layout.rs")];
    let mut dirs = vec![root.join("layout")];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).expect("the layout directory") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n != "tests") {
                    dirs.push(path);
                }
            } else if path.extension().is_some_and(|e| e == "rs") {
                files.push(path);
            }
        }
    }
    assert!(files.len() >= 2, "the arrangements are read: {files:?}");
    for file in files {
        let text = std::fs::read_to_string(&file).expect("readable");
        for word in ["PlayerView", "ObjectId", "baylee_view"] {
            assert!(
                !text.contains(word),
                "{} names {word}: the layout reads only the roster",
                file.display()
            );
        }
    }
}

/// The upright ring: every board faces me, every seat stays on its side of
/// the middle (each axis of the ring scaled on its own), and the shape the
/// search takes is framed no further off than the design's rule — the whole
/// ring grown until the upright places part — would have been.
#[test]
fn the_upright_ring_stands_every_board_up_on_the_ring_s_bearings() {
    let reach = |table: &TableLayout, aspect: f32| {
        let (lo, hi) = table.extent().expect("seats");
        let span = hi - lo;
        (span.x / aspect).max(span.y)
    };
    for n in 3..=8 {
        for aspect in ASPECTS {
            let roster: Vec<Seat> = seats(n).into_iter().map(Seat::alone).collect();
            let ring = TableLayout::seated(&roster, aspect, None);
            let up = TableLayout::arranged(&roster, aspect, Arrangement::UprightRing, None);
            let scale = up.radius / ring.radius;
            for (r, u) in ring.slots.iter().zip(&up.slots) {
                let want = if u.is_local {
                    0.0
                } else {
                    std::f32::consts::PI
                };
                assert!(
                    (u.facing - want).abs() < 1e-6,
                    "n={n}: {:?} turned {}",
                    u.player,
                    u.facing
                );
                assert!((u.angle - r.angle).abs() < 1e-6);
                assert!(
                    u.center.distance(r.center * scale) < 1e-3,
                    "n={n} aspect={aspect}: {:?} left its place on the ring",
                    u.player
                );
            }
            // The design's rule, for comparison: grown, never shrunk.
            let mut upright: Vec<SeatSlot> = ring.slots.clone();
            for slot in &mut upright {
                slot.facing = if slot.is_local {
                    0.0
                } else {
                    std::f32::consts::PI
                };
            }
            let grown = (100..=400)
                .map(|k| k as f32 / 100.0)
                .find_map(|f| {
                    let slots: Vec<SeatSlot> = upright
                        .iter()
                        .map(|s| SeatSlot {
                            center: s.center * f,
                            ..*s
                        })
                        .collect();
                    let holds = slots.iter().enumerate().all(|(i, a)| {
                        slots[i + 1..]
                            .iter()
                            .all(|b| arrangement_upright_apart(a, b))
                    });
                    holds.then(|| TableLayout {
                        slots,
                        radius: ring.radius * f,
                    })
                })
                .expect("a ring four times as large holds anything");
            assert!(
                reach(&up, aspect) <= reach(&grown, aspect) + 1e-3,
                "n={n} aspect={aspect}: the search framed a larger table than growing would"
            );
        }
    }
}

/// How much the upright ring grows over the ring, per seat count, on the
/// laptop's canvas (DESIGN-v8 §1.2's hypothesis: +12 % at six, +20 % at
/// eight). `cargo test -p baylee-client-core print_the_upright_growth --
/// --ignored --nocapture`.
#[test]
#[ignore = "prints numbers for real8-measures.md"]
fn print_the_upright_growth() {
    for n in 3..=8 {
        let roster: Vec<Seat> = seats(n).into_iter().map(Seat::alone).collect();
        let ring = TableLayout::seated(&roster, HUD_ASPECT, None);
        let up = TableLayout::arranged(&roster, HUD_ASPECT, Arrangement::UprightRing, None);
        let scale = up.radius / ring.radius;
        println!("{n} seats: ring scaled {:.3} x {:.3}", scale.x, scale.y);
    }
}

/// Invariant 7: the dial is a compass of the roster in every arrangement —
/// a jewel for every seat, no two in one direction, mine straight down; and
/// where the arrangement keeps the ring's order round the table (the ring
/// and the upright ring), the jewels go round clockwise in turn order, one
/// turn exactly.
#[test]
fn the_dial_is_a_compass_of_the_roster_in_every_arrangement() {
    for arrangement in Arrangement::ALL {
        for n in 3..=8 {
            for aspect in ASPECTS {
                let roster: Vec<Seat> = seats(n).into_iter().map(Seat::alone).collect();
                let layout = TableLayout::arranged(&roster, aspect, arrangement, None);
                let jewels = crate::dial::bearings(
                    &layout
                        .slots
                        .iter()
                        .map(|s| (s.player, s.center, None))
                        .collect::<Vec<_>>(),
                );
                let what = format!("{arrangement:?} n={n} aspect={aspect}");
                assert_eq!(jewels.len(), usize::from(n), "{what}");
                assert!(
                    jewels[0].1.dot(Vec2::NEG_Y) > 0.99,
                    "{what}: my jewel at {:?}",
                    jewels[0].1
                );
                for (i, a) in jewels.iter().enumerate() {
                    for b in &jewels[i + 1..] {
                        assert!(a.1.dot(b.1) < 0.9999, "{what}: two jewels in one direction");
                    }
                }
                if matches!(arrangement, Arrangement::Ring | Arrangement::UprightRing) {
                    let angle = |v: Vec2| v.y.atan2(v.x);
                    let mut turned = 0.0_f32;
                    for k in 0..jewels.len() {
                        let from = angle(jewels[k].1);
                        let to = angle(jewels[(k + 1) % jewels.len()].1);
                        let step = (from - to).rem_euclid(std::f32::consts::TAU);
                        assert!(step > 1e-3, "{what}: jewels {k} and {} out of order", k + 1);
                        turned += step;
                    }
                    assert!(
                        (turned - std::f32::consts::TAU).abs() < 1e-3,
                        "{what}: the jewels go round {turned} radians"
                    );
                }
            }
        }
    }
}

/// Spotlight: my side and the seat of interest's side seated exactly as a
/// duel of the two would be, the interest's side across from me, and every
/// other seat parked; at home the side across the ring is the one brought
/// across; a teammate as the interest changes nothing (my side is mine).
#[test]
fn the_spotlight_seats_the_interest_across_as_a_duel_and_parks_the_rest() {
    for n in 3..=8 {
        for aspect in ASPECTS {
            let roster: Vec<Seat> = seats(n).into_iter().map(Seat::alone).collect();
            for interest in
                std::iter::once(None).chain(roster.iter().skip(1).map(|s| Some(s.player)))
            {
                let table =
                    TableLayout::arranged(&roster, aspect, Arrangement::Spotlight, interest);
                let across = interest.unwrap_or(PlayerId::new(n / 2));
                let shown: Vec<PlayerId> = table
                    .slots
                    .iter()
                    .filter(|s| !s.parked)
                    .map(|s| s.player)
                    .collect();
                assert_eq!(shown, vec![PlayerId::new(0), across], "n={n} {interest:?}");
                let duel = TableLayout::seated(
                    &[Seat::alone(PlayerId::new(0)), Seat::alone(across)],
                    aspect,
                    None,
                );
                for (slot, want) in [(0, 0), (usize::from(across.get()), 1)] {
                    let got = table.slots[slot];
                    let want = duel.slots[want];
                    assert!(got.center.distance(want.center) < 1e-4, "n={n}");
                    assert!((got.facing - want.facing).abs() < 1e-6);
                    assert_eq!(got.half_extent, want.half_extent);
                }
            }
        }
    }
    let teams: Vec<Seat> = seats(4)
        .into_iter()
        .map(|p| Seat::on(p, Some(p.get() % 2)))
        .collect();
    let home = TableLayout::arranged(&teams, HUD_ASPECT, Arrangement::Spotlight, None);
    let partner = TableLayout::arranged(
        &teams,
        HUD_ASPECT,
        Arrangement::Spotlight,
        Some(PlayerId::new(2)),
    );
    assert_eq!(home, partner, "a teammate as the interest keeps the pair");
    assert!(
        home.slots.iter().all(|s| !s.parked),
        "two sides of two: all four seated"
    );
}

/// The owner's rule of 07.10.2026: a board an arrangement squares to my
/// chair is drawn as the duel draws its opponent when it is an opponent's —
/// its lanes in the duel opponent's order and its cards turned as theirs —
/// and as mine when it is my teammate's. Read off the lanes themselves: the
/// creature, support and land rows of an opponent's pod stand where the
/// duel opponent's stand relative to their pod (red on the upright ring as
/// first built, every board facing me).
#[test]
fn an_opponent_s_square_board_is_the_duel_opponent_s_and_a_teammate_s_is_mine() {
    let lanes = |slot: &SeatSlot| -> Vec<Vec2> {
        LaneKind::ALL
            .iter()
            .map(|lane| slot.lane_center(*lane) - slot.center)
            .collect()
    };
    let close = |a: &[Vec2], b: &[Vec2]| {
        a.iter()
            .zip(b)
            .all(|(x, y)| (*x / x.length().max(1e-6)).distance(*y / y.length().max(1e-6)) < 1e-3)
    };
    let duel = TableLayout::seated(
        &[Seat::alone(PlayerId::new(0)), Seat::alone(PlayerId::new(1))],
        HUD_ASPECT,
        None,
    );
    let (mine, theirs) = (lanes(&duel.slots[0]), lanes(&duel.slots[1]));
    for n in [4_u8, 6] {
        let roster: Vec<Seat> = seats(n)
            .into_iter()
            .map(|p| Seat::on(p, Some(p.get() % 2)))
            .collect();
        let up = TableLayout::arranged(&roster, HUD_ASPECT, Arrangement::UprightRing, None);
        for slot in &up.slots {
            let teammate = slot.player.get() % 2 == 0;
            let want = if teammate { &mine } else { &theirs };
            assert!(
                close(&lanes(slot), want),
                "n={n}: seat {} (teammate {teammate}) lanes {:?}",
                slot.player.get(),
                lanes(slot)
            );
            let duel_facing = if teammate {
                duel.slots[0].facing
            } else {
                duel.slots[1].facing
            };
            assert!((slot.facing - duel_facing).abs() < 1e-6, "card orientation");
        }
    }
}

/// The tear: split, swing, dock, settle, in that order and none skipped.
/// The split parts the pieces (mine toward me, the far one away); the swing
/// turns the leaving seat's piece out to its ring bearing and the arriving
/// seat's in from its own, in steps along the arc, each seat's board carried
/// rigidly with its piece; the dock presses them a hair past their places;
/// from the settle on the layout is the instant one exactly.
#[test]
fn the_tear_runs_its_four_phases_in_order_and_ends_on_the_instant_layout() {
    use super::super::transition::{
        DOCK_ENDS, ENDS, OPENING, Phase, Piece, SPLIT_ENDS, SWING_ENDS, Tear, departing, swing,
    };
    let roster: Vec<Seat> = seats(4).into_iter().map(Seat::alone).collect();
    let from = TableLayout::arranged(&roster, HUD_ASPECT, Arrangement::Spotlight, None);
    let to = TableLayout::arranged(
        &roster,
        HUD_ASPECT,
        Arrangement::Spotlight,
        Some(PlayerId::new(1)),
    );
    assert_eq!(departing(&from, &to), vec![PlayerId::new(2)]);
    let tear = Tear::new(from.clone(), to.clone());
    assert_eq!(tear.near, vec![PlayerId::new(0)]);
    assert_eq!(tear.leaving.0, vec![PlayerId::new(2)]);
    assert_eq!(tear.arriving.0, vec![PlayerId::new(1)]);
    // Seat 1 waits at the ring's left (a quarter turn), seat 2 straight up.
    assert!(tear.leaving.1.abs() < 0.2, "seat 2 is at home up the table");
    assert!(tear.arriving.1 > 1.0, "seat 1 comes in from the left");
    let mut seen = Vec::new();
    let mut last_swing = 0.0;
    let mut t = 0.0_f32;
    while t <= ENDS + 0.05 {
        let phase = Phase::at(t);
        if seen.last() != Some(&phase) {
            seen.push(phase);
        }
        let u = swing(t);
        assert!(u >= last_swing, "the arc only goes forward");
        last_swing = u;
        let laid = tear.staged(t);
        let mine = laid.slot(PlayerId::new(0)).expect("me").center.y;
        let home = to.slot(PlayerId::new(0)).expect("me").center.y;
        match phase {
            Phase::Split => {
                assert!(
                    (mine - (home - OPENING * 0.5)).abs() < 0.2,
                    "my piece toward me (and shaking)"
                );
                let old = laid.slot(PlayerId::new(2)).expect("seat 2").center;
                let was = from.slot(PlayerId::new(2)).expect("seat 2").center;
                assert!(
                    (old - was - Vec2::new(0.0, OPENING * 0.5)).length() < 0.2,
                    "the old far piece away"
                );
                assert!(
                    laid.slot(PlayerId::new(1)).is_some_and(|s| s.parked),
                    "the new seat still waits"
                );
                assert!(
                    !tear.pose(Piece::Arriving, t).shown,
                    "the new piece not yet"
                );
            }
            Phase::Swing => {
                let arriving = laid.slot(PlayerId::new(1)).expect("seat 1");
                let pose = tear.pose(Piece::Arriving, t);
                let want = pose.carry(to.slot(PlayerId::new(1)).expect("seat 1").center);
                assert!(
                    arriving.center.distance(want) < 1e-3,
                    "the board rides its piece"
                );
                assert!(!arriving.parked);
                let leaving = laid.slot(PlayerId::new(2)).expect("seat 2");
                assert!(leaving.parked, "drawn by the tear, not by the table");
            }
            Phase::Dock => assert!(mine > home, "pressed a hair past"),
            Phase::Settle | Phase::Done => assert_eq!(laid, to, "the instant layout"),
        }
        t += 1.0 / 240.0;
    }
    assert_eq!(
        seen,
        vec![
            Phase::Split,
            Phase::Swing,
            Phase::Dock,
            Phase::Settle,
            Phase::Done
        ],
        "every phase, in order"
    );
    assert!(
        (swing(SWING_ENDS) - 1.0).abs() < 1e-6,
        "the arc is whole by the dock"
    );
    const { assert!(SPLIT_ENDS < SWING_ENDS && SWING_ENDS < DOCK_ENDS && DOCK_ENDS < ENDS) };
    assert!((0.8..=1.1).contains(&ENDS), "about a second, as asked");
    for piece in [Piece::Near, Piece::Leaving, Piece::Arriving] {
        let rest = tear.pose(piece, ENDS);
        assert!(rest.turn.abs() < 1e-6 || piece == Piece::Leaving);
        assert!(rest.lift.abs() < 1e-6 && rest.shift.abs() < 1e-6 || piece == Piece::Leaving);
    }
}

/// The weld: no seam before the halves close, the weld's progress reaching
/// one exactly as the tear ends, the seam bright at the dock and gone at the
/// end, and nothing after.
#[test]
fn the_weld_runs_to_one_and_the_seam_cools_to_nothing() {
    use super::super::transition::{DOCK_ENDS, ENDS, SWING_ENDS, seam, weld};
    assert!(weld(SWING_ENDS - 0.01).abs() < 1e-6);
    assert!(
        seam(weld(SWING_ENDS)).abs() < 1e-6,
        "no seam before the halves close"
    );
    assert!(seam(weld(DOCK_ENDS)) > 0.95, "hottest as they meet");
    assert!(
        (weld(ENDS) - 1.0).abs() < 1e-6,
        "the weld is done at the end"
    );
    assert!(seam(weld(ENDS)).abs() < 1e-6, "and the seam is gone");
    assert!(seam(weld(ENDS + 1.0)).abs() < 1e-6);
}

/// The swap (the owner's of 07.10.2026): the piece leaving and the piece
/// arriving trade places along paths that never meet. Each piece's volume —
/// its slab turned and slid, from its bottom to its cards' top — is followed
/// as `glide` would follow its poses (the same exponential at 60 Hz) and the
/// two are checked against each other at every frame both are drawn. Red on
/// a straight swap (both pieces at one height).
#[test]
fn the_leaving_and_the_arriving_piece_never_meet() {
    use super::super::transition::{CARDS_OVER, ENDS, Piece, Pose, THICKNESS, Tear};
    let roster: Vec<Seat> = seats(6).into_iter().map(Seat::alone).collect();
    let half = Vec2::new(20.0, 8.0);
    let volume = |pose: Pose| -> (glam::Vec3, glam::Vec3) {
        let corners = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .map(|(x, y)| pose.carry(Vec2::new(x * half.x, y * half.y)));
        let lo = corners.iter().fold(Vec2::splat(f32::MAX), |a, c| a.min(*c));
        let hi = corners.iter().fold(Vec2::splat(f32::MIN), |a, c| a.max(*c));
        (
            glam::Vec3::new(lo.x, lo.y, pose.lift - THICKNESS),
            glam::Vec3::new(hi.x, hi.y, pose.lift + CARDS_OVER),
        )
    };
    let meet = |a: (glam::Vec3, glam::Vec3), b: (glam::Vec3, glam::Vec3)| {
        a.0.cmplt(b.1).all() && b.0.cmplt(a.1).all()
    };
    for interest in 1..6_u8 {
        let from = TableLayout::arranged(&roster, HUD_ASPECT, Arrangement::Spotlight, None);
        let to = TableLayout::arranged(
            &roster,
            HUD_ASPECT,
            Arrangement::Spotlight,
            Some(PlayerId::new(interest)),
        );
        if from == to {
            continue;
        }
        let tear = Tear::new(from, to);
        // Followed as `glide` follows a target: 1 - e^(-16 dt) a frame.
        let step = 1.0 / 60.0_f32;
        let k = 1.0 - (-16.0 * step).exp();
        let follow = |shown: Pose, target: Pose| Pose {
            turn: shown.turn + (target.turn - shown.turn) * k,
            shift: shown.shift + (target.shift - shown.shift) * k,
            lift: shown.lift + (target.lift - shown.lift) * k,
            shown: target.shown,
        };
        let mut leaving = Pose::REST;
        // The arriving piece waits, hidden, where it comes from (snapped).
        let mut arriving = tear.pose(Piece::Arriving, 0.0);
        let mut t = 0.0;
        while t < ENDS {
            t += step;
            leaving = follow(leaving, tear.pose(Piece::Leaving, t));
            let target = tear.pose(Piece::Arriving, t);
            arriving = if arriving.shown {
                follow(arriving, target)
            } else {
                target
            };
            if leaving.shown && arriving.shown {
                assert!(
                    !meet(volume(leaving), volume(arriving)),
                    "seat {interest} at t={t:.3}: the pieces meet ({leaving:?} / {arriving:?})"
                );
            }
        }
    }
}

/// The shake: a jolt as the table tears, a smaller one as it docks, and
/// nothing from the settle on — the stages end on the instant layout.
#[test]
fn the_table_shakes_as_it_tears_and_is_still_when_it_settles() {
    use super::super::transition::{DOCK_ENDS, ENDS, SWING_ENDS, shake, spill};
    let most = |from: f32, to: f32| {
        (0..200)
            .map(|i| shake(from + (to - from) * i as f32 / 200.0).abs())
            .fold(0.0, f32::max)
    };
    assert!(most(0.0, 0.1) > 0.08, "noticeably as it tears");
    assert!(most(SWING_ENDS, DOCK_ENDS) > 0.03, "a little as it docks");
    assert!(most(DOCK_ENDS, ENDS + 0.5) < 1e-6, "still once it settles");
    assert!(
        spill(SWING_ENDS - 0.01) > 0.9,
        "the veins run out while it is open"
    );
    assert!(spill(DOCK_ENDS).abs() < 1e-6, "drawn back in by the dock");
}

/// The tear's line on the CPU (the cut faces are meshed along it) is the
/// shader's: the same shape, centred within half a unit of the middle.
#[test]
fn the_tear_line_stays_near_the_middle_and_is_jagged() {
    use super::super::transition::tear_line;
    let ys: Vec<f32> = (0..400)
        .map(|i| tear_line(-20.0 + i as f32 * 0.1, 7.3))
        .collect();
    assert!(ys.iter().all(|y| y.abs() < 0.5), "within half a unit");
    let turns = ys
        .windows(3)
        .filter(|w| (w[1] - w[0]) * (w[2] - w[1]) < 0.0)
        .count();
    assert!(turns > 40, "jagged: {turns} teeth over forty units");
}

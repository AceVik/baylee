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
            let up = TableLayout::arranged(&roster, aspect, Arrangement::UprightRing, None);
            // Stood up from the table `seated` lays or from the ellipse,
            // whichever frames closer: the one whose bearings it keeps.
            let ring = [
                TableLayout::seated(&roster, aspect, None),
                TableLayout::on_ring(&roster, aspect, None),
            ]
            .into_iter()
            .find(|ring| {
                ring.slots
                    .iter()
                    .zip(&up.slots)
                    .all(|(r, u)| (u.angle - r.angle).abs() < 1e-6)
            })
            .expect("the upright ring keeps a ring's bearings");
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
/// a jewel for every seat, no two in one direction, mine down at my board
/// (straight down unless a frame seats me off the near edge's middle); and
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
                // Straight down, or — where a frame seats me a board off the
                // near edge's middle (six seats: two to a long edge) — down
                // towards my board, never round the side.
                assert!(
                    jewels[0].1.dot(Vec2::NEG_Y) > 0.6,
                    "{what}: my jewel at {:?}",
                    jewels[0].1
                );
                assert!(
                    jewels[0].1.dot(layout.slots[0].center.normalize()) > 0.999,
                    "{what}: my jewel points past my board"
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

use super::super::transition;

/// A tear of the Spotlight at `n` seats from home to `interest`.
fn spotlight_tear(n: u8, interest: u8) -> Option<transition::Tear> {
    let roster: Vec<Seat> = seats(n).into_iter().map(Seat::alone).collect();
    let from = TableLayout::arranged(&roster, HUD_ASPECT, Arrangement::Spotlight, None);
    let to = TableLayout::arranged(
        &roster,
        HUD_ASPECT,
        Arrangement::Spotlight,
        Some(PlayerId::new(interest)),
    );
    (from != to).then(|| transition::Tear::new(from, to))
}

/// The tear: split, swing, dock, settle, in that order and none skipped.
/// The split parts the pieces (mine toward me, the far one away); in the
/// swing the far piece sinks into the void and is gone before the new
/// seat's piece is drawn at all, which then rises in its place, each seat's
/// board carried rigidly with its piece; in the dock the two close; from the
/// settle on the layout is the instant one exactly.
#[test]
#[allow(clippy::too_many_lines)] // one tear, followed through every stage
fn the_tear_runs_its_four_phases_in_order_and_ends_on_the_instant_layout() {
    use super::super::transition::{
        CLOSING, DEEP, DOCK_ENDS, ENDS, EXIT_ENDS, OPENING, Phase, Piece, SPLIT_ENDS, SWING_ENDS,
        departing,
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
    let tear = spotlight_tear(4, 1).expect("a change");
    assert_eq!(tear.near, vec![PlayerId::new(0)]);
    assert_eq!(tear.leaving, vec![PlayerId::new(2)]);
    assert_eq!(tear.arriving, vec![PlayerId::new(1)]);
    let mut seen = Vec::new();
    let mut t = 0.0_f32;
    while t <= ENDS + 0.05 {
        let phase = Phase::at(t);
        if seen.last() != Some(&phase) {
            seen.push(phase);
        }
        let laid = tear.staged(t);
        let mine = laid.slot(PlayerId::new(0)).expect("me").center.y;
        let home = to.slot(PlayerId::new(0)).expect("me").center.y;
        let leaving = tear.pose(Piece::Leaving, t);
        let arriving = tear.pose(Piece::Arriving, t);
        assert!(
            !(leaving.shown && arriving.shown),
            "t={t}: the new piece only once the old one is gone"
        );
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
                assert!(leaving.lift.abs() < 1e-6, "level while it tears");
            }
            Phase::Swing => {
                if t < EXIT_ENDS {
                    assert!((leaving.lift + DEEP).abs() < 1e-6, "sinking into the void");
                    assert!(
                        laid.slot(PlayerId::new(1)).is_some_and(|s| s.parked),
                        "the new seat waits while the old one goes"
                    );
                } else {
                    let rising = laid.slot(PlayerId::new(1)).expect("seat 1");
                    let want = arriving.carry(to.slot(PlayerId::new(1)).expect("seat 1").center);
                    assert!(
                        rising.center.distance(want) < 1e-3,
                        "the board rides its piece"
                    );
                    assert!(!rising.parked);
                }
                let gone = laid.slot(PlayerId::new(2)).expect("seat 2");
                assert!(gone.parked, "drawn by the tear, not by the table");
            }
            Phase::Dock => {
                if t < CLOSING {
                    assert!(
                        mine < home - 1.0,
                        "still open while the new piece comes down"
                    );
                    assert!(arriving.lift.abs() < 1e-6, "level with mine");
                }
            }
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
    const { assert!(SPLIT_ENDS < EXIT_ENDS && EXIT_ENDS < SWING_ENDS) };
    const { assert!(SWING_ENDS < CLOSING && CLOSING < DOCK_ENDS && DOCK_ENDS < ENDS) };
    assert!((0.8..=1.1).contains(&ENDS), "about a second, as asked");
    for piece in [Piece::Near, Piece::Arriving] {
        let rest = tear.pose(piece, ENDS);
        assert!(rest.turn.abs() < 1e-6 && rest.lift.abs() < 1e-6 && rest.shift.abs() < 1e-6);
    }
    assert!(!tear.pose(Piece::Leaving, ENDS).shown);
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

/// A piece's ground, for the swap's geometry: the table's box (the slab is
/// cut to the extent plus a margin) on one side of the tear's line.
#[derive(Clone, Copy)]
struct Ground {
    half: Vec2,
    near: bool,
    seed: f32,
}

impl Ground {
    /// Whether a point of the piece's own ground is on it (strictly inside).
    fn holds(self, p: Vec2) -> bool {
        let line = transition::tear_line(p.x, self.seed);
        let inside = p.x.abs() < self.half.x - 1e-3 && p.y.abs() < self.half.y - 1e-3;
        inside
            && if self.near {
                p.y < line - 1e-3
            } else {
                p.y > line + 1e-3
            }
    }

    /// Points along the piece's edge: the jagged line and the box's sides.
    fn edge(self) -> Vec<Vec2> {
        let mut points = Vec::new();
        let (w, h) = (self.half.x, self.half.y);
        let mut x = -w;
        while x <= w {
            let line = transition::tear_line(x, self.seed);
            points.push(Vec2::new(x, line));
            points.push(Vec2::new(x, if self.near { -h } else { h }));
            x += 0.1;
        }
        let mut y = -h;
        while y <= h {
            let on = |x: f32| {
                let line = transition::tear_line(x, self.seed);
                if self.near { y <= line } else { y >= line }
            };
            for x in [-w, w] {
                if on(x) {
                    points.push(Vec2::new(x, y));
                }
            }
            y += 0.1;
        }
        points
    }
}

/// Whether two posed grounds cover a common point of table space, seen
/// from above: any edge point of one inside the other.
fn grounds_meet(a: (Ground, transition::Pose), b: (Ground, transition::Pose)) -> bool {
    let into = |p: (Ground, transition::Pose), q: (Ground, transition::Pose)| {
        p.0.edge()
            .into_iter()
            .any(|point| q.0.holds(q.1.uncarry(p.1.carry(point))))
    };
    into(a, b) || into(b, a)
}

/// Whether two posed pieces' volumes — each from its slab's underside to its
/// cards' top — share a height.
fn heights_meet(a: transition::Pose, b: transition::Pose) -> bool {
    use super::super::transition::{CARDS_OVER, THICKNESS};
    a.lift - THICKNESS < b.lift + CARDS_OVER && b.lift - THICKNESS < a.lift + CARDS_OVER
}

/// The swap (the owner's of 07.10.2026): *"the old one has to get out of
/// the way … the incoming one docks only into an empty slot"*. Each piece
/// is followed as `glide` follows its keyframes (the same exponential at
/// 60 Hz; a hidden piece snaps) and, at every frame:
///
/// - the leaving and the arriving piece are never both drawn over a common
///   point of the table — whatever their heights;
/// - no two drawn pieces' volumes meet, my piece included (the static
///   table is a piece too);
/// - every drawn piece stays within the table's own footprint and a margin
///   (a piece sweeping the screen is what the owner saw first);
/// - the floating dial stays over every piece while it floats (it was cut
///   by a piece passing over it, measured live).
///
/// Red on a straight swap (the new piece drawn at the table's height from
/// the split on).
#[test]
fn the_pieces_never_meet() {
    use super::super::transition::{DIAL_THICKNESS, ENDS, Phase, Piece, Pose, dial_lift};
    let follow = |shown: Pose, target: Pose, k: f32| Pose {
        turn: shown.turn + (target.turn - shown.turn) * k,
        shift: shown.shift + (target.shift - shown.shift) * k,
        lift: shown.lift + (target.lift - shown.lift) * k,
        ..target
    };
    let mut checked = 0;
    for n in 3..=8_u8 {
        for interest in 1..n {
            let Some(tear) = spotlight_tear(n, interest) else {
                continue;
            };
            let (lo, hi) = tear.to.extent().expect("seated");
            let half = (hi - lo) * 0.5 + Vec2::splat(1.5);
            let seed = 7.3;
            let ground = |piece: Piece| Ground {
                half,
                near: piece == Piece::Near,
                seed,
            };
            let step = 1.0 / 60.0_f32;
            let k = 1.0 - (-16.0 * step).exp();
            let pieces = [Piece::Near, Piece::Leaving, Piece::Arriving];
            let mut poses = pieces.map(|p| tear.pose(p, 0.0));
            poses[0] = Pose {
                pivot: tear.pivot,
                ..Pose::REST
            };
            poses[1] = poses[0];
            let mut dial = 0.0_f32;
            let mut t = 0.0;
            while t < ENDS + 0.2 {
                t += step;
                for (i, piece) in pieces.into_iter().enumerate() {
                    let target = tear.pose(piece, t);
                    poses[i] = if poses[i].shown {
                        follow(poses[i], target, k)
                    } else {
                        target
                    };
                }
                dial += (dial_lift(t) - dial) * k;
                let what = format!("n={n} seat {interest} t={t:.3}");
                if Phase::at(t) < Phase::Dock {
                    for (i, pose) in poses.iter().enumerate() {
                        assert!(
                            !pose.shown || dial - DIAL_THICKNESS > pose.lift,
                            "{what}: {:?} at {} passes through the dial at {dial}",
                            pieces[i],
                            pose.lift
                        );
                    }
                }
                let [near, leaving, arriving] = poses;
                assert!(
                    !(leaving.shown
                        && arriving.shown
                        && grounds_meet(
                            (ground(Piece::Leaving), leaving),
                            (ground(Piece::Arriving), arriving)
                        )),
                    "{what}: the new piece over the old one's place while it is there"
                );
                for (a, b) in [(0, 1), (0, 2), (1, 2)] {
                    let (pa, pb) = (poses[a], poses[b]);
                    if pa.shown && pb.shown && heights_meet(pa, pb) {
                        assert!(
                            !grounds_meet((ground(pieces[a]), pa), (ground(pieces[b]), pb)),
                            "{what}: {:?} and {:?} meet ({pa:?} / {pb:?})",
                            pieces[a],
                            pieces[b]
                        );
                    }
                }
                for (i, pose) in poses.iter().enumerate() {
                    if !pose.shown {
                        continue;
                    }
                    for point in ground(pieces[i]).edge() {
                        let at = pose.carry(point);
                        assert!(
                            at.x.abs() <= half.x + 3.0 && at.y.abs() <= half.y + 3.0,
                            "{what}: {:?} strays off the table to {at}",
                            pieces[i]
                        );
                    }
                }
                let _ = near;
                checked += 1;
            }
        }
    }
    assert!(checked > 1000, "{checked} frames checked");
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

/// The tear's line (the coordinator's of 07.10.2026: *"a periodic comb"*):
/// within its reach of the middle, jagged, and natural — slow chunks under
/// the teeth (its five-unit running mean wanders by more than 0.4 units)
/// and teeth of very different sizes (their heights' spread over their
/// mean above 0.38). The first line, a triangle wave of near-even pitch and
/// size, measured 0.09 to 0.30 and 0.17 to 0.27: red on both.
#[test]
fn the_tear_line_is_jagged_and_never_a_comb() {
    use super::super::transition::{LINE_REACH, tear_line};
    for seed in [0.0_f32, 7.3, 31.9, 113.0] {
        let step = 0.02;
        let ys: Vec<f32> = (0..2000)
            .map(|i| tear_line(-20.0 + i as f32 * step, seed))
            .collect();
        assert!(ys.iter().all(|y| y.abs() <= LINE_REACH), "within reach");
        // The turning points of the teeth (the grain's small wiggles are
        // read through: only turns a twentieth of a unit apart count).
        let mut turns: Vec<(f32, f32)> = Vec::new();
        for i in 1..ys.len() - 1 {
            if (ys[i] - ys[i - 1]) * (ys[i + 1] - ys[i]) < 0.0 {
                let at = (i as f32 * step, ys[i]);
                if turns.last().is_none_or(|last| (at.1 - last.1).abs() > 0.05) {
                    turns.push(at);
                }
            }
        }
        assert!(
            turns.len() > 40,
            "seed {seed}: jagged, {} turns",
            turns.len()
        );
        let heights: Vec<f32> = turns.windows(2).map(|w| (w[1].1 - w[0].1).abs()).collect();
        let spread = |v: &[f32]| {
            let mean = v.iter().sum::<f32>() / v.len() as f32;
            let var = v.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / v.len() as f32;
            var.sqrt() / mean
        };
        assert!(
            spread(&heights) > 0.38,
            "seed {seed}: teeth of one size ({:.2})",
            spread(&heights)
        );
        let window = (5.0 / step) as usize;
        let means: Vec<f32> = ys
            .windows(window)
            .step_by(25)
            .map(|w| w.iter().sum::<f32>() / w.len() as f32)
            .collect();
        let wander = means.iter().copied().fold(f32::MIN, f32::max)
            - means.iter().copied().fold(f32::MAX, f32::min);
        assert!(wander > 0.4, "seed {seed}: no chunks ({wander:.2})");
    }
}

/// The Turntable (DESIGN-v8 §1 row 3): the seat of interest's side across
/// as the duel's, at the duel's size, and every other seat a side mat on a
/// flank, drawn at its seat count's scale and squared to my chair; the
/// sides between mine and the one across, clockwise, on the left from the
/// bottom up, the rest on the right from the top down — the ring's order
/// read round from my chair.
#[test]
fn the_turntable_seats_the_pair_and_stands_the_rest_on_the_flanks() {
    use super::super::arrangement::side_scale;
    for n in 3..=8_u8 {
        for aspect in ASPECTS {
            let roster: Vec<Seat> = seats(n).into_iter().map(Seat::alone).collect();
            for across in 1..n {
                let table = TableLayout::arranged(
                    &roster,
                    aspect,
                    Arrangement::Turntable,
                    Some(PlayerId::new(across)),
                );
                let what = format!("n={n} aspect={aspect} across={across}");
                let duel = TableLayout::seated(
                    &[
                        Seat::alone(PlayerId::new(0)),
                        Seat::alone(PlayerId::new(across)),
                    ],
                    aspect,
                    None,
                );
                let there = table.slots[usize::from(across)];
                assert!(there.center.x.abs() < 1e-3, "{what}: across, in the middle");
                assert!(
                    (there.scale - 1.0).abs() < 1e-6,
                    "{what}: across at duel size"
                );
                assert!(
                    table.slots.iter().all(|s| !s.parked),
                    "{what}: nobody parked"
                );
                // The pair keeps the duel's depth: narrowed, not shrunk (what
                // the camera then draws it at is measured in the client,
                // `every_board_is_on_screen_or_one_interest_away`).
                let (lo, hi) = table.extent().expect("seated");
                let (dlo, dhi) = duel.extent().expect("a duel");
                assert!(
                    n > 6 || (hi.y - lo.y) <= (dhi.y - dlo.y) + 1e-3,
                    "{what}: no deeper than the duel"
                );
                let pair_reach = there.footprint().x;
                let left: Vec<&SeatSlot> = table.slots[1..usize::from(across)].iter().collect();
                let right: Vec<&SeatSlot> = table.slots[usize::from(across) + 1..].iter().collect();
                for slot in left.iter().chain(&right) {
                    assert!(
                        (slot.scale - side_scale(usize::from(n))).abs() < 1e-6,
                        "{what}: {:?} drawn at {}",
                        slot.player,
                        slot.scale
                    );
                    assert!(slot.center.x.abs() > pair_reach, "{what}: on a flank");
                    assert!((slot.facing - core::f32::consts::PI).abs() < 1e-6, "{what}");
                }
                assert!(left.iter().all(|s| s.center.x < 0.0), "{what}: left");
                assert!(right.iter().all(|s| s.center.x > 0.0), "{what}: right");
                assert!(
                    left.windows(2).all(|w| w[0].center.y < w[1].center.y),
                    "{what}: the left flank from the bottom up"
                );
                assert!(
                    right.windows(2).all(|w| w[0].center.y > w[1].center.y),
                    "{what}: the right flank from the top down"
                );
            }
        }
    }
}

/// A scaled pod is a duel's pod drawn smaller about its centre: every place
/// on it (rows, piles, the ledge) is the unscaled one moved in by the
/// scale, every length it reports is scaled, and its rows pack exactly as
/// the duel's do — the same cards, the same overlaps, only drawn smaller.
#[test]
fn a_scaled_pod_is_the_duel_pod_drawn_smaller() {
    let pair: Vec<Seat> = seats(2).into_iter().map(Seat::alone).collect();
    let full = TableLayout::seated(&pair, HUD_ASPECT, None).slots[1];
    for scale in [0.33_f32, 0.4, 0.55] {
        let small = SeatSlot {
            half_extent: full.half_extent * scale,
            scale,
            ..full
        };
        let shrunk = |p: Vec2| full.center + (p - full.center) * scale;
        for lane in LaneKind::ALL {
            assert!(
                small
                    .lane_center(lane)
                    .distance(shrunk(full.lane_center(lane)))
                    < 1e-4
            );
        }
        for pile in [PileKind::Library, PileKind::Graveyard, PileKind::Exile] {
            assert!(
                small
                    .pile_center(pile)
                    .distance(shrunk(full.pile_center(pile)))
                    < 1e-4
            );
        }
        for (a, b) in small.ledge_corners().into_iter().zip(full.ledge_corners()) {
            assert!(a.distance(shrunk(b)) < 1e-4);
        }
        assert!((small.lane_height() - full.lane_height() * scale).abs() < 1e-5);
        assert!((small.footprint() - full.footprint() * scale).length() < 1e-4);
        assert_eq!(small.badge_place(), full.badge_place());
        assert!(small.reach().abs_diff_eq(full.reach(), 1e-4));
    }
}

/// Pods (DESIGN-v8 §1 row 5): my pod where the ring has it, every other
/// seat a pod of the ring's own size (a duel's board wide) in a grid above
/// it — one row up to three, two from four — squared to my chair, in turn
/// order from the front row's left; a seat visited moves no card.
#[test]
fn the_pods_stand_in_a_grid_above_me() {
    use super::super::arrangement::rows_for;
    for n in 3..=8_u8 {
        for aspect in ASPECTS {
            let roster: Vec<Seat> = seats(n).into_iter().map(Seat::alone).collect();
            let table = TableLayout::arranged(&roster, aspect, Arrangement::Pods, None);
            let what = format!("n={n} aspect={aspect}");
            // Mine just in front of the dial's gap; every board one size.
            let mine = table.slots[0];
            let board = mine.half_extent;
            assert!(
                mine.center.x.abs() < 1e-4 && mine.facing.abs() < 1e-6,
                "{what}"
            );
            assert!(
                (mine.center.y + mine.footprint().y + super::super::arrangement::DIAL_GAP * 0.5)
                    .abs()
                    < 1e-4,
                "{what}: just in front of the dial"
            );
            let others = &table.slots[1..];
            let mut rows: Vec<f32> = others.iter().map(|s| s.center.y).collect();
            rows.sort_by(f32::total_cmp);
            rows.dedup_by(|a, b| (*a - *b).abs() < 1e-3);
            assert_eq!(rows.len(), rows_for(others.len()), "{what}: rows");
            for slot in others {
                assert!(slot.center.y > table.slots[0].center.y, "{what}: above me");
                assert_eq!(slot.half_extent, board, "{what}: every board one size");
                assert!(
                    (slot.facing - core::f32::consts::PI).abs() < 1e-6,
                    "{what}: squared to my chair"
                );
            }
            // Turn order: the front row left to right, then the next.
            for pair in others.windows(2) {
                let (a, b) = (pair[0].center, pair[1].center);
                assert!(
                    (b.y > a.y + 1e-3) || ((b.y - a.y).abs() < 1e-3 && b.x > a.x),
                    "{what}: {a} then {b}"
                );
            }
            for interest in roster.iter().map(|s| Some(s.player)) {
                assert_eq!(
                    TableLayout::arranged(&roster, aspect, Arrangement::Pods, interest),
                    table,
                    "{what}: a visit moves no card"
                );
            }
        }
    }
}

/// The arc rail (DESIGN-v8 §1 row 4): every other seat on one arc above my
/// board, in turn order from left to right, the middle of the arc straight
/// across from me, each board turned to face the arc's middle (the board
/// straight across turned as a duel's across board is); a visit moves no
/// card, and the rail's window round its middle holds three boards abreast
/// whole (two round an even arc's middle gap).
#[test]
fn the_arc_rail_stands_the_others_on_one_arc() {
    for n in 3..=8_u8 {
        for aspect in ASPECTS {
            let roster: Vec<Seat> = seats(n).into_iter().map(Seat::alone).collect();
            let table = TableLayout::arranged(&roster, aspect, Arrangement::ArcRail, None);
            let what = format!("n={n} aspect={aspect}");
            let mine = table.slots[0];
            let others = &table.slots[1..];
            for pair in others.windows(2) {
                assert!(
                    pair[0].center.x < pair[1].center.x,
                    "{what}: left to right in turn order"
                );
            }
            let middle = others.len() / 2;
            if others.len() % 2 == 1 {
                assert!(others[middle].center.x.abs() < 1e-3, "{what}: across");
                assert!(
                    (others[middle].facing - core::f32::consts::PI).abs() < 1e-4,
                    "{what}: a duel's across board"
                );
            }
            // Every board's forward points at one point below the arc: the
            // circle's middle.
            let centres: Vec<Vec2> = others
                .iter()
                .map(|s| {
                    let forward = s.forward();
                    // Where the forward line crosses x = 0.
                    if forward.x.abs() < 1e-6 {
                        Vec2::new(0.0, f32::NAN)
                    } else {
                        s.center - forward * (s.center.x / forward.x)
                    }
                })
                .filter(|c| c.y.is_finite())
                .collect();
            for pair in centres.windows(2) {
                assert!(
                    (pair[0].y - pair[1].y).abs() < 1e-3 * pair[0].y.abs().max(1.0),
                    "{what}: one circle"
                );
            }
            for slot in others {
                assert!(slot.center.y > mine.center.y, "{what}: above me");
            }
            for interest in roster.iter().map(|s| Some(s.player)) {
                assert_eq!(
                    TableLayout::arranged(&roster, aspect, Arrangement::ArcRail, interest),
                    table,
                    "{what}: a visit moves no card"
                );
            }
            let (lo, hi) =
                super::super::rail_window(&table, 0.0, super::super::IN_VIEW).expect("mine");
            let whole = others
                .iter()
                .filter(|s| {
                    s.center.x - s.footprint().x >= lo.x - 1e-3
                        && s.center.x + s.footprint().x <= hi.x + 1e-3
                })
                .count();
            // Three whole round an arc's middle board, two round the gap in
            // an even arc's middle.
            let want = if others.len() % 2 == 1 { 3 } else { 2 };
            assert_eq!(whole, others.len().min(want), "{what}: in view");
        }
    }
}

/// The Focus ring (DESIGN-v8 §1 row 8): the Spotlight's pair, and every
/// parked seat a peek beside the felt — the sides between mine and the one
/// across, clockwise, in the left column from the bottom up, the rest in
/// the right column from the top down, the Turntable's flank rule. Every
/// parked seat has a peek, and no seat on the felt has one.
#[test]
fn the_focus_ring_peeks_at_every_parked_seat_on_its_side() {
    use super::super::peeks;
    for n in 3..=8_u8 {
        let roster: Vec<Seat> = seats(n).into_iter().map(Seat::alone).collect();
        for interest in std::iter::once(None).chain((1..n).map(|i| Some(PlayerId::new(i)))) {
            let table =
                TableLayout::arranged(&roster, HUD_ASPECT, Arrangement::FocusRing, interest);
            assert_eq!(
                table,
                TableLayout::arranged(&roster, HUD_ASPECT, Arrangement::Spotlight, interest),
                "the Spotlight's pair"
            );
            let (left, right) = peeks(&roster, interest);
            let mut peeked: Vec<PlayerId> = left.iter().chain(&right).copied().collect();
            peeked.sort_unstable();
            let parked: Vec<PlayerId> = table
                .slots
                .iter()
                .filter(|s| s.parked)
                .map(|s| s.player)
                .collect();
            assert_eq!(
                peeked, parked,
                "n={n} {interest:?}: a peek for every parked seat"
            );
            let across = interest.unwrap_or(PlayerId::new(n / 2));
            assert!(
                left.iter().all(|p| p.get() < across.get()),
                "n={n}: left before across"
            );
            assert!(
                right.iter().all(|p| p.get() > across.get()),
                "n={n}: right after it"
            );
            assert!(
                left.windows(2).all(|w| w[0].get() < w[1].get()),
                "bottom up"
            );
            assert!(
                right.windows(2).all(|w| w[0].get() < w[1].get()),
                "top down"
            );
        }
    }
}

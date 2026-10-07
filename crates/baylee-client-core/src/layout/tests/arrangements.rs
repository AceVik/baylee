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

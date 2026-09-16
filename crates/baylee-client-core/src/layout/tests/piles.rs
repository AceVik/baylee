//! Where the five piles stand against the ground they serve: beside the mat and never on it, on the hand `PileKind::side` names, and far enough apart to be five places rather than one stack. A pile lying on the mat would read as a permanent in play, so every measurement here is taken across the seat's own side axis against `half_extent.x`, and two seats' piles are held apart by the separating-axis test written out rather than approximated by a circle round each box — a circle bound is sufficient and fails at seat counts where nothing actually touches. What opens out of a pile on hover is in `fan`; whether the camera can see one at all is in `extent`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn a_pile_stands_beside_the_ground_and_never_on_it() {
    for n in [2, 3, 4, 6, 8] {
        let layout = TableLayout::new(&seats(n), 2.0, None);
        for slot in &layout.slots {
            let side = Vec2::new(slot.facing.cos(), -slot.facing.sin());
            for pile in PileKind::ALL {
                let across = (slot.pile_center(pile) - slot.center).dot(side);
                let near_edge = across.abs() - CARD_WIDTH * 0.5;
                assert!(
                    near_edge > slot.half_extent.x,
                    "{n} seats: the near edge of the {} is {near_edge} out from \
                     the middle of a mat {} wide — it is lying on the board",
                    pile.label(),
                    slot.half_extent.x
                );
                assert!(
                    (across.signum() - pile.side()).abs() < 1e-6,
                    "{n} seats: the {} came out on the seat's other hand",
                    pile.label()
                );
            }
        }
    }
}

#[test]
fn the_four_piles_are_four_places() {
    let layout = TableLayout::new(&seats(2), 2.0, None);
    let slot = layout.local().expect("a local seat");
    for (i, a) in PileKind::ALL.iter().enumerate() {
        for b in &PileKind::ALL[i + 1..] {
            let gap = slot.pile_center(*a).distance(slot.pile_center(*b));
            assert!(
                gap > CARD_HEIGHT,
                "the {} and the {} are {gap} apart, and a card is {CARD_HEIGHT} \
                 long — they would be stacked on each other",
                a.label(),
                b.label()
            );
        }
    }
}

#[test]
fn no_two_seats_piles_stand_on_each_other() {
    for n in [3, 4, 5, 6, 7, 8] {
        let layout = TableLayout::new(&seats(n), 2.0, None);
        for (i, a) in layout.slots.iter().enumerate() {
            for b in &layout.slots[i + 1..] {
                for pa in PileKind::ALL {
                    for pb in PileKind::ALL {
                        assert!(
                            !quads_overlap(pile_corners(a, pa), pile_corners(b, pb)),
                            "{n} seats: seat {:?}'s {} lies on top of seat {:?}'s {}",
                            a.player,
                            pa.label(),
                            b.player,
                            pb.label()
                        );
                    }
                }
            }
        }
    }
}

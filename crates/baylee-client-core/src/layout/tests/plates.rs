//! What a row leaves each card's plate ([`LanePacking::plate_room`]), and
//! the guard it is for: no plate, and no plate's shadow, lies on the print
//! of a card drawn under it (`docs/legal.md` §3).
//!
//! The row's own frame: `x` along the row from the lane's centre, `y` down
//! the table towards the seat from the row's middle line, in table units. A
//! card later in the row lies higher, over the plates of the cards before
//! it; a card before it lies lower, and a plate over its print would be
//! drawn on it.

use super::*;
use crate::cardplate::{KIND_FIGHT, PlateSpot, plate_quad, plate_rect};

/// Card `j`'s print, the cards of its pile included, in the row's frame.
fn print(row: &LanePacking, j: usize, tapped: &[bool], staged: &[bool]) -> [f32; 4] {
    let (half_w, half_h) = if tapped[j] {
        (CARD_SPAN * 0.5, CARD_WIDTH * 0.5)
    } else {
        (CARD_WIDTH * 0.5, CARD_SPAN * 0.5)
    };
    let dy = if staged[j] { -STAGE_STEP } else { 0.0 };
    let x = row.offsets[j];
    [
        x - half_w - row.reach[j],
        dy - half_h,
        x + half_w,
        dy + half_h,
    ]
}

/// Card `i`'s plate with its shadow, in the row's frame, and where it
/// stands; `None` where the row leaves it no room.
fn plate(
    row: &LanePacking,
    window: &RowWindow,
    i: usize,
    tapped: &[bool],
    staged: &[bool],
) -> Option<([f32; 4], PlateSpot)> {
    let room = row.plate_room(i, window, tapped, staged);
    let (body, spot) = plate_rect(KIND_FIGHT, tapped[i], room, None)?;
    let [x0, y0, x1, y1] = plate_quad(body);
    let left = row.offsets[i] - CARD_WIDTH * 0.5;
    let top = -CARD_HEIGHT * 0.5 - if staged[i] { STAGE_STEP } else { 0.0 };
    Some((
        [
            left + x0 * CARD_WIDTH,
            top + y0 * CARD_WIDTH,
            left + x1 * CARD_WIDTH,
            top + y1 * CARD_WIDTH,
        ],
        spot,
    ))
}

fn overlaps(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] < b[2] - 1e-5 && b[0] < a[2] - 1e-5 && a[1] < b[3] - 1e-5 && b[1] < a[3] - 1e-5
}

/// What of `a` lies outside `b`, as at most four rectangles.
fn minus(a: [f32; 4], b: [f32; 4]) -> Vec<[f32; 4]> {
    if !overlaps(a, b) {
        return vec![a];
    }
    let [x0, y0, x1, y1] = a;
    let (ix0, ix1) = (x0.max(b[0]), x1.min(b[2]));
    [
        [x0, y0, ix0, y1],
        [ix1, y0, x1, y1],
        [ix0, y0, ix1, y0.max(b[1])],
        [ix0, y1.min(b[3]), ix1, y1],
    ]
    .into_iter()
    .filter(|r| r[2] > r[0] + 1e-5 && r[3] > r[1] + 1e-5)
    .collect()
}

/// Every width a lane is laid out at: each seat's at every table from one
/// seat to eight, wide and square.
fn widths() -> Vec<f32> {
    let mut out = Vec::new();
    for seats in 1..=8u8 {
        let players: Vec<PlayerId> = (0..seats).map(PlayerId::new).collect();
        for aspect in [16.0 / 9.0, 4.0 / 3.0] {
            for slot in &TableLayout::new(&players, aspect, None).slots {
                out.push(slot.lane_width());
            }
        }
    }
    out.sort_by(f32::total_cmp);
    out.dedup_by(|a, b| (*a - *b).abs() < 1e-3);
    out
}

/// Which cards of a row of `n` are turned and which stepped out: none, all,
/// every other one, every third, and one alone or all but one at either end
/// and in the middle; attacking, where the tapped ones stepped out, and
/// blocking, where untapped ones did.
fn states(n: usize) -> Vec<(Vec<bool>, Vec<bool>)> {
    let mut tapped: Vec<Vec<bool>> = vec![
        vec![false; n],
        vec![true; n],
        (0..n).map(|i| i % 2 == 0).collect(),
        (0..n).map(|i| i % 2 == 1).collect(),
        (0..n).map(|i| i % 3 == 1).collect(),
    ];
    for k in [0, 1, n / 2, n.saturating_sub(2), n - 1] {
        tapped.push((0..n).map(|i| i == k).collect());
        tapped.push((0..n).map(|i| i != k).collect());
    }
    let mut out = Vec::new();
    for t in tapped {
        let attacking = t.clone();
        let blocking: Vec<bool> = t.iter().map(|&x| !x).collect();
        let alternate: Vec<bool> = (0..n).map(|i| i % 2 == 1).collect();
        for s in [vec![false; n], attacking, blocking, alternate] {
            out.push((t.clone(), s));
        }
    }
    out
}

/// No plate, nor its shadow, lies on the print of a card before it in its
/// row, which is drawn under it: at every lane width a table has, for rows
/// from one card to sixty, loose and piled, tapped and untapped in every
/// pattern, in combat and out, scrolled to either end. Sixty, because the
/// lanes are 16.5 to 23.5 wide and a row reaches the tightest fan only past
/// forty-five. And every place a plate can stand is reached, and so is a
/// tapped card its neighbours leave no air, or the loop said nothing about
/// either.
#[test]
fn no_plate_lies_on_a_print_drawn_under_it() {
    let mut seen = Vec::new();
    let mut placed = 0usize;
    let mut none = 0usize;
    for width in widths() {
        for n in [1usize, 2, 3, 5, 8, 13, 20, 30, 40, 48, 54, 60] {
            for piled in [false, true] {
                let reach: Vec<f32> = (0..n)
                    .map(|i| pile_reach(if piled && i % 4 == 2 { 3 } else { 1 }))
                    .collect();
                let row = pack_gaps(&vec![Gap::Free; n], &reach, width);
                for first in [0, row.last_first()] {
                    let window = row.window(first);
                    for (tapped, staged) in states(n) {
                        for i in window.shown.clone() {
                            let Some((quad, spot)) = plate(&row, &window, i, &tapped, &staged)
                            else {
                                // Only a tapped card whose neighbours' prints
                                // leave the air under it narrower than a plate
                                // with its shadow and its air: one between two
                                // untapped cards below a pitch of 0.62, or one
                                // stepped into combat between two tapped cards
                                // that stayed, below 0.82.
                                let room = row.plate_room(i, &window, &tapped, &staged);
                                let [left, _, right, _] = crate::cardplate::turned([
                                    0.0,
                                    0.0,
                                    1.0,
                                    crate::cardrail::CARD_TALL,
                                ]);
                                let air = room.below[1].min(right) - room.below[0].max(left);
                                let plate = crate::cardplate::PLATE_W
                                    + crate::cardplate::PLATE_SHADE[0]
                                    + 2.0 * crate::cardplate::PLATE_AIR;
                                assert!(
                                    tapped[i] && air < plate + 1e-4,
                                    "{n} cards in {width} at {}: card {i} shows no plate \
                                     with {air} of air under it",
                                    row.pitch
                                );
                                none += 1;
                                continue;
                            };
                            placed += 1;
                            if !seen.contains(&spot) {
                                seen.push(spot);
                            }
                            // What lies on its own card is on the card over
                            // every one before it.
                            let off = minus(quad, print(&row, i, &tapped, &staged));
                            for j in window.shown.start..i {
                                let under = print(&row, j, &tapped, &staged);
                                assert!(
                                    off.iter().all(|&part| !overlaps(part, under)),
                                    "{n} cards in {width}, tapped {tapped:?}, staged \
                                     {staged:?}: card {i}'s plate {quad:?} lies on card \
                                     {j}'s print {under:?}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(seen.len(), 3, "only {seen:?} reached");
    // The squeeze is reached, or the arm above said nothing.
    assert!(none > 0 && placed > 0, "no tapped card was squeezed");
}

/// A row attacking all at once keeps every plate: the cards stepped out
/// together, and each tapped card's air is its own, however tight the fan.
#[test]
fn an_attacking_row_keeps_its_plates() {
    for width in widths() {
        for n in 1..=60usize {
            let row = pack_lane(n, width);
            let window = row.window(0);
            let (tapped, staged) = (vec![true; n], vec![true; n]);
            for i in window.shown.clone() {
                assert!(
                    plate(&row, &window, i, &tapped, &staged).is_some(),
                    "{n} attackers in {width}: card {i} shows no plate"
                );
            }
        }
    }
}

/// Where the row leaves room, every plate stands beside its printed box;
/// where it fans, each lies on its own card, and the last one, with
/// nothing after it, beside its box again.
#[test]
fn a_fan_lays_each_plate_on_its_own_card() {
    let width = 10.0;
    let untapped = |n: usize| (vec![false; n], vec![false; n]);
    let spots = |n: usize| {
        let row = pack_lane(n, width);
        let window = row.window(0);
        let (tapped, staged) = untapped(n);
        window
            .shown
            .clone()
            .map(|i| plate(&row, &window, i, &tapped, &staged).map(|(_, spot)| spot))
            .collect::<Vec<_>>()
    };
    assert!(spots(3).iter().all(|&s| s == Some(PlateSpot::Beside)));
    let fanned = spots(14);
    assert!(pack_lane(14, width).fanned);
    let (last, rest) = fanned.split_last().expect("a row");
    assert!(
        rest.iter().all(|&s| s == Some(PlateSpot::OnCard)),
        "{fanned:?}"
    );
    assert_eq!(*last, Some(PlateSpot::Beside));
}

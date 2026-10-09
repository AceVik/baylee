//! The packed table (owner, 08.10.2026): neighbouring boards a small,
//! constant gap apart and never on each other, every board's hearth band
//! clear for its plate and steps, the dial's circle clear in the middle,
//! and the frame taken only where it draws the least favoured board larger
//! than the ring would.

use super::*;

/// The canvases the packing is asked on: an ultrawide or a phone on its
/// side, a laptop's HUD, a tablet's, a square and a window held upright.
const ASPECTS: [f32; 5] = [2.8, 2.0, 1.44, 1.0, 0.6];

/// Every roster the packing is asked at `n` seats: everybody alone, two
/// teams alternating in turn order, three teams alternating, and two mixed
/// tables (a pair against two singles, and teams of one, two and three).
fn rosters(n: u8) -> Vec<(String, Vec<Seat>)> {
    let on = |team: &dyn Fn(u8) -> Option<u8>| -> Vec<Seat> {
        seats(n)
            .into_iter()
            .map(|p| Seat::on(p, team(p.get())))
            .collect()
    };
    let mut out = vec![("free-for-all".to_owned(), on(&|_| None))];
    if n >= 4 {
        out.push(("two teams".to_owned(), on(&|p| Some(p % 2))));
        out.push((
            "pair and singles".to_owned(),
            on(&|p| (p >= 2).then_some(9)),
        ));
    }
    if n >= 6 {
        out.push(("three teams".to_owned(), on(&|p| Some(p % 3))));
        out.push((
            "one, two and three".to_owned(),
            on(&|p| {
                Some(match p {
                    0 => 0,
                    1 | 3 => 1,
                    _ => 2,
                })
            }),
        ));
    }
    out
}

/// How far apart two seats' whole places (grounds and pile strips) stand:
/// the largest gap any of their four axes shows, negative where they meet.
/// A separating axis's gap is never more than the true distance, so a
/// bound on it is a bound on the distance.
fn gap(a: &SeatSlot, b: &SeatSlot) -> f32 {
    let axes = |s: &SeatSlot| {
        let (sin, cos) = s.facing.sin_cos();
        [Vec2::new(cos, -sin), Vec2::new(sin, cos)]
    };
    let reach = |s: &SeatSlot, u: Vec2| {
        let [along, away] = axes(s);
        let f = s.footprint();
        f.x.mul_add(u.dot(along).abs(), f.y * u.dot(away).abs())
    };
    axes(a)
        .into_iter()
        .chain(axes(b))
        .map(|u| {
            (b.footprint_center() - a.footprint_center()).dot(u).abs() - reach(a, u) - reach(b, u)
        })
        .fold(f32::NEG_INFINITY, f32::max)
}

/// A seat's hearth band: the strip [`HEARTH_BAND`] deep along its mat's
/// edge nearest the middle, where its plate and steps hang, as a quad.
fn hearth(slot: &SeatSlot) -> [Vec2; 4] {
    let (sin, cos) = slot.facing.sin_cos();
    let (along, away) = (Vec2::new(cos, -sin), Vec2::new(sin, cos));
    let half = slot.footprint();
    let edge = slot.footprint_center() + away * half.y;
    [
        edge - along * half.x,
        edge + along * half.x,
        edge + along * half.x + away * HEARTH_BAND,
        edge - along * half.x + away * HEARTH_BAND,
    ]
}

/// A seat's whole place as a quad.
fn place(slot: &SeatSlot) -> [Vec2; 4] {
    let (sin, cos) = slot.facing.sin_cos();
    let (along, away) = (Vec2::new(cos, -sin), Vec2::new(sin, cos));
    let half = slot.footprint();
    let c = slot.footprint_center();
    [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
        .map(|(x, y)| c + along * (x * half.x) + away * (y * half.y))
}

/// How far a quad stands from the middle of the table.
fn from_middle(quad: [Vec2; 4]) -> f32 {
    let inside = (0..4).all(|i| {
        let (p, q) = (quad[i], quad[(i + 1) % 4]);
        (q - p).perp_dot(-p) >= 0.0
    }) || (0..4).all(|i| {
        let (p, q) = (quad[i], quad[(i + 1) % 4]);
        (q - p).perp_dot(-p) <= 0.0
    });
    if inside {
        return 0.0;
    }
    (0..4)
        .map(|i| {
            let (p, q) = (quad[i], quad[(i + 1) % 4]);
            let t = (-p).dot(q - p) / (q - p).length_squared().max(1e-9);
            (p + (q - p) * t.clamp(0.0, 1.0)).length()
        })
        .fold(f32::INFINITY, f32::min)
}

/// No two boards on the felt meet, in any arrangement, at two to eight
/// seats, for every roster and canvas, at home and with each seat of
/// interest; and every pair of them keeps at least [`POD_GAP`] of felt
/// between their whole places — the gap the frame packs to, which no
/// arrangement goes under (the Turntable's side mats stand exactly that far
/// apart, the ring's frame too). The canvas held upright (0.6) is asked of
/// the ring and the upright ring, the arrangements a window that shape is
/// offered (`Arrangement::offered`: a compact window resolves the rest to
/// the ring).
#[test]
fn neighbouring_boards_keep_the_gap_and_never_meet() {
    let mut worst = f32::INFINITY;
    for arrangement in Arrangement::ALL {
        let upright_window_too =
            matches!(arrangement, Arrangement::Ring | Arrangement::UprightRing);
        for n in 2..=8_u8 {
            for (who, roster) in rosters(n) {
                for aspect in ASPECTS {
                    if aspect < 1.0 && !upright_window_too {
                        continue;
                    }
                    let interests =
                        std::iter::once(None).chain(roster.iter().map(|s| Some(s.player)));
                    for interest in interests {
                        let layout = TableLayout::arranged(&roster, aspect, arrangement, interest);
                        let felt: Vec<&SeatSlot> = layout.on_felt().collect();
                        for (i, a) in felt.iter().enumerate() {
                            for b in &felt[i + 1..] {
                                let g = gap(a, b);
                                worst = worst.min(g);
                                assert!(
                                    g >= POD_GAP - 1e-3,
                                    "{arrangement:?}, {n} seats ({who}) at {aspect}, \
                                     interest {interest:?}: {:?} and {:?} stand {g:.3} apart",
                                    a.player,
                                    b.player
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    // The bound is met, not merely cleared: the packing is as close as
    // it says.
    assert!(
        worst < POD_GAP + 0.01,
        "the closest pair stands {worst} apart"
    );
}

/// The ring's boards stand on one another's hearth band nowhere: what a
/// seat's plate and steps hang in (`seatbar::attached`, flush with the
/// mat's edge nearest the middle) is free felt, at every seat count,
/// roster and canvas.
#[test]
fn no_board_stands_in_another_s_hearth_band() {
    for n in 3..=8_u8 {
        for (who, roster) in rosters(n) {
            for aspect in ASPECTS {
                let layout = TableLayout::arranged(&roster, aspect, Arrangement::Ring, None);
                for a in &layout.slots {
                    for b in &layout.slots {
                        if a.player == b.player {
                            continue;
                        }
                        assert!(
                            !quads_overlap(hearth(a), place(b)),
                            "{n} seats ({who}) at {aspect}: {:?} stands in {:?}'s hearth band",
                            b.player,
                            a.player
                        );
                    }
                }
            }
        }
    }
}

/// The middle of a ring of three or more holds the dial at its smallest:
/// nothing drawn — a board with its printed border, a hearth band — comes
/// nearer the middle than [`DIAL_CLEAR`]. The constant the dial lane reads
/// (its `MIN_FREE_RADIUS`) is what the frame packs to, so the bound is met
/// somewhere as well as kept everywhere.
#[test]
fn the_middle_of_a_ring_holds_the_dial() {
    let mut tightest = f32::INFINITY;
    for n in 3..=8_u8 {
        for (who, roster) in rosters(n) {
            for aspect in ASPECTS {
                let layout = TableLayout::arranged(&roster, aspect, Arrangement::Ring, None);
                for slot in &layout.slots {
                    let drawn = {
                        let mut s = *slot;
                        s.half_extent += Vec2::splat(crate::tabletop::MAT_MARGIN);
                        from_middle(place(&s))
                    };
                    let band = from_middle(hearth(slot));
                    tightest = tightest.min(band);
                    assert!(
                        drawn >= DIAL_CLEAR - 1e-3 && band >= DIAL_CLEAR - 1e-3,
                        "{n} seats ({who}) at {aspect}: {:?}'s board is {drawn:.2} and its \
                         hearth band {band:.2} from the middle",
                        slot.player
                    );
                }
            }
        }
    }
    assert!(
        tightest < DIAL_CLEAR + 0.01,
        "no ring packs to the dial's circle: the tightest stands {tightest} off"
    );
}

/// What a table costs the least favoured board on it ([`frame::price`]).
fn price(layout: &TableLayout, aspect: f32) -> f32 {
    let (lo, hi) = layout.extent().expect("an extent");
    frame::price(hi - lo, aspect.clamp(0.45, 2.8), &layout.slots)
}

/// Whether a table keeps the gap, the hearth bands and the dial's circle —
/// what the three tests above hold every ring to — asked with this file's
/// own measures.
fn keeps(layout: &TableLayout) -> bool {
    let slots = &layout.slots;
    slots.iter().enumerate().all(|(i, a)| {
        let mut drawn = *a;
        drawn.half_extent += Vec2::splat(crate::tabletop::MAT_MARGIN);
        from_middle(place(&drawn)) >= DIAL_CLEAR - 1e-3
            && from_middle(hearth(a)) >= DIAL_CLEAR - 1e-3
            && slots.iter().enumerate().all(|(j, b)| {
                i == j || (gap(a, b) >= POD_GAP - 1e-3 && !quads_overlap(hearth(a), place(b)))
            })
    })
}

/// The frame is taken only where it draws the least favoured board larger
/// than the ellipse would, by the price both are judged on, or where the
/// ellipse brings two boards too near; and at the tables the owner named
/// — six and eight playing for themselves on a laptop — it is taken.
#[test]
fn a_frame_is_taken_only_where_it_pays() {
    for n in 3..=8_u8 {
        for (who, roster) in rosters(n) {
            for aspect in ASPECTS {
                let laid = TableLayout::seated(&roster, aspect, None);
                let ring = TableLayout::on_ring(&roster, aspect, None);
                if laid != ring {
                    assert!(
                        price(&laid, aspect) < price(&ring, aspect) * 0.99 || !keeps(&ring),
                        "{n} seats ({who}) at {aspect}: a frame that does not pay"
                    );
                }
            }
        }
    }
    for n in [6_u8, 8] {
        let alone: Vec<Seat> = seats(n).into_iter().map(Seat::alone).collect();
        assert_ne!(
            TableLayout::seated(&alone, HUD_ASPECT, None),
            TableLayout::on_ring(&alone, HUD_ASPECT, None),
            "{n} seats on a laptop stay on the ellipse"
        );
    }
}

/// A frame keeps the ring's order: clockwise from my seat in turn order,
/// one turn exactly, every seat facing the middle (its hearth band nearer
/// the middle than its back), and a team on one edge, facing one way.
#[test]
fn a_frame_seats_the_table_in_turn_order_facing_the_middle() {
    for n in 4..=8_u8 {
        for (who, roster) in rosters(n) {
            for aspect in ASPECTS {
                let layout = TableLayout::seated(&roster, aspect, None);
                if layout == TableLayout::on_ring(&roster, aspect, None) {
                    continue;
                }
                let bearing = |s: &SeatSlot| {
                    (-s.center.x)
                        .atan2(-s.center.y)
                        .rem_euclid(core::f32::consts::TAU)
                };
                // By side: allies share a bearing, the sides follow in turn.
                let mut sides: Vec<f32> = Vec::new();
                for slot in &layout.slots {
                    let b = bearing(slot);
                    let ally = roster[slot.ring_index].team.is_some_and(|team| {
                        layout.slots[..slot.ring_index]
                            .iter()
                            .any(|s| roster[s.ring_index].team == Some(team))
                    });
                    if !ally {
                        sides.push(b);
                    }
                    let away = Vec2::new(slot.facing.sin(), slot.facing.cos());
                    assert!(
                        away.dot(-slot.center) > 0.0,
                        "{n} seats ({who}) at {aspect}: {:?} faces away from the middle",
                        slot.player
                    );
                }
                let mut turned = 0.0_f32;
                for k in 0..sides.len() {
                    let step = (sides[(k + 1) % sides.len()] - sides[k])
                        .rem_euclid(core::f32::consts::TAU);
                    turned += step;
                }
                assert!(
                    (turned - core::f32::consts::TAU).abs() < 1e-3,
                    "{n} seats ({who}) at {aspect}: the sides go round {turned}"
                );
                for a in &layout.slots {
                    for b in &layout.slots {
                        let allies = a.player != b.player
                            && roster[a.ring_index].team.is_some()
                            && roster[a.ring_index].team == roster[b.ring_index].team;
                        if allies {
                            assert!(
                                (a.facing - b.facing).abs() < 1e-4,
                                "{n} seats ({who}) at {aspect}: allies facing apart"
                            );
                        }
                    }
                }
            }
        }
    }
}

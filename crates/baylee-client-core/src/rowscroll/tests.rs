//! The scroll state of the battlefield's rows, on a one-seat table: the
//! wheel moves a row a card a notch and stops at its ends, travel under a
//! notch is kept, a row that fits never moves, and the card cursor brings a
//! hidden card into view once and does not fight the wheel afterwards.

use super::*;
use crate::board::{CardGroup, Lane, Provenance, SeatPod, SeatRole, Section};
use crate::layout::{LaneKind, TableLayout};
use baylee_view::ObjectStatus;

fn group(slot: u32, merged: bool) -> CardGroup {
    let representative = ObjectId::new(slot, 0);
    let mut members = vec![representative];
    if merged {
        members.extend((1..4).map(|k| ObjectId::new(10_000 + slot * 4 + k, 0)));
    }
    CardGroup {
        base_power: None,
        base_toughness: None,
        representative,
        members,
        name: "x".into(),
        power: None,
        toughness: None,
        damage: 0,
        loyalty: None,
        status: ObjectStatus::default(),
        counters: Vec::new(),
        badges: Vec::new(),
        art: None,
        provenance: Provenance::Printed,
        original: None,
        summoning_sick: false,
        activatable: false,
        commander: false,
        individual: None,
        proposed: None,
        section: Section::Centre,
    }
}

/// A one-seat table whose land row holds `merged.len()` cards, merged where
/// `merged` says, and whose other rows are empty.
fn table(merged: &[bool]) -> (BoardModel, TableLayout) {
    let player = PlayerId::new(0);
    let lanes = LaneKind::ALL
        .iter()
        .map(|&kind| Lane {
            kind,
            groups: if kind == LaneKind::Lands {
                merged
                    .iter()
                    .enumerate()
                    .map(|(i, &m)| group(u32::try_from(i).expect("a small row") + 1, m))
                    .collect()
            } else {
                Vec::new()
            },
        })
        .collect();
    let pod = SeatPod {
        player,
        life: 20,
        poison: 0,
        energy: 0,
        hand_count: 0,
        library_count: 40,
        graveyard_count: 0,
        has_lost: false,
        is_local: true,
        is_active: true,
        is_awaited: true,
        role: SeatRole::Present,
        lanes,
        piles: crate::PileKind::ALL
            .into_iter()
            .map(crate::ZonePile::empty)
            .collect(),
        tokens: Vec::new(),
        threat: crate::ThreatSummary::default(),
    };
    let board = BoardModel {
        seq: 1,
        local: player,
        turn: 1,
        step: baylee_view::Step::Main,
        pods: vec![pod],
        stack: Vec::new(),
        hand: Vec::new(),
    };
    (board, TableLayout::new(&[player], 16.0 / 9.0, None))
}

const LANDS: RowKey = (PlayerId::new(0), LaneKind::Lands);

fn shown(board: &BoardModel, layout: &TableLayout, scroll: &RowScroll) -> std::ops::Range<usize> {
    packing_of(board, layout, LANDS)
        .expect("the row is on the table")
        .window(scroll.first(LANDS))
        .shown
}

#[test]
fn the_wheel_moves_a_scrolled_row_a_card_a_notch_and_stops_at_its_ends() {
    let (board, layout) = table(&[true; 40]);
    let packing = packing_of(&board, &layout, LANDS).expect("the row");
    assert!(packing.overflowing, "forty merged cards fit a lane");
    let mut scroll = RowScroll::default();
    assert!(scroll.wheel(&board, &layout, LANDS, ROW_STEP));
    assert_eq!(shown(&board, &layout, &scroll).start, 1);
    assert!(scroll.wheel(&board, &layout, LANDS, -ROW_STEP * 5.0));
    assert_eq!(shown(&board, &layout, &scroll).start, 0, "past the start");
    assert!(
        !scroll.wheel(&board, &layout, LANDS, -ROW_STEP),
        "already at the start"
    );
    scroll.wheel(&board, &layout, LANDS, ROW_STEP * 100.0);
    assert_eq!(shown(&board, &layout, &scroll).end, 40, "past the end");
    assert_eq!(scroll.first(LANDS), packing.last_first());
}

#[test]
fn travel_under_a_notch_is_kept_until_it_makes_one() {
    let (board, layout) = table(&[true; 40]);
    let mut scroll = RowScroll::default();
    assert!(!scroll.wheel(&board, &layout, LANDS, ROW_STEP * 0.5));
    assert!(scroll.wheel(&board, &layout, LANDS, ROW_STEP * 0.5));
    assert_eq!(scroll.first(LANDS), 1);
}

#[test]
fn a_row_that_fits_never_scrolls() {
    let (board, layout) = table(&[false, true, false]);
    let mut scroll = RowScroll::default();
    assert!(!scroll.wheel(&board, &layout, LANDS, ROW_STEP * 3.0));
    assert_eq!(shown(&board, &layout, &scroll), 0..3);
}

#[test]
fn the_cursor_brings_a_hidden_card_into_view_once() {
    let (board, layout) = table(&[true; 40]);
    let mut scroll = RowScroll::default();
    let last = ObjectId::new(40, 0);
    assert!(
        !shown(&board, &layout, &scroll).contains(&39),
        "the last card starts hidden"
    );
    scroll.follow(&board, &layout, Some(last));
    assert!(
        shown(&board, &layout, &scroll).contains(&39),
        "the cursor's card is shown"
    );
    // The wheel takes the row back, and the same hovered card does not
    // pull it forward again on the next frame.
    scroll.wheel(&board, &layout, LANDS, -ROW_STEP * 100.0);
    scroll.follow(&board, &layout, Some(last));
    assert_eq!(scroll.first(LANDS), 0);
    // A member of a merged card is found in its row too.
    scroll.follow(&board, &layout, Some(ObjectId::new(10_000 + 40 * 4 + 1, 0)));
    assert!(shown(&board, &layout, &scroll).contains(&39));
}

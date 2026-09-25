//! Where a scrolled row's bar lies and what its thumb says, and that the
//! table lays one for a row that scrolls and none for a row that fits.

use super::*;
use baylee_client_core::board::{BoardModel, CardGroup, Lane, Provenance, SeatPod, SeatRole};
use baylee_client_core::layout::CARD_HEIGHT;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_view::ObjectStatus;

fn group(slot: u32, merged: bool) -> CardGroup {
    let representative = ObjectId::new(slot, 0);
    let mut members = vec![representative];
    if merged {
        members.extend((1..4).map(|k| ObjectId::new(100_000 + slot * 4 + k, 0)));
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
    }
}

/// A table of `seats` whose land rows hold `row` merged cards each, the
/// other rows empty: forty of them scroll on any lane.
pub(crate) fn long_rows(seats: u8, row: usize) -> Duel {
    let players: Vec<PlayerId> = (0..seats).map(PlayerId::new).collect();
    let mut next = 1u32;
    let pods = players
        .iter()
        .enumerate()
        .map(|(seat, &player)| SeatPod {
            player,
            life: 20,
            poison: 0,
            energy: 0,
            hand_count: 0,
            library_count: 40,
            graveyard_count: 0,
            has_lost: false,
            is_local: seat == 0,
            is_active: seat == 0,
            is_awaited: seat == 0,
            role: SeatRole::Present,
            lanes: LaneKind::ALL
                .iter()
                .map(|&kind| Lane {
                    kind,
                    groups: if kind == LaneKind::Lands {
                        (0..row)
                            .map(|_| {
                                next += 1;
                                group(next, true)
                            })
                            .collect()
                    } else {
                        Vec::new()
                    },
                    overflowing: false,
                })
                .collect(),
            piles: baylee_client_core::PileKind::ALL
                .into_iter()
                .map(baylee_client_core::ZonePile::empty)
                .collect(),
            tokens: Vec::new(),
            threat: baylee_client_core::ThreatSummary::default(),
        })
        .collect();
    Duel {
        board: Some(BoardModel {
            seq: 1,
            local: PlayerId::new(0),
            turn: 1,
            step: baylee_view::Step::Main,
            pods,
            stack: Vec::new(),
            hand: Vec::new(),
        }),
        layout: Some(TableLayout::new(&players, 16.0 / 9.0, None)),
        ..Duel::default()
    }
}

/// Where along and across a seat's own frame a table point lies, from the
/// lane's centre.
fn in_frame(slot: &SeatSlot, lane: LaneKind, at: Vec2) -> (f32, f32) {
    let along = Vec2::new(slot.facing.cos(), -slot.facing.sin());
    let off = at - slot.lane_center(lane);
    (off.dot(along), off.dot(slot.forward()))
}

/// A duel's bar is a hairline in the seam under its own row, clear of the
/// row's cards and of the next row's; a ring's three bars stand one behind
/// another in the mat's outer margin, clear of every row.
#[test]
fn a_bar_lies_in_the_seam_in_a_duel_and_in_the_margin_round_a_ring() {
    for seats in [2u8, 3, 4, 8] {
        let layout = TableLayout::new(
            &(0..seats).map(PlayerId::new).collect::<Vec<_>>(),
            16.0 / 9.0,
            None,
        );
        for slot in &layout.slots {
            let half = slot.lane_height() * 0.5;
            let mut behind = Vec::new();
            for lane in LaneKind::ALL {
                let [_, grab, track, thumb] = lay(&layout, slot, lane, 0..5, 20, 15);
                let (_, depth) = in_frame(slot, lane, track.at);
                let near = depth + track.depth * 0.5;
                let far = depth - track.depth * 0.5;
                if seats == 2 {
                    // Clear of this row's cards and the next row's.
                    assert!(
                        near <= -(CARD_HEIGHT * 0.5) + 1e-4,
                        "{lane:?}: the bar reaches under its row's cards"
                    );
                    assert!(
                        far >= -(half * 2.0 - CARD_HEIGHT * 0.5) - 1e-4,
                        "{lane:?}: the bar reaches the next row"
                    );
                } else {
                    let (_, edge) = in_frame(slot, lane, slot.lane_center(LaneKind::Lands));
                    let back = edge - half;
                    assert!(
                        near < back,
                        "{seats} seats, {lane:?}: the bar is on the rows"
                    );
                    assert!(
                        far > back - baylee_client_core::tabletop::MAT_MARGIN,
                        "{seats} seats, {lane:?}: the bar is off the mat"
                    );
                    // Against the land row's centre, one frame for all three.
                    behind.push((near - edge, far - edge));
                }
                assert!((grab.at - track.at).length() < 1e-5 && grab.depth >= track.depth);
                assert!((thumb.depth - track.depth).abs() < 1e-6 && thumb.length <= track.length);
            }
            for pair in behind.windows(2) {
                assert!(
                    pair[1].0 < pair[0].1,
                    "{seats} seats: two rows' bars overlap"
                );
            }
        }
    }
}

/// The thumb is the share of the row shown, at the track's start when the
/// row shows its first card and at its end when it shows its last.
#[test]
fn the_thumb_is_the_shown_share_where_the_run_is() {
    let layout = TableLayout::new(&[PlayerId::new(0), PlayerId::new(1)], 16.0 / 9.0, None);
    let slot = layout.slots[0];
    let lane = LaneKind::Lands;
    let [.., track, start] = lay(&layout, &slot, lane, 0..10, 40, 30);
    let [.., _, end] = lay(&layout, &slot, lane, 30..40, 40, 30);
    assert!(
        (start.length - track.length * 0.25).abs() < 1e-4,
        "a quarter shown"
    );
    let (s, _) = in_frame(&slot, lane, start.at);
    let (e, _) = in_frame(&slot, lane, end.at);
    let (t, _) = in_frame(&slot, lane, track.at);
    assert!(
        (s - start.length * 0.5 - (t - track.length * 0.5)).abs() < 1e-4,
        "not at the start"
    );
    assert!(
        (e + end.length * 0.5 - (t + track.length * 0.5)).abs() < 1e-4,
        "not at the end"
    );
}

/// A row that scrolls gets its four parts, a row that fits none, and a row
/// that stops scrolling loses them.
#[test]
fn only_a_row_that_scrolls_has_a_bar() {
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .insert_resource(long_rows(2, 40))
        .add_systems(Update, sync_row_bars);
    app.update();
    let parts = |app: &mut App| {
        let mut found: Vec<(RowKey, Part)> = app
            .world_mut()
            .query::<&RowBar>()
            .iter(app.world())
            .map(|bar| (bar.row, bar.part))
            .collect();
        found.sort_by_key(|(row, part)| (row.0, Part::ALL.iter().position(|p| p == part)));
        found
    };
    let found = parts(&mut app);
    assert_eq!(
        found.len(),
        8,
        "two seats' land rows, four parts each: {found:?}"
    );
    assert!(found.iter().all(|(row, _)| row.1 == LaneKind::Lands));
    app.insert_resource(long_rows(2, 3));
    app.update();
    assert!(parts(&mut app).is_empty(), "a row that fits keeps its bar");
}

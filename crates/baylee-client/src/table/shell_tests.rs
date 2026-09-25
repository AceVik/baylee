//! The shells round a protected permanent, on real tables (#298): a rim or a
//! dome that stands never lands on another card's print, seen from the real
//! camera of every seat; the mask still leaves a rim to see; and both halves
//! of each choice are taken, so neither test is true of nothing. Defender's
//! wall stands under every face, so every print in front of it hides it.

use super::flying_tests::creature;
use super::*;
use crate::shellmat::{self, Dome, Footprint};
use baylee_client_core::board::{BoardModel, Lane, SeatPod};
use baylee_client_core::layout::LaneKind;

/// A table of `seats` players with the same three rows each, `row` cards in
/// each, tapped, flying and merged in patterns that differ by row and seat,
/// so the sweep meets every neighbour a rim can have.
fn table(seats: u8, row: [usize; 3], window: Vec2) -> Duel {
    let players: Vec<PlayerId> = (0..seats).map(PlayerId::new).collect();
    let mut next = 1u32;
    let pods = players
        .iter()
        .enumerate()
        .map(|(seat, &player)| {
            let lanes = LaneKind::ALL
                .iter()
                .zip(row)
                .enumerate()
                .map(|(r, (&kind, n))| {
                    let groups = (0..n)
                        .map(|i| {
                            let slot = next;
                            next += 1;
                            let flying = kind == LaneKind::Creatures && (i + seat) % 3 == 0;
                            let mut group = creature(
                                slot,
                                if flying {
                                    vec![KeywordBadge::Flying]
                                } else {
                                    Vec::new()
                                },
                            );
                            if (i + 2 * seat + r) % 2 == 0 {
                                group.status = baylee_view::ObjectStatus::TAPPED;
                            }
                            if i % 4 == 3 {
                                group.members.extend(
                                    (0..3).map(|k| ObjectId::new(100_000 + slot * 4 + k, 0)),
                                );
                            }
                            group
                        })
                        .collect();
                    Lane { kind, groups }
                })
                .collect();
            SeatPod {
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
                role: baylee_client_core::board::SeatRole::Present,
                lanes,
                piles: baylee_client_core::PileKind::ALL
                    .into_iter()
                    .map(baylee_client_core::ZonePile::empty)
                    .collect(),
                tokens: Vec::new(),
                threat: baylee_client_core::ThreatSummary::default(),
            }
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
        layout: Some(TableLayout::new(
            &players,
            Canvas::hud(window).aspect(),
            None,
        )),
        ..Duel::default()
    }
}

/// Where a placement's card stands, as `sync_scene` puts it: its row, its
/// deck, a flier at the top of its bob, and the hover's lift and growth.
fn pose(placement: &Placement, hovered: bool) -> Transform {
    let float = if placement.flying {
        airborne::RESTING + airborne::SWAY
    } else {
        0.0
    };
    let mut at = card_transform(
        &placement.slot,
        placement.position,
        placement.tapped,
        placement.lift + stack_rise(placement.count.saturating_sub(1)) + float,
    );
    if hovered {
        at.translation.y += HOVER_LIFT;
        at.scale *= HOVER_SCALE;
    }
    at
}

/// Every shot a player gets of `duel`: the whole table, and each seat
/// framed on its own; and the whole table from every other seat's side, as
/// that player's own client shows it, turned round the table's middle.
fn eyes(duel: &Duel, window: Vec2) -> Vec<Vec3> {
    let layout = duel.layout.as_ref().expect("a layout");
    let home = CameraRig::home(layout, Canvas::hud(window))
        .eye()
        .translation;
    let mut out = vec![home];
    for slot in &layout.slots {
        let world = Vec2::new(slot.center.x, -slot.center.y);
        out.push(CameraRig::framing(slot, world).eye().translation);
    }
    let seats = layout.slots.len();
    for seat in 1..seats {
        #[expect(clippy::cast_precision_loss)] // a handful of seats
        let turn = std::f32::consts::TAU * seat as f32 / seats as f32;
        out.push(Quat::from_rotation_y(turn) * home);
    }
    out
}

/// The rim's points, in the rim's own space, all the way round: its two
/// edges and three points down its slope between them, which are where its
/// throw is furthest; and whether the point is one of the four that sample
/// its width evenly, which the share of it the mask leaves is counted on.
fn rim_points() -> Vec<(Vec3, bool)> {
    let band = shellmat::band(shellmat::RIM_EDGES[0], shellmat::RIM_EDGES[1]);
    band.chunks(2)
        .flat_map(|pair| {
            [
                (0.0, false),
                (0.1, false),
                (0.125, true),
                (0.25, false),
                (0.375, true),
                (0.5, false),
                (0.625, true),
                (0.875, true),
                (1.0, false),
            ]
            .map(|(f, even)| (pair[0].at.lerp(pair[1].at, f), even))
        })
        .collect()
}

/// What one sweep found.
#[derive(Default, Debug)]
struct Sweep {
    standing: usize,
    lying: usize,
    /// For domes: how many stood at each step.
    steps: [usize; shellmat::DOME_STEPS.len()],
    /// Rim points drawn over another card's print: where, and on what.
    trespass: Vec<String>,
    /// Of the standing rims' points, how many the mask leaves more than
    /// half of, out of how many.
    seen: (usize, usize),
}

/// Stands or lays every card's rim as `fit_the_shells` would, then follows
/// the ray from `eye` through every point of every standing rim down to
/// every card's face below it, and records each that lands on a print.
fn sweep(placed: &[Placement], hovered: &[bool], eye: Vec3, found: &mut Sweep) {
    let poses: Vec<Transform> = placed
        .iter()
        .zip(hovered)
        .map(|(p, &h)| pose(p, h))
        .collect();
    let faces: Vec<Footprint> = poses.iter().map(Footprint::of).collect();
    let table = posed(&poses);
    let points = rim_points();
    for (i, at) in poses.iter().enumerate() {
        let others = faces
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, f)| *f);
        if !shellmat::rim_stands(&faces[i], others, eye, 0.0) {
            found.lying += 1;
            continue;
        }
        found.standing += 1;
        let rim = at.to_matrix() * Mat4::from_translation(Vec3::Z * CARD_THICKNESS);
        let cam = rim.inverse().transform_point3(eye);
        for &(p, even) in &points {
            let clear = shellmat::clear_over_print(cam, p);
            if even {
                found.seen.1 += 1;
                if clear > 0.5 {
                    found.seen.0 += 1;
                }
            }
            if clear == 0.0 {
                continue;
            }
            land(rim.transform_point3(p), i, placed, &table, eye, found);
        }
    }
}

/// A card as it stands for a sweep: its pose, its face's height, and the
/// way into its own space, worked out once a table rather than once a ray.
struct Posed {
    face: f32,
    into: Mat4,
    /// The other cards within reach of anything this card's shells throw.
    near: Vec<usize>,
}

fn posed(poses: &[Transform]) -> Vec<Posed> {
    poses
        .iter()
        .enumerate()
        .map(|(i, at)| Posed {
            face: at.translation.y + CARD_THICKNESS * at.scale.y,
            into: at.to_matrix().inverse(),
            near: (0..poses.len())
                .filter(|&j| j != i && poses[j].translation.distance(at.translation) <= 3.0)
                .collect(),
        })
        .collect()
}

/// Follows the ray from `eye` through `world`, a point of card `i`'s shell,
/// down to the face of every other card it stands above, and records it if
/// it lands on that card's print. Card `i`'s own print, where the ray
/// crosses it, hides the point if it is in front of it, and whatever lies
/// under it if it is behind.
fn land(
    world: Vec3,
    i: usize,
    placed: &[Placement],
    table: &[Posed],
    eye: Vec3,
    found: &mut Sweep,
) {
    let own = &table[i];
    let crossing = eye + (world - eye) * ((eye.y - own.face) / (eye.y - world.y));
    let through_own = shellmat::card_sdf(own.into.transform_point3(crossing).truncate()) < 0.0;
    if through_own && world.y < own.face {
        return;
    }
    for &j in &own.near {
        let other = &table[j];
        let face = other.face;
        if world.y <= face || (through_own && face < own.face) {
            continue;
        }
        let landing = eye + (world - eye) * ((eye.y - face) / (eye.y - world.y));
        let on = other.into.transform_point3(landing);
        if shellmat::card_sdf(on.truncate()) < -1e-4 {
            found.trespass.push(format!(
                "shell of {:?} at {world} lands {:.4} inside {:?}",
                placed[i].object,
                -shellmat::card_sdf(on.truncate()),
                placed[j].object
            ));
        }
    }
}

/// A dome's points at `step`, in its own space: every vertex of its mesh,
/// and the middle of every run between two rings.
fn dome_points(row: shellmat::DomeRow, step: shellmat::DomeStep) -> Vec<Vec3> {
    let mesh = shellmat::dome_mesh(row, step);
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(at)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("a dome has positions");
    };
    let at: Vec<Vec3> = at.iter().map(|p| Vec3::from_array(*p)).collect();
    let around = (at.len() - 1) / shellmat::DOME_RINGS;
    let mut out = at.clone();
    for k in 0..(shellmat::DOME_RINGS - 1) * around {
        out.push(at[k].lerp(at[k + around], 0.5));
    }
    out
}

/// Stands every card's dome as `fit_the_shells` would at its most
/// permissive, already standing at full height, and follows the ray from
/// `eye` through every point of every standing dome down to every card's
/// face below it. A dome has no mask: all of it counts.
fn sweep_domes(placed: &[Placement], hovered: &[bool], eye: Vec3, found: &mut Sweep) {
    let row = Dome::Hexproof.row();
    let points: Vec<Vec<Vec3>> = shellmat::DOME_STEPS
        .iter()
        .map(|&step| dome_points(row, step))
        .collect();
    let poses: Vec<Transform> = placed
        .iter()
        .zip(hovered)
        .map(|(p, &h)| pose(p, h))
        .collect();
    let faces: Vec<Footprint> = poses.iter().map(Footprint::of).collect();
    let table = posed(&poses);
    for (i, at) in poses.iter().enumerate() {
        let others = faces
            .iter()
            .enumerate()
            .filter(move |(j, _)| *j != i)
            .map(|(_, f)| *f);
        let Some(step) = shellmat::dome_step(row, Some(0), &faces[i], others, eye) else {
            found.lying += 1;
            continue;
        };
        found.standing += 1;
        found.steps[step] += 1;
        let dome = at.to_matrix() * Mat4::from_translation(Vec3::Z * CARD_THICKNESS);
        for &p in &points[step] {
            land(dome.transform_point3(p), i, placed, &table, eye, found);
        }
    }
}

/// The tables the sweeps are taken on: a duel at three windows, including
/// the wide one that leans the camera furthest, with sparse rows,
/// comfortable ones, full ones and fanned ones, and rings of three, four and
/// eight with all but the fanned.
fn tables() -> Vec<(u8, [usize; 3], Vec2)> {
    let mut out = Vec::new();
    for row in [[2, 1, 3], [5, 4, 6], [14, 11, 16], [30, 27, 32]] {
        for window in [
            Vec2::new(800.0, 600.0),
            Vec2::new(1280.0, 800.0),
            Vec2::new(2560.0, 1080.0),
        ] {
            out.push((2, row, window));
        }
        // Every seat at a ring has a duel's board since #264, so its rows
        // fan where a duel's do; the longest rows are a duel's alone, where
        // thirty cards fan at every window and none is scrolled away
        // (measured 25.09.2026: sixteen no longer fan at all).
        if row[0] < 30 {
            for seats in [3, 4, 8] {
                out.push((seats, row, Vec2::new(1920.0, 1080.0)));
            }
        }
    }
    out
}

/// A standing rim never lands on another card's print: from every shot the
/// table is seen from, with and without cards lifted under a hover. And the
/// sweep is not true of nothing: many rims stand, many lie down.
#[test]
fn a_standing_rim_never_lands_on_another_cards_print() {
    let mut found = Sweep::default();
    for (seats, row, window) in tables() {
        let duel = table(seats, row, window);
        let placed = placements(&duel);
        for hovered in [
            vec![false; placed.len()],
            (0..placed.len()).map(|i| i % 5 == 2).collect(),
        ] {
            for eye in eyes(&duel, window) {
                sweep(&placed, &hovered, eye, &mut found);
            }
        }
    }
    assert!(
        found.trespass.is_empty(),
        "{} rim points land on a print, the first: {:?}",
        found.trespass.len(),
        &found.trespass[..found.trespass.len().min(5)]
    );
    assert!(
        found.standing >= 200 && found.lying >= 200,
        "the sweep takes one side only: {} standing, {} lying",
        found.standing,
        found.lying
    );
}

/// The mask leaves a rim to see. It takes away what the card itself hides,
/// the far foot behind the card, and what the rim's top would throw onto the
/// print, and from the table's real shots that leaves most of the rim's
/// width drawn: 0.647 measured when this was written (25.09.2026), against
/// 0.023 with a feather as wide as a card's border.
#[test]
fn the_mask_leaves_a_standing_rim_to_see() {
    let mut found = Sweep::default();
    for (seats, row, window) in tables() {
        let duel = table(seats, row, window);
        let placed = placements(&duel);
        for eye in eyes(&duel, window) {
            sweep(&placed, &vec![false; placed.len()], eye, &mut found);
        }
    }
    let (seen, of) = found.seen;
    #[expect(clippy::cast_precision_loss)] // counts in the thousands
    let share = seen as f32 / of as f32;
    assert!(of > 10_000, "only {of} rim points were looked at");
    assert!(share > 0.6, "the mask leaves {share:.3} of the rim to see");
}

/// A card lying flat with nothing near it stands its rim, and the same card
/// with a neighbour's face lying in the rim's throw lays it down.
#[test]
fn a_rim_lies_down_for_a_neighbour_and_stands_up_without_one() {
    let slot = *TableLayout::new(&[PlayerId::new(0)], 16.0 / 9.0, None)
        .slot(PlayerId::new(0))
        .expect("a one-seat table seats its one player");
    let eye = Vec3::new(0.0, 20.0, 8.0);
    let me = Footprint::of(&card_transform(&slot, Vec2::ZERO, false, 0.004));
    assert!(shellmat::rim_stands(&me, [], eye, 0.0), "alone");
    // Two untapped cards in neighbouring rows of a ring, 0.0185 apart and a
    // row's rise lower: the rim's top throws less than that.
    let behind = Footprint::of(&card_transform(
        &slot,
        slot.forward() * -(CARD_HEIGHT + 0.0185),
        false,
        0.0,
    ));
    assert!(
        shellmat::rim_stands(&me, [behind], eye, 0.0),
        "a ring's rows"
    );
    // The same neighbour lying in a fanned row's overlap.
    let under = Footprint::of(&card_transform(&slot, Vec2::new(0.3, 0.0), false, 0.0));
    assert!(!shellmat::rim_stands(&me, [under], eye, 0.0), "a fan");
    // A card higher than the rim's top hides it: a flier over it.
    let over = Footprint::of(&card_transform(&slot, Vec2::new(0.3, 0.0), false, 0.2));
    assert!(shellmat::rim_stands(&me, [over], eye, 0.0), "under a flier");
    // Hysteresis: a rim that would only just fit does not stand up again.
    let near = Footprint::of(&card_transform(&slot, Vec2::new(1.012, 0.0), false, 0.0));
    assert!(shellmat::rim_stands(&me, [near], eye, 0.0), "just fits");
    assert!(
        !shellmat::rim_stands(&me, [near], eye, shellmat::STAND_AGAIN),
        "stood up again at the edge"
    );
}

/// A standing dome never lands on another card's print: from every shot the
/// table is seen from, every seat's side included, with and without cards
/// lifted under a hover. And the sweep is not true of nothing: domes stand,
/// at more than one height, and domes lie down.
#[test]
fn a_standing_dome_never_lands_on_another_cards_print() {
    let mut found = Sweep::default();
    for (seats, row, window) in tables() {
        let duel = table(seats, row, window);
        let placed = placements(&duel);
        for hovered in [
            vec![false; placed.len()],
            (0..placed.len()).map(|i| i % 5 == 2).collect(),
        ] {
            for eye in eyes(&duel, window) {
                sweep_domes(&placed, &hovered, eye, &mut found);
            }
        }
    }
    assert!(
        found.trespass.is_empty(),
        "{} dome points land on a print, the first: {:?}",
        found.trespass.len(),
        &found.trespass[..found.trespass.len().min(5)]
    );
    assert!(
        found.standing >= 100 && found.lying >= 100,
        "the sweep takes one side only: {} standing, {} lying",
        found.standing,
        found.lying
    );
    assert!(
        found.steps.iter().filter(|&&n| n > 0).count() >= 2,
        "domes stood at one height only: {:?}",
        found.steps
    );
    eprintln!(
        "{} standing {:?}, {} lying",
        found.standing, found.steps, found.lying
    );
}

/// The wall's points, in its own space: every corner of its mesh, which is
/// where a face of it is highest and furthest out.
fn wall_points() -> Vec<Vec3> {
    let mesh = shellmat::wall_mesh();
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(at)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("a wall has positions");
    };
    let mut out: Vec<Vec3> = Vec::new();
    for p in at.iter().map(|p| Vec3::from_array(*p)) {
        if !out.iter().any(|q| q.distance_squared(p) < 1e-10) {
            out.push(p);
        }
    }
    out
}

/// What a wall sweep found.
#[derive(Default, Debug)]
struct WallSweep {
    walls: usize,
    points: usize,
    /// Points some print is in front of, and of those, their own card's.
    hidden: usize,
    hidden_by_own: usize,
    /// Wall points drawn over a print: where, and on what.
    trespass: Vec<String>,
}

/// Stands a wall at every card, where `fit_the_shells` puts one
/// ([`wall_pose`]), and follows the ray from `eye` through every point of
/// every wall to every card's face near it, the wall's own card's included.
/// Where it meets a print past the point, the wall would be drawn over that
/// print; where it meets one before it, that print hides it.
fn sweep_walls(placed: &[Placement], hovered: &[bool], eye: Vec3, found: &mut WallSweep) {
    let points = wall_points();
    let poses: Vec<Transform> = placed
        .iter()
        .zip(hovered)
        .map(|(p, &h)| pose(p, h))
        .collect();
    let faces: Vec<(f32, Mat4)> = poses
        .iter()
        .map(|at| {
            (
                at.translation.y + CARD_THICKNESS * at.scale.y,
                at.to_matrix().inverse(),
            )
        })
        .collect();
    for (i, at) in poses.iter().enumerate() {
        found.walls += 1;
        let wall = at.to_matrix() * wall_pose(at).to_matrix();
        let near: Vec<usize> = (0..poses.len())
            .filter(|&j| poses[j].translation.xz().distance(at.translation.xz()) < 2.5)
            .collect();
        for &p in &points {
            found.points += 1;
            let world = wall.transform_point3(p);
            let mut hidden = false;
            for &j in &near {
                let (face, into) = faces[j];
                let landing = eye + (world - eye) * ((eye.y - face) / (eye.y - world.y));
                let on = into.transform_point3(landing);
                if shellmat::card_sdf(on.truncate()) >= -1e-4 {
                    continue;
                }
                if world.y > face {
                    found.trespass.push(format!(
                        "wall of {:?} at {world} lands {:.4} inside {:?}",
                        placed[i].object,
                        -shellmat::card_sdf(on.truncate()),
                        placed[j].object
                    ));
                } else {
                    hidden = true;
                    if j == i {
                        found.hidden_by_own += 1;
                    }
                }
            }
            if hidden {
                found.hidden += 1;
            }
        }
    }
}

/// Defender's wall never draws over a print: every point of it, stood at
/// every card of every table, hovered and flying ones included, is under
/// the face of every card whose print the ray to it crosses, from every
/// shot the table is seen from, every other seat's included. And the sweep
/// is not true of nothing: prints stand in front of walls, and much of
/// every wall is left to see: measured when this was written (25.09.2026),
/// 0.55 of the points behind some print, 0.010 behind their own card's.
/// Then 0.14 and 0.008, once every seat at a ring was handed a duel's board
/// (#264): the rows at three, four and eight seats stopped overlapping, and
/// a print stands in front of a wall only where a row does. Then 0.17 and
/// 0.008, once the duels were given rows long enough to fan.
#[test]
fn a_wall_never_draws_over_a_print() {
    let mut found = WallSweep::default();
    for (seats, row, window) in tables() {
        let duel = table(seats, row, window);
        let placed = placements(&duel);
        for hovered in [
            vec![false; placed.len()],
            (0..placed.len()).map(|i| i % 5 == 2).collect(),
        ] {
            for eye in eyes(&duel, window) {
                sweep_walls(&placed, &hovered, eye, &mut found);
            }
        }
    }
    assert!(
        found.trespass.is_empty(),
        "{} wall points land on a print, the first: {:?}",
        found.trespass.len(),
        &found.trespass[..found.trespass.len().min(5)]
    );
    assert!(
        found.hidden > 500_000,
        "prints stood in front of walls only {} times",
        found.hidden
    );
    assert!(
        (found.points - found.hidden) * 10 > found.points * 3,
        "prints hide most of every wall: {found:?}"
    );
    assert!(
        found.hidden_by_own * 20 < found.points,
        "walls stand behind their own cards: {found:?}"
    );
}

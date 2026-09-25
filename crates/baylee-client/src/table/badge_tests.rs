//! The count badge on the table (#261): where its quad lies on the card, what
//! it lies over in a fanned row, and that it comes and goes with the count.

use super::flying_tests::{creature, duel};
use super::plate_tests::home;
use super::*;
use baylee_client_core::layout::CARD_SPAN;
use bevy::ecs::world::CommandQueue;

/// The badge's quad is where `cardplate` says, in the card's own space, at
/// either place: over the card, wholly above its top edge; beside it, off
/// its right edge.
///
/// Read back out of the transform and the mesh's size rather than restated,
/// so a flipped axis — a card's `+y` is its top, and its UV's is its bottom —
/// stands the badge at another corner and this goes red.
#[test]
fn the_badge_lies_where_cardplate_puts_it() {
    for place in [BadgePlace::Above, BadgePlace::Beside] {
        let at = badge_transform(0.0, place).translation;
        let size = crate::badgemat::quad_size();
        let (w, h) = (size.x * CARD_WIDTH, size.y * DOWN_THE_CARD);
        let top = (CARD_HEIGHT * 0.5 - (at.y + h * 0.5)) / DOWN_THE_CARD;
        let left = (at.x - w * 0.5) / CARD_WIDTH + 0.5;
        let [x0, y0, x1, y1] = cardplate::badge_quad_rect(place);
        assert!(
            (top - y0).abs() < 1e-5,
            "{place:?}: the badge's quad starts {top} card widths down the card, not at {y0}"
        );
        assert!(
            (left - x0).abs() < 1e-5,
            "{place:?}: the badge's quad starts {left} across the card, not at {x0}"
        );
        match place {
            BadgePlace::Above => assert!(
                top + (y1 - y0) < 0.0,
                "the badge over the card reaches down onto it"
            ),
            BadgePlace::Beside => assert!(
                left + (x1 - x0) > 1.0,
                "the badge beside the card does not hang off it"
            ),
        }
    }
}

/// A row of merged creatures, `n` of them, each standing for two.
fn merged_row(n: usize) -> Vec<Placement> {
    let groups = (0..n)
        .map(|i| {
            let slot = u32::try_from(i + 1).expect("a small row");
            let mut group = creature(slot, Vec::new());
            group.members.push(obj(slot + 1000));
            group
        })
        .collect();
    let mut placed = placements(&duel(groups));
    assert_eq!(placed.len(), n);
    placed.sort_by(|a, b| a.lift.total_cmp(&b.lift));
    placed
}

fn obj(n: u32) -> ObjectId {
    ObjectId::new(n, 0)
}

/// Every badge in a fanned row lies above its own card's face and below the
/// face of the card laid over it.
///
/// Above its own face is also what lays the overhang over the card *before*
/// it, which the row puts lower still: a badge under its own face would be
/// cut off by its neighbour wherever it hangs over it, and its count cut in
/// half. Below the next face keeps it in the strip's place in the transparent
/// pass, however long the row.
#[test]
fn a_badge_lies_on_its_card_and_under_the_next_one() {
    for n in [1usize, 2, 5, 17, 40] {
        let placed = merged_row(n);
        for (i, card) in placed.iter().enumerate() {
            assert_eq!(card.badge, 2, "the placement carries the count");
            let face = card.lift + CARD_THICKNESS;
            let badge = card.lift
                + badge_transform(card.rung, card.slot.badge_place())
                    .translation
                    .z;
            assert!(
                badge > face,
                "in a row of {n}, badge {i} is not above its card"
            );
            if let Some(next) = placed.get(i + 1) {
                let over = next.lift + CARD_THICKNESS;
                assert!(
                    badge < over,
                    "in a row of {n}, badge {i} at {badge} is over the next card's face at {over}"
                );
            }
        }
    }
}

/// A badge over its card stands where it stands on the card untapped while
/// the card taps, and all the way round (the owner, 25.09): tapping moves
/// nothing in a row, and a badge turned with its card would stand beside it,
/// over the next card in a fan.
#[test]
fn a_badge_over_its_card_stays_upright_as_the_card_taps() {
    use bevy::ecs::system::RunSystemOnce;
    let mut card = placements(&crowded(2, 3, |_| false))
        .into_iter()
        .find(|p| p.badge > 0)
        .expect("a merged card");
    assert_eq!(
        card.slot.badge_place(),
        BadgePlace::Above,
        "the premise: a duel"
    );
    let pose = |tapped| card_transform(&card.slot, card.position, tapped, card.lift);
    let (rest, tapped) = (pose(false), pose(true));
    let mut world = World::new();
    let mut index = SceneIndex {
        badge_quad: Some(Handle::default()),
        ..default()
    };
    let mut materials = Assets::<BadgeMaterial>::default();
    let entity = world.spawn(rest).id();
    let mut sync = |world: &mut World, placement: &Placement| {
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, world);
        sync_badge(
            &mut commands,
            &mut index,
            &mut materials,
            entity,
            placement,
            &home(),
        );
        queue.apply(world);
    };
    let badge_at = |world: &mut World| {
        let local = *world
            .query_filtered::<&Transform, With<CountBadge>>()
            .single(world)
            .expect("one badge");
        let card = *world.get::<Transform>(entity).expect("the card");
        card.mul_transform(local)
    };
    sync(&mut world, &card);
    let upright = badge_at(&mut world);
    let same = |a: Transform, b: Transform| {
        a.translation.distance(b.translation) < 1e-4
            && a.rotation.dot(b.rotation).abs() > 1.0 - 1e-6
    };

    card.tapped = true;
    sync(&mut world, &card);
    *world.get_mut::<Transform>(entity).expect("the card") = tapped;
    assert!(
        same(badge_at(&mut world), upright),
        "tapped, the badge turned"
    );

    for share in [0.25, 0.5, 0.9] {
        world
            .get_mut::<Transform>(entity)
            .expect("the card")
            .rotation = rest.rotation.slerp(tapped.rotation, share);
        world
            .run_system_once(keep_upright)
            .expect("the system runs");
        assert!(
            same(badge_at(&mut world), upright),
            "{share} of the way round, the badge turned"
        );
    }
}

/// The badge goes on with a count, changes with it, comes off when the
/// group is one card again, and a count is one material however many cards
/// say it.
#[test]
fn a_badge_comes_and_goes_with_the_count() {
    let mut world = World::new();
    let mut materials = Assets::<BadgeMaterial>::default();
    let mut index = SceneIndex {
        badge_quad: Some(Handle::default()),
        ..default()
    };
    let mut placed = merged_row(2);
    let cards = [world.spawn_empty().id(), world.spawn_empty().id()];

    let mut sync = |world: &mut World, index: &mut SceneIndex, placement: &Placement, card| {
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, world);
        sync_badge(
            &mut commands,
            index,
            &mut materials,
            card,
            placement,
            &home(),
        );
        queue.apply(world);
    };
    let badges = |world: &mut World| {
        world
            .query_filtered::<&ChildOf, With<CountBadge>>()
            .iter(world)
            .map(ChildOf::parent)
            .collect::<Vec<_>>()
    };

    for (placement, card) in placed.iter().zip(cards) {
        sync(&mut world, &mut index, placement, card);
    }
    assert_eq!(
        badges(&mut world),
        cards.to_vec(),
        "one badge on each merged card"
    );
    assert_eq!(
        index.badge_materials.len(),
        1,
        "two cards saying ×2 are one material"
    );

    // Nothing moved: nothing is spawned again.
    sync(&mut world, &mut index, &placed[0], cards[0]);
    assert_eq!(
        badges(&mut world).len(),
        2,
        "a card that did not change got a second badge"
    );

    // A third creature joins the first group.
    placed[0].badge = 3;
    sync(&mut world, &mut index, &placed[0], cards[0]);
    assert_eq!(
        badges(&mut world).len(),
        2,
        "a new count spawned a second badge on the card"
    );
    assert_eq!(index.badge_materials.len(), 2);

    // And the second group is one card again.
    placed[1].badge = 0;
    sync(&mut world, &mut index, &placed[1], cards[1]);
    assert_eq!(
        badges(&mut world),
        vec![cards[0]],
        "a lone card kept its badge"
    );
    assert!(!index.badges.contains_key(&placed[1].object));
}

/// A table of `seats` chairs, every lane of every seat holding `n`
/// creatures, every third of them (the first included) merged of two and
/// the other two each with a card tucked under it (#305), tapped where
/// `tapped` says by their place in the row: so a row has cards that fan,
/// merged cards that hold their cells between them, and on either side of
/// each a host that never merges, with a print peeking out past it.
fn crowded(seats: u8, n: usize, tapped: fn(usize) -> bool) -> Duel {
    use baylee_client_core::board::{BoardModel, Individual, Lane, SeatPod};
    use baylee_client_core::layout::LaneKind;
    let players: Vec<PlayerId> = (0..seats).map(PlayerId::new).collect();
    let mut slot = 0;
    let mut group = |i: usize| {
        slot += 1;
        let mut group = creature(slot, Vec::new());
        if i.is_multiple_of(3) {
            group.members.push(obj(slot + 100_000));
        }
        if !i.is_multiple_of(3) {
            let mut under = creature(slot + 200_000, Vec::new());
            under.individual = Some(Individual::Attached);
            group.individual = Some(Individual::HasAttachments);
            group.attached.push(under);
        }
        if tapped(i) {
            group.status = baylee_view::ObjectStatus::TAPPED;
        }
        group
    };
    let pods = players
        .iter()
        .map(|&player| SeatPod {
            player,
            life: 20,
            poison: 0,
            energy: 0,
            hand_count: 0,
            library_count: 40,
            graveyard_count: 0,
            has_lost: false,
            is_local: player == PlayerId::new(0),
            is_active: player == PlayerId::new(0),
            is_awaited: player == PlayerId::new(0),
            role: baylee_client_core::board::SeatRole::Present,
            lanes: LaneKind::ALL
                .iter()
                .map(|&kind| Lane {
                    kind,
                    groups: (0..n).map(&mut group).collect(),
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

/// A rectangle on the card, `[x0, y0, x1, y1]` in card widths from its
/// top-left corner, `y` down the card, as four corners on the table.
fn on_table(card: &Transform, [x0, y0, x1, y1]: [f32; 4]) -> [Vec2; 4] {
    [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].map(|(x, y)| {
        let local = Vec3::new(
            (x - 0.5) * CARD_WIDTH,
            CARD_HEIGHT * 0.5 - y * DOWN_THE_CARD,
            CARD_THICKNESS,
        );
        let world = card.transform_point(local);
        Vec2::new(world.x, -world.z)
    })
}

/// Whether two convex quads on the table overlap by more than a hair: no
/// edge of either separates them.
fn overlap(a: &[Vec2; 4], b: &[Vec2; 4]) -> bool {
    let reach = |quad: &[Vec2; 4], axis: Vec2| {
        quad.iter()
            .map(|p| p.dot(axis))
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), d| {
                (lo.min(d), hi.max(d))
            })
    };
    [a, b].into_iter().all(|quad| {
        (0..4).all(|i| {
            let edge = quad[(i + 1) % 4] - quad[i];
            let axis = Vec2::new(-edge.y, edge.x).normalize();
            let (a_lo, a_hi) = reach(a, axis);
            let (b_lo, b_hi) = reach(b, axis);
            a_hi > b_lo + 1e-4 && b_hi > a_lo + 1e-4
        })
    })
}

/// How a test row is tapped: named, and by a card's place in its row.
type Tap = (&'static str, fn(usize) -> bool);

/// A badge's quad on the table, with the card it lies on.
type Laid = (Entity, [Vec2; 4]);

/// Every card of `placed` at its resting pose with its badge on it, as the
/// scene spawns them: each card's pose and entity, and each badge's quad on
/// the table.
fn laid_badges(placed: &[Placement]) -> (Vec<Transform>, Vec<Entity>, Vec<Laid>) {
    let mut world = World::new();
    let mut index = SceneIndex {
        badge_quad: Some(Handle::default()),
        ..default()
    };
    let mut materials = Assets::<BadgeMaterial>::default();
    let poses: Vec<Transform> = placed
        .iter()
        .map(|p| card_transform(&p.slot, p.position, p.tapped, p.lift))
        .collect();
    let mut cards = Vec::new();
    for (placement, pose) in placed.iter().zip(&poses) {
        let card = world.spawn(*pose).id();
        cards.push(card);
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        sync_badge(
            &mut commands,
            &mut index,
            &mut materials,
            card,
            placement,
            &home(),
        );
        queue.apply(&mut world);
    }
    let size = crate::badgemat::quad_size();
    let half = Vec2::new(size.x * CARD_WIDTH, size.y * DOWN_THE_CARD) * 0.5;
    let laid = world
        .query_filtered::<(&ChildOf, &Transform), With<CountBadge>>()
        .iter(&world)
        .map(|(parent, local)| {
            // The quad in the badge's own space, laid on the card by the
            // badge's transform and not by `badge_rect`.
            let card = world.get::<Transform>(parent.parent()).expect("a card");
            let at = card.mul_transform(*local);
            let corners = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(sx, sy)| {
                let w = at.transform_point((half * Vec2::new(sx, sy)).extend(0.0));
                Vec2::new(w.x, -w.z)
            });
            (parent.parent(), corners)
        })
        .collect();
    (poses, cards, laid)
}

/// No count badge lies on another card's print (#274; the owner, 25.09),
/// wherever the table puts the cards.
///
/// The print fills the card since #298 and the badge stands at the card's
/// top-right corner, outside it: over the card in a duel, whose rows leave
/// it the felt, upright however the card turns; beside its right edge at a
/// ring, turning with the card, where the merged card holds its cell whole
/// in its row (`layout::HELD_PITCH`), so the next card does not reach into
/// it. A row that cannot fan legibly scrolls rather than packing tighter:
/// only the run it shows is drawn, and only that run is laid against.
///
/// Every badge is spawned as the scene spawns it, on its card at the card's
/// resting pose, and each badge's quad, shadow and all, is laid against
/// every other drawn card on the table, the print being the whole card. A
/// duel and a ring of eight; rows of two to seventy-two in all three lanes,
/// so the tightest fan (a third of a card, asserted) and rows that scroll
/// are there, each scrolled to its start, a few cards in and past its end;
/// untapped, all tapped and every other one tapped; a card tucked under
/// the cards on both sides of every merged one, which never merge and so
/// carry no badge themselves, peeking out where a badge above the top edge
/// stands (#305; the row holds the cell after a merged card for it,
/// `Lane::gaps`); and a creature staged into combat beside a merged one,
/// half a card forward of its row, which is where a badge above the top
/// edge once failed.
#[test]
fn no_badge_lies_on_another_cards_print() {
    use baylee_client_core::layout::{MIN_VISIBLE_FRACTION, STAGE_STEP};
    use baylee_client_core::rowscroll::ROW_STEP;
    let taps: [Tap; 3] = [
        ("untapped", |_| false),
        ("tapped", |_| true),
        ("every other tapped", |i| i % 2 == 1),
    ];
    let (mut count, mut hidden, mut tightest) = (Tally::default(), 0, f32::INFINITY);
    let mut plates = 0;
    for seats in [2u8, 8] {
        for n in [2usize, 3, 4, 5, 6, 8, 10, 13, 17, 24, 32, 40, 72] {
            for (tap, tapped) in taps {
                for (staged, first) in [(false, 0), (true, 0), (false, 3), (false, 1000)] {
                    let mut duel = crowded(seats, n, tapped);
                    let rows: Vec<_> = duel
                        .board
                        .as_ref()
                        .expect("a board")
                        .pods
                        .iter()
                        .flat_map(|pod| pod.lanes.iter().map(move |lane| (pod.player, lane.kind)))
                        .collect();
                    #[allow(clippy::cast_precision_loss)] // a few cards
                    let travel = first as f32 * ROW_STEP;
                    for row in rows {
                        let Duel {
                            rows,
                            board,
                            layout,
                            ..
                        } = &mut duel;
                        rows.wheel(
                            board.as_ref().expect("a board"),
                            layout.as_ref().expect("a layout"),
                            row,
                            travel,
                        );
                    }
                    let all = placements(&duel);
                    assert_eq!(
                        all.len(),
                        usize::from(seats) * 3 * (2 * n - n.div_ceil(3)),
                        "three rows a seat, a card under two of every three"
                    );
                    hidden += all.iter().filter(|p| !p.shown).count();
                    let mut placed: Vec<Placement> = all.into_iter().filter(|p| p.shown).collect();
                    // Neighbours in their row: what is tucked under a
                    // card is drawn between it and the next.
                    let rowed: Vec<&Placement> = placed
                        .iter()
                        .filter(|p| p.object.slot() < 200_000)
                        .collect();
                    for pair in rowed.windows(2) {
                        if pair[0].slot.player == pair[1].slot.player
                            && (pair[0].position - pair[1].position)
                                .dot(pair[0].slot.forward())
                                .abs()
                                < 1e-3
                        {
                            tightest = tightest.min(pair[0].position.distance(pair[1].position));
                        }
                    }
                    let table = format!(
                        "{seats} seats, rows of {n}, {tap}{}, from card {first}",
                        if staged { ", one staged" } else { "" }
                    );
                    // Before the hand-made step below, which moves a card
                    // without the room its row would leave it staged.
                    plates += lay_plates_against_the_tucked(&placed, &table);
                    // The first card of the local seat's creature row, which
                    // is merged, staged out of its group as the board model
                    // does for a declared attacker, and forward of the row.
                    if staged && let Some(p) = placed.iter_mut().find(|p| p.object == obj(1)) {
                        let forward = p.slot.forward();
                        p.position += forward * STAGE_STEP;
                        p.badge = 0;
                    }
                    lay_against_the_prints(&placed, &table, &mut count);
                }
            }
        }
    }
    assert!(
        (tightest - CARD_WIDTH * MIN_VISIBLE_FRACTION).abs() < 1e-4,
        "the rows never reach the tightest fan: {tightest}"
    );
    assert!(
        hidden > 1000,
        "only {hidden} cards were scrolled out of view"
    );
    let Tally {
        badges,
        pairs,
        above,
        beside,
    } = count;
    assert!(
        badges > 5_000 && pairs > badges,
        "{badges} badges, {pairs} pairs"
    );
    assert!(
        above > 1_000 && beside > 1_000,
        "{above} badges over their cards and {beside} beside them"
    );
    assert!(plates > 10_000, "only {plates} plates beside a tucked card");
}

/// What laying tables' badges against the prints counted: badges, badge and
/// card pairs, and how many badges stood over their cards and beside them.
#[derive(Default)]
struct Tally {
    badges: usize,
    pairs: usize,
    above: usize,
    beside: usize,
}

/// Spawns every badge of `placed` as the scene does and lays each one's
/// quad against every other drawn card, the print being the whole card:
/// it lies on none. `table` names the table in a failure.
fn lay_against_the_prints(placed: &[Placement], table: &str, count: &mut Tally) {
    let (poses, cards, laid) = laid_badges(placed);
    let whole = [0.0, 0.0, 1.0, cardrail::CARD_TALL];
    let bodies: Vec<[Vec2; 4]> = poses.iter().map(|pose| on_table(pose, whole)).collect();
    for (card, badge) in &laid {
        count.badges += 1;
        let own = cards
            .iter()
            .position(|c| c == card)
            .expect("a badge on a card");
        match placed[own].slot.badge_place() {
            BadgePlace::Above => count.above += 1,
            BadgePlace::Beside => count.beside += 1,
        }
        for (other, body) in bodies.iter().enumerate() {
            if own == other {
                continue;
            }
            count.pairs += 1;
            assert!(
                !overlap(badge, body),
                "{table}: the badge of {:?} lies on {:?}",
                placed[own].object,
                placed[other].object
            );
        }
    }
}

/// Lays every row card's plate, shadow and all, where `plate_rect` puts it
/// from the room its row leaves it, against what shows of every card tucked
/// under another (#305): it lies on none. What shows is the strip between
/// the host's front edge and the tucked card's; the rest lies under the
/// host, which lies over any plate before it in the row. The room ends at
/// the next card's left edge, and a tucked card stands where its host does
/// along the row, turned with it, so the strip starts no further left.
/// Returns how many plates were laid beside a tucked card.
fn lay_plates_against_the_tucked(placed: &[Placement], table: &str) -> usize {
    let whole = [0.0, 0.0, 1.0, cardrail::CARD_TALL];
    let print = |p: &Placement| {
        on_table(
            &card_transform(&p.slot, p.position, p.tapped, p.lift),
            whole,
        )
    };
    let strips: Vec<(&Placement, [Vec2; 4])> = placed
        .iter()
        .filter(|t| t.object.slot() > 200_000)
        .map(|t| {
            let host = placed
                .iter()
                .find(|h| h.object.slot() == t.object.slot() - 200_000)
                .expect("a tucked card's host is drawn");
            let forward = host.slot.forward();
            // The host's front edge, and the same edge moved as far as the
            // tucked card lies past it.
            let mut corners = print(host);
            corners.sort_by(|x, y| y.dot(forward).total_cmp(&x.dot(forward)));
            let (a, b) = (corners[0], corners[1]);
            let past = t.position - host.position;
            (t, [a, b, b + past, a + past])
        })
        .collect();
    let mut laid = 0;
    for p in placed.iter().filter(|p| p.object.slot() < 200_000) {
        let badge = (p.badge > 0 && p.tapped && p.slot.badge_place() == BadgePlace::Beside)
            .then(|| cardplate::badge_quad_rect(BadgePlace::Beside));
        let Some((body, _)) = cardplate::plate_rect(cardplate::KIND_FIGHT, p.tapped, p.room, badge)
        else {
            continue;
        };
        // In the frame of the card lying untapped, which is the plate's.
        let flat = card_transform(&p.slot, p.position, false, p.lift);
        let plate = on_table(&flat, cardplate::plate_quad(body));
        for (t, strip) in &strips {
            if t.slot.player != p.slot.player {
                continue;
            }
            laid += 1;
            assert!(
                !overlap(&plate, strip),
                "{table}: the plate of {:?} lies on what shows of {:?}",
                p.object,
                t.object
            );
        }
    }
    laid
}

/// Every card a row draws stands inside its lane, however many the row
/// holds (#298): a row that does not fit, merged cells held whole, scrolls
/// rather than running on past its lane into the piles beside it, which is
/// where forty distinct creatures used to stand. And the rows that do not
/// fit are there: cards are left out of the picture.
#[test]
fn every_drawn_card_stands_inside_its_lane() {
    let mut hidden = 0;
    for seats in [2u8, 3, 4, 8] {
        for n in [5usize, 17, 40, 70] {
            let duel = crowded(seats, n, |i| i % 2 == 1);
            let board = duel.board.as_ref().expect("a board");
            let row: HashMap<ObjectId, baylee_client_core::layout::LaneKind> = board
                .pods
                .iter()
                .flat_map(|pod| {
                    pod.lanes.iter().flat_map(|lane| {
                        lane.groups
                            .iter()
                            .map(move |g| (g.representative, lane.kind))
                    })
                })
                .collect();
            for p in placements(&duel) {
                let Some(&kind) = row.get(&p.object) else {
                    continue;
                };
                if !p.shown {
                    hidden += 1;
                    continue;
                }
                let along = Vec2::new(p.slot.facing.cos(), -p.slot.facing.sin());
                let off = (p.position - p.slot.lane_center(kind)).dot(along);
                assert!(
                    off.abs() + CARD_SPAN * 0.5 <= p.slot.half_extent.x + 1e-3,
                    "{seats} seats, rows of {n}: {:?} stands {off} along a lane {} wide",
                    p.object,
                    p.slot.lane_width()
                );
            }
        }
    }
    assert!(hidden > 500, "only {hidden} cards were left out");
}

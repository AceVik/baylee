use super::*;
use baylee_client_core::board::FannedCard;
use baylee_client_core::cardplate::Tally;
use baylee_client_core::{BoardModel, Lane, LaneKind, PileKind, SeatPod, ZonePile};

/// What one seat's counted places hold: cards in library, graveyard and
/// exile pile and in hand, and the time counters of its suspended cards,
/// the one that comes back next first.
#[derive(Clone, Copy, Default)]
struct Holds {
    library: u32,
    graveyard: u32,
    exile: u32,
    hand: u32,
    waiting: &'static [u16],
    /// What each command slot's commander costs more.
    taxes: [u32; 2],
}

/// One seat's pod holding `holds`. The waiting place is there only while a
/// card waits, as the board model has it.
fn pod(player: PlayerId, holds: Holds) -> SeatPod {
    SeatPod {
        player,
        life: 20,
        poison: 0,
        energy: 0,
        hand_count: holds.hand,
        library_count: holds.library,
        graveyard_count: holds.graveyard,
        has_lost: false,
        is_local: player == PlayerId::new(0),
        is_active: true,
        is_awaited: true,
        role: baylee_client_core::board::SeatRole::Present,
        lanes: vec![Lane {
            kind: LaneKind::Creatures,
            groups: Vec::new(),
        }],
        piles: PileKind::ALL
            .into_iter()
            .filter(|kind| *kind != PileKind::Waiting || !holds.waiting.is_empty())
            .map(|kind| {
                let mut pile = ZonePile::empty(kind);
                pile.count = match kind {
                    PileKind::Library => holds.library,
                    PileKind::Graveyard => holds.graveyard,
                    PileKind::Exile => holds.exile,
                    PileKind::Waiting => holds.waiting.len() as u32,
                    _ => 0,
                };
                pile.tax = match kind {
                    PileKind::Command => Some(holds.taxes[0]),
                    PileKind::Command2 => Some(holds.taxes[1]),
                    _ => None,
                };
                if kind == PileKind::Waiting {
                    pile.fan = (0u32..)
                        .zip(holds.waiting)
                        .take(ZonePile::FAN_MAX)
                        .map(|(i, time)| FannedCard {
                            object: baylee_core::ids::ObjectId::new(900 + i, 0),
                            art: None,
                            name: String::new(),
                            time: Some(*time),
                        })
                        .collect();
                }
                pile
            })
            .collect(),
        tokens: Vec::new(),
        threat: baylee_client_core::ThreatSummary::default(),
    }
}

/// Whether the last frame wrote the scene's index, or a badge's material.
#[derive(Resource, Default)]
struct Wrote {
    index: bool,
    materials: bool,
}

fn watch(index: Res<SceneIndex>, materials: Res<Assets<BadgeMaterial>>, mut wrote: ResMut<Wrote>) {
    wrote.index = index.is_changed();
    wrote.materials = materials.is_changed();
}

/// An app with `seats` at the table, each with a zone already built where
/// its slot is (which is `sync_zones`' work and needs a renderer's assets),
/// and the one handle the system refuses to draw without.
fn running(seats: u8, holds: Holds) -> App {
    let players: Vec<PlayerId> = (0..seats).map(PlayerId::new).collect();
    let pods: Vec<SeatPod> = players.iter().map(|player| pod(*player, holds)).collect();
    let duel = Duel {
        board: Some(BoardModel {
            seq: 1,
            local: PlayerId::new(0),
            turn: 1,
            step: baylee_view::Step::Main,
            pods: pods.clone(),
            stack: Vec::new(),
            hand: Vec::new(),
        }),
        layout: Some(TableLayout::new(&players, 1.78, None)),
        ..Duel::default()
    };
    let mut index = SceneIndex {
        badge_quad: Some(Handle::default()),
        ..SceneIndex::default()
    };
    for pod in &pods {
        let slot = *duel.drawn_slot(pod.player).expect("every seat is drawn");
        index.zones.insert(
            pod.player,
            Zone {
                slot,
                raised: 0.0,
                mat: Entity::PLACEHOLDER,
                glow: Entity::PLACEHOLDER,
                mat_material: Handle::default(),
                glow_material: Handle::default(),
                piles: Vec::new(),
                counts: Vec::new(),
                library_shown: 0,
                places: places_of(&pod.piles),
                mood: Mood::of(pod),
                accent: seat_accent(&slot),
            },
        );
    }
    let mut app = App::new();
    app.insert_resource(duel)
        .insert_resource(index)
        .init_resource::<ShownRig>()
        .init_resource::<Assets<BadgeMaterial>>()
        .init_resource::<Wrote>()
        .add_systems(Update, (sync_pile_counts, watch).chain());
    app.update();
    app
}

/// What the numbers on the table say: each tally, which of its numbers, and
/// the number, in the tallies' own order.
fn said(app: &mut App) -> Vec<(Tally, u8, u32)> {
    let mut out: Vec<(Tally, u8, u32)> = app
        .world_mut()
        .query::<(&PileCount, &MeshMaterial3d<BadgeMaterial>)>()
        .iter(app.world())
        .map(|(count, material)| {
            let material = app
                .world()
                .resource::<Assets<BadgeMaterial>>()
                .get(&material.0)
                .expect("a badge has its material");
            let word = material.params.count;
            assert_eq!(
                word,
                cardplate::pile_word(
                    word & cardplate::COUNT_MAX.next_power_of_two().wrapping_sub(1),
                    count.tally
                ),
                "{:?} does not wear its own mark",
                count.tally
            );
            (
                count.tally,
                count.nth,
                word & (cardplate::COUNT_MAX.next_power_of_two() - 1),
            )
        })
        .collect();
    out.sort_by_key(|(tally, nth, _)| (tally.mark(), *nth));
    assert!(
        out.windows(2)
            .all(|pair| pair[0].0 != pair[1].0 || pair[0].1 != pair[1].1),
        "a number is on the table twice"
    );
    out
}

fn set_holds(app: &mut App, holds: Holds) {
    let mut duel = app.world_mut().resource_mut::<Duel>();
    let board = duel.board.as_mut().expect("a board");
    for old in &mut board.pods {
        *old = pod(old.player, holds);
    }
}

/// A library and a hand say how many from nought: an empty library is the
/// fact that loses a game, and an empty hand is one the table plays on. A
/// graveyard and exile say it from their first card and say nothing while
/// they are bare table.
#[test]
fn a_library_and_a_hand_always_wear_their_number_and_the_others_from_their_first_card() {
    let mut app = running(1, Holds::default());
    assert_eq!(
        said(&mut app),
        [(Tally::Library, 0, 0), (Tally::Hand, 0, 0)]
    );

    set_holds(
        &mut app,
        Holds {
            library: 53,
            graveyard: 2,
            exile: 1,
            hand: 7,
            waiting: &[],
            taxes: [0, 0],
        },
    );
    app.update();
    assert_eq!(
        said(&mut app),
        [
            (Tally::Library, 0, 53),
            (Tally::Graveyard, 0, 2),
            (Tally::Exile, 0, 1),
            (Tally::Hand, 0, 7),
        ]
    );

    // A graveyard emptied again is bare table again.
    set_holds(
        &mut app,
        Holds {
            library: 52,
            graveyard: 0,
            exile: 1,
            hand: 8,
            waiting: &[],
            taxes: [0, 0],
        },
    );
    app.update();
    assert_eq!(
        said(&mut app),
        [
            (Tally::Library, 0, 52),
            (Tally::Exile, 0, 1),
            (Tally::Hand, 0, 8)
        ]
    );
}

/// Exile's number is the zone's, its waiting cards among it; and a waiting
/// place wears the time counters of its top three cards, the one that comes
/// back next first. Nothing waits, nothing is said.
#[test]
fn exile_counts_its_waiting_cards_and_they_wear_their_time() {
    let mut app = running(
        1,
        Holds {
            library: 40,
            exile: 2,
            waiting: &[1, 4, 4, 9],
            ..Holds::default()
        },
    );
    assert_eq!(
        said(&mut app),
        [
            (Tally::Library, 0, 40),
            (Tally::Exile, 0, 6),
            (Tally::Hand, 0, 0),
            (Tally::Waiting, 0, 1),
            (Tally::Waiting, 1, 4),
            (Tally::Waiting, 2, 4),
        ]
    );
    // Only suspended cards in exile: the zone still counts them, beside a
    // pile's place that holds none.
    set_holds(
        &mut app,
        Holds {
            library: 40,
            waiting: &[3],
            ..Holds::default()
        },
    );
    app.update();
    assert_eq!(
        said(&mut app),
        [
            (Tally::Library, 0, 40),
            (Tally::Exile, 0, 1),
            (Tally::Hand, 0, 0),
            (Tally::Waiting, 0, 3),
        ]
    );
    set_holds(
        &mut app,
        Holds {
            library: 40,
            ..Holds::default()
        },
    );
    app.update();
    assert_eq!(
        said(&mut app),
        [(Tally::Library, 0, 40), (Tally::Hand, 0, 0)]
    );
}

/// A fan opens over the felt between its pile and the mat. Where the
/// reader sits a pile's numbers lie outside it and stay; at a seat seen
/// from the side they lie under the fan, and a fanned pile's step aside for
/// as long as it is open. Every other number stays.
#[test]
fn a_fanned_piles_numbers_step_aside_where_they_lie_under_its_fan() {
    let mut app = running(
        4,
        Holds {
            library: 40,
            graveyard: 3,
            waiting: &[1, 4],
            ..Holds::default()
        },
    );
    let worn = |app: &mut App, player: PlayerId, tally: Tally| {
        app.world_mut()
            .query::<&PileCount>()
            .iter(app.world())
            .filter(|count| count.player == player && count.tally == tally)
            .count()
    };
    let fan = |app: &mut App, player: PlayerId, kind: PileKind| {
        let mut index = app.world_mut().resource_mut::<SceneIndex>();
        index.fanned.clear();
        index.fanned.insert(
            baylee_core::ids::ObjectId::new(900, 0),
            (Place::Exile(player), player, kind),
        );
    };
    let turn = upright(&CameraRig::default().eye());
    let mine = PlayerId::new(0);
    let flank = (1..4)
        .map(PlayerId::new)
        .find(|player| {
            let index = app.world().resource::<SceneIndex>();
            !beside(&index.zones[player].slot, turn)
        })
        .expect("a ring of four has a seat seen from the side");
    assert_eq!(worn(&mut app, mine, Tally::Waiting), 2, "a column");
    assert_eq!(worn(&mut app, flank, Tally::Waiting), 1, "the top card's");

    fan(&mut app, mine, PileKind::Waiting);
    app.update();
    assert_eq!(
        worn(&mut app, mine, Tally::Waiting),
        2,
        "outside its pile, the fan does not reach them"
    );

    fan(&mut app, flank, PileKind::Waiting);
    app.update();
    assert_eq!(worn(&mut app, flank, Tally::Waiting), 0, "under the fan");
    assert_eq!(worn(&mut app, flank, Tally::Graveyard), 1, "another pile's");
    assert_eq!(worn(&mut app, flank, Tally::Hand), 1);

    fan(&mut app, flank, PileKind::Graveyard);
    app.update();
    assert_eq!(worn(&mut app, flank, Tally::Graveyard), 0, "under the fan");
    assert_eq!(worn(&mut app, flank, Tally::Waiting), 1, "the fan closed");

    app.world_mut().resource_mut::<SceneIndex>().fanned.clear();
    app.update();
    assert_eq!(worn(&mut app, flank, Tally::Graveyard), 1, "it is back");
}

/// A command slot wears what its commander costs more to cast from there,
/// from the first cast and each partner's its own. Before the first cast it
/// wears nothing: the commander costs what is printed on it.
#[test]
fn a_command_slot_wears_its_commanders_tax_from_the_first_cast() {
    let holds = |taxes| Holds {
        library: 40,
        taxes,
        ..Holds::default()
    };
    let mut app = running(1, holds([0, 0]));
    let rest = [(Tally::Library, 0, 40), (Tally::Hand, 0, 0)];
    assert_eq!(said(&mut app), rest);
    set_holds(&mut app, holds([2, 0]));
    app.update();
    assert_eq!(said(&mut app), [rest[0], rest[1], (Tally::Tax, 0, 2)]);
    set_holds(&mut app, holds([4, 6]));
    app.update();
    assert_eq!(
        said(&mut app),
        [rest[0], rest[1], (Tally::Tax, 0, 4), (Tally::Tax2, 0, 6)]
    );
}

/// At rest nothing is written: not the index, not a material. The first
/// frame is the counter-arm, since a watch that never saw a write would
/// pass the rest of this.
#[test]
fn a_table_at_rest_rewrites_no_count() {
    let holds = Holds {
        library: 40,
        graveyard: 3,
        hand: 7,
        waiting: &[2, 5],
        ..Holds::default()
    };
    let mut app = running(4, holds);
    {
        let wrote = app.world().resource::<Wrote>();
        assert!(
            wrote.index && wrote.materials,
            "laying the counts wrote nothing"
        );
    }
    for _ in 0..3 {
        app.update();
        let wrote = app.world().resource::<Wrote>();
        assert!(!wrote.index, "a frame at rest wrote the scene's index");
        assert!(!wrote.materials, "a frame at rest wrote a badge's material");
    }
    // And one card drawn is one write, of the material it changes.
    set_holds(
        &mut app,
        Holds {
            library: 39,
            hand: 8,
            ..holds
        },
    );
    app.update();
    let wrote = app.world().resource::<Wrote>();
    assert!(wrote.index && wrote.materials);
}

/// The corners of the body of a number laid at `number`, in the world.
fn body_corners(number: &Transform, tally: Tally, count: u32) -> [Vec3; 4] {
    let [x0, y0, x1, y1] = cardplate::pile_badge_rect(cardplate::pile_word(count, tally));
    let quad = on_card(0.0, cardplate::pile_badge_quad_rect()).translation;
    [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].map(|(x, y)| {
        let corner = on_card(0.0, [x, y, x, y]).translation - quad;
        // The mesh is stretched along its length to the pile quad's.
        number.transform_point(Vec3::new(corner.x / quad_stretch(), corner.y, 0.0))
    })
}

/// A convex quad on the table, from corners in the world.
fn on_table(corners: [Vec3; 4]) -> [Vec2; 4] {
    corners.map(|corner| Vec2::new(corner.x, -corner.z))
}

/// Whether two convex quads overlap: the separating-axis test, written out.
fn overlap(a: [Vec2; 4], b: [Vec2; 4]) -> bool {
    for quad in [a, b] {
        for i in 0..4 {
            let edge = quad[(i + 1) % 4] - quad[i];
            let axis = Vec2::new(-edge.y, edge.x);
            let span = |q: [Vec2; 4]| {
                q.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| {
                    let d = p.dot(axis);
                    (lo.min(d), hi.max(d))
                })
            };
            let ((a0, a1), (b0, b1)) = (span(a), span(b));
            if a1 <= b0 + 1e-4 || b1 <= a0 + 1e-4 {
                return false;
            }
        }
    }
    true
}

/// A seat with every number to wear, and its three piles as tall as a pile
/// is drawn: the most felt hidden behind them.
const FULL: Holds = Holds {
    library: 40,
    graveyard: 40,
    exile: 40,
    hand: 7,
    waiting: &[1, 3, 10],
    taxes: [2, 14],
};

/// The numbers every seat of `layout` wears with a full set of places,
/// laid for the home view of a window: whose, which, and its plate's
/// corners in the world.
fn laid_for(layout: &TableLayout, window: Vec2) -> Vec<(PlayerId, Tally, u8, [Vec3; 4])> {
    let rig = CameraRig::home(layout, Canvas::hud(window));
    let reader = Reader::of(rig, Some(window));
    let mut out = Vec::new();
    for slot in layout.slots.iter().filter(|slot| !slot.parked) {
        let seat = pod(slot.player, FULL);
        let open = open_left(slot, layout.slots.iter());
        let numbers = numbers_of(&seat, reader.column(slot));
        let column = numbers
            .iter()
            .flatten()
            .filter(|(tally, ..)| *tally == Tally::Waiting)
            .count() as u8;
        for (tally, nth, count) in numbers.into_iter().flatten() {
            let number = reader.lay(
                &(*slot, 0.0, column, stood(&seat), open),
                (tally, nth),
                count,
            );
            out.push((slot.player, tally, nth, body_corners(&number, tally, count)));
        }
    }
    out
}

/// Every table this client lays, at a laptop's window and a small one.
fn tables() -> Vec<(String, TableLayout, Vec2)> {
    use baylee_client_core::layout::{Arrangement, Seat};
    let mut out = Vec::new();
    for window in [Vec2::new(1708.0, 1032.0), Vec2::new(1280.0, 720.0)] {
        for n in 2..=8u8 {
            let seats: Vec<Seat> = (0..n).map(PlayerId::new).map(Seat::alone).collect();
            for arrangement in Arrangement::ALL {
                let layout = TableLayout::arranged(&seats, window.x / window.y, arrangement, None);
                out.push((
                    format!("{n} seats, {arrangement:?}, {window}"),
                    layout,
                    window,
                ));
            }
        }
    }
    out
}

/// No number lies on a pile's card, on a seat's rows or on another number,
/// at any table: not its own seat's, not a neighbour's.
#[test]
fn no_number_lies_on_a_card_a_row_or_another_number() {
    let mut wrong: Vec<String> = Vec::new();
    for (table, layout, window) in tables() {
        let numbers = laid_for(&layout, window);
        let drawn: Vec<&SeatSlot> = layout.slots.iter().filter(|slot| !slot.parked).collect();
        let taxes = numbers
            .iter()
            .filter(|(_, tally, ..)| matches!(tally, Tally::Tax | Tally::Tax2))
            .count();
        assert_eq!(
            taxes,
            2 * drawn.len(),
            "{table}: a command slot's tax was not laid, and nothing here looked at it"
        );
        for (i, (player, tally, nth, corners)) in numbers.iter().enumerate() {
            let plate = on_table(*corners);
            let name = format!("{table}: seat {}'s {tally:?} {nth}", player.get());
            for slot in &drawn {
                if overlap(plate, seat_rect(slot.center, slot.facing, slot.half_extent)) {
                    wrong.push(format!("{name} lies on seat {}'s rows", slot.player.get()));
                }
                for pile in PileKind::ALL {
                    let card = Vec2::new(CARD_WIDTH, CARD_HEIGHT) * (0.5 * slot.scale);
                    if overlap(plate, seat_rect(slot.pile_center(pile), slot.facing, card)) {
                        wrong.push(format!(
                            "{name} lies on seat {}'s {}",
                            slot.player.get(),
                            pile.label()
                        ));
                    }
                }
            }
            for (other, what, which, theirs) in &numbers[i + 1..] {
                if overlap(plate, on_table(*theirs)) {
                    wrong.push(format!(
                        "{name} lies on seat {}'s {what:?} {which}",
                        other.get()
                    ));
                }
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} numbers lie on something:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// Every number is on the cloth, short of the rail, and in the window above
/// the hand: a number nobody can see counts nothing.
#[test]
fn every_number_is_on_the_cloth_and_in_the_window() {
    let mut wrong: Vec<String> = Vec::new();
    for (table, layout, window) in tables() {
        let rig = CameraRig::home(&layout, Canvas::hud(window));
        let lens = Lens::new(rig, window);
        let (lo, hi) = layout.extent().expect("a table has seats");
        let cloth = SLAB_MARGIN - baylee_client_core::tabletop::RAIL_WIDTH;
        for (player, tally, nth, corners) in laid_for(&layout, window) {
            let name = format!("{table}: seat {}'s {tally:?} {nth}", player.get());
            for corner in corners {
                let at = Vec2::new(corner.x, -corner.z);
                if !(at.cmpge(lo - cloth).all() && at.cmple(hi + cloth).all()) {
                    wrong.push(format!(
                        "{name} lies at {at}, off the cloth ({lo} to {hi} and {cloth} round it)"
                    ));
                }
                let seen = lens
                    .project_world(corner)
                    .expect("the table is before the eye");
                if !(seen.x >= 0.0
                    && seen.x <= window.x
                    && seen.y >= 0.0
                    && seen.y <= window.y - crate::hud::HAND_ZONE_H)
                {
                    wrong.push(format!(
                        "{name} is drawn at {seen}, outside a window of {window} over its hand"
                    ));
                }
            }
        }
    }
    wrong.dedup_by_key(|line| line.split(" at [").next().map(str::to_owned));
    assert!(
        wrong.is_empty(),
        "{} numbers cannot be seen:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// How tall a laid number's figures are on the screen, in the interface's
/// pixels: its plate's height as the lens draws it, and the share of that
/// the figures take.
fn figures_px(lens: &Lens, tally: Tally, corners: [Vec3; 4]) -> f32 {
    let word = cardplate::pile_word(1, tally);
    let [_, y0, _, y1] = cardplate::pile_badge_rect(word);
    let top = lens.project_world((corners[0] + corners[1]) * 0.5);
    let bottom = lens.project_world((corners[3] + corners[2]) * 0.5);
    let plate = top.zip(bottom).map_or(0.0, |(a, b)| a.distance(b));
    plate * cardplate::badge_cap(word) / (y1 - y0)
}

/// How large the figures are is a promise with a floor under it. Nine
/// pixels need a plate some three and a half times as long, and a seat has
/// that much felt beside a pile only where its cards are drawn large: at a
/// duel, in Spotlight, at one's own seat and the flanks of a ring. Elsewhere
/// a plate is as large as its felt, and never smaller than one and a half
/// times a merged card's count. Measured here so the documentation's numbers
/// are this test's: in a laptop's window a duel reads at nine pixels and
/// the smallest number at a ring of three or four at five and a half or
/// more, and in a small window a duel reads at five and a half.
#[test]
fn the_figures_have_a_floor_at_every_table() {
    let laptop = Vec2::new(1708.0, 1032.0);
    for (table, layout, window) in tables() {
        let rig = CameraRig::home(&layout, Canvas::hud(window));
        let lens = Lens::new(rig, window);
        let mut least = f32::MAX;
        for (player, tally, nth, corners) in laid_for(&layout, window) {
            let slot = layout.slot(player).expect("a seat that wears a number");
            let [_, y0, _, y1] = cardplate::pile_badge_rect(cardplate::pile_word(1, tally));
            let scale = corners[0].distance(corners[3]) / ((y1 - y0) * CARD_WIDTH * slot.scale);
            assert!(
                scale >= cardplate::PILE_BADGE_SCALE - 1e-3,
                "{table}: {tally:?} {nth} at seat {player:?} is set {scale} times a card's count"
            );
            least = least.min(figures_px(&lens, tally, corners));
        }
        let seats = layout.slots.iter().filter(|slot| !slot.parked).count();
        let floor = match (window == laptop, seats) {
            (true, 2) => 8.5,
            (true, 3 | 4) if table.contains(" Ring,") => 5.5,
            (false, 2) => 5.5,
            _ => continue,
        };
        assert!(
            least >= floor,
            "{table}: the smallest figures are {least} px, under {floor}"
        );
    }
}

/// What the top of a pile `high` at a seat's `kind` place covers of the felt
/// for an eye at `eye`: the top card's outline, thrown onto the table.
fn covered(slot: &SeatSlot, kind: PileKind, tall: f32, eye: Vec3) -> [Vec2; 4] {
    let half = Vec2::new(0.5 * CARD_WIDTH, 0.5 * CARD_HEIGHT) * slot.scale;
    let eye2 = Vec2::new(eye.x, -eye.z);
    seat_rect(slot.pile_center(kind), slot.facing, half)
        .map(|corner| corner + (corner - eye2) * (tall / (eye.y - tall)))
}

/// A pile is a block of cardboard, and a number lying behind it from where
/// the reader looks begins past what the block covers: no pile at any table
/// stands in front of its own number, however tall.
///
/// And the block does cover that felt: laid as beside a pile with no
/// height, the hand's number at the seat across a table of four lies under
/// its library's top card, which is how it was played on 13.11.2026.
#[test]
fn no_pile_stands_in_front_of_its_own_number() {
    let mut wrong = Vec::new();
    for (table, layout, window) in tables() {
        let eye = CameraRig::home(&layout, Canvas::hud(window))
            .eye()
            .translation;
        for (player, tally, nth, corners) in laid_for(&layout, window) {
            let slot = layout.slot(player).expect("a seat that wears a number");
            let cards = stood(&pod(player, FULL))[pile_at(tally)];
            let tall = high(cards, slot.scale);
            if overlap(on_table(corners), covered(slot, tally.pile(), tall, eye)) {
                wrong.push(format!("{table}: {tally:?} {nth} at seat {player:?}"));
            }
        }
    }
    assert!(wrong.is_empty(), "behind their own pile: {wrong:#?}");

    let window = Vec2::new(1708.0, 1032.0);
    let players: Vec<PlayerId> = (0..4).map(PlayerId::new).collect();
    let layout = TableLayout::new(&players, window.x / window.y, None);
    let rig = CameraRig::home(&layout, Canvas::hud(window));
    let reader = Reader::of(rig, Some(window));
    let across = layout
        .slots
        .iter()
        .find(|slot| slot.facing.cos() < -0.9)
        .expect("a seat across a table of four");
    let tall = high(40, across.scale);
    let flat = reader.lay(&(*across, 0.0, 1, [0; 6], true), (Tally::Hand, 0), 7);
    assert!(
        overlap(
            on_table(body_corners(&flat, Tally::Hand, 7)),
            covered(across, PileKind::Library, tall, rig.eye().translation)
        ),
        "a tall library covers nothing behind it: this test proves nothing"
    );
}

/// Where the reader sits and across from them, every number lies outside
/// its pile: to the right of the library, the graveyard and exile as that
/// seat has right, and to the left of the waiting place and the command
/// slots, the seat across as the mirror image. At a seat seen from the side
/// it lies between pile and mat. And no seat of any table stands near the
/// line between the two answers, so two seats that are each other's mirror
/// image answer alike.
#[test]
fn a_number_lies_outside_its_pile_where_the_piles_stand_beside_the_mat() {
    let players: Vec<PlayerId> = (0..4).map(PlayerId::new).collect();
    let layout = TableLayout::new(&players, 1.78, None);
    let turn = upright(&CameraRig::default().eye());
    let mut seats = [0, 0];
    for slot in &layout.slots {
        let squarely = beside(slot, turn);
        seats[usize::from(squarely)] += 1;
        let seat = card_transform(slot, slot.center, false, 0.0);
        let right = seat.rotation * Vec3::X;
        for tally in [
            Tally::Library,
            Tally::Graveyard,
            Tally::Exile,
            Tally::Waiting,
            Tally::Tax,
            Tally::Tax2,
        ] {
            let kind = tally.pile();
            let pile = card_transform(slot, slot.pile_center(kind), false, 0.0);
            let number =
                pile_count_transform(slot, (tally, 0, 1), 40, turn, 2.0, Vec3::ZERO, squarely);
            let corners = body_corners(&number, tally, 40);
            let middle = (corners[0] + corners[2]) * 0.5;
            // To the seat's right, in card widths at this seat's size.
            let rightwards = (middle - pile.translation).dot(right) / slot.scale;
            if squarely {
                assert!(
                    rightwards * kind.side() > 0.0,
                    "{tally:?} at a seat facing {}: {rightwards} to its pile's right, not outside it",
                    slot.facing
                );
            } else {
                assert!(
                    rightwards * kind.side() < 0.0,
                    "{tally:?} at a seat facing {}: not between its pile and its mat",
                    slot.facing
                );
            }
            // Clear of the pile's card by the same air wherever it lies.
            let nearest = corners
                .iter()
                .map(|c| ((*c - pile.translation).dot(right) / slot.scale).abs())
                .fold(f32::MAX, f32::min);
            let air = 0.5 * CARD_WIDTH + cardplate::PILE_BADGE_AIR;
            assert!(
                (nearest - air).abs() < 1e-3,
                "{tally:?} begins {nearest} out, not {air}"
            );
        }
        // The hand's lies past the library's end on the seat's own side.
        let library = card_transform(slot, slot.pile_center(PileKind::Library), false, 0.0);
        let far = seat.rotation * Vec3::Y;
        let number = pile_count_transform(
            slot,
            (Tally::Hand, 0, 1),
            7,
            turn,
            2.0,
            Vec3::ZERO,
            squarely,
        );
        let nearest = body_corners(&number, Tally::Hand, 7)
            .iter()
            .map(|c| -(*c - library.translation).dot(far) / slot.scale)
            .fold(f32::MAX, f32::min);
        let air = 0.5 * CARD_HEIGHT + cardplate::PILE_BADGE_AIR;
        assert!(
            (nearest - air).abs() < 1e-3,
            "the hand's number begins {nearest} towards its owner from the library, not {air}"
        );
    }
    assert_eq!(
        seats,
        [2, 2],
        "a ring of four has two seats beside the reader and two across"
    );
    for (table, layout, _) in tables() {
        for slot in layout.slots.iter().filter(|slot| !slot.parked) {
            let squarely = slot.facing.cos().abs();
            assert!(
                (squarely - 0.9).abs() > 0.05,
                "{table}: a seat facing {} is {squarely} beside the reader, on the line",
                slot.facing
            );
        }
    }
}

/// The felt outside a seat's left-hand column is open at every table of two
/// to four seats, so there its waiting place's and its command slots'
/// numbers lie to their left (the owner, 13.11.2026). Where two seats stand
/// abreast, as in a ring of eight, the one on the right keeps them between
/// place and mat: its neighbour writes into the felt between the two.
#[test]
fn the_felt_left_of_a_seat_is_open_at_every_table_of_up_to_four() {
    let mut shut = 0;
    for (table, layout, window) in tables() {
        let turn = upright(&CameraRig::home(&layout, Canvas::hud(window)).eye());
        for slot in layout.slots.iter().filter(|slot| !slot.parked) {
            let open = open_left(slot, layout.slots.iter());
            if layout.slots.len() <= 4 && beside(slot, turn) {
                assert!(
                    open,
                    "{table}: seat {} has a neighbour's piles on its left",
                    slot.player.get()
                );
            }
            shut += usize::from(!open && table.starts_with("8 seats, Ring,"));
        }
    }
    assert!(
        shut > 0,
        "no seat of a ring of eight has a neighbour abreast: this test proves nothing"
    );
}

/// A number is written for the one looking: at every seat of a ring it
/// stands the same way up, its tops up the screen and its figures running
/// to the screen's right, also once the camera has gone round the table.
#[test]
fn every_seats_number_stands_upright_for_the_viewer() {
    let app = running(4, Holds::default());
    let index = app.world().resource::<SceneIndex>();
    for yaw in [0.0, 1.1, std::f32::consts::PI, -2.0] {
        let eye = CameraRig {
            yaw,
            ..CameraRig::default()
        }
        .eye();
        let turn = upright(&eye);
        let (screen_right, screen_up) = (eye.rotation * Vec3::X, eye.rotation * Vec3::Y);
        for zone in index.zones.values() {
            for tally in Tally::ALL {
                let outside = outside(&zone.slot, turn, tally.pile(), true);
                let what = (tally, 0, 1);
                let number =
                    pile_count_transform(&zone.slot, what, 40, turn, 2.0, Vec3::ZERO, outside);
                let (runs, tops) = (number.rotation * Vec3::X, number.rotation * Vec3::Y);
                assert!(
                    runs.dot(screen_right) > 0.999,
                    "yaw {yaw}: the figures run {runs}, the screen's right is {screen_right}"
                );
                assert!(
                    tops.dot(screen_up) > 0.0,
                    "yaw {yaw}: the tops point down the screen"
                );
                assert!(
                    (number.rotation * Vec3::Z).dot(Vec3::Y) > 0.999,
                    "yaw {yaw}: the number does not lie flat"
                );
            }
        }
    }
    // The four seats face four ways, or this proved nothing about turning.
    let facings: Vec<f32> = index.zones.values().map(|zone| zone.slot.facing).collect();
    assert!(
        facings
            .iter()
            .any(|a| (a - facings[0]).rem_euclid(std::f32::consts::TAU) > 1.0)
    );
}

/// Through a window, a number's figures are the height the interface asks
/// for at every seat, near or far, wherever its bounds leave the room.
#[test]
fn the_figures_are_as_tall_on_the_screen_at_every_seat() {
    let window = Vec2::new(1708.0, 1032.0);
    let players: Vec<PlayerId> = (0..4).map(PlayerId::new).collect();
    let layout = TableLayout::new(&players, window.x / window.y, None);
    let rig = CameraRig::home(&layout, Canvas::hud(window));
    let reader = Reader::of(rig, Some(window));
    let lens = Lens::new(rig, window);
    let mut asked = 0;
    for player in &players {
        let slot = *layout.slot(*player).expect("every seat has a place");
        for tally in Tally::ALL {
            let number = reader.lay(&(slot, 0.0, 1, [0; 6], true), (tally, 0), 40);
            let scale = number.scale.y / slot.scale;
            // The figures' height in the world, drawn from the body's middle.
            let word = cardplate::pile_word(40, tally);
            let cap = cardplate::badge_cap(word) * CARD_WIDTH * number.scale.y;
            let corners = body_corners(&number, tally, 40);
            let middle = (corners[0] + corners[2]) * 0.5;
            let tall = lens
                .project_world(middle)
                .zip(lens.project_world(middle + number.rotation * Vec3::Y * cap))
                .map(|(a, b)| a.distance(b))
                .expect("the table is in front of the eye");
            if (scale - cardplate::PILE_BADGE_SCALE).abs() < 1e-4 {
                assert!(
                    tall >= cardplate::PILE_FIGURE_PX - 0.5,
                    "{tally:?}: {tall} px at the least"
                );
            } else {
                // The asked size, or what its room or the most allow of it.
                assert!(
                    tall <= cardplate::PILE_FIGURE_PX + 0.5,
                    "{tally:?} at seat {player:?}: figures {tall} px tall"
                );
                assert!(scale <= cardplate::PILE_BADGE_MOST + 1e-4);
                if (tall - cardplate::PILE_FIGURE_PX).abs() < 0.5 {
                    asked += 1;
                }
            }
        }
    }
    // A table of four on a laptop's window is the case this was built for:
    // the size is the asked one there, not a bound's.
    assert!(
        asked >= 10,
        "only {asked} of twenty numbers are the asked size"
    );
}

/// A camera going round re-lays the numbers and leaves the scene's index
/// alone: the index is what every other table system watches.
#[test]
fn a_camera_going_round_moves_the_numbers_and_writes_no_index() {
    let mut app = running(
        4,
        Holds {
            library: 40,
            graveyard: 3,
            hand: 7,
            ..Holds::default()
        },
    );
    let before: Vec<Transform> = app
        .world_mut()
        .query_filtered::<&Transform, With<PileCount>>()
        .iter(app.world())
        .copied()
        .collect();
    assert_eq!(before.len(), 12);
    app.insert_resource(ShownRig::standing(CameraRig {
        yaw: 1.2,
        ..CameraRig::default()
    }));
    app.update();
    let wrote = app.world().resource::<Wrote>();
    assert!(!wrote.index, "a turn of the camera wrote the scene's index");
    assert!(
        !wrote.materials,
        "a turn of the camera wrote a badge's material"
    );
    let after: Vec<Transform> = app
        .world_mut()
        .query_filtered::<&Transform, With<PileCount>>()
        .iter(app.world())
        .copied()
        .collect();
    assert!(
        before
            .iter()
            .zip(&after)
            .all(|(a, b)| a.rotation.angle_between(b.rotation) > 1.0),
        "the numbers did not turn with the eye"
    );
}

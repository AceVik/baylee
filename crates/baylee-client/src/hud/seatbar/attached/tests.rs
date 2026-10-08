//! The plate, the steps and their poses.

use super::*;

fn fonts() -> UiFonts {
    UiFonts {
        text: default(),
        medium: default(),
        bold: default(),
        italic: default(),
        medium_italic: default(),
        serif: default(),
        serif_italic: default(),
        icons: default(),
        mana: default(),
    }
}

/// Every seat's plates spawned over `view`, in a world of their own.
fn plates_over(view: &PlayerView) -> App {
    let mut app = App::new();
    let fonts = fonts();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, app.world());
    let root = commands.spawn_empty().id();
    for seat in &view.seats {
        spawn(
            &mut commands,
            root,
            Lang::De,
            view,
            None,
            seat,
            SeatRole::Present,
            &PhaseOrders::default(),
            &fonts,
        );
    }
    queue.apply(app.world_mut());
    app
}

fn words(app: &mut App) -> Vec<String> {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.clone())
        .collect()
}

/// Every seat's floating mana is on its own plate, plain and restricted,
/// and an empty pool draws no third line (and no "Manavorrat: 0").
#[test]
fn every_seats_pool_is_on_its_plate_and_an_empty_pool_draws_nothing() {
    let mut view = baylee_client_core::test_support::ViewBuilder::new(3).build();
    view.seats[1].mana_pool.blue = 7;
    view.seats[2].mana_pool.restricted[0] = 9;
    let mut app = plates_over(&view);
    let said = words(&mut app);
    assert!(said.iter().any(|t| t == "\u{00d7}7"), "{said:?}");
    assert!(said.iter().any(|t| t == "\u{00d7}9"), "{said:?}");
    assert!(
        !said.iter().any(|t| t.contains("Manavorrat")),
        "the pool's old label is gone: {said:?}"
    );
    let pools = app
        .world_mut()
        .query::<&PlateMark>()
        .iter(app.world())
        .filter(|m| m.kind == PlateMarkKind::Pool)
        .count();
    assert_eq!(pools, 2, "a third line on the two seats with mana only");
}

/// The owner's ∞: a hand with no maximum size shows the mark and no
/// words — not "7/∞" and not "Kein Handkartenlimit".
#[test]
fn no_maximum_hand_size_is_the_infinity_mark_alone() {
    let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    view.seats[1].no_max_hand_size = true;
    let mut app = plates_over(&view);
    let marks: Vec<PlayerId> = app
        .world_mut()
        .query::<&PlateMark>()
        .iter(app.world())
        .filter(|m| m.kind == PlateMarkKind::Unlimited)
        .map(|m| m.player)
        .collect();
    assert_eq!(marks, [PlayerId::new(1)]);
    let said = words(&mut app);
    assert!(said.iter().any(|t| t == &glyph::INFINITY.to_string()));
    assert!(
        !said
            .iter()
            .any(|t| t.contains("/∞") || t.contains(Phrase::NoMaxHandSize.text(Lang::De))),
        "{said:?}"
    );
}

/// The crown stands on the monarch's plate and on no other.
#[test]
fn the_crown_is_on_the_monarchs_plate() {
    let mut view = baylee_client_core::test_support::ViewBuilder::new(4).build();
    let crowns = |view: &PlayerView| -> Vec<PlayerId> {
        let mut app = plates_over(view);
        app.world_mut()
            .query::<&PlateMark>()
            .iter(app.world())
            .filter(|m| m.kind == PlateMarkKind::Crown)
            .map(|c| c.player)
            .collect()
    };
    assert!(crowns(&view).is_empty());
    view.monarch = Some(PlayerId::new(2));
    assert_eq!(crowns(&view), [PlayerId::new(2)]);
}

/// The owner's rule for the plate: everything written on it lets the
/// pointer through, so the plate itself takes the hover and the press.
/// Red if any label, icon or line under a plate is pickable — which is
/// what a label swallowing the hover looks like from here.
#[test]
fn nothing_on_a_plate_takes_the_pointer_from_it() {
    let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    view.monarch = Some(PlayerId::new(0));
    view.seats[1].no_max_hand_size = true;
    view.seats[1].mana_pool.green = 2;
    view.seats[1].poison = 3;
    let mut app = plates_over(&view);
    let world = app.world_mut();
    let plates: Vec<Entity> = world
        .query_filtered::<Entity, With<PlateTab>>()
        .iter(world)
        .collect();
    assert_eq!(plates.len(), 2);
    for plate in plates {
        assert!(
            world.entity(plate).get::<Pickable>() != Some(&Pickable::IGNORE),
            "the plate itself must be pickable"
        );
        let mut stack: Vec<Entity> = world
            .entity(plate)
            .get::<Children>()
            .map(|c| c.iter().collect())
            .unwrap_or_default();
        let mut seen = 0;
        while let Some(e) = stack.pop() {
            seen += 1;
            let entity = world.entity(e);
            assert_eq!(
                entity.get::<Pickable>(),
                Some(&Pickable::IGNORE),
                "{:?} on a plate takes the pointer ({:?})",
                e,
                entity.get::<Text>()
            );
            if let Some(children) = entity.get::<Children>() {
                stack.extend(children.iter());
            }
        }
        assert!(seen > 6, "a plate with {seen} things on it was not built");
    }
}

/// A press on the plate chooses the seat while a target question allows
/// it, and moves no camera; without a question it does nothing at all.
/// The click lands on a *label* of the plate, as a pointer would.
#[test]
fn a_press_on_a_plates_words_targets_the_seat_and_never_moves_the_camera() {
    use baylee_engine::choice::Pending;
    let asked = || {
        Some(baylee_client_core::interaction::Interaction::new(
            Pending::ChooseTargets {
                player: PlayerId::new(0),
                options: vec![],
                player_options: vec![PlayerId::new(1)],
                min: 1,
                max: 1,
                reason: baylee_engine::choice::TargetPrompt::Targets,
            },
            PlayerId::new(0),
        ))
    };
    for question in [true, false] {
        let mut app = crate::input::tests::pointer_app(crate::Duel {
            interaction: if question { asked() } else { None },
            ..default()
        });
        let plate = app
            .world_mut()
            .spawn(PlateTab {
                player: PlayerId::new(1),
            })
            .id();
        let label = app.world_mut().spawn(ChildOf(plate)).id();
        crate::input::tests::click(&mut app, label);
        let duel = app.world().resource::<crate::Duel>();
        assert!(duel.visiting.is_none(), "the plate moved the camera");
        assert_eq!(
            duel.interaction
                .as_ref()
                .is_some_and(|i| i.is_seat_selected(PlayerId::new(1))),
            question,
            "question {question}: the seat was not chosen as it should be"
        );
    }
}

/// A band as [`crate::table::Lens`] hands it over: the two corners on the
/// rim first, then the two that meet the lane behind it. `flip` is the
/// seat across the table, whose own left is this screen's right.
fn band(length: f32, depth: f32, slope: f32, flip: bool) -> [Vec2; 4] {
    let mid = Vec2::new(1500.0, 620.0);
    let along = Vec2::new(1.0, slope).normalize() * (length * 0.5);
    let across = Vec2::new(-along.y, along.x).normalize() * (depth * 0.5);
    let (near, far) = (mid - across, mid + across);
    if flip {
        [near + along, near - along, far - along, far + along]
    } else {
        [near - along, near + along, far + along, far - along]
    }
}

/// The four corners a panel is actually drawn at.
fn drawn(corners: [Vec2; 4], panel: Panel) -> [Vec2; 4] {
    let (corner, tilt, scale) = pose_on(corners, panel);
    drawn_quad(corner, panel.size(), tilt, scale)
}

/// Where [`place`] puts one panel of `player`'s under this rig, if it
/// puts it anywhere.
fn placed(
    layout: &baylee_client_core::layout::TableLayout,
    rig: crate::table::CameraRig,
    window: Vec2,
    player: PlayerId,
    panel: Panel,
) -> Display {
    let duel = Duel {
        layout: Some(layout.clone()),
        ..Duel::default()
    };
    let lens = crate::table::Lens::new(rig, window);
    if pose(&duel, Some(&lens), player, panel, 1.0).is_some() {
        Display::Flex
    } else {
        Display::None
    }
}

/// Placing a panel where it already stands writes nothing: its `Node`
/// and `UiTransform` do not read as changed, so a table at rest costs
/// the interface no relayout. The first placement does write.
#[test]
fn a_panel_that_stays_put_is_not_touched() {
    use crate::table::{CameraRig, Canvas};
    use baylee_client_core::layout::TableLayout;
    let window = Vec2::new(1728.0, 1052.0);
    let canvas = Canvas::hud(window);
    let seats = [PlayerId::new(0), PlayerId::new(1)];
    let layout = TableLayout::new(&seats, canvas.aspect(), None);
    let lens = crate::table::Lens::new(CameraRig::home(&layout, canvas), window);
    let duel = Duel {
        layout: Some(layout),
        ..Duel::default()
    };
    let mut world = World::new();
    let panel = world.spawn((Node::default(), UiTransform::default())).id();
    let mut parts = world.query::<(&mut Node, &mut UiTransform)>();
    let mut place_once = |world: &mut World| {
        world.clear_trackers();
        let (mut node, mut turn) = parts.get_mut(world, panel).expect("the panel");
        place(
            &duel,
            Some(&lens),
            (seats[0], Panel::Identity, 1.0),
            &mut node,
            &mut turn,
        );
        let entity = world.entity(panel);
        (
            entity.get_ref::<Node>().expect("a node").is_changed(),
            entity
                .get_ref::<UiTransform>()
                .expect("a turn")
                .is_changed(),
        )
    };
    assert_eq!(
        place_once(&mut world),
        (true, true),
        "placed the first time"
    );
    assert_eq!(
        place_once(&mut world),
        (false, false),
        "placed again where it stands: nothing written"
    );
}

/// #303 and the strips: at home every plate, steps panel and turn number
/// is drawn, at every seat count; aimed at an opponent, the camera puts
/// my own band under the hand and my plate is not drawn there.
#[test]
fn no_ink_is_written_under_the_hand() {
    use crate::table::{CameraRig, Canvas};
    use baylee_client_core::layout::TableLayout;
    let window = Vec2::new(1728.0, 1052.0);
    let canvas = Canvas::hud(window);
    for n in 2..=8u8 {
        let seats: Vec<PlayerId> = (0..n).map(PlayerId::new).collect();
        let layout = TableLayout::new(&seats, canvas.aspect(), None);
        let rig = CameraRig::home(&layout, canvas);
        for &player in &seats {
            for panel in [Panel::Identity, Panel::Phases, Panel::Turn] {
                assert_eq!(
                    placed(&layout, rig, window, player, panel),
                    Display::Flex,
                    "{n} seats at home: seat {player:?}'s {panel:?} is hidden"
                );
            }
        }
    }

    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let layout = TableLayout::new(&[me, them], canvas.aspect(), Some(them));
    let slot = *layout.slot(them).expect("the opponent has a seat");
    let rig = CameraRig {
        yaw: 0.0,
        ..CameraRig::framing(&slot, Vec2::new(slot.center.x, -slot.center.y))
    };
    let lens = crate::table::Lens::new(rig, window);
    let mine = layout.slot(me).expect("so do I");
    let (corner, tilt, scale) = pose_on(
        lens.corners(mine.ledge_corners())
            .expect("my band is in front of the camera"),
        Panel::Identity,
    );
    assert!(
        under_the_hand(
            corner,
            Panel::Identity.size(),
            tilt,
            scale,
            window.y - crate::hud::HAND_ZONE_H
        ),
        "aimed at the opponent, my name no longer stands under the hand, so \
         this no longer tests the rule"
    );
    assert_eq!(
        placed(&layout, rig, window, me, Panel::Identity),
        Display::None,
        "my name is written under the hand zone"
    );
    assert_eq!(
        placed(&layout, rig, window, them, Panel::Identity),
        Display::Flex,
        "the seat the camera was aimed at lost its name"
    );
}

/// The owner's "flush" and his two alignment rules (08.10.2026): in every
/// built arrangement, at two to eight seats, in a laptop's and a phone's
/// window, every plate's edge **and** every steps panel's edge lie
/// [`BAND_AIR`] off their mat's drawn edge (within 1.5 px: one constant for
/// both), every steps panel's end on the seat's right is on the mat's
/// corner there (within 2 px; on the screen's left for a seat across the
/// table), the plates stay clear of the HUD's corners and strips, and no
/// plate meets another plate or any steps panel. At a laptop's home shot
/// every seat's plate is drawn.
#[test]
#[allow(clippy::too_many_lines)] // one sweep: every arrangement, seat count and window
fn every_plate_is_flush_with_its_battlefield_and_meets_no_other() {
    use crate::table::{CameraRig, Canvas, Shot};
    use baylee_client_core::layout::{Seat, TableLayout};
    use baylee_client_core::tableview::{Arrangement, TableFrame};
    for window in [
        Vec2::new(1708.0, 1032.0),
        Vec2::new(1280.0, 800.0),
        Vec2::new(844.0, 390.0),
    ] {
        let canvas = Canvas::hud(window);
        let frame = TableFrame::of(window.x, window.y);
        let hand_top = window.y - crate::hud::HAND_ZONE_H;
        for wanted in Arrangement::ALL.into_iter().filter(|a| a.built()) {
            for n in 2..=8u8 {
                let roster: Vec<Seat> = (0..n).map(PlayerId::new).map(Seat::alone).collect();
                let arrangement = wanted.effective(usize::from(n), frame);
                let layout = TableLayout::arranged(&roster, canvas.aspect(), arrangement, None);
                let shot = Shot {
                    arrangement,
                    ..Shot::default()
                };
                let rig = CameraRig::home_shot(&layout, canvas, shot).0;
                let lens = crate::table::Lens::new(rig, window);
                let mut quads: Vec<(PlayerId, [Vec2; 4])> = Vec::new();
                let mut steps: Vec<(PlayerId, [Vec2; 4])> = Vec::new();
                let mut seen = 0;
                for slot in layout.on_felt() {
                    seen += 1;
                    let Some(corners) = lens.corners(slot.ledge_corners()) else {
                        continue;
                    };
                    let edge = mat_edge(slot, &lens).expect("the edge is in front");
                    let (at, tilt, scale, _) =
                        steps_on(slot, &lens, corners).expect("so are the steps");
                    let quad = drawn_quad(at, Panel::Phases.size(), tilt, scale);
                    let gap = quad
                        .iter()
                        .map(|p| (*p - edge.left).dot(edge.away))
                        .fold(f32::INFINITY, f32::min);
                    assert!(
                        (gap - BAND_AIR).abs() <= 1.5,
                        "{window}, {arrangement:?}, {n} seats, seat {:?}: the steps \
                         stand {gap:.1} px off their mat's edge",
                        slot.player
                    );
                    let reach = quad
                        .iter()
                        .map(|p| (*p - edge.right).dot(edge.along))
                        .fold(f32::NEG_INFINITY, f32::max);
                    assert!(
                        reach.abs() <= 2.0,
                        "{window}, {arrangement:?}, {n} seats, seat {:?}: the steps \
                         end {reach:.1} px off the mat's corner on the seat's right",
                        slot.player
                    );
                    steps.push((slot.player, quad));
                    let Some((at, tilt, scale)) =
                        plate_on(&layout, &lens, slot.player, |_| 2, (1.0, hand_top))
                    else {
                        continue;
                    };
                    let quad = drawn_quad(at, plate_size(2), tilt, scale);
                    let MatEdge {
                        left: corner, away, ..
                    } = mat_edge(slot, &lens).expect("the edge is in front");
                    // How far the plate's nearest point stands off the
                    // mat's edge, measured away from the mat.
                    let gap = quad
                        .iter()
                        .map(|p| (*p - corner).dot(away))
                        .fold(f32::INFINITY, f32::min);
                    assert!(
                        (gap - BAND_AIR).abs() <= 1.5,
                        "{window}, {arrangement:?}, {n} seats, seat {:?}: the plate \
                         stands {gap:.1} px off its mat's edge",
                        slot.player
                    );
                    assert!(clear_of_the_hud(&quad, window, hand_top));
                    quads.push((slot.player, quad));
                }
                for (at, (a, qa)) in quads.iter().enumerate() {
                    for (b, qb) in &quads[at + 1..] {
                        assert!(
                            !overlaps(qa, qb),
                            "{window}, {arrangement:?}, {n} seats: the plates of \
                             {a:?} and {b:?} overlap"
                        );
                    }
                    for (b, qb) in &steps {
                        assert!(
                            !overlaps(qa, qb),
                            "{window}, {arrangement:?}, {n} seats: {a:?}'s plate \
                             meets {b:?}'s steps"
                        );
                    }
                }
                if window.y > 700.0 && arrangement == Arrangement::Ring {
                    assert_eq!(
                        quads.len(),
                        seen,
                        "{window}, {n} seats at home: a plate is not drawn"
                    );
                }
            }
        }
    }
}

/// Identity and phases stay separated, including when the band is only
/// wide enough for their combined widths and gaps. Their order follows
/// the owner's frame even when that seat faces away from the reader.
#[test]
fn identity_and_phases_never_overlap() {
    let want = HEADER_W + TRACK_H + BAND_GAP * 3.0;
    for length in [want, want * 2.0, 2160.0] {
        for slope in [0.0_f32, 0.06, -0.06] {
            for flip in [false, true] {
                let corners = band(length, 110.0, slope, flip);
                let boxes = [Panel::Identity, Panel::Phases].map(|panel| drawn(corners, panel));
                let along = Vec2::new(1.0, slope).normalize() * if flip { -1.0 } else { 1.0 };
                let at = |b: [Vec2; 4], pick: fn(f32, f32) -> f32| {
                    b.iter().map(|c| c.dot(along)).fold(f32::NAN, pick)
                };
                for pair in boxes.windows(2) {
                    let gap = at(pair[1], f32::min) - at(pair[0], f32::max);
                    assert!(
                        gap >= -0.01,
                        "{length} long, slope {slope}, flipped {flip}: two \
                         panels overlap by {}",
                        -gap
                    );
                }
                assert!(
                    (boxes[0][0].x < boxes[1][0].x) != flip,
                    "{length} long, slope {slope}, flipped {flip}: the \
                     identity panel is not the leftmost"
                );
            }
        }
    }
}

/// The anchor another lane hangs things beside a plate by is the plate as
/// it is placed: the same quad, turn and scale, and an `along` that points
/// from the plate's corner end to its free one.
#[test]
fn the_plates_anchor_is_where_the_plate_is_drawn() {
    use crate::table::{CameraRig, Canvas};
    use baylee_client_core::layout::TableLayout;
    let window = Vec2::new(1708.0, 1032.0);
    let canvas = Canvas::hud(window);
    let seats = [PlayerId::new(0), PlayerId::new(1)];
    let layout = TableLayout::new(&seats, canvas.aspect(), None);
    let lens = crate::table::Lens::new(CameraRig::home(&layout, canvas), window);
    let duel = Duel {
        layout: Some(layout),
        ..Duel::default()
    };
    for player in seats {
        let (at, tilt, scale) =
            pose(&duel, Some(&lens), player, Panel::Identity, 1.0).expect("the plate is drawn");
        let beside = plate_beside(&duel, &lens, player, 1.0).expect("and so is its anchor");
        assert_eq!(beside.quad, drawn_quad(at, plate_size(2), tilt, scale));
        assert_eq!((beside.tilt, beside.scale), (tilt, scale));
        assert!((beside.along.length() - 1.0).abs() < 1e-4);
        assert!(
            beside.along.dot(beside.away).abs() < 1e-4,
            "square to each other"
        );
    }
}

/// The steps' names face away from the battlefield (the owner, 08.10.2026:
/// the opponent's under its bar, mirrored to mine): in a duel at home my
/// mat is under my steps and the column reads names then tiles; the seat
/// across has its mat above its steps and the column turned round — red if
/// both read the same way.
#[test]
fn the_steps_names_face_away_from_the_battlefield() {
    use crate::table::{CameraRig, Canvas};
    use baylee_client_core::layout::TableLayout;
    for window in [Vec2::new(1708.0, 1032.0), Vec2::new(844.0, 390.0)] {
        let canvas = Canvas::hud(window);
        let (me, them) = (PlayerId::new(0), PlayerId::new(1));
        let layout = TableLayout::new(&[me, them], canvas.aspect(), None);
        let lens = crate::table::Lens::new(CameraRig::home(&layout, canvas), window);
        let duel = Duel {
            layout: Some(layout),
            ..Duel::default()
        };
        let flow = |player| {
            let mut world = World::new();
            let panel = world.spawn((Node::default(), UiTransform::default())).id();
            let mut parts = world.query::<(&mut Node, &mut UiTransform)>();
            let (mut node, mut turn) = parts.get_mut(&mut world, panel).expect("the panel");
            place(
                &duel,
                Some(&lens),
                (player, Panel::Phases, 1.0),
                &mut node,
                &mut turn,
            );
            (node.display, node.flex_direction)
        };
        assert_eq!(
            flow(me),
            (Display::Flex, FlexDirection::Column),
            "{window}: my steps' names stand over the tiles"
        );
        assert_eq!(
            flow(them),
            (Display::Flex, FlexDirection::ColumnReverse),
            "{window}: the opponent's names stand under its tiles"
        );
    }
}

/// Nothing is written outside the strip the model keeps clear of cards.
#[test]
fn no_panel_stands_off_its_own_band() {
    for depth in [110.0_f32, 60.0, 26.0] {
        for slope in [0.0_f32, 0.06, -0.06] {
            for flip in [false, true] {
                let corners = band(2160.0, depth, slope, flip);
                let along = Vec2::new(1.0, slope).normalize();
                let across = Vec2::new(-along.y, along.x);
                let mid = corners
                    .iter()
                    .fold(Vec2::ZERO, |sum, c| sum + *c / corners.len() as f32);
                for panel in [Panel::Identity, Panel::Phases] {
                    for corner in drawn(corners, panel) {
                        let off = (corner - mid).dot(across).abs();
                        assert!(
                            off <= depth * 0.5 + 0.01,
                            "{depth} deep, slope {slope}, flipped {flip}: a \
                             {panel:?} corner is {off} off the band's middle, \
                             past its {} of room",
                            depth * 0.5
                        );
                    }
                }
            }
        }
    }
}

/// The ink reads in the viewer's order at every seat.
#[test]
fn a_seat_across_the_table_has_its_bar_upright() {
    for slope in [0.0_f32, 0.3, -0.3] {
        for flip in [false, true] {
            let (_, tilt, _) = pose_on(band(2160.0, 110.0, slope, flip), Panel::Identity);
            assert!(
                tilt.abs() <= std::f32::consts::FRAC_PI_2,
                "slope {slope}, flipped {flip}: the bar is turned {tilt} and \
                 would be read upside-down"
            );
        }
    }
}

/// The plate's lines fit its box with the shipped fonts, laid out by bevy:
/// the widest life and counts a game reaches, every counter, a crown, an ∞
/// and a pool of every colour. (A text step scales the whole plate through
/// its `UiTransform`, so what fits at one step fits at all of them.)
#[test]
fn the_plate_holds_its_widest_lines() {
    let (mut app, fonts) = crate::face::tests::layout_app();
    let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    view.monarch = Some(PlayerId::new(1));
    let seat = &mut view.seats[1];
    seat.life = -123;
    seat.hand_count = 17;
    seat.no_max_hand_size = true;
    seat.library_count = 100;
    seat.graveyard_count = 88;
    seat.poison = 9;
    seat.energy = 99;
    seat.commander_damage.push(baylee_view::CommanderDamage {
        source: ObjectId::new(9, 0),
        amount: 20,
    });
    let pool = &mut seat.mana_pool;
    (
        pool.white,
        pool.blue,
        pool.black,
        pool.red,
        pool.green,
        pool.colorless,
    ) = (9, 9, 9, 9, 9, 9);
    view.exile[1] = (0..12)
        .map(|i| baylee_client_core::test_support::token(40 + i, 1, "Exiled", 1, 1))
        .collect();
    let root = app
        .world_mut()
        .spawn(Node {
            width: px(1708),
            height: px(1032),
            ..default()
        })
        .id();
    let mut commands = app.world_mut().commands();
    spawn(
        &mut commands,
        root,
        Lang::De,
        &view,
        None,
        &view.seats[1],
        SeatRole::House,
        &PhaseOrders::default(),
        &fonts,
    );
    app.world_mut().flush();
    let world = app.world_mut();
    let plate = world
        .query_filtered::<Entity, With<PlateTab>>()
        .single(world)
        .expect("one plate");
    world.get_mut::<Node>(plate).expect("a node").display = Display::Flex;
    app.update();
    app.update();
    let world = app.world();
    let width = |e: Entity| {
        let c = world.get::<ComputedNode>(e).expect("laid out");
        c.size().x * c.inverse_scale_factor()
    };
    // The column of lines is the plate's second child, after the spine.
    let column = world.get::<Children>(plate).expect("a spine and a column")[1];
    let lines: Vec<Entity> = world
        .get::<Children>(column)
        .expect("lines")
        .iter()
        .collect();
    assert_eq!(lines.len(), 3, "name, details and the pool");
    for (at, line) in lines.into_iter().enumerate() {
        let room = width(line);
        let gap = match world.get::<Node>(line).expect("a node").column_gap {
            Val::Px(gap) => gap,
            _ => 0.0,
        };
        let kids: Vec<Entity> = world.get::<Children>(line).expect("parts").iter().collect();
        #[allow(clippy::cast_precision_loss)]
        let used: f32 =
            kids.iter().map(|&k| width(k)).sum::<f32>() + gap * kids.len().saturating_sub(1) as f32;
        assert!(
            used <= room + 0.5,
            "plate line {at} needs {used:.1} px and has {room:.1}"
        );
    }
}

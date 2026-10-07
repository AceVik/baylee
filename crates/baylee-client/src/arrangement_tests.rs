//! The switcher's keys and choices (DESIGN-v8 §2.3, §2.6), without a window.

use super::*;
use crate::keys::Fired;
use baylee_client_core::prefs::Action;

fn table(seats: usize) -> ArrangementFrame {
    ArrangementFrame {
        frame: Some(TableFrame::Wide),
        seats,
        flash: 0.0,
        height: 1028.0,
    }
}

fn press(
    actions: &[Action],
    digits: &[u32],
    duel: &mut Duel,
    settings: &mut crate::settings::ClientSettings,
    measured: &mut ArrangementFrame,
) -> bool {
    keys(Fired::of_actions(actions), digits, duel, settings, measured)
}

/// `P` opens the menu on the row in effect and holds the keyboard; `Esc`
/// and `P` again shut it; at two seats there is nothing to open.
#[test]
fn p_opens_the_menu_and_esc_shuts_it() {
    let mut duel = Duel::default();
    let mut settings = crate::settings::ClientSettings::default();
    let mut measured = table(4);
    assert!(press(
        &[Action::ArrangementMenu],
        &[],
        &mut duel,
        &mut settings,
        &mut measured
    ));
    assert_eq!(duel.arrangement_menu, Some(Arrangement::Ring.index()));
    assert!(
        press(
            &[Action::Confirm],
            &[],
            &mut duel,
            &mut settings,
            &mut measured
        ) || duel.arrangement_menu.is_none(),
        "while it stands, the menu has the keys"
    );
    duel.arrangement_menu = Some(0);
    assert!(press(
        &[Action::FocusNextSeat],
        &[],
        &mut duel,
        &mut settings,
        &mut measured
    ));
    assert_eq!(
        duel.visiting, None,
        "the table hears nothing under the menu"
    );
    press(
        &[Action::Cancel],
        &[],
        &mut duel,
        &mut settings,
        &mut measured,
    );
    assert_eq!(duel.arrangement_menu, None);
    press(
        &[Action::ArrangementMenu],
        &[],
        &mut duel,
        &mut settings,
        &mut measured,
    );
    press(
        &[Action::ArrangementMenu],
        &[],
        &mut duel,
        &mut settings,
        &mut measured,
    );
    assert_eq!(duel.arrangement_menu, None, "P again shuts it");

    let mut duel = Duel::default();
    let mut measured = table(2);
    press(
        &[Action::ArrangementMenu],
        &[],
        &mut duel,
        &mut settings,
        &mut measured,
    );
    assert_eq!(duel.arrangement_menu, None, "a duel has one arrangement");
    assert!(
        !press(
            &[Action::FocusHome],
            &[],
            &mut duel,
            &mut settings,
            &mut measured
        ),
        "with the menu shut, other keys are the table's"
    );
}

/// `↑↓` walk the rows and come round, the remember row included.
#[test]
fn the_arrows_walk_the_rows_round() {
    let mut duel = Duel::default();
    let mut settings = crate::settings::ClientSettings::default();
    let mut measured = table(4);
    duel.arrangement_menu = Some(0);
    press(
        &[Action::NumberUp],
        &[],
        &mut duel,
        &mut settings,
        &mut measured,
    );
    assert_eq!(duel.arrangement_menu, Some(REMEMBER_ROW));
    press(
        &[Action::NumberDown],
        &[],
        &mut duel,
        &mut settings,
        &mut measured,
    );
    assert_eq!(duel.arrangement_menu, Some(0));
    press(
        &[Action::CursorDown],
        &[],
        &mut duel,
        &mut settings,
        &mut measured,
    );
    assert_eq!(duel.arrangement_menu, Some(1));
}

/// `Shift+P` cycles the offered arrangements only: where the ring is the
/// one offered it changes nothing, and the pill flashes all the same.
#[test]
fn shift_p_cycles_only_what_is_offered_and_flashes() {
    let mut duel = Duel::default();
    let mut settings = crate::settings::ClientSettings::default();
    let mut measured = table(4);
    let offered = Arrangement::offered_at(4, TableFrame::Wide);
    press(
        &[Action::NextArrangement],
        &[],
        &mut duel,
        &mut settings,
        &mut measured,
    );
    assert!((measured.flash - FLASH_SECS).abs() < f32::EPSILON);
    let now = chosen(&duel, &settings.table, 4);
    assert!(offered.contains(&now), "{now:?} is offered");
    assert_eq!(now, Arrangement::Ring.next_offered(4, TableFrame::Wide));
    for _ in 0..offered.len() - 1 {
        press(
            &[Action::NextArrangement],
            &[],
            &mut duel,
            &mut settings,
            &mut measured,
        );
    }
    assert_eq!(
        chosen(&duel, &settings.table, 4),
        Arrangement::Ring,
        "past the last is the first"
    );
}

/// A digit chooses its row; a greyed row chooses nothing and leaves the
/// menu standing; an offered one shuts it.
#[test]
fn a_digit_chooses_its_row_and_a_greyed_row_nothing() {
    let mut duel = Duel::default();
    let mut settings = crate::settings::ClientSettings::default();
    let mut measured = table(4);
    let greyed = Arrangement::ALL
        .into_iter()
        .find(|a| a.offered(4, TableFrame::Wide).is_err());
    if let Some(greyed) = greyed {
        duel.arrangement_menu = Some(0);
        let digit = u32::try_from(greyed.index() + 1).expect("one digit");
        press(&[], &[digit], &mut duel, &mut settings, &mut measured);
        assert_eq!(
            duel.arrangement_game, None,
            "{greyed:?} is not offered here"
        );
        assert!(duel.arrangement_menu.is_some(), "the menu stays");
    }
    duel.arrangement_menu = Some(3);
    press(&[], &[1], &mut duel, &mut settings, &mut measured);
    assert_eq!(duel.arrangement_game, Some(Arrangement::Ring));
    assert_eq!(duel.arrangement_menu, None, "chosen: the menu shuts");
}

/// Unticked, a choice is this game's alone; ticked, it is the device's
/// memory for the seat count, and the per-game switch is dropped.
#[test]
fn remember_writes_the_seat_count_s_memory() {
    let mut duel = Duel::default();
    let mut settings = crate::settings::ClientSettings::default();
    let measured = table(5);
    assert!(pick(&mut duel, &mut settings, &measured, Arrangement::Ring));
    assert_eq!(duel.arrangement_game, Some(Arrangement::Ring));
    assert_eq!(settings.table.arrangement_by_seats.get(5), None);
    press_row(&mut duel, &mut settings, &measured, REMEMBER_ROW);
    assert!(duel.arrangement_remember);
    assert_eq!(
        settings.table.arrangement_by_seats.get(5),
        Some(Arrangement::Ring)
    );
    assert_eq!(duel.arrangement_game, None);
    press_row(&mut duel, &mut settings, &measured, REMEMBER_ROW);
    assert_eq!(
        settings.table.arrangement_by_seats.get(5),
        None,
        "unticked: forgotten"
    );
}

/// What is in effect: the game's switch over the per-count memory over the
/// default, and the ring wherever the choice is not offered.
#[test]
fn the_arrangement_in_effect_reads_game_then_count_then_default() {
    let mut duel = Duel::default();
    let mut view = TableView {
        arrangement: Arrangement::Pods,
        ..TableView::default()
    };
    view.arrangement_by_seats
        .set(4, Some(Arrangement::Spotlight));
    assert_eq!(chosen(&duel, &view, 4), Arrangement::Spotlight);
    assert_eq!(chosen(&duel, &view, 5), Arrangement::Pods);
    duel.arrangement_game = Some(Arrangement::UprightRing);
    assert_eq!(chosen(&duel, &view, 4), Arrangement::UprightRing);
    for frame in TableFrame::ALL {
        for seats in 2..=8 {
            let (effective, _) = in_effect(&duel, &view, seats, frame);
            assert!(
                effective.offered(seats, frame).is_ok(),
                "{effective:?} at {seats} on {frame:?}"
            );
        }
    }
    let (at_two, _) = in_effect(&duel, &view, 2, TableFrame::Wide);
    assert_eq!(at_two, Arrangement::Ring, "a duel is a duel");
}

/// The settings' per-count stepper walks *Default* and the built
/// arrangements, round both ways.
#[test]
fn the_per_count_stepper_walks_default_and_the_built_ones() {
    let built: Vec<Option<Arrangement>> = std::iter::once(None)
        .chain(Arrangement::ALL.into_iter().filter(|a| a.built()).map(Some))
        .collect();
    let mut at = None;
    for want in built.iter().cycle().skip(1).take(built.len()) {
        at = step_remembered(at, 1);
        assert_eq!(at, *want);
    }
    assert_eq!(at, None, "round to the default");
    assert_eq!(
        step_remembered(None, -1),
        *built.last().expect("one at least")
    );
}

/// A four-seat duel model at home, under `arrangement`, its board seated.
fn seated_duel(arrangement: Arrangement) -> Duel {
    use baylee_client_core::test_support::{ViewBuilder, statics};
    let mut duel = Duel::default();
    let mut table = statics(0);
    table.seats = (0..4)
        .map(|i| baylee_view::SeatIdentity {
            player: baylee_core::ids::PlayerId::new(i),
            display_name: format!("Seat {i}"),
            is_ai: i != 0,
            away: false,
            team: None,
        })
        .collect();
    duel.statics = Some(table);
    duel.receive_view(ViewBuilder::new(4).build());
    duel.arrangement = arrangement;
    crate::rebuild_board(&mut duel);
    duel
}

fn parked(duel: &Duel) -> Vec<u8> {
    duel.settled_layout()
        .expect("seated")
        .slots
        .iter()
        .filter(|s| s.parked)
        .map(|s| s.player.get())
        .collect()
}

/// A layout arrangement is seated again whenever the seat of interest
/// moves, whoever moved it — here `navigate_to_player`, which knows nothing
/// of arrangements (v7's note: it does not call `rebuild_board`).
#[test]
fn a_layout_arrangement_follows_the_seat_of_interest_wherever_it_moved() {
    use baylee_core::ids::PlayerId;
    let mut app = App::new();
    app.insert_resource(seated_duel(Arrangement::Spotlight))
        .add_systems(Update, lay_the_interest);
    assert_eq!(
        parked(app.world().resource::<Duel>()),
        vec![1, 3],
        "home: seat 2 across"
    );
    crate::input::navigate_to_player(
        &mut app.world_mut().resource_mut::<Duel>(),
        PlayerId::new(1),
    );
    app.update();
    assert_eq!(
        parked(app.world().resource::<Duel>()),
        vec![2, 3],
        "seat 1 across"
    );
    crate::input::navigate_home(&mut app.world_mut().resource_mut::<Duel>());
    app.update();
    assert_eq!(
        parked(app.world().resource::<Duel>()),
        vec![1, 3],
        "home again"
    );
    // A camera arrangement never re-seats: the visit moves the camera.
    let mut ring = App::new();
    ring.insert_resource(seated_duel(Arrangement::Ring))
        .add_systems(Update, lay_the_interest);
    let before = ring.world().resource::<Duel>().layout.clone();
    crate::input::navigate_to_player(
        &mut ring.world_mut().resource_mut::<Duel>(),
        PlayerId::new(1),
    );
    ring.update();
    assert_eq!(ring.world().resource::<Duel>().layout, before);
    assert_eq!(
        ring.world().resource::<Duel>().camera_visit(),
        Some(PlayerId::new(1))
    );
}

/// *Tisch folgt dem Zug*: off, another player's turn shows nothing; on, it
/// shows the active seat — deferred while a question is open for me, shown
/// at the first view without one — and my own turn brings the table home.
#[test]
fn the_follow_switch_shows_the_active_seat_and_waits_out_my_question() {
    use baylee_client_core::test_support::ViewBuilder;
    use baylee_core::ids::PlayerId;
    let turn = |active: u8, awaiting: Option<u8>| {
        let mut view = ViewBuilder::new(4).with_awaiting(awaiting).build();
        view.active = PlayerId::new(active);
        view
    };
    let mut duel = seated_duel(Arrangement::Spotlight);
    duel.receive_view(turn(1, Some(1)));
    assert_eq!(duel.visiting, None, "off by default");

    let mut duel = seated_duel(Arrangement::Spotlight);
    duel.follow = true;
    duel.receive_view(turn(1, Some(1)));
    assert_eq!(duel.visiting, Some(PlayerId::new(1)));
    assert!(
        duel.follow_settling,
        "a Space now is dropped until it settles"
    );
    duel.receive_view(turn(2, Some(0)));
    assert_eq!(
        duel.visiting,
        Some(PlayerId::new(1)),
        "my question holds the table"
    );
    assert_eq!(duel.follow_pending, Some(PlayerId::new(2)));
    duel.receive_view(turn(2, Some(2)));
    assert_eq!(duel.visiting, Some(PlayerId::new(2)), "shown once answered");
    duel.receive_view(turn(0, Some(0)));
    assert_eq!(duel.visiting, None, "my turn is home");
}

/// The owner's tear, run in the client: a chip press on a Spotlight table
/// starts it, its stages come in order (split, swing, dock, settle) as the
/// layout the cards glide to, the new seat drawn only once its piece is, it is over in about a second, and it ends on
/// exactly the instant layout. Under reduced motion it is the cut.
#[test]
fn a_change_of_seat_tears_the_table_and_docks_on_the_instant_layout() {
    use baylee_client_core::layout::transition::Phase;
    use baylee_core::ids::PlayerId;
    for still in [false, true] {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<crate::prefs::Prefs>()
            .insert_resource(seated_duel(Arrangement::Spotlight))
            .add_systems(Update, (lay_the_interest, run_the_tear).chain());
        app.world_mut()
            .resource_mut::<crate::prefs::Prefs>()
            .edit()
            .reduce_motion = still;
        let instant = {
            let mut duel = seated_duel(Arrangement::Spotlight);
            duel.visiting = Some(PlayerId::new(1));
            crate::rebuild_board(&mut duel);
            duel.layout.expect("seated")
        };
        crate::input::navigate_to_player(
            &mut app.world_mut().resource_mut::<Duel>(),
            PlayerId::new(1),
        );
        let step = std::time::Duration::from_secs_f32(1.0 / 60.0);
        let mut phases = Vec::new();
        let mut frames = 0;
        loop {
            app.world_mut().resource_mut::<Time>().advance_by(step);
            app.update();
            frames += 1;
            let duel = app.world().resource::<Duel>();
            let Some(tear) = duel.tear.as_ref() else {
                break;
            };
            if phases.last() != Some(&tear.phase()) {
                phases.push(tear.phase());
            }
            assert_eq!(tear.to(), &instant, "the tear ends where the cut would");
            if matches!(tear.phase(), Phase::Split | Phase::Swing | Phase::Dock) {
                assert_ne!(
                    duel.layout.as_ref(),
                    Some(&instant),
                    "a stage the cards glide to"
                );
            }
            let across = duel
                .layout
                .as_ref()
                .and_then(|l| l.slot(PlayerId::new(1)))
                .is_some_and(|s| !s.parked);
            assert_eq!(
                across,
                tear.plan
                    .pose(
                        baylee_client_core::layout::transition::Piece::Arriving,
                        tear.t
                    )
                    .shown,
                "the new seat comes up once the old one has gone, not before"
            );
            assert!(frames < 120, "the tear never ended");
        }
        let duel = app.world().resource::<Duel>();
        assert_eq!(
            duel.layout.as_ref(),
            Some(&instant),
            "docked on the instant layout"
        );
        if still {
            assert!(phases.is_empty(), "reduced motion is the cut");
            assert_eq!(frames, 1);
        } else {
            assert_eq!(
                phases,
                vec![Phase::Split, Phase::Swing, Phase::Dock, Phase::Settle],
                "every stage, in order"
            );
            assert!(
                (50..=66).contains(&frames),
                "about a second: {frames} frames"
            );
        }
    }
}

/// The follow switch is on unless a player turned it off (the owner,
/// 07.10.2026): a device with default settings, an opponent's turn
/// beginning, and that seat is the seat of interest — red while it was off
/// by default; turned off, nothing moves.
#[test]
fn by_default_the_table_follows_the_turn() {
    use baylee_client_core::test_support::ViewBuilder;
    use baylee_core::ids::PlayerId;
    for on in [true, false] {
        let mut app = App::new();
        let mut settings = crate::settings::ClientSettings::default();
        if !on {
            settings.table.follow = false;
        }
        app.init_resource::<Time>()
            .init_resource::<ArrangementFrame>()
            .insert_resource(settings)
            .insert_resource(seated_duel(Arrangement::Spotlight))
            .add_systems(Update, choose);
        app.update();
        let mut view = ViewBuilder::new(4).with_awaiting(Some(1)).build();
        view.active = PlayerId::new(1);
        app.world_mut().resource_mut::<Duel>().receive_view(view);
        let visiting = app.world().resource::<Duel>().visiting;
        if on {
            assert_eq!(
                visiting,
                Some(PlayerId::new(1)),
                "the opponent's turn is shown"
            );
        } else {
            assert_eq!(visiting, None, "off: nothing moves");
        }
    }
}

/// The Turntable's change of seat of interest is two slides, not a tear
/// (DESIGN-v8 §1 row 3): every seat stands on the felt, so the cards glide
/// straight to the instant layout — the side mat brought across at the
/// duel's size, the one across to its flank at the side mats'. Red while
/// every layout arrangement tore.
#[test]
fn a_turntable_change_slides_to_the_instant_layout_without_a_tear() {
    use baylee_core::ids::PlayerId;
    let mut app = App::new();
    app.init_resource::<Time>()
        .init_resource::<crate::prefs::Prefs>()
        .insert_resource(seated_duel(Arrangement::Turntable))
        .add_systems(Update, (lay_the_interest, run_the_tear).chain());
    let instant = {
        let mut duel = seated_duel(Arrangement::Turntable);
        duel.visiting = Some(PlayerId::new(1));
        crate::rebuild_board(&mut duel);
        duel.layout.expect("seated")
    };
    let before = app
        .world()
        .resource::<Duel>()
        .layout
        .as_ref()
        .and_then(|l| l.slot(PlayerId::new(1)).copied())
        .expect("seat 1");
    assert!(before.scale < 1.0, "seat 1 starts on a flank");
    crate::input::navigate_to_player(
        &mut app.world_mut().resource_mut::<Duel>(),
        PlayerId::new(1),
    );
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f32(1.0 / 60.0));
    app.update();
    let duel = app.world().resource::<Duel>();
    assert!(duel.tear.is_none(), "no tear");
    assert_eq!(duel.layout.as_ref(), Some(&instant), "the instant layout");
    let across = instant.slot(PlayerId::new(1)).expect("seat 1");
    assert!(
        (across.scale - 1.0).abs() < 1e-6,
        "brought across at the duel's size"
    );
}

/// The Focus ring's peeks (DESIGN-v8 §1 row 8): at home on four seats the
/// pair is me and seat 2, seat 1 peeks from the left column and seat 3
/// from the right; brought across, seat 1 leaves its peek and seat 2 takes
/// one. Another arrangement has no peeks. The camera frames the table
/// between the two columns, whose width is the window's alone (invariant
/// 5's one declared exception: constant against the game).
#[test]
fn the_focus_ring_peeks_beside_the_pair_and_frames_the_table_between() {
    use crate::hud::peeks::{column_width, columns};
    use baylee_core::ids::PlayerId;
    let ids = |list: &[crate::hud::peeks::PeekFacts]| -> Vec<u8> {
        list.iter().map(|f| f.player.get()).collect()
    };
    let mut duel = seated_duel(Arrangement::FocusRing);
    let (left, right) = columns(&duel, Lang::En);
    assert_eq!((ids(&left), ids(&right)), (vec![1], vec![3]));
    crate::input::navigate_to_player(&mut duel, PlayerId::new(1));
    crate::rebuild_board(&mut duel);
    let (left, right) = columns(&duel, Lang::En);
    assert_eq!((ids(&left), ids(&right)), (vec![], vec![2, 3]));
    let spotlight = seated_duel(Arrangement::Spotlight);
    assert_eq!(columns(&spotlight, Lang::En), (vec![], vec![]), "no peeks");

    for window in [Vec2::new(1708.0, 1028.0), Vec2::new(1000.0, 760.0)] {
        let frame = TableFrame::of(window.x, window.y);
        let canvas = crate::table::Canvas::for_table(window, Arrangement::FocusRing);
        assert!((canvas.left - column_width(frame)).abs() < 1e-6);
        assert!((canvas.right - column_width(frame)).abs() < 1e-6);
        let plain = crate::table::Canvas::for_table(window, Arrangement::Spotlight);
        assert!(plain.left.abs() < 1e-6 && plain.right.abs() < 1e-6);
        assert!(
            canvas.aspect() < plain.aspect(),
            "the table framed between them"
        );
    }
}

/// A peek shows its parked board as chips (WA9): seat 1's thirty Soldiers
/// are one chip on its peek, counted; a chip stands for the pile's
/// representative, which a press answers with as the card on the table.
#[test]
fn a_peek_shows_its_parked_board_as_chips() {
    use baylee_client_core::test_support::{ViewBuilder, token};
    let mut duel = seated_duel(Arrangement::FocusRing);
    let soldiers: Vec<_> = (0..30)
        .map(|i| token(500 + i, 1, "Soldier", 1, 1))
        .collect();
    duel.receive_view(ViewBuilder::new(4).with_battlefield(1, soldiers).build());
    crate::rebuild_board(&mut duel);
    let (left, _) = crate::hud::peeks::columns(&duel, Lang::En);
    let seat_one = left.first().expect("seat 1 peeks from the left");
    let chip = seat_one.chips.chips.first().expect("a chip");
    assert_eq!(chip.count, 30, "thirty tokens, one chip");
    assert!(
        (500..530).contains(&chip.object.slot()),
        "the pile's representative: {:?}",
        chip.object
    );
}

/// The hand's drawer on a phone (DESIGN-v8 WA11): shut by default, the
/// actions bar and the hand zone stand the cards' height lower and the
/// table's canvas reaches down to the bar; `I` opens it in 0.25–0.3 s, back
/// to where they stood; reduced motion is the cut; a desktop window has no
/// drawer and nothing is written; and while the table is still being
/// prepared under its cover the drawer stands open (the entrance shows the
/// screen as it is first played). Red before the drawer: the zone stood
/// where it stands open, always.
#[test]
fn the_hand_slides_down_on_a_phone_and_up_on_its_key() {
    use crate::hud::hand_drawer::{drop_at, slide_the_hand};
    use crate::hud::{HAND_ZONE_H, LEDGE_H, LedgeShelf};
    let run_in = |phase: DuelPhase, width: f32, height: f32, still: bool| {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<crate::prefs::Prefs>()
            .insert_resource(State::new(phase))
            .insert_resource(seated_duel(Arrangement::Ring))
            .add_systems(Update, slide_the_hand);
        app.world_mut()
            .resource_mut::<crate::prefs::Prefs>()
            .edit()
            .reduce_motion = still;
        app.world_mut().spawn(Window {
            resolution: bevy::window::WindowResolution::new(width as u32, height as u32),
            ..default()
        });
        // The actions bar as the HUD stands it, and a dialog pinned top and
        // bottom beside it, which does not ride.
        let hud = app.world_mut().spawn(crate::hud::HudRoot).id();
        let shelf = app
            .world_mut()
            .spawn((
                LedgeShelf,
                Node {
                    position_type: PositionType::Absolute,
                    bottom: px(HAND_ZONE_H - LEDGE_H),
                    ..default()
                },
                UiTransform::default(),
                ChildOf(hud),
            ))
            .id();
        let dialog = app
            .world_mut()
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: px(60.0),
                    bottom: px(HAND_ZONE_H),
                    ..default()
                },
                UiTransform::default(),
                ChildOf(hud),
            ))
            .id();
        let step = std::time::Duration::from_secs_f32(1.0 / 60.0);
        let dropped = |app: &App| match app
            .world()
            .get::<UiTransform>(shelf)
            .map(|t| t.translation.y)
        {
            Some(Val::Px(y)) => y,
            _ => 0.0,
        };
        for _ in 0..30 {
            app.world_mut().resource_mut::<Time>().advance_by(step);
            app.update();
        }
        let shut = dropped(&app);
        assert_eq!(
            app.world()
                .get::<UiTransform>(dialog)
                .map(|t| t.translation.y),
            Some(Val::Px(0.0)),
            "a dialog pinned top and bottom stays"
        );
        app.world_mut().resource_mut::<Duel>().toggle_hand_drawer();
        let mut frames = 0;
        loop {
            app.world_mut().resource_mut::<Time>().advance_by(step);
            app.update();
            frames += 1;
            if dropped(&app).abs() < 1e-3 || frames > 60 {
                break;
            }
        }
        (shut, frames)
    };
    let run = |width, height, still| run_in(DuelPhase::Playing, width, height, still);
    let (shut, frames) = run(844.0, 390.0, false);
    assert!(
        (shut - (HAND_ZONE_H - LEDGE_H)).abs() < 1e-3,
        "shut: the cards' height lower"
    );
    assert!((15..=19).contains(&frames), "opened in {frames} frames");
    assert!((drop_at(1.0)).abs() < 1e-6);
    let (_, frames) = run(844.0, 390.0, true);
    assert_eq!(frames, 1, "reduced motion: the cut");
    let (desk, _) = run(1708.0, 1028.0, false);
    assert!(desk.abs() < 1e-6, "no drawer off a phone");
    let (opening, _) = run_in(DuelPhase::Opening, 844.0, 390.0, false);
    assert!(
        opening.abs() < 1e-6,
        "open while the table is being prepared"
    );

    let phone = crate::table::Canvas::hud(Vec2::new(844.0, 390.0));
    assert!((phone.with_drawer(false).bottom - LEDGE_H).abs() < 1e-6);
    assert!((phone.with_drawer(true).bottom - HAND_ZONE_H).abs() < 1e-6);
    let desk = crate::table::Canvas::hud(Vec2::new(1708.0, 1028.0));
    assert!((desk.with_drawer(false).bottom - HAND_ZONE_H).abs() < 1e-6);
}

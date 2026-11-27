//! The old release contracts, exercised through Bevy's real pointer
//! observers and the drawer's production handler. Toggles expose a second
//! activation: counting widget events alone would miss a domain double click.

use super::*;
use crate::ui_button::testing;
use bevy::picking::backend::HitData;
use bevy::picking::hover::HoverMap;
use bevy::picking::pointer::{
    PointerAction, PointerButton, PointerId, PointerInput, PointerLocation, PointerMap,
};
use bevy::ui::Pressed;

fn app() -> App {
    let mut app = testing::app();
    app.init_resource::<SocialHost>()
        .init_resource::<ChatUi>()
        .add_systems(Update, presses);
    app
}

fn control(app: &mut App, press: DrawerPress) -> Entity {
    let entity = small_button(
        &mut app.world_mut().commands(),
        &UiFonts::default(),
        "test control",
        press,
    );
    app.world_mut().flush();
    entity
}

fn point(app: &mut App, target: Option<Entity>, actions: &[PointerAction]) {
    testing::frame(app, PointerId::Mouse, target, actions);
}

#[test]
fn a_press_released_over_its_control_clicks_once() {
    let mut app = app();
    let toggle = control(&mut app, DrawerPress::Toggle);
    point(
        &mut app,
        Some(toggle),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    assert!(app.world().get::<Pressed>(toggle).is_some());
    assert!(!app.world().resource::<ChatUi>().drawer, "down is no click");
    point(
        &mut app,
        Some(toggle),
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert!(
        app.world().resource::<ChatUi>().drawer,
        "one toggle, not two"
    );
    assert!(app.world().get::<Pressed>(toggle).is_none());
    point(&mut app, None, &[]);
    point(&mut app, Some(toggle), &[]);
    assert!(
        app.world().resource::<ChatUi>().drawer,
        "hover is no second click"
    );
    assert_eq!(app.world().resource::<testing::Activations>().0, [toggle]);
}

#[test]
fn a_press_released_elsewhere_clicks_nothing() {
    let mut app = app();
    let toggle = control(&mut app, DrawerPress::Toggle);
    point(
        &mut app,
        Some(toggle),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    point(
        &mut app,
        None,
        &[PointerAction::Move {
            delta: Vec2::new(30.0, 0.0),
        }],
    );
    point(
        &mut app,
        None,
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert!(!app.world().resource::<ChatUi>().drawer);
    assert!(app.world().get::<Pressed>(toggle).is_none());
    point(&mut app, Some(toggle), &[]);
    assert!(
        !app.world().resource::<ChatUi>().drawer,
        "returning is not a release"
    );
}

#[test]
fn a_hover_that_was_never_pressed_clicks_nothing() {
    let mut app = app();
    let toggle = control(&mut app, DrawerPress::Toggle);
    for target in [Some(toggle), None, Some(toggle)] {
        point(&mut app, target, &[]);
        assert!(!app.world().resource::<ChatUi>().drawer);
        assert!(app.world().get::<Pressed>(toggle).is_none());
    }
}

#[test]
fn a_release_over_another_control_clicks_neither() {
    let mut app = app();
    let toggle = control(&mut app, DrawerPress::Toggle);
    let more = control(&mut app, DrawerPress::More("alice".into()));
    point(
        &mut app,
        Some(toggle),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    point(
        &mut app,
        Some(more),
        &[PointerAction::Move {
            delta: Vec2::new(30.0, 0.0),
        }],
    );
    point(
        &mut app,
        Some(more),
        &[PointerAction::Release(PointerButton::Primary)],
    );
    let ui = app.world().resource::<ChatUi>();
    assert!(!ui.drawer);
    assert!(ui.drawer_menu.is_none());
    assert!(app.world().get::<Pressed>(toggle).is_none());
    assert!(app.world().get::<Pressed>(more).is_none());
}

#[test]
fn two_controls_pressed_in_one_frame_each_click_on_their_own_release() {
    let mut app = app();
    let toggle = control(&mut app, DrawerPress::Toggle);
    let more = control(&mut app, DrawerPress::More("alice".into()));
    // Queue both raw touch downs before advancing the app once. No widget
    // component or synthetic Click is written by this test.
    for (id, target) in [(PointerId::Touch(1), toggle), (PointerId::Touch(2), more)] {
        app.world_mut().resource_mut::<HoverMap>().insert(
            id,
            [(target, HitData::new(Entity::PLACEHOLDER, 0.0, None, None))].into(),
        );
        let pointer = app.world().resource::<PointerMap>().get_entity(id).unwrap();
        let location = app
            .world()
            .get::<PointerLocation>(pointer)
            .unwrap()
            .location
            .clone()
            .unwrap();
        app.world_mut().write_message(PointerInput::new(
            id,
            location,
            PointerAction::Press(PointerButton::Primary),
        ));
    }
    app.update();
    assert!(app.world().get::<Pressed>(toggle).is_some());
    assert!(app.world().get::<Pressed>(more).is_some());
    assert!(!app.world().resource::<ChatUi>().drawer);
    assert!(app.world().resource::<ChatUi>().drawer_menu.is_none());
    testing::frame(
        &mut app,
        PointerId::Touch(2),
        None,
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert_eq!(
        app.world().resource::<ChatUi>().drawer_menu.as_deref(),
        Some("alice")
    );
    assert!(!app.world().resource::<ChatUi>().drawer);
    assert!(app.world().get::<Pressed>(toggle).is_some());
    testing::frame(
        &mut app,
        PointerId::Touch(1),
        None,
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert!(app.world().resource::<ChatUi>().drawer);
    assert_eq!(
        app.world().resource::<ChatUi>().drawer_menu.as_deref(),
        Some("alice")
    );
    assert!(app.world().get::<Pressed>(toggle).is_none());
    assert!(app.world().get::<Pressed>(more).is_none());
    assert_eq!(
        app.world().resource::<testing::Activations>().0,
        [more, toggle]
    );
}

#[test]
fn a_second_press_of_the_same_control_clicks_again() {
    let mut app = app();
    let toggle = control(&mut app, DrawerPress::Toggle);
    testing::click(&mut app, toggle);
    assert!(app.world().resource::<ChatUi>().drawer);
    testing::click(&mut app, toggle);
    assert!(!app.world().resource::<ChatUi>().drawer);
    assert_eq!(
        app.world().resource::<testing::Activations>().0,
        [toggle, toggle]
    );
}

/// A list a person can read through stays whole, whatever it is scrolled to.
#[test]
fn a_short_list_of_friends_draws_every_row() {
    assert_eq!(row_window(DRAWN_WHOLE, 3, 2), 0..DRAWN_WHOLE);
}

/// The window reaches over what is on screen from the top to the last row,
/// and never past the end.
#[test]
fn a_long_list_draws_the_rows_around_where_it_is_scrolled_to() {
    let total = 500;
    let at_top = row_window(total, 0, 3);
    assert_eq!(at_top, 0..WINDOW_ROWS);
    let row = 200;
    let window = row_window(total, row / WINDOW_STEP, 3);
    assert!(window.start < row && row + 30 < window.end, "{window:?}");
    assert_eq!(window.len(), WINDOW_ROWS);
    let at_end = row_window(total, total / WINDOW_STEP, 3);
    assert_eq!(at_end, total - WINDOW_ROWS..total);
}

/// What is not drawn stands in as height, in one piece on each side of the
/// window, so the list's length and scrollbar hold.
#[test]
fn the_rows_left_out_keep_the_lists_length() {
    let mut app = app();
    let mut commands = app.world_mut().commands();
    let mut kids = Vec::new();
    let mut window = RowWindow::new(10..12, 50.0);
    let drawn = (0..30)
        .filter(|_| window.takes_row(&mut commands, &mut kids))
        .count();
    window.close(&mut commands, &mut kids);
    app.update();
    assert_eq!(drawn, 2);
    let heights: Vec<Val> = kids
        .iter()
        .map(|kid| app.world().get::<Node>(*kid).expect("a spacer").height)
        .collect();
    assert_eq!(heights, [Val::Px(500.0), Val::Px(900.0)]);
}

/// The window moves in steps smaller than its reach above the view, so a
/// scroll never lands on rows it has not drawn yet.
#[test]
fn the_window_moves_before_the_view_reaches_its_edge() {
    let pitch = 56.0;
    assert_eq!(window_chunk(0.0, pitch), 0);
    assert_eq!(window_chunk(-30.0, pitch), 0);
    assert_eq!(window_chunk(pitch * 8.5, pitch), 1);
}

fn app_with_machine() -> App {
    use baylee_client_core::social::{ChatPrefs, Me, Social};

    let (machine, _) = Social::start(
        Me {
            tag: "AB12".into(),
            handle: "Alice#AB12".into(),
            guest: false,
            gateway: "https://example.test".into(),
        },
        None,
        None,
        ChatPrefs::default(),
        1_000,
        &mut crate::social::OsEntropy,
    );
    let mut app = app();
    app.insert_resource(UiFonts::default())
        .init_resource::<crate::social::zoom::Zoom>()
        .insert_resource(SocialHost {
            machine: Some(machine),
            ..default()
        });
    app
}

#[test]
fn door_and_drawer_use_single_layer_glass_grounds() {
    use bevy::ecs::system::RunSystemOnce;

    let mut app = app_with_machine();
    app.insert_resource(State::new(crate::DuelPhase::Closed));
    app.world_mut().resource_mut::<ChatUi>().drawer = true;
    app.world_mut().run_system_once(sync_door).unwrap();
    app.world_mut().run_system_once(sync_drawer).unwrap();

    let door = app
        .world_mut()
        .query_filtered::<Entity, With<FriendsDoor>>()
        .single(app.world())
        .unwrap();
    assert_eq!(
        app.world().get::<BackgroundColor>(door).unwrap().0,
        crate::social::rows::PILL_GLASS
    );
    assert!(
        app.world()
            .get::<bevy::ui::BackgroundGradient>(door)
            .is_none()
    );

    let drawer = app
        .world_mut()
        .query_filtered::<Entity, With<Drawer>>()
        .single(app.world())
        .unwrap();
    assert_eq!(
        app.world().get::<BackgroundColor>(drawer).unwrap().0,
        Color::NONE
    );
}

#[test]
fn contacts_pill_stays_hidden_while_arrival_cover_stands() {
    use bevy::ecs::system::RunSystemOnce;

    // During DuelPhase::Opening (table loading portal), FriendsDoor must not show.
    let mut opening = app_with_machine();
    opening.insert_resource(State::new(crate::DuelPhase::Opening));
    opening.world_mut().run_system_once(sync_door).unwrap();
    assert!(
        opening
            .world_mut()
            .query_filtered::<&Visibility, With<FriendsDoor>>()
            .iter(opening.world())
            .all(|v| *v == Visibility::Hidden),
        "Contacts pill must stay hidden during DuelPhase::Opening"
    );

    // While Journey::active() holds the arrival cover, an existing FriendsDoor hides.
    let mut covered = app_with_machine();
    covered.insert_resource(State::new(crate::DuelPhase::Closed));
    covered.world_mut().run_system_once(sync_door).unwrap();
    covered.init_resource::<crate::arrival::Journey>();
    crate::arrival::start_login(&mut covered);
    covered.world_mut().run_system_once(sync_door).unwrap();
    assert!(
        covered
            .world_mut()
            .query_filtered::<&Visibility, With<FriendsDoor>>()
            .iter(covered.world())
            .all(|v| *v == Visibility::Hidden),
        "Contacts pill must hide while the arrival cover is active"
    );
}

#[test]
fn entry_prompt_distributes_buttons_symmetrically_and_disables_empty_ok_with_hint() {
    use baylee_client_core::textbuf::TextBuffer;
    use bevy::ecs::system::RunSystemOnce;

    let mut app = app_with_machine();
    app.world_mut().resource_mut::<ChatUi>().drawer = true;
    app.world_mut().resource_mut::<ChatUi>().entry = Some((Entry::NewGroup, TextBuffer::default()));
    app.world_mut().run_system_once(sync_drawer).unwrap();

    let cancel = app
        .world_mut()
        .query::<(Entity, &DrawerPress)>()
        .iter(app.world())
        .find_map(|(e, p)| (*p == DrawerPress::EntryCancel).then_some(e))
        .expect("Cancel button present");
    let has_ok_press = app
        .world_mut()
        .query::<&DrawerPress>()
        .iter(app.world())
        .any(|p| *p == DrawerPress::EntryOk);
    assert!(!has_ok_press, "empty NewGroup must disable EntryOk");

    let row = app.world().get::<ChildOf>(cancel).unwrap().parent();
    assert_eq!(
        app.world().get::<Node>(row).unwrap().width,
        Val::Percent(100.0)
    );
    let kids: Vec<Entity> = app.world().get::<Children>(row).unwrap().iter().collect();
    assert_eq!(kids, [kids[0], cancel]);
    for btn in [kids[0], cancel] {
        let node = app.world().get::<Node>(btn).unwrap();
        assert!((node.flex_grow - 1.0).abs() < f32::EPSILON);
        assert_eq!(node.flex_basis, Val::Px(0.0));
    }
    assert!(app.world().get::<UiButton>(kids[0]).is_none());
    assert!(
        app.world()
            .get::<crate::shellkit::controls::Disabled>(kids[0])
            .is_some()
    );
    assert!(
        app.world()
            .get::<crate::shellkit::controls::Disabled>(cancel)
            .is_none()
    );
    let tip = app
        .world()
        .get::<crate::shellkit::controls::Tooltip>(kids[0])
        .expect("disabled OK button carries floating kit tooltip")
        .0;
    assert!(app.world().get::<ChildOf>(tip).is_none());
    let tip_text = app.world().get::<Children>(tip).unwrap()[0];
    assert_eq!(
        app.world().get::<Text>(tip_text).unwrap().0,
        ChatPhrase::GroupNameLabel.text(Lang::En)
    );

    app.world_mut().resource_mut::<ChatUi>().entry =
        Some((Entry::NewGroup, TextBuffer::new("Cube Pod")));
    app.world_mut().run_system_once(sync_drawer).unwrap();
    assert!(
        app.world_mut()
            .query::<(&DrawerPress, Has<UiButton>)>()
            .iter(app.world())
            .any(|(p, b)| *p == DrawerPress::EntryOk && b)
    );

    app.world_mut().resource_mut::<ChatUi>().entry =
        Some((Entry::Nickname("CD34".into()), TextBuffer::default()));
    app.world_mut().run_system_once(sync_drawer).unwrap();
    assert!(
        app.world_mut()
            .query::<(&DrawerPress, Has<UiButton>)>()
            .iter(app.world())
            .any(|(p, b)| *p == DrawerPress::EntryOk && b)
    );
}

/// The status menu as drawn: every press it offers, and its heading's words.
fn status_menu(app: &mut App) -> (Vec<DrawerPress>, Vec<String>) {
    use bevy::ecs::system::RunSystemOnce;

    app.world_mut().run_system_once(sync_context_menu).unwrap();
    let presses = app
        .world_mut()
        .query::<&DrawerPress>()
        .iter(app.world())
        .filter(|press| matches!(press, DrawerPress::Choose(_) | DrawerPress::Quiet(_)))
        .cloned()
        .collect();
    let words = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect();
    (presses, words)
}

fn press_of(app: &mut App, wanted: &DrawerPress) -> Entity {
    app.world_mut()
        .query::<(Entity, &DrawerPress)>()
        .iter(app.world())
        .find_map(|(entity, press)| (press == wanted).then_some(entity))
        .expect("the control is drawn")
}

/// The status is chosen from a menu of three, and do not disturb is a switch
/// beside them that is no fourth choice (`docs/client.md` §"The contact
/// status").
#[test]
fn the_status_menu_offers_three_choices_and_a_switch_of_this_devices() {
    use baylee_client_core::social::presence::Choice;

    let mut app = app_with_machine();
    app.init_resource::<DrawerMenuAt>();
    app.world_mut().resource_mut::<ChatUi>().drawer = true;
    let status = control(&mut app, DrawerPress::Status);
    testing::click(&mut app, status);
    assert_eq!(
        app.world().resource::<ChatUi>().drawer_menu.as_deref(),
        Some(STATUS_MENU)
    );
    let (presses, _) = status_menu(&mut app);
    assert_eq!(
        presses,
        [
            DrawerPress::Choose(Choice::Online),
            DrawerPress::Choose(Choice::Afk),
            DrawerPress::Choose(Choice::Offline),
            DrawerPress::Quiet(true),
        ]
    );

    let afk = press_of(&mut app, &DrawerPress::Choose(Choice::Afk));
    testing::click(&mut app, afk);
    let host = app.world().resource::<SocialHost>();
    assert_eq!(host.machine().unwrap().choice(), Choice::Afk);
    assert!(!host.machine().unwrap().quiet());
    assert_eq!(app.world().resource::<ChatUi>().drawer_menu, None);

    // The switch keeps the choice, and the menu then offers to turn it off.
    app.world_mut().resource_mut::<ChatUi>().drawer_menu = Some(STATUS_MENU.to_owned());
    status_menu(&mut app);
    let quiet = press_of(&mut app, &DrawerPress::Quiet(true));
    testing::click(&mut app, quiet);
    let host = app.world().resource::<SocialHost>();
    assert_eq!(host.machine().unwrap().choice(), Choice::Afk);
    assert!(host.machine().unwrap().quiet());
    app.world_mut().resource_mut::<ChatUi>().drawer_menu = Some(STATUS_MENU.to_owned());
    let (presses, words) = status_menu(&mut app);
    assert!(presses.contains(&DrawerPress::Quiet(false)));
    assert!(!presses.contains(&DrawerPress::Quiet(true)));
    let quiet_hint = ChatPhrase::QuietHint.text(Lang::En);
    assert!(words.iter().any(|line| line == quiet_hint), "{words:?}");

    // Offline says what it means in the heading, before the quiet.
    let offline = press_of(&mut app, &DrawerPress::Choose(Choice::Offline));
    testing::click(&mut app, offline);
    app.world_mut().resource_mut::<ChatUi>().drawer_menu = Some(STATUS_MENU.to_owned());
    let (_, words) = status_menu(&mut app);
    let offline_hint = ChatPhrase::OfflineHint.text(Lang::En);
    assert!(words.iter().any(|line| line == offline_hint), "{words:?}");
}

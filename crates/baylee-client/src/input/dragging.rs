use super::*;
use crate::hud::{TrayGrip, TrayPanel, TrayResize};
use bevy::picking::events::{Pointer, Press, Release};
use bevy::picking::pointer::{Location, PointerId};
use bevy::window::{PrimaryWindow, WindowRef};

/// A window with a sheet in it and nothing else.
///
/// The window is given a cursor because the system reads one: a press
/// with no cursor position starts no drag, which is the branch that keeps
/// a harness that never moved a mouse from throwing the sheet across the
/// band.
fn harness() -> (App, Entity, Entity, Entity) {
    let mut app = App::new();
    app.add_message::<Pointer<Press>>()
        .add_message::<Pointer<Release>>()
        .init_resource::<Duel>()
        .init_resource::<ClientSettings>()
        .add_systems(Update, tray_drag);
    let mut window = Window::default();
    window.resolution.set(1728.0, 1052.0);
    window.set_cursor_position(Some(Vec2::new(800.0, 400.0)));
    let win = app.world_mut().spawn((window, PrimaryWindow)).id();
    // The node the overlay would have built: an explicit rectangle, so
    // that "it moved" is a comparison of two numbers rather than of a
    // number against `Auto`.
    let band = (1728.0, 1052.0 - crate::hud::EDGE - crate::hud::HAND_ZONE_H);
    let home = Placement::centred(band);
    let panel = app
        .world_mut()
        .spawn((
            TrayPanel,
            Node {
                position_type: PositionType::Absolute,
                left: px(home.left),
                top: px(home.top),
                width: px(home.width),
                height: px(home.height),
                ..default()
            },
        ))
        .id();
    let grip = app.world_mut().spawn((TrayGrip, Node::default())).id();
    let corner = app.world_mut().spawn((TrayResize, Node::default())).id();
    app.world_mut()
        .entity_mut(panel)
        .add_children(&[grip, corner]);
    let _ = win;
    (app, panel, grip, corner)
}

/// Where the pointer is, as the picking backend would report it.
fn at(app: &mut App, position: Vec2) -> Location {
    use bevy::camera::NormalizedRenderTarget;
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("the harness made a window");
    let target = WindowRef::Entity(window)
        .normalize(Some(window))
        .expect("a window is a render target");
    Location {
        target: NormalizedRenderTarget::Window(target),
        position,
    }
}

fn press(app: &mut App, entity: Entity) {
    let location = at(app, Vec2::ZERO);
    let event = Press {
        button: bevy::picking::pointer::PointerButton::Primary,
        hit: bevy::picking::backend::HitData::new(entity, 0.0, None, None),
        count: 1,
    };
    app.world_mut()
        .write_message(Pointer::new(PointerId::Mouse, location, event, entity));
}

fn release(app: &mut App, entity: Entity) {
    let location = at(app, Vec2::ZERO);
    let event = Release {
        button: bevy::picking::pointer::PointerButton::Primary,
        hit: bevy::picking::backend::HitData::new(entity, 0.0, None, None),
    };
    app.world_mut()
        .write_message(Pointer::new(PointerId::Mouse, location, event, entity));
}

/// Moves the cursor and runs one frame.
fn cursor_to(app: &mut App, position: Vec2) {
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("the harness made a window");
    app.world_mut()
        .entity_mut(window)
        .get_mut::<Window>()
        .expect("a window")
        .set_cursor_position(Some(position));
    app.update();
}

fn node_of(app: &App, panel: Entity) -> Node {
    app.world()
        .entity(panel)
        .get::<Node>()
        .expect("a node")
        .clone()
}

#[test]
fn dragging_the_header_moves_the_sheet() {
    let (mut app, panel, grip, _) = harness();
    press(&mut app, grip);
    app.update();
    let before = node_of(&app, panel);

    cursor_to(&mut app, Vec2::new(860.0, 430.0));
    let after = node_of(&app, panel);
    assert_ne!(after.left, before.left, "the sheet did not move sideways");
    assert_ne!(after.top, before.top, "the sheet did not move down");
    assert_eq!(after.width, before.width, "a drag resized it");

    // The position is remembered, and it is the *settings* that hold it
    // rather than the node — so a rebuild for any other reason lands
    // where the pointer left it rather than back in the middle.
    assert!(
        app.world()
            .resource::<ClientSettings>()
            .zone_browser
            .is_some(),
        "nothing was remembered"
    );

    // And a release lets go: the sheet stops following the pointer.
    release(&mut app, grip);
    app.update();
    let parked = node_of(&app, panel);
    cursor_to(&mut app, Vec2::new(300.0, 200.0));
    assert_eq!(
        node_of(&app, panel).left,
        parked.left,
        "the sheet is still following a pointer nobody is holding"
    );
}

#[test]
fn dragging_the_corner_stretches_the_sheet() {
    let (mut app, panel, _, corner) = harness();
    press(&mut app, corner);
    app.update();
    let before = node_of(&app, panel);

    cursor_to(&mut app, Vec2::new(900.0, 500.0));
    let after = node_of(&app, panel);
    assert_ne!(after.width, before.width, "the corner did not stretch it");
    assert_eq!(after.left, before.left, "the corner moved the sheet");
}

/// Clicking the corner fills the band, and clicking it again puts the
/// sheet back where it was.
///
/// The corner draws a ⤢ and is read as a maximise button, and it was a
/// drag handle and nothing else — a click on it did nothing at all, which
/// is what the owner reported. The two gestures share one control, so the
/// third case is the one that matters: a *drag* must not maximise.
#[test]
fn clicking_the_corner_maximises_and_restores_the_sheet() {
    let (mut app, panel, _, corner) = harness();
    let band = (1728.0, 1052.0 - crate::hud::EDGE - crate::hud::HAND_ZONE_H);
    let home = node_of(&app, panel);

    press(&mut app, corner);
    app.update();
    release(&mut app, corner);
    app.update();
    let full = node_of(&app, panel);
    assert_ne!(full.width, home.width, "the click did nothing");
    assert_eq!(full.width, px(Placement::maximised(band).width));
    assert_eq!(full.height, px(Placement::maximised(band).height));

    press(&mut app, corner);
    app.update();
    release(&mut app, corner);
    app.update();
    let back = node_of(&app, panel);
    assert_eq!(back.width, home.width, "it did not go back");
    assert_eq!(back.left, home.left, "it went back somewhere else");
}

/// And a drag on the corner is a resize, never a maximise.
#[test]
fn dragging_the_corner_does_not_maximise_it() {
    let (mut app, panel, _, corner) = harness();
    let band = (1728.0, 1052.0 - crate::hud::EDGE - crate::hud::HAND_ZONE_H);
    press(&mut app, corner);
    app.update();
    cursor_to(&mut app, Vec2::new(900.0, 500.0));
    let stretched = node_of(&app, panel);
    release(&mut app, corner);
    app.update();

    assert_eq!(
        node_of(&app, panel).width,
        stretched.width,
        "letting go of a resize maximised the sheet"
    );
    assert_ne!(
        node_of(&app, panel).width,
        px(Placement::maximised(band).width)
    );
}

/// The ✕ stands *on* the header, and pressing it must not start a move.
///
/// It would be a slow leak rather than a visible bug: a hand that wobbles
/// a pixel between the press and the release moves the sheet a pixel, the
/// release saves it, and the sheet creeps a little further from where it
/// was put with every close.
#[test]
fn pressing_the_close_button_does_not_start_a_drag() {
    let (mut app, panel, grip, _) = harness();
    let close = app.world_mut().spawn((TrayClose, Node::default())).id();
    app.world_mut().entity_mut(grip).add_children(&[close]);

    press(&mut app, close);
    app.update();
    let before = node_of(&app, panel);
    cursor_to(&mut app, Vec2::new(1000.0, 600.0));
    assert_eq!(node_of(&app, panel).left, before.left);
    assert!(
        app.world()
            .resource::<ClientSettings>()
            .zone_browser
            .is_none(),
        "closing the sheet wrote a place nobody chose"
    );
}

/// And neither does a zone tab, which is what let them move up there.
///
/// The tabs were kept out of the title row precisely so that ticking a
/// pile could not drag the sheet sideways. They went in on 14.09.2026 —
/// the owner asked for it, and the chips say what the panel is far better
/// than the word "Zonen" did — so the guarantee has to come from the same
/// place the ✕'s does: the specific control claims the press before the
/// row it stands on. A chip is pressed at every merge, so this is the
/// noisier half of the pair rather than the quieter one.
#[test]
fn pressing_a_zone_tab_does_not_start_a_drag() {
    let (mut app, panel, grip, _) = harness();
    let tab = app
        .world_mut()
        .spawn((
            crate::hud::TrayTab {
                zone: Some(baylee_client_core::BrowseZone::Stack),
            },
            Node::default(),
        ))
        .id();
    app.world_mut().entity_mut(grip).add_children(&[tab]);

    press(&mut app, tab);
    app.update();
    let before = node_of(&app, panel);
    cursor_to(&mut app, Vec2::new(1000.0, 600.0));
    assert_eq!(node_of(&app, panel).left, before.left);
    assert!(
        app.world()
            .resource::<ClientSettings>()
            .zone_browser
            .is_none(),
        "ticking a zone wrote a place nobody chose"
    );
}

/// A press somewhere else is not a drag.
#[test]
fn pressing_the_sheet_itself_does_not_move_it() {
    let (mut app, panel, _, _) = harness();
    press(&mut app, panel);
    app.update();
    let before = node_of(&app, panel);
    cursor_to(&mut app, Vec2::new(1000.0, 600.0));
    assert_eq!(node_of(&app, panel).left, before.left);
}

/// The sheet a question opened is not furniture, and cannot be rearranged.
///
/// The half that is easy to miss is the *writing*: `Browser::placement`
/// already refuses to read a stored rectangle for such a sheet, so a drag
/// that still wrote one would move the panel under the hand, snap it back
/// to the middle at the next rebuild, and leave the remembered place of
/// the hand-opened sheet somewhere nobody chose.
#[test]
fn a_sheet_a_question_opened_cannot_be_dragged() {
    use baylee_client_core::test_support::ViewBuilder;
    use baylee_core::ids::PlayerId;
    use baylee_engine::choice::{ChoicePrompt, Pending};

    let (mut app, panel, grip, _) = harness();
    let view = ViewBuilder::new(2).build();
    let asked = Pending::ChooseCards {
        player: PlayerId::new(0),
        options: vec![ObjectId::new(7, 0)],
        min: 1,
        max: 1,
        prompt: ChoicePrompt::SearchLibrary,
    };
    let interaction = baylee_client_core::interaction::Interaction::new(asked, PlayerId::new(0));
    app.world_mut()
        .resource_mut::<Duel>()
        .browser
        .follow(&view, Some(&interaction));
    assert!(
        app.world().resource::<Duel>().browser.for_choice(),
        "the harness did not open the sheet for a question"
    );

    press(&mut app, grip);
    app.update();
    let before = node_of(&app, panel);
    cursor_to(&mut app, Vec2::new(1200.0, 700.0));
    assert_eq!(
        node_of(&app, panel).left,
        before.left,
        "a question's sheet followed the pointer"
    );
    assert!(
        app.world()
            .resource::<ClientSettings>()
            .zone_browser
            .is_none(),
        "a question's sheet wrote a place the player will never see it in"
    );
}

//! The stack panel on a phone held sideways (08./09.10.2026): at 844 × 390
//! it had room for its head and its controls and showed no entry at all.

#[allow(clippy::wildcard_imports)] // the tests' shared fixtures
use super::*;
use baylee_core::ids::{DamageSourceRef, TargetRef};

/// The players' strip as a duel's two chips leave it on a phone, measured
/// live at 844 × 390 and 640 × 360: its right edge, from the window's left.
const DUEL_STRIP_RIGHT: f32 = 258.0;

/// The windows a phone is held at, smallest first: an Android phone's 640 ×
/// 360 and an iPhone's 844 × 390, with the hand's drawer shut (the phone's
/// default) and open (a player who opened it to cast in response).
const PHONES: [(f32, f32); 2] = [(640.0, 360.0), (844.0, 390.0)];

/// The bar's top edge, where the drawer has it.
fn zone_top(window: Vec2, shown: f32) -> f32 {
    window.y - crate::hud::HAND_ZONE_H + crate::hud::hand_drawer::drop_at(shown)
}

/// Every phone, either way the drawer stands, leaves the body a whole top
/// row under the panel's head; the panel ends above the bar, stands off the
/// drawer's tab and the corner, and off the players' strip. Off a phone the
/// panel stands where it always has.
#[test]
fn a_phone_leaves_the_stack_a_whole_top_row_above_the_bar() {
    for (w, h) in PHONES {
        let window = Vec2::new(w, h);
        for shown in [0.0, 1.0] {
            let room = stack::panel_room(window, shown, DUEL_STRIP_RIGHT);
            let at = format!("{w}x{h}, drawer {shown}");
            assert!(room.phone, "{at}");
            assert!(
                room.body_cap() >= stack::STACK_PHONE_FULL_HEIGHT,
                "{at}: {} under the head, a top row is {}",
                room.body_cap(),
                stack::STACK_PHONE_FULL_HEIGHT
            );
            assert!(
                room.top + room.max_height <= zone_top(window, shown),
                "{at}: the panel reaches the bar"
            );
            let left = w - room.right - room.width;
            assert!(
                room.right >= crate::hud::EDGE + crate::hud::hand_drawer::TAB_W,
                "{at}: over the hand's tab"
            );
            assert!(
                room.right >= crate::hud::BESIDE_CORNER + crate::hud::CORNER_BUTTON,
                "{at}: over the corner's buttons"
            );
            assert!(left >= DUEL_STRIP_RIGHT, "{at}: over the players' strip");
        }
    }
    // A narrow phone with many seats keeps its width and stops above the
    // strip instead of ending beside it.
    let window = Vec2::new(640.0, 360.0);
    let crowded = stack::panel_room(window, 0.0, 480.0);
    assert!(crowded.width >= 240.0, "{crowded:?}");
    assert!(
        crowded.top + crowded.max_height
            <= zone_top(window, 0.0) - crate::hud::ledge::players::STRIPS_H,
        "{crowded:?}"
    );
    // The counter-test: a desktop's panel is where it always was.
    let desk = stack::panel_room(Vec2::new(1708.0, 1032.0), 1.0, 300.0);
    assert!(!desk.phone);
    assert!((desk.top - crate::hud::TOP_CLEAR).abs() < f32::EPSILON);
    assert!((desk.right - crate::hud::EDGE).abs() < f32::EPSILON);
}

/// A logical-pixel rectangle bevy laid a node out at.
fn rect(world: &World, entity: Entity) -> Rect {
    let node = world.get::<ComputedNode>(entity).expect("laid out");
    let place = world
        .get::<bevy::ui::UiGlobalTransform>(entity)
        .expect("placed");
    let scale = node.inverse_scale_factor();
    Rect::from_center_size(place.translation * scale, node.size() * scale)
}

fn single<C: Component>(app: &mut App) -> Entity {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, With<C>>()
        .single(world)
        .expect("exactly one")
}

/// The panel of an ability that targets a creature and a player, built and
/// laid out by bevy with the shipped fonts in a phone's window, the drawer
/// `shown` open, and kept by [`stack::fold_the_stack`] as the client keeps
/// it.
fn laid_out_on_a_phone(window: Vec2, shown: f32) -> App {
    bevy::tasks::IoTaskPool::get_or_init(Default::default);
    let (mut app, fonts) = crate::face::tests::layout_app();
    let (mut duel, texts) = stack_tests::hovering_the_stack(false);
    let mut view = duel.view.clone().expect("a view");
    view.stack[0].targets = vec![
        TargetRef::Object(DamageSourceRef {
            object: ObjectId::new(7, 0),
            version: 0,
        }),
        TargetRef::Player(PlayerId::new(1)),
    ];
    duel.receive_view(view);
    crate::rebuild_board(&mut duel);
    duel.hand_shown = shown;
    let room = stack::panel_room(window, shown, DUEL_STRIP_RIGHT);
    let mut textures = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        CardTextures::new(&mut images, 1 << 20)
    };
    let assets = app.world().resource::<AssetServer>().clone();
    let settings = crate::settings::ClientSettings::default();
    let mode = crate::face::FaceMode::default();
    let root = app
        .world_mut()
        .spawn(Node {
            width: px(window.x),
            height: px(window.y),
            ..default()
        })
        .id();
    {
        let view = duel.view.as_ref().expect("a view");
        let faces = FaceCtx {
            texts: &texts,
            mode: &mode,
            settings: &settings,
            view: Some(view),
            widths: crate::face::Widths::of(None),
        };
        let mut commands = app.world_mut().commands();
        let panel = spawn_stack_panel(
            &mut commands,
            None,
            &[],
            ScrollPosition::default(),
            (
                stack::STACK_PHONE_FULL_HEIGHT,
                stack::text_lines(window.y),
                room,
            ),
            Lang::De,
            duel.board.as_ref().expect("a board"),
            view,
            duel.statics.as_ref().expect("a table"),
            &stack::Picks {
                hovered: None,
                selected: &[],
                selectable: &[],
            },
            &mut textures,
            &assets,
            &fonts,
            &faces,
            None,
        );
        commands.entity(root).add_child(panel);
    }
    app.world_mut().flush();
    let mut prefs = crate::prefs::Prefs::default();
    prefs.edit().reduce_motion = true;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // whole pixels
    let (width, height) = (window.x as u32, window.y as u32);
    app.world_mut().spawn(Window {
        resolution: bevy::window::WindowResolution::new(width, height)
            .with_scale_factor_override(1.0),
        ..default()
    });
    app.insert_resource(duel)
        .insert_resource(prefs)
        .init_resource::<stack::StackFold>()
        .add_systems(Update, stack::fold_the_stack);
    for _ in 0..3 {
        app.update();
    }
    app
}

/// On a phone the stack's top entry is seen whole — its name, what it
/// targets and its sentence's two-line box with the bar beside it — inside
/// the panel's list and above the bar, at every phone size, the drawer shut
/// or open. It was the panel's head and controls, and no entry at all.
#[test]
fn a_phone_shows_the_top_entry_its_targets_and_its_sentence() {
    for (w, h) in PHONES {
        let window = Vec2::new(w, h);
        for shown in [0.0, 1.0] {
            let at = format!("{w}x{h}, drawer {shown}");
            let mut app = laid_out_on_a_phone(window, shown);
            let viewport = single::<stack::StackViewport>(&mut app);
            let row = single::<crate::hud::StackRowCard>(&mut app);
            let targets = single::<stack::StackTargets>(&mut app);
            let text_box = single::<stack::StackTextBox>(&mut app);
            let world = app.world();
            let seen = rect(world, viewport);
            let inside = |what: &str, r: Rect| {
                assert!(
                    r.height() > 1.0 && r.width() > 1.0,
                    "{at}: the {what} was never laid out: {r:?}"
                );
                assert!(
                    r.min.y >= seen.min.y - 0.5 && r.max.y <= seen.max.y + 0.5,
                    "{at}: the {what} {r:?} is cut by the list {seen:?}"
                );
            };
            inside("entry", rect(world, row));
            inside("targets", rect(world, targets));
            let sentence = rect(world, text_box);
            inside("sentence", sentence);
            // Two lines of it, no more and no fewer: the box is the phone's.
            let line = stack::STACK_SENTENCE_LINE;
            assert!(
                (sentence.height() - stack::STACK_SENTENCE_LINES_PHONE * line).abs() < 1.0,
                "{at}: the sentence's box is {} tall",
                sentence.height()
            );
            assert!(
                seen.max.y <= zone_top(window, shown),
                "{at}: the list reaches under the bar"
            );
        }
    }
}

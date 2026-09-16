//! A split bar's tiles are the one ink on any bar whose width is not a fixed
//! number of pixels, and the tree they live in is built only when the
//! *density* changes. So the width they are born with is the length the ledge
//! projected at that moment, and the camera is still easing towards its home
//! then — every duel opened with a bar whose box covered the whole shelf and
//! whose tiles covered nine tenths of it, the difference going quietly into
//! the phase gaps. The claim here is about an `App` that has actually run: a
//! shelf that grows takes its tiles with it.

use super::*;
use crate::hud::{SeatTile, Shelf, Shelves, stretch_step_tiles};
use baylee_client_core::seatbar::Density;
use baylee_core::ids::PlayerId;

const SEAT: PlayerId = PlayerId::new(0);

/// How long a duel's near shelf projects while the camera is still on its
/// way in, and how long it is once it has arrived. Both measured on the
/// running client at 1728x1052.
const ARRIVING: f32 = 981.0;
const HOME: f32 = 1127.1;

#[derive(Resource, Default)]
struct Relayouts(usize);

fn count(mut seen: ResMut<Relayouts>, tiles: Query<(), Changed<Node>>) {
    seen.0 += tiles.iter().count();
}

fn shelf(along: f32) -> Shelf {
    Shelf {
        middle: Vec2::new(864.0, 479.0),
        along,
        depth: 61.0,
        tilt: 0.0,
        density: Density::Split,
    }
}

fn width(app: &App, tile: Entity) -> f32 {
    match app
        .world()
        .entity(tile)
        .get::<Node>()
        .expect("a node")
        .width
    {
        Val::Px(w) => w,
        other => panic!("a tile is sized in pixels, not {other:?}"),
    }
}

/// A bar as the tree builder leaves it: one ordinary step and one main
/// phase, both born on the shorter shelf.
fn table(along: f32) -> (App, Entity, Entity) {
    let mut app = App::new();
    app.init_resource::<crate::Duel>()
        .init_resource::<Relayouts>()
        .insert_resource(Shelves(vec![(SEAT, shelf(along))]))
        .add_systems(Update, (stretch_step_tiles, count).chain());
    let born = Density::Split.tile_width_on(along);
    let mut tile = |span: f32| {
        app.world_mut()
            .spawn((
                SeatTile { player: SEAT, span },
                Node {
                    width: px(born * span),
                    ..default()
                },
            ))
            .id()
    };
    let step = tile(1.0);
    let main = tile(Density::Split.main_span());
    (app, step, main)
}

#[test]
fn a_camera_that_dollies_in_widens_the_tiles_with_the_shelf() {
    let (mut app, step, _) = table(ARRIVING);
    app.update();
    let born = width(&app, step);

    app.insert_resource(Shelves(vec![(SEAT, shelf(HOME))]));
    app.update();

    let want = Density::Split.tile_width_on(HOME);
    assert!(
        (width(&app, step) - want).abs() < 1e-3,
        "the shelf grew from {ARRIVING} to {HOME} and the tile stayed at \
         {born}: it is {} where the model says {want}",
        width(&app, step)
    );
    // And the counter-test the bug would have passed: the tile really did
    // have to move. A system that wrote nothing at all would agree with
    // the model here if the model happened to answer the same twice.
    assert!(
        want > born,
        "the two shelves have to disagree or this proves nothing: \
         {born} at {ARRIVING}, {want} at {HOME}"
    );
}

#[test]
fn a_main_phase_grows_by_its_own_span() {
    let (mut app, step, main) = table(ARRIVING);
    app.insert_resource(Shelves(vec![(SEAT, shelf(HOME))]));
    app.update();

    let span = Density::Split.main_span();
    assert!(
        (width(&app, main) - width(&app, step) * span).abs() < 1e-3,
        "a main phase is {span} steps wide wherever the shelf is: \
         {} against {}",
        width(&app, main),
        width(&app, step)
    );
}

/// Touching a `Node` at all relays out the bar it belongs to, so a camera
/// standing still — which is most frames — has to cost nothing.
#[test]
fn a_shelf_that_has_not_moved_writes_nothing() {
    let (mut app, _, _) = table(HOME);
    app.update();
    let settled = app.world().resource::<Relayouts>().0;

    app.update();
    assert_eq!(
        app.world().resource::<Relayouts>().0,
        settled,
        "a still camera relaid out the tiles anyway"
    );

    app.insert_resource(Shelves(vec![(SEAT, shelf(ARRIVING))]));
    app.update();
    assert_eq!(
        app.world().resource::<Relayouts>().0,
        settled + 2,
        "and a shelf that did move has to reach both tiles"
    );
}

/// A bar the camera cannot see is hidden rather than despawned, and its
/// tiles keep what they had: the shelf comes back at the length it went
/// away with far more often than not, and a tile written to some fallback
/// width would be wrong for the frame the bar is shown again.
#[test]
fn a_shelf_that_is_gone_leaves_its_tiles_alone() {
    let (mut app, step, _) = table(HOME);
    app.update();
    let held = width(&app, step);

    app.insert_resource(Shelves(Vec::new()));
    app.update();
    assert!(
        (width(&app, step) - held).abs() < 1e-3,
        "a hidden bar had its tiles rewritten: {held} became {}",
        width(&app, step)
    );
}

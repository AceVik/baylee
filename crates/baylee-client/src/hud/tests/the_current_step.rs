//! The step light is *run*, not merely declared.
//!
//! The step a game is in is drawn by systems rather than by a colour written
//! when a tile is built, so the thing that can go wrong is a system never
//! being scheduled — a class of bug this client has shipped before, and
//! which every assertion about the colour it *would* write would have missed.
//! So the claims here are about an `App` that has actually run: the gold
//! passes from the step left to the step shown over several frames, from
//! the step the table's pace shows, and settled it writes nothing.

use super::*;
use crate::prefs::Prefs;
use baylee_client_core::phasepace::Shown;
use baylee_core::ids::PlayerId;
use baylee_view::{Phase, Step};

/// The parts of one tile the light writes, by seat and step.
struct Tile {
    node: Entity,
    fill: Entity,
    ink: Entity,
}

fn tile(app: &mut App, player: PlayerId, row: RailRow) -> Tile {
    let world = app.world_mut();
    let tile = world
        .spawn((
            StepGlow {
                player,
                row,
                frame: Color::NONE,
            },
            BorderColor::all(Color::NONE),
            BoxShadow::new(Color::NONE, px(0), px(0), px(0), px(0)),
        ))
        .id();
    let fill = world
        .spawn((StepFill((player, row)), BackgroundColor(Color::NONE)))
        .id();
    let ink = world
        .spawn((
            StepInk {
                key: (player, row),
                rest: palette::PARCHMENT,
            },
            TextColor(palette::PARCHMENT),
            // A text node has a ground too, which must stay clear.
            BackgroundColor(Color::NONE),
        ))
        .id();
    Tile {
        node: tile,
        fill,
        ink,
    }
}

fn gold(app: &App, tile: &Tile) -> f32 {
    app.world()
        .entity(tile.fill)
        .get::<BackgroundColor>()
        .expect("the gold is still there")
        .0
        .alpha()
}

fn at(player: PlayerId, phase: Phase, step: Step) -> Shown {
    Shown {
        turn: 3,
        active: player,
        phase,
        step,
    }
}

/// Entities whose gold, ink or frame was written this frame.
#[derive(Resource, Default)]
struct Written(usize);

fn count_writes(
    mut written: ResMut<Written>,
    fills: Query<(), Changed<BackgroundColor>>,
    inks: Query<(), Changed<TextColor>>,
    borders: Query<(), Changed<BorderColor>>,
    shadows: Query<(), Changed<BoxShadow>>,
) {
    written.0 = fills.iter().count()
        + inks.iter().count()
        + borders.iter().count()
        + shadows.iter().count();
}

fn harness() -> App {
    let mut app = App::new();
    app.init_resource::<Time>()
        .init_resource::<Prefs>()
        .init_resource::<StepPace>()
        .init_resource::<StepLights>()
        .init_resource::<Written>()
        .add_systems(Update, (glow_the_steps, count_writes).chain());
    app
}

fn show(app: &mut App, shown: Shown) {
    // Shown at once, as a fresh pace shows its first step: the pace itself
    // is client-core's to test. The light is this file's, and it is kept
    // apart from the pace, so starting the pace over leaves it as it was.
    let mut pace = app.world_mut().resource_mut::<StepPace>();
    pace.0.reset();
    pace.0.follow(0.0, shown, false, false);
}

fn frame(app: &mut App) {
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(16));
    app.update();
}

#[test]
fn the_gold_passes_from_the_step_left_to_the_step_shown() {
    let mut app = harness();
    let me = PlayerId::new(0);
    let upkeep = tile(&mut app, me, RailRow::Upkeep);
    let main = tile(&mut app, me, RailRow::Main1);
    show(&mut app, at(me, Phase::Beginning, Step::Upkeep));
    for _ in 0..200 {
        frame(&mut app);
    }
    assert!(
        (gold(&app, &upkeep) - 1.0).abs() < 1e-6,
        "the upkeep is gold"
    );
    assert!(gold(&app, &main).abs() < 1e-6, "and the main phase is not");

    // A frame at a time, because the ease is exponential: the first frame
    // must land between the two ends on both tiles, which is the whole
    // claim. The old light arrived on the new tile and died at once on the
    // old one, because the old tile went with the tree.
    show(&mut app, at(me, Phase::FirstMain, Step::Main));
    frame(&mut app);
    let (left, coming) = (gold(&app, &upkeep), gold(&app, &main));
    assert!(left > 0.0 && left < 1.0, "the upkeep's gold cut to {left}");
    assert!(
        coming > 0.0 && coming < 1.0,
        "the main phase's gold cut to {coming}"
    );

    for _ in 0..200 {
        frame(&mut app);
    }
    assert!(gold(&app, &upkeep).abs() < 1e-6, "the upkeep kept its gold");
    assert!(
        (gold(&app, &main) - 1.0).abs() < 1e-6,
        "the main phase never finished lighting"
    );
    let ink = app
        .world()
        .entity(main.ink)
        .get::<TextColor>()
        .expect("set")
        .0;
    let (got, want) = (ink.to_srgba(), palette::PARCHMENT_INK.to_srgba());
    assert!(
        (got.red - want.red).abs() < 1e-4
            && (got.green - want.green).abs() < 1e-4
            && (got.blue - want.blue).abs() < 1e-4,
        "the ink does not read on gold: {got:?}"
    );
    let ground = app
        .world()
        .entity(main.ink)
        .get::<BackgroundColor>()
        .expect("set")
        .0;
    assert_eq!(ground, Color::NONE, "the glyph was painted a solid block");
    let shadow = app
        .world()
        .entity(main.node)
        .get::<BoxShadow>()
        .expect("set");
    assert!(
        shadow.first().is_some_and(|s| s.color.alpha() > 0.0),
        "and it casts no light at all"
    );

    // Settled, nothing is written: a resting table changes no node.
    frame(&mut app);
    frame(&mut app);
    assert_eq!(
        app.world().resource::<Written>().0,
        0,
        "a settled light wrote"
    );
}

/// Untap and cleanup grant no priority, so the light leaves the bar rather
/// than gilding a dead tile; and with motion reduced it goes in one frame.
#[test]
fn a_dead_step_is_not_gilded_and_stillness_cuts() {
    let mut app = harness();
    app.world_mut().resource_mut::<Prefs>().edit().reduce_motion = true;
    let me = PlayerId::new(0);
    let untap = tile(&mut app, me, RailRow::Untap);
    let upkeep = tile(&mut app, me, RailRow::Upkeep);
    show(&mut app, at(me, Phase::Beginning, Step::Upkeep));
    frame(&mut app);
    assert!((gold(&app, &upkeep) - 1.0).abs() < 1e-6, "stillness eased");
    show(&mut app, at(me, Phase::Beginning, Step::Untap));
    frame(&mut app);
    assert!(gold(&app, &upkeep).abs() < 1e-6, "the light stayed");
    assert!(gold(&app, &untap).abs() < 1e-6, "a dead step was gilded");
}

use super::*;

/// Both ends of the passage are stills: one gate standing, no haze, and
/// the rim at rest. Anything else would be a scene that is still moving
/// when nothing is.
#[test]
#[allow(clippy::float_cmp)] // the ends are exact: `smoothstep` clamps to 0 and 1
fn a_passage_starts_and_ends_on_a_still() {
    for peak in [HAZE_IN, HAZE_OUT] {
        let before = passage(0.0, peak);
        assert_eq!(before.hour, 0.0);
        assert!(before.haze.abs() < 1e-6, "{before:?}");
        assert_eq!((before.before.alpha, before.before.zoom), (1.0, 1.0));
        assert_eq!(before.after.alpha, 0.0);
        assert!((before.glow - 1.0).abs() < 1e-6);

        let after = passage(1.0, peak);
        assert_eq!(after.hour, 1.0);
        assert!(after.haze.abs() < 1e-6, "{after:?}");
        assert_eq!(after.before.alpha, 0.0);
        assert_eq!(after.after.alpha, 1.0);
        assert!((after.after.opening - INSIDE).abs() < 1e-6);
        assert!((after.glow - 1.0).abs() < 1e-6);
    }
}

/// The hour turns only behind the haze, so the ridge changing its
/// skyline is never seen happening, and the haze never passes its peak,
/// which is the accent and never white.
#[test]
fn the_hour_turns_only_inside_the_haze() {
    for step in 0..=1000 {
        let p = step as f32 / 1000.0;
        let stage = passage(p, HAZE_IN);
        if stage.hour > 0.0 && stage.hour < 1.0 {
            assert!(stage.haze >= 0.15, "at {p}: {stage:?}");
        }
        assert!(stage.haze <= HAZE_IN + 1e-6);
        assert!(passage(p, HAZE_OUT).haze <= HAZE_OUT + 1e-6);
    }
}

/// A wait holds the gate open and turns the hour over the longest wait,
/// and a wait too short to be read shows none of its own light.
#[test]
#[allow(clippy::float_cmp)] // the ends are exact: `smoothstep` clamps to 0 and 1
fn a_wait_is_the_passage_held_open() {
    let fresh = waiting(0.0);
    assert_eq!((fresh.haze, fresh.glow, fresh.hour), (0.0, 1.0, 0.0));
    let held = waiting(10.0);
    assert!(held.before.alpha == 1.0 && held.before.opening > 1.0);
    assert!(held.hour > 0.2 && held.hour < 0.3, "{held:?}");
    assert_eq!(waiting(60.0).hour, 1.0);
}

/// The WGSL is parsed and validated with the same front end wgpu uses,
/// as the ambient field's is.
#[test]
fn the_vista_shader_compiles() {
    let prelude = "\
struct UiVertexOutput {
@location(0) uv: vec2<f32>,
@location(1) border_widths: vec4<f32>,
@location(2) border_radius_x: vec4<f32>,
@location(3) border_radius_y: vec4<f32>,
@location(4) @interpolate(flat) size: vec2<f32>,
@builtin(position) position: vec4<f32>,
};
struct Globals { time: f32 };
";
    let prelude = format!("{prelude}{}", include_str!("../shaders/noise.wesl"));
    crate::cardmat::tests::check_wgsl(include_str!("../shaders/vista.wesl"), &prelude);
    crate::cardmat::tests::check_wgsl(include_str!("../shaders/memorial.wesl"), &prelude);
    crate::cardmat::tests::check_wgsl(include_str!("../shaders/memorial_paper.wesl"), &prelude);
    crate::cardmat::tests::check_wgsl(include_str!("../shaders/memorial_preview.wesl"), &prelude);
}

/// The lobby's world (Baylee's fireflies are drawn from its clock,
/// `portal.z`, and only while `view.w`, its energy, is not zero), painted
/// for `frames` frames of 50 ms; the last params, and each frame's clock.
/// `panel_x` is the current scene's panel centre in screen heights.
fn lobby_world(
    reduce_motion: bool,
    frames: usize,
    panel_x: Option<f32>,
) -> (VistaParams, Vec<f32>) {
    use bevy::time::TimeUpdateStrategy;
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(50),
        ))
        .insert_resource(Assets::<VistaMaterial>::default())
        .insert_resource(FrontScene {
            shown: false,
            ..FrontScene::default()
        })
        .init_resource::<Gaze>()
        .add_systems(Update, paint);
    let mut prefs = crate::prefs::Prefs::default();
    prefs.edit().reduce_motion = reduce_motion;
    app.insert_resource(prefs);
    let handle = app
        .world_mut()
        .resource_mut::<Assets<VistaMaterial>>()
        .add(VistaMaterial {
            params: VistaParams::default(),
            sanctuary: Handle::default(),
        });
    app.world_mut().spawn((
        Vista::Interior,
        Settled::default(),
        ComputedNode {
            size: Vec2::new(1708.0, 1032.0),
            ..ComputedNode::default()
        },
        MaterialNode(handle.clone()),
        Visibility::Inherited,
    ));
    if let Some(x) = panel_x {
        app.world_mut().spawn((
            Framed(Vista::Interior),
            ComputedNode {
                size: Vec2::new(300.0, 500.0),
                ..ComputedNode::default()
            },
            UiGlobalTransform::from_translation(Vec2::new(x * 1032.0, 400.0)),
        ));
    }
    let mut clock = Vec::new();
    for _ in 0..frames {
        app.update();
        let materials = app.world().resource::<Assets<VistaMaterial>>();
        clock.push(materials.get(&handle).expect("painted").params.portal.z);
    }
    let materials = app.world().resource::<Assets<VistaMaterial>>();
    (materials.get(&handle).expect("painted").params, clock)
}

/// Reduced motion stops Baylee's fireflies entirely: the lobby's world
/// has no energy (the shader draws no mote then, only her still glow)
/// and its clock stands. And the counter-test: with motion allowed the
/// motes have energy and their clock runs, by the frame's delta.
#[test]
#[allow(clippy::float_cmp)] // a stopped clock and zero energy are exact
fn reduced_motion_stops_the_lobby_fireflies() {
    let (still, clock) = lobby_world(true, 8, None);
    assert_eq!(still.view.w, 0.0, "{still:?}");
    assert!(clock.iter().all(|t| *t == clock[0]), "{clock:?}");
    let (moving, clock) = lobby_world(false, 8, None);
    assert_eq!(moving.view.w, 1.0, "{moving:?}");
    assert!(clock.windows(2).skip(2).all(|w| w[1] > w[0]), "{clock:?}");
    assert_eq!(moving.gate_b.w, 1.0, "the interior draws Baylee at home");
}

/// Play's side panel, the unframed Decks screen and a differently placed
/// builder panel may move the dimming mask, but never Baylee's home.
#[test]
#[allow(clippy::float_cmp)] // every anchor and panel centre is exact by construction
fn baylees_home_anchor_is_the_same_across_scenes() {
    let scenes = [Some(0.24), None, Some(0.71)].map(|panel| lobby_world(false, 3, panel).0);
    let home = scenes[0].home;
    assert!(scenes.iter().all(|scene| scene.home == home));
    assert_ne!(scenes[0].panel.x, scenes[1].panel.x);
    assert_ne!(scenes[1].panel.x, scenes[2].panel.x);
}

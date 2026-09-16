//! The rail's light is *run*, not merely declared.
//!
//! The step a game is in is drawn by a system rather than by a colour written
//! when the button is built, so the thing that can go wrong is the system
//! never being scheduled — a class of bug this client has shipped before, and
//! which every assertion about the colour it *would* write would have missed.
//! So the claim here is about an `App` that has actually run: the light rises
//! from nothing, and it rises towards the colour a player is meant to read as
//! "here".

use super::*;

fn lit(app: &App, button: Entity) -> f32 {
    app.world()
        .entity(button)
        .get::<PhaseNow>()
        .expect("the button still carries its light")
        .lit
}

#[test]
fn the_light_arrives_over_several_frames_rather_than_cutting() {
    let mut app = App::new();
    app.init_resource::<Time>()
        .add_systems(Update, light_the_current_step);
    let button = app
        .world_mut()
        .spawn((
            PhaseNow::default(),
            BorderColor::all(palette::PANEL),
            BoxShadow::new(Color::NONE, px(0), px(0), px(0), px(0)),
        ))
        .id();
    assert!(lit(&app, button).abs() < 1e-6, "it starts dark");

    // A frame at a time, because the ease is exponential: the first frame
    // must land somewhere between the two ends, which is the whole claim.
    // A `Time` with no delta would satisfy "not one" by never moving at
    // all, so the delta is written by hand.
    let frame = std::time::Duration::from_millis(16);
    app.world_mut().resource_mut::<Time>().advance_by(frame);
    app.update();
    let first = lit(&app, button);
    assert!(
        first > 0.0 && first < 1.0,
        "one frame took the light from 0 to {first}"
    );

    for _ in 0..80 {
        app.world_mut().resource_mut::<Time>().advance_by(frame);
        app.update();
    }
    assert!(
        (lit(&app, button) - 1.0).abs() < 1e-6,
        "the light never finished arriving: {}",
        lit(&app, button)
    );
    let border = app
        .world()
        .entity(button)
        .get::<BorderColor>()
        .expect("set");
    assert_eq!(
        border.top,
        palette::ACTIVE,
        "the step the game is in is not drawn in the colour that says so"
    );
    let shadow = app.world().entity(button).get::<BoxShadow>().expect("set");
    assert!(
        shadow.first().is_some_and(|s| s.color.alpha() > 0.0),
        "and it casts no light at all"
    );
}

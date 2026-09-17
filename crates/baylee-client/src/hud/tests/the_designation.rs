//! The day/night block: when it exists at all, and the flash that marks a
//! change surviving a rebuild.

use super::*;
use baylee_view::DayNight;

fn glow(app: &App, block: Entity) -> f32 {
    app.world()
        .entity(block)
        .get::<BoxShadow>()
        .and_then(|s| s.first().map(|l| l.color.alpha()))
        .expect("the block still carries its light")
}

fn spawn(app: &mut App, now: DayNight) -> Entity {
    app.world_mut()
        .spawn((
            Designation(now),
            BorderColor::all(Color::NONE),
            BoxShadow::new(Color::NONE, px(0), px(0), px(0), px(0)),
        ))
        .id()
}

fn frame(app: &mut App, ms: u64) {
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(ms));
    app.update();
}

/// The flash is anchored to the *change*, not to the entity, and that is
/// the whole design: the HUD tree is rebuilt on every hover, so a light
/// that eased from zero at spawn would fire again every time the pointer
/// crossed a card. Here the block is despawned and respawned with the
/// same designation — a rebuild, exactly — and the decay has to carry on
/// from where it was rather than start over.
#[test]
fn a_rebuild_does_not_restart_the_flash() {
    let mut app = App::new();
    app.init_resource::<Time>()
        .init_resource::<DesignationFlash>()
        .add_systems(Update, flash_the_designation);

    let first = spawn(&mut app, DayNight::Day);
    frame(&mut app, 16);
    let arrival = glow(&app, first);
    assert!(
        arrival > 0.4,
        "the block did not announce itself: {arrival}"
    );

    frame(&mut app, 400);
    let decayed = glow(&app, first);
    assert!(
        decayed < arrival * 0.5,
        "the flash did not decay: {arrival} -> {decayed}"
    );

    // The rebuild.
    app.world_mut().entity_mut(first).despawn();
    let second = spawn(&mut app, DayNight::Day);
    frame(&mut app, 16);
    let after = glow(&app, second);
    assert!(
        after <= decayed,
        "the rebuild restarted the flash: {decayed} -> {after}"
    );
}

/// And the counter-test, or the one above would pass just as well on a
/// system that never lit anything after the first frame: a block that
/// comes back carrying the *other* designation is a change, and a change
/// flashes.
#[test]
fn the_other_designation_is_a_change_and_flashes() {
    let mut app = App::new();
    app.init_resource::<Time>()
        .init_resource::<DesignationFlash>()
        .add_systems(Update, flash_the_designation);

    let day = spawn(&mut app, DayNight::Day);
    frame(&mut app, 16);
    frame(&mut app, 800);
    let quiet = glow(&app, day);
    assert!(quiet < 0.1, "the flash never settled: {quiet}");

    app.world_mut().entity_mut(day).despawn();
    let night = spawn(&mut app, DayNight::Night);
    frame(&mut app, 16);
    assert!(
        glow(&app, night) > 0.4,
        "night arrived without saying so: {}",
        glow(&app, night)
    );
}

/// The designation's two glyphs belong to nothing else on a seat's bar.
///
/// The sun was the untap step's until the day designation wanted it, and
/// two suns a hundred pixels apart on one bar would have said the untap
/// step *is* the daytime. The check is the two sets being disjoint and
/// not every glyph being unique, because the two main phases share a flag
/// on purpose — they are one phase kind twice, and "M1" and "M2" under
/// them are what tell them apart.
///
/// The two halves used to live in one file and the test split it at
/// `fn spawn_designation`. They are two files now — the steps' glyphs
/// stayed with [`row_visual`](crate::hud::rail) when the rail went, and
/// the hinge that carries the designation is on the bar — so the test
/// reads both instead of splitting one.
#[test]
fn the_designation_does_not_borrow_a_step_glyph() {
    let glyphs = |src: &str| -> Vec<String> {
        src.match_indices("'\\u{f")
            .map(|(at, _)| src[at + 1..at + 9].to_string())
            .collect()
    };
    let steps: Vec<String> = baylee_client_core::tableicons::PHASES
        .iter()
        .map(|glyph| format!("\\u{{{:x}}}", *glyph as u32))
        .collect();
    let bar = include_str!("../seatbar.rs");
    let designation = glyphs(
        bar.split_once("fn designation_of")
            .expect("the hinge still names its two glyphs")
            .1
            .split_once("\n}")
            .expect("and still closes")
            .0,
    );
    assert_eq!(
        designation.len(),
        2,
        "the sun and the moon: {designation:?}"
    );
    assert_eq!(steps.len(), 12, "the rows did not parse: {steps:?}");
    for glyph in &designation {
        assert!(
            !steps.contains(glyph),
            "{glyph} is drawn both as a step and as the designation"
        );
    }
}

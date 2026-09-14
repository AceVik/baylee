use super::*;
use baylee_client_core::{PileKind, TableLayout, layout};

/// The window the rest of the camera tests measure in.
const WIN: Vec2 = Vec2::new(1728.0, 1052.0);

/// How far apart the fan's cards are drawn, in **physical** pixels, rung
/// by rung — the window is logical and the screen is twice it.
///
/// Built from the client's own rig and clip matrix rather than from a
/// second projection written out by hand, because what is being asserted
/// here is a number of pixels and not an agreement between two formulas;
/// `the_lens_and_the_written_out_projection_agree` is where that is
/// checked.
fn rungs(float: f32, rise: f32, step: f32) -> Vec<f32> {
    let canvas = Canvas::hud(WIN);
    let seats: Vec<_> = (0..2).map(baylee_core::ids::PlayerId::new).collect();
    let table = TableLayout::new(&seats, canvas.aspect(), None);
    let lens = Lens::new(CameraRig::home(&table, canvas), canvas.window);
    let slot = table.local().expect("a local seat");
    let at = slot.pile_center(PileKind::Graveyard);
    let away = Vec2::new(slot.facing.sin(), slot.facing.cos());
    // The sign `fan_pose` gives the local seat, written out here so this
    // measures the shape rather than borrowing it.
    let back = if slot.facing.cos() > 0.08 { 1.0 } else { -1.0 };
    let screen: Vec<Vec2> = (0..layout::FAN_MAX)
        .map(|j| {
            let table = at + away * (back * step * j as f32);
            let world = to_world(table, TABLE_Y + CARD_LIFT + float + rise * j as f32);
            let clip = lens.clip_from_world * world.extend(1.0);
            let ndc = clip.truncate() / clip.w;
            Vec2::new(
                ndc.x.mul_add(0.5, 0.5) * WIN.x,
                0.5f32.mul_add(-ndc.y, 0.5) * WIN.y,
            ) * 2.0
        })
        .collect();
    screen.windows(2).map(|w| (w[1] - w[0]).length()).collect()
}

/// A card a rung further back has to show enough of itself to be told
/// apart — its name and its mana cost, which is about the top sixth of a
/// card, and a card stands about 126 px tall at this camera once it is
/// tipped.
const READABLE: f32 = 18.0;

#[test]
fn every_card_of_the_fan_shows_its_name_above_the_one_in_front() {
    let gaps = rungs(layout::FAN_FLOAT, layout::FAN_RISE, layout::FAN_STEP);
    for (rung, gap) in gaps.iter().enumerate() {
        assert!(
            *gap >= READABLE,
            "card {} of the fan shows {gap} px, which is a border and not \
             a name: {gaps:?}",
            rung + 1
        );
    }
    // And evenly, which is what says the fan is a fan: perspective
    // shortens the far end, and a spacing that halved down the line would
    // be a shape that works for the first two cards.
    let (near, far) = (gaps[0], gaps[gaps.len() - 1]);
    assert!(
        (near - far).abs() < near * 0.15,
        "the fan opens {near} px at one end and {far} at the other"
    );
}

/// The counter-arm, and the reason this file has a screen-space test at
/// all: the fan as it was first built — rising 1.5 units and stepping
/// 0.72 towards the camera — passes every table-space assertion about it
/// and fails this one on every rung.
#[test]
fn the_fan_that_rose_into_the_air_showed_nothing() {
    let gaps = rungs(0.18, 0.22, -0.12);
    assert!(
        gaps.iter().all(|gap| *gap < READABLE * 0.5),
        "the fan this test was written against is legible after all: {gaps:?}"
    );
}

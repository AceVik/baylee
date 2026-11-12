use super::*;
use baylee_client_core::{PileKind, TableLayout, layout};

/// The window the rest of the camera tests measure in.
const WIN: Vec2 = Vec2::new(1728.0, 1052.0);

/// Where the middle of each card of the local seat's graveyard fan is
/// drawn, in **physical** pixels, newest first — the window is logical and
/// the screen is twice it.
///
/// Built from the client's own rig and clip matrix rather than from a
/// second projection written out by hand, because what is being asserted
/// here is a number of pixels and not an agreement between two formulas;
/// `the_lens_and_the_written_out_projection_agree` is where that is
/// checked.
fn drawn() -> Vec<Vec2> {
    let canvas = Canvas::hud(WIN);
    let seats: Vec<_> = (0..2).map(baylee_core::ids::PlayerId::new).collect();
    let table = TableLayout::new(&seats, canvas.aspect(), None);
    let lens = Lens::new(CameraRig::home(&table, canvas), canvas.window);
    let slot = table.local().expect("a local seat");
    (0..layout::FAN_MAX)
        .map(|j| {
            let pose = slot.fan_pose(PileKind::Graveyard, j, layout::FAN_MAX, false);
            let world = to_world(pose.at, TABLE_Y + CARD_LIFT + pose.lift);
            let clip = lens.clip_from_world * world.extend(1.0);
            let ndc = clip.truncate() / clip.w;
            Vec2::new(
                ndc.x.mul_add(0.5, 0.5) * WIN.x,
                0.5f32.mul_add(-ndc.y, 0.5) * WIN.y,
            ) * 2.0
        })
        .collect()
}

/// What a card of the fan has to show beside its newer neighbour to be told
/// apart: the start of its name and a third of its picture. A card lies
/// about 90 px wide at this camera.
const SHOWN: f32 = 34.0;

#[test]
fn every_card_of_the_fan_shows_itself_beside_its_neighbour() {
    let at = drawn();
    let gaps: Vec<f32> = at.windows(2).map(|w| (w[1] - w[0]).length()).collect();
    for (card, gap) in gaps.iter().enumerate() {
        assert!(
            *gap >= SHOWN,
            "card {} of the fan shows {gap} px, which is a border and not \
             a card: {gaps:?}",
            card + 1
        );
    }
    // And evenly, which is what says the fan is a fan: a spacing that
    // halved down the line would be a shape that works for two cards.
    let (near, far) = (gaps[0], gaps[gaps.len() - 1]);
    assert!(
        (near - far).abs() < near * 0.15,
        "the fan opens {near} px at one end and {far} at the other"
    );
}

/// The local seat's piles stand at its right hand, so its fan opens to the
/// left on the screen, over the strip between the pile and the mat, and
/// sinks a little towards the seat's own edge as a hand does.
#[test]
fn the_local_seats_fan_opens_towards_its_own_board() {
    let at = drawn();
    for pair in at.windows(2) {
        assert!(
            pair[1].x < pair[0].x,
            "the fan opened away from the board: {at:?}"
        );
        assert!(
            pair[1].y >= pair[0].y - 1.0,
            "the fan climbs the screen instead of opening sideways: {at:?}"
        );
    }
}

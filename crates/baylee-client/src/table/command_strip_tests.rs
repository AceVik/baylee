//! A command-zone consumable needs the same reserved space as a commander.
use super::*;
use baylee_client_core::test_support::{ViewBuilder, printed, token};

fn receive(duel: &mut Duel, view: baylee_view::PlayerView) {
    duel.receive_view(view);
    crate::rebuild_board(duel);
}

#[test]
fn command_strip_preserves_visible_non_commander_objects_and_releases_only_empty_space() {
    let player = PlayerId::new(0);
    let mut duel = Duel::default();
    receive(&mut duel, ViewBuilder::new(2).build());
    let original = *duel.layout.as_ref().unwrap().slot(player).unwrap();
    assert!(
        original.reclaimed > 0.0,
        "ordinary empty games gain lane space"
    );

    let potion = token(90, 0, "Mana Draught", 0, 0);
    receive(
        &mut duel,
        ViewBuilder::new(2)
            .with_command(0, vec![potion.clone()])
            .build(),
    );
    let occupied = *duel.layout.as_ref().unwrap().slot(player).unwrap();
    assert!(
        occupied.reclaimed.abs() < 1e-6,
        "the potion needs its strip"
    );
    assert!(
        occupied
            .footprint_center()
            .abs_diff_eq(original.footprint_center(), 1e-5)
    );
    assert!(
        occupied
            .pile_center(PileKind::Library)
            .abs_diff_eq(original.pile_center(PileKind::Library), 1e-5)
    );

    let commander = printed(91, 0, "Commander", 0);
    receive(
        &mut duel,
        ViewBuilder::new(2)
            .with_commanders(0, &[&commander])
            .build(),
    );
    let designated = *duel.layout.as_ref().unwrap().slot(player).unwrap();
    assert_eq!(
        occupied, designated,
        "occupied and designated command zones reserve the same space"
    );

    receive(&mut duel, ViewBuilder::new(2).build());
    assert_eq!(
        *duel.layout.as_ref().unwrap().slot(player).unwrap(),
        original
    );
    assert!(!placements(&duel).iter().any(|p| p.object == potion.id));
}

#[test]
fn command_strip_keeps_the_whole_potion_inside_the_native_duel_frame() {
    for window in [Vec2::new(1440.0, 900.0), Vec2::new(1280.0, 720.0)] {
        for seat in 0..2 {
            let canvas = Canvas::hud(window);
            let mut duel = Duel {
                canvas_aspect: Some(canvas.aspect()),
                ..default()
            };
            let potion = token(90, seat, "Mana Draught", 0, 0);
            receive(
                &mut duel,
                ViewBuilder::new(2)
                    .with_command(seat, vec![potion.clone()])
                    .build(),
            );
            let layout = duel.layout.as_ref().unwrap();
            let rig = CameraRig::home(layout, canvas);
            let at = placements(&duel)
                .into_iter()
                .find(|p| p.object == potion.id)
                .expect("native command-zone placement");
            // Project forwards, independently of the camera's framing inverse.
            let eye = rig.distance * (1.0 + rig.lean * rig.lean).sqrt();
            let cos = 1.0 / (1.0 + rig.lean * rig.lean).sqrt();
            let tangent = (FOV * 0.5).tan();
            for x in [-CARD_WIDTH * 0.5, CARD_WIDTH * 0.5] {
                for y in [-CARD_HEIGHT * 0.5, CARD_HEIGHT * 0.5] {
                    let corner = at.position + Vec2::new(x, y) * at.slot.scale;
                    let along = corner.y + rig.target.y;
                    let depth = rig.lean.mul_add(cos * along, eye);
                    let ndc = Vec2::new(
                        (corner.x - rig.target.x) / (depth * tangent * window.x / window.y),
                        cos * along / (depth * tangent),
                    );
                    assert!(
                        ndc.x.abs() < 1.0 && ndc.y.abs() < 1.0,
                        "{window:?}, seat {seat}: clipped potion corner {ndc:?}"
                    );
                }
            }
        }
    }
}

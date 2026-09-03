//! Turning a double-faced card over in the preview.
//!
//! Hold shift and the card in the preview rotates about its vertical axis to
//! show its back; let go and it turns back. It answers a question a static
//! preview cannot — "what does this transform into?" — without a click, a
//! second panel, or a mode to leave.
//!
//! # It is a scale, not a rotation
//!
//! A UI node has no third dimension: [`UiTransform`] carries a 2D scale and a
//! rotation *in the plane*. A rotation about the y-axis is therefore played
//! as `scale.x = cos(angle)` — the card narrows to a line at 90° and widens
//! again — which is exactly what the projection of a real rotation looks like
//! from straight on. Nobody can tell the difference on a card, and it costs
//! no camera, no mesh and no second render pass.
//!
//! Two consequences fall out of that and both are load-bearing. Past 90° the
//! scale is *negative*, so the face shown there would be mirrored: the back
//! carries its own `scale.x = -1` so the two cancel and its art reads the
//! right way round. And the swap has to happen exactly at the quarter turn,
//! where the card is a line and there is nothing to see — swapping anywhere
//! else shows the change.

use bevy::prelude::*;
use bevy::ui::UiTransform;

/// How fast the card turns, in half-turns per second.
///
/// A card that took as long as a page transition would make shift feel like a
/// mode; one that snapped would lose the reading that it is the *same* card
/// seen from the other side.
const TURN_RATE: f32 = 3.4;

/// A preview that can be turned over, and how far round it is.
#[derive(Component, Default)]
pub struct Flip {
    /// 0.0 face up, 1.0 fully turned. Runs through the quarter turn where
    /// the card is edge-on and the faces swap.
    pub turn: f32,
}

/// Which side of a [`Flip`] a node is.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The printed front; visible below the quarter turn.
    Front,
    /// The back; visible above it, and mirrored so it reads correctly
    /// through the parent's negative scale.
    Back,
}

/// Turns previews towards the state the shift key is asking for.
pub struct FlipPlugin;

impl Plugin for FlipPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, turn);
    }
}

/// Adds [`FlipPlugin`] unless another plugin already did.
pub(crate) fn install(app: &mut App) {
    if !app.is_plugin_added::<FlipPlugin>() {
        app.add_plugins(FlipPlugin);
    }
}

/// How far round a card should be, and what that means for each side.
///
/// Split out from the system so the arithmetic — which is the whole of the
/// behaviour — can be tested without a window.
#[must_use]
pub fn faces(turn: f32) -> (f32, bool) {
    let angle = turn * std::f32::consts::PI;
    (angle.cos(), turn >= 0.5)
}

/// Advances every flip and writes the two sides.
fn turn(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut flips: Query<(&mut Flip, &mut UiTransform, &Children)>,
    mut sides: Query<(&Side, &mut Visibility, &mut UiTransform), Without<Flip>>,
) {
    let wants_back = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let step = if still {
        1.0
    } else {
        (TURN_RATE * time.delta_secs()).min(1.0)
    };
    for (mut flip, mut transform, children) in &mut flips {
        // A card with nothing on the other side does not turn. Without this
        // the frame still rotated, the front hid at the quarter turn and the
        // preview simply went blank for as long as shift was held — which is
        // what a single-faced card did on the first attempt.
        let has_back = children
            .iter()
            .any(|child| sides.get(child).is_ok_and(|(side, ..)| *side == Side::Back));
        let target = if wants_back && has_back { 1.0 } else { 0.0 };
        // Linear rather than exponential, unlike everything else that moves
        // in this client: a turn has a *far* side, and an asymptote would
        // leave the card a hair short of flat for as long as shift is held.
        flip.turn += (target - flip.turn).clamp(-step, step);
        let (squeeze, showing_back) = faces(flip.turn);
        transform.scale.x = squeeze;
        for child in children {
            let Ok((side, mut visibility, mut child_transform)) = sides.get_mut(*child) else {
                continue;
            };
            let shown = (*side == Side::Back) == showing_back;
            *visibility = if shown {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            // The back cancels the parent's negative scale; the front never
            // sees one, because it is hidden before the sign changes.
            child_transform.scale.x = if *side == Side::Back { -1.0 } else { 1.0 };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The card narrows to nothing at the quarter turn and comes back the
    /// same width, and the faces swap exactly where it is edge-on — swapping
    /// anywhere else would show the change.
    #[test]
    fn a_card_is_edge_on_at_the_quarter_turn() {
        let (front, back) = faces(0.0);
        assert!((front - 1.0).abs() < 1e-6);
        assert!(!back, "face up at rest");

        let (edge, _) = faces(0.5);
        assert!(edge.abs() < 1e-6, "a line, so the swap cannot be seen");

        let (turned, back) = faces(1.0);
        assert!((turned + 1.0).abs() < 1e-6, "fully round, and mirrored");
        assert!(back);
    }

    /// The half-way point belongs to the back: at exactly 0.5 the card has no
    /// width, and the side chosen there is the one that widens out of it.
    #[test]
    fn the_back_owns_the_far_half() {
        assert!(!faces(0.49).1);
        assert!(faces(0.5).1);
        assert!(faces(0.9).1);
    }

    /// Builds an app with one preview frame, with or without a far side.
    fn harness(with_back: bool) -> (App, Entity, Entity, Option<Entity>) {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Time>()
            .add_systems(Update, turn);
        let front = app
            .world_mut()
            .spawn((Node::default(), Side::Front, Visibility::Inherited))
            .id();
        let back = with_back.then(|| {
            app.world_mut()
                .spawn((Node::default(), Side::Back, Visibility::Hidden))
                .id()
        });
        let frame = app
            .world_mut()
            .spawn((Node::default(), Flip::default()))
            .id();
        app.world_mut().entity_mut(frame).add_child(front);
        if let Some(back) = back {
            app.world_mut().entity_mut(frame).add_child(back);
        }
        (app, frame, front, back)
    }

    /// Runs enough frames for a turn to finish, with a real time step so the
    /// rate is exercised rather than bypassed.
    fn settle(app: &mut App) {
        for _ in 0..30 {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_millis(16));
            app.update();
        }
    }

    /// Shift turns a two-faced card over and hides the front; letting go
    /// brings it back.
    #[test]
    fn shift_turns_a_two_faced_card_over() {
        let (mut app, frame, front, back) = harness(true);
        let back = back.unwrap();

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ShiftLeft);
        settle(&mut app);
        assert!((app.world().get::<Flip>(frame).unwrap().turn - 1.0).abs() < 1e-3);
        assert_eq!(
            app.world().get::<Visibility>(back),
            Some(&Visibility::Inherited)
        );
        assert_eq!(
            app.world().get::<Visibility>(front),
            Some(&Visibility::Hidden)
        );
        // Mirrored, so the back reads the right way round through the
        // parent's negative scale.
        assert!(app.world().get::<UiTransform>(back).unwrap().scale.x < 0.0);
        assert!(app.world().get::<UiTransform>(frame).unwrap().scale.x < 0.0);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::ShiftLeft);
        settle(&mut app);
        assert!(app.world().get::<Flip>(frame).unwrap().turn.abs() < 1e-3);
        assert_eq!(
            app.world().get::<Visibility>(front),
            Some(&Visibility::Inherited)
        );
        assert_eq!(
            app.world().get::<Visibility>(back),
            Some(&Visibility::Hidden)
        );
    }

    /// A card with nothing on the other side does not turn at all. Without
    /// the guard the frame still rotated, the front hid at the quarter turn,
    /// and the preview went blank for as long as shift was held — which is
    /// exactly what a single-faced card did on the first attempt.
    #[test]
    fn a_single_faced_card_does_not_turn() {
        let (mut app, frame, front, _) = harness(false);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ShiftLeft);
        settle(&mut app);
        assert!(app.world().get::<Flip>(frame).unwrap().turn.abs() < 1e-6);
        assert_eq!(
            app.world().get::<Visibility>(front),
            Some(&Visibility::Inherited),
            "the card a player is looking at must not disappear"
        );
    }
}

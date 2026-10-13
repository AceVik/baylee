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
//!
//! # Some cards are read turned in the plane
//!
//! A scan is always portrait, and some cards are not printed to be read that
//! way (owner, 13.10.2026): a split card and a battle are printed sideways, a
//! flip card's other half is upside down, an Aftermath card's second half
//! runs up its side. [`Reading`] says how the shown picture stands at rest and
//! what shift does instead of turning such a card over: it *rotates* it, in
//! the plane, to the other half — the card has nothing on its back but the
//! printed back. The frame keeps its box; a card turned across it is shrunk
//! to fit ([`fit`]), as a card turned in the hand still fits the hand.

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
    /// Turned over by a control rather than by Shift (the card window's
    /// Flip): the card stays turned while this holds.
    pub held: bool,
    /// How the picture is turned in the plane to be read.
    pub reading: Reading,
}

/// How a card's picture is turned in the plane to be read, in quarter turns
/// clockwise from the portrait scan (`baylee_cards::sides::quarter_turns`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Reading {
    /// The shown picture's, at rest.
    pub rest: u8,
    /// Once turned: the far side's own when the card turns over, the other
    /// half's when it only rotates.
    pub turned: u8,
    /// Whether shift turns the card over to its back. When it does not, it
    /// rotates the card from `rest` to `turned`, and a card whose two are
    /// equal does not move at all.
    pub over: bool,
}

impl Default for Reading {
    fn default() -> Self {
        Self::UPRIGHT
    }
}

impl Reading {
    /// An ordinary card's: upright, turned over by shift.
    pub const UPRIGHT: Self = Self {
        rest: 0,
        turned: 0,
        over: true,
    };

    /// How a card with these per-face quarter turns reads with face `face`
    /// shown, given whether it has a picture on its back.
    ///
    /// A card with a back turns over to it, standing as the back's face
    /// reads (a battle's front is sideways, its creature back upright). A
    /// card without one whose halves read alike turns over to the printed
    /// back, as every ordinary card does — unless it is read sideways (a
    /// split card shows both halves at once, and the owner's rule is that a
    /// card with two faces on its front shows no back). A card whose halves
    /// read differently (a flip card, an Aftermath card) rotates to the
    /// other half.
    #[must_use]
    pub fn of(turns: &[u8], face: u8, back_image: bool) -> Self {
        let at = |face: usize| turns.get(face).copied().unwrap_or(0) % 4;
        let shown = usize::from(face);
        let rest = at(shown);
        let other = usize::from(shown == 0);
        if back_image {
            return Self {
                rest,
                turned: at(other),
                over: true,
            };
        }
        let turned = if turns.len() > 1 { at(other) } else { rest };
        Self {
            rest,
            turned,
            over: rest == turned && rest == 0,
        }
    }

    /// Whether the card lies across its box at rest: a quarter turn either
    /// way, so the box it is placed in is landscape.
    #[must_use]
    pub const fn lies_across(self) -> bool {
        self.rest % 2 == 1
    }

    /// The angle, clockwise in radians, at `turn` of the way from rest.
    #[must_use]
    pub fn angle(self, turn: f32) -> f32 {
        let delta = match (self.turned + 4 - self.rest) % 4 {
            1 => 1.0,
            2 => 2.0,
            3 => -1.0,
            _ => 0.0,
        };
        (f32::from(self.rest % 4) + delta * turn) * std::f32::consts::FRAC_PI_2
    }
}

/// The scale a card standing at `angle` (clockwise, radians) takes to fit the
/// box it stands in at rest — portrait, or landscape when `across`. One at
/// rest; below it while the card is turned across its box.
#[must_use]
pub fn fit(angle: f32, across: bool) -> f32 {
    let aspect = baylee_client_core::layout::CARD_ASPECT;
    let (sin, cos) = (angle.sin().abs(), angle.cos().abs());
    // The card, a wide by 1 tall, turned: its bounding box.
    let (wide, tall) = (aspect * cos + sin, aspect * sin + cos);
    let (room_w, room_h) = if across { (1.0, aspect) } else { (aspect, 1.0) };
    (room_w / wide).min(room_h / tall).min(1.0)
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

/// The frame's scale and rotation `turn` of the way round: the squeeze of a
/// turn over (`faces`), the reading's angle, and the fit that keeps a card
/// turned across its box inside it.
#[must_use]
pub fn pose(reading: Reading, turn: f32, squeeze: f32) -> (Vec2, Rot2) {
    let angle = reading.angle(turn);
    let fit = fit(angle, reading.lies_across());
    // Clockwise on the screen, whose y runs down: a positive angle in the
    // interface's own frame.
    (Vec2::new(squeeze * fit, fit), Rot2::radians(angle))
}

/// The frame's transform at rest, to spawn it with: a card that waited for
/// the first frame of [`turn`] would stand upright for that frame.
#[must_use]
pub fn rest_transform(reading: Reading) -> UiTransform {
    let (scale, rotation) = pose(reading, 0.0, 1.0);
    UiTransform {
        scale,
        rotation,
        ..default()
    }
}

/// Advances every flip and writes the two sides.
#[allow(clippy::float_cmp)] // exact: a write is skipped only when nothing would change
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
        // A card read by rotating it (a flip card, an Aftermath card) turns
        // whenever its other half stands differently; one that turns over
        // needs something on its back.
        let reading = flip.reading;
        let turns = if reading.over {
            has_back
        } else {
            reading.rest != reading.turned
        };
        let target = if (wants_back || flip.held) && turns {
            1.0
        } else {
            0.0
        };
        // Linear rather than exponential, unlike everything else that moves
        // in this client: a turn has a *far* side, and an asymptote would
        // leave the card a hair short of flat for as long as shift is held.
        // Every write below is guarded on what is already there: a preview
        // standing still is touched by nothing, so the interface is not laid
        // out again on every frame it stays open.
        let turned = flip.turn + (target - flip.turn).clamp(-step, step);
        if turned != flip.turn {
            flip.turn = turned;
        }
        let (squeeze, showing_back) = if reading.over {
            faces(turned)
        } else {
            (1.0, false)
        };
        let (scale, rotation) = pose(reading, turned, squeeze);
        if transform.scale != scale {
            transform.scale = scale;
        }
        if transform.rotation != rotation {
            transform.rotation = rotation;
        }
        for child in children {
            let Ok((side, mut visibility, mut child_transform)) = sides.get_mut(*child) else {
                continue;
            };
            let shown = (*side == Side::Back) == showing_back;
            visibility.set_if_neq(if shown {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
            // The back cancels the parent's negative scale; the front never
            // sees one, because it is hidden before the sign changes.
            let mirror = if *side == Side::Back { -1.0 } else { 1.0 };
            if child_transform.scale.x != mirror {
                child_transform.scale.x = mirror;
            }
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

    /// A preview at rest is written by nothing: once a turn has finished,
    /// another frame leaves the frame's and both sides' transforms and
    /// visibilities unchanged — a changed `UiTransform` lays the interface
    /// out again, every frame the preview stands open. Read by a system
    /// right after the turn, in the same frame, because `App::update` clears
    /// the change trackers on its way out.
    #[test]
    fn a_preview_at_rest_is_not_touched() {
        #[derive(Resource, Default)]
        struct Written(usize);
        fn watch(mut written: ResMut<Written>, parts: Query<(Ref<UiTransform>, Ref<Visibility>)>) {
            written.0 = parts
                .iter()
                .filter(|(t, v)| t.is_changed() || v.is_changed())
                .count();
        }
        let (mut app, ..) = harness(true);
        app.init_resource::<Written>()
            .add_systems(Update, watch.after(turn));
        settle(&mut app);
        assert_eq!(app.world().resource::<Written>().0, 0, "written at rest");
    }

    /// A harness whose frame carries `reading` and the rest transform it is
    /// spawned with, with or without a far side.
    fn harness_reading(reading: Reading, with_back: bool) -> (App, Entity, Entity) {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Time>()
            .add_systems(Update, turn);
        let front = app
            .world_mut()
            .spawn((Node::default(), Side::Front, Visibility::Inherited))
            .id();
        let frame = app
            .world_mut()
            .spawn((
                Node::default(),
                Flip {
                    reading,
                    ..default()
                },
                rest_transform(reading),
            ))
            .id();
        app.world_mut().entity_mut(frame).add_child(front);
        if with_back {
            let back = app
                .world_mut()
                .spawn((Node::default(), Side::Back, Visibility::Hidden))
                .id();
            app.world_mut().entity_mut(frame).add_child(back);
        }
        (app, frame, front)
    }

    fn press_shift(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ShiftLeft);
    }

    fn release_shift(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::ShiftLeft);
    }

    fn reading(rest: u8, turned: u8, over: bool) -> Reading {
        Reading { rest, turned, over }
    }

    /// An ordinary card, and an Adventure (two names, no second side, no
    /// turns recorded), read upright and turn over.
    #[test]
    fn an_ordinary_card_reads_upright() {
        assert_eq!(Reading::of(&[], 0, true), Reading::UPRIGHT);
        assert_eq!(Reading::of(&[], 0, false), Reading::UPRIGHT);
        assert_eq!(Reading::of(&[], 1, false), Reading::UPRIGHT);
        assert_eq!(Reading::default(), Reading::UPRIGHT);
        assert!(!Reading::UPRIGHT.lies_across());
    }

    /// A split card shows both halves at once, sideways, and has no far
    /// side to turn to: shift would do nothing.
    #[test]
    fn a_split_card_lies_across_and_does_not_turn() {
        for face in [0, 1] {
            let read = Reading::of(&[1, 1], face, false);
            assert_eq!(read, reading(1, 1, false), "face {face}");
            assert!(read.lies_across());
        }
    }

    /// A battle's front is sideways, its back an upright creature, and
    /// shift turns it over to that back; read from the back it is the
    /// other way round.
    #[test]
    fn a_battle_turns_over_between_sideways_and_upright() {
        assert_eq!(Reading::of(&[1, 0], 0, true), reading(1, 0, true));
        assert_eq!(Reading::of(&[1, 0], 1, true), reading(0, 1, true));
        assert!(Reading::of(&[1, 0], 0, true).lies_across());
        assert!(!Reading::of(&[1, 0], 1, true).lies_across());
    }

    /// A flip card rotates to its other half, which is upside down.
    #[test]
    fn a_flip_card_rotates_to_its_other_half() {
        assert_eq!(Reading::of(&[0, 2], 0, false), reading(0, 2, false));
        assert_eq!(Reading::of(&[0, 2], 1, false), reading(2, 0, false));
        assert!(!Reading::of(&[0, 2], 1, false).lies_across());
    }

    /// An Aftermath card rotates to its second half, which runs up the
    /// card's side; that is a quarter turn the *short* way round, back.
    #[test]
    fn an_aftermath_card_rotates_a_quarter_counter_clockwise() {
        use std::f32::consts::{FRAC_PI_2, PI};
        let first = Reading::of(&[0, 3], 0, false);
        assert_eq!(first, reading(0, 3, false));
        assert!((first.angle(0.0)).abs() < 1e-6);
        assert!((first.angle(1.0) + FRAC_PI_2).abs() < 1e-6, "short way");
        let flip = Reading::of(&[0, 2], 0, false);
        assert!((flip.angle(1.0) - PI).abs() < 1e-6);
        assert!((flip.angle(0.5) - FRAC_PI_2).abs() < 1e-6);
        // From the second half, back to the first: the reverse quarter.
        let second = Reading::of(&[0, 3], 1, false);
        assert_eq!(second, reading(3, 0, false));
        assert!((second.angle(1.0) - 4.0 * FRAC_PI_2).abs() < 1e-6);
    }

    /// The fit is one for a card standing as its box does, and the
    /// card's own aspect when it is turned across the other way.
    #[test]
    fn a_card_fits_its_box_by_the_turn() {
        use baylee_client_core::layout::CARD_ASPECT;
        use std::f32::consts::FRAC_PI_2;
        assert!((fit(0.0, false) - 1.0).abs() < 1e-6, "upright at rest");
        assert!((fit(FRAC_PI_2, true) - 1.0).abs() < 1e-6, "across at rest");
        assert!((fit(FRAC_PI_2, false) - CARD_ASPECT).abs() < 1e-6);
        assert!((fit(0.0, true) - CARD_ASPECT).abs() < 1e-6);
    }

    /// Whatever the angle, a card is never scaled up to fit.
    #[test]
    fn a_card_never_grows_to_fit() {
        for across in [false, true] {
            for step in 0..=360_u16 {
                let angle = f32::from(step).to_radians();
                let scale = fit(angle, across);
                assert!(
                    scale > 0.0 && scale <= 1.0,
                    "fit({angle}, {across}) = {scale}"
                );
            }
        }
    }

    /// A split card has nothing to turn to: under shift it stays where it
    /// is, rotation included, and its front never hides.
    #[test]
    fn shift_leaves_a_split_card_where_it_lies() {
        let read = Reading::of(&[1, 1], 0, false);
        let (mut app, frame, front) = harness_reading(read, false);
        let at_rest = *app.world().get::<UiTransform>(frame).unwrap();
        press_shift(&mut app);
        settle(&mut app);
        assert!(app.world().get::<Flip>(frame).unwrap().turn.abs() < 1e-6);
        assert_eq!(*app.world().get::<UiTransform>(frame).unwrap(), at_rest);
        assert_eq!(
            app.world().get::<UiTransform>(frame).unwrap().rotation,
            Rot2::radians(std::f32::consts::FRAC_PI_2)
        );
        assert_eq!(
            app.world().get::<Visibility>(front),
            Some(&Visibility::Inherited)
        );
    }

    /// A flip card with no picture on its back rotates half way round under
    /// shift, keeps its front showing, and comes back on release.
    #[test]
    fn shift_rotates_a_flip_card_to_its_other_half() {
        let (mut app, frame, front) = harness_reading(Reading::of(&[0, 2], 0, false), false);
        press_shift(&mut app);
        settle(&mut app);
        assert!((app.world().get::<Flip>(frame).unwrap().turn - 1.0).abs() < 1e-3);
        let transform = *app.world().get::<UiTransform>(frame).unwrap();
        assert!(
            transform
                .rotation
                .angle_to(Rot2::radians(std::f32::consts::PI))
                .abs()
                < 1e-3,
            "rotated {:?}",
            transform.rotation
        );
        assert!(transform.scale.x > 0.0, "rotated, not mirrored");
        assert_eq!(
            app.world().get::<Visibility>(front),
            Some(&Visibility::Inherited),
            "the front never hides when the card only rotates"
        );

        release_shift(&mut app);
        settle(&mut app);
        assert!(app.world().get::<Flip>(frame).unwrap().turn.abs() < 1e-3);
        let transform = *app.world().get::<UiTransform>(frame).unwrap();
        assert!(transform.rotation.angle_to(Rot2::IDENTITY).abs() < 1e-3);
    }

    /// A card that rotates ignores a back it was somehow given: it does not
    /// swap sides at the quarter turn.
    #[test]
    fn a_rotating_card_never_shows_a_back() {
        let (mut app, frame, front) = harness_reading(Reading::of(&[0, 2], 0, false), true);
        press_shift(&mut app);
        settle(&mut app);
        assert_eq!(
            app.world().get::<Visibility>(front),
            Some(&Visibility::Inherited)
        );
        assert!(app.world().get::<UiTransform>(frame).unwrap().scale.x > 0.0);
    }

    /// A card spawned lying across, with the transform `rest_transform`
    /// names, is not touched by the first frame: it never stood upright.
    #[test]
    fn a_card_spawned_across_is_not_rewritten_at_rest() {
        #[derive(Resource, Default)]
        struct Written(usize);
        fn watch(mut written: ResMut<Written>, parts: Query<Ref<UiTransform>, With<Flip>>) {
            written.0 = parts
                .iter()
                .filter(|t| t.is_added() || t.is_changed())
                .count();
        }
        let read = Reading::of(&[1, 1], 0, false);
        let (mut app, frame, _) = harness_reading(read, false);
        app.init_resource::<Written>()
            .add_systems(Update, watch.after(turn));
        // The spawn itself counts as a change on the first frame; the second
        // is the one that shows whether the system wrote anything.
        app.update();
        app.update();
        assert_eq!(app.world().resource::<Written>().0, 0, "written at rest");
        let now = *app.world().get::<UiTransform>(frame).unwrap();
        let spawned = rest_transform(read);
        assert_eq!(now.rotation, spawned.rotation);
        assert_eq!(now.scale, spawned.scale);
        assert!(read.lies_across());
        assert!(
            spawned.rotation.angle_to(Rot2::IDENTITY).abs() > 1.0,
            "turned"
        );
    }
}

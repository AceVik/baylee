//! The hover preview's coming and going.
//!
//! The preview is part of the tree [`sync_overlay`] sweeps on every pointer
//! move, so until the owner asked for it (12.11.2026: "schöne Animationen,
//! bei hoch und weg kommen") it had neither: it was there on one frame and
//! gone on another. Now it rises from the card it describes and sinks back
//! towards it, on the HUD's own curves ([`motion::opening`],
//! [`motion::shutting`]).
//!
//! Three things make that possible in a tree rebuilt under it:
//!
//! - **The opening's clock is a resource** ([`PreviewPop`]), not a component:
//!   a preview rebuilt while it opens (its art arrived, the stack grew) is
//!   spawned at the pose the clock is at and carries on from there.
//! - **A preview the pointer has left outlives the sweep** ([`PreviewLeave`]):
//!   the rebuild hands the old nodes over instead of despawning them, and
//!   [`pop_the_preview`] despawns them once they have sunk.
//! - **One preview handed straight on to the next does not open again.**
//!   Running the pointer along a hand is one panel showing card after card;
//!   a pop per card would be a strobe.
//!
//! Only the nodes move: their scale, their place and the shadow under the
//! card. Nothing is drawn on a print and no print is faded
//! (`docs/legal.md` §3).
//!
//! [`sync_overlay`]: super::sync_overlay

use crate::hud::{PreviewResize, motion, palette};
use baylee_client_core::images::ImageKey;
use baylee_core::ids::ObjectId;
use bevy::prelude::*;
use bevy::ui::{ShadowStyle, UiTransform, Val2};

/// How far from its place a preview starts, towards the card it describes,
/// in logical pixels.
const RISE: f32 = 12.0;

/// How far back towards it a leaving preview sinks.
const SINK: f32 = 8.0;

/// The light round a preview held up: the table's moonlight, never an
/// offer's gold (which says "playable") and never on the card itself.
const HALO: Color = Color::srgb(0.56, 0.68, 1.0);

/// What a preview is of: the hovered object, or the printing a log line
/// named. A change of it is a new preview; a rebuild that leaves it alone is
/// the same one drawn again.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct PreviewSubject {
    pub(crate) object: Option<ObjectId>,
    pub(crate) art: Option<ImageKey>,
}

/// The standing preview's opening: what it is of and how far it has come.
#[derive(Resource, Default)]
pub struct PreviewPop {
    subject: Option<PreviewSubject>,
    /// 0 → 1 over [`motion::ZOOM_IN`].
    t: f32,
}

impl PreviewPop {
    /// A preview of `subject` stands from this rebuild on. `handed_on` says
    /// one stood before it, which is a panel changing its card and not a
    /// panel opening; `still` is a player who turned motion off. Returns how
    /// far open it is to be drawn.
    pub(crate) fn stand(&mut self, subject: PreviewSubject, handed_on: bool, still: bool) -> f32 {
        if self.subject != Some(subject) {
            self.subject = Some(subject);
            self.t = if handed_on || still { 1.0 } else { 0.0 };
        }
        self.t
    }

    /// No preview stands.
    pub(crate) fn fall(&mut self) {
        if self.subject.is_some() {
            self.subject = None;
            self.t = 0.0;
        }
    }

    /// Whether the standing preview is still on its way up.
    fn opening(&self) -> bool {
        self.subject.is_some() && self.t < 1.0
    }
}

/// A node of the standing preview, which opens with it: the bubble itself
/// and the small cards beside it.
#[derive(Component, Clone, Copy, Debug)]
pub struct PreviewRise {
    /// The unit vector from the preview towards what it describes, or zero
    /// for a preview that stands nowhere in particular.
    pub(crate) toward: Vec2,
    /// This node's centre from the bubble's, so that the bubble and what
    /// stands beside it grow as one thing about one point.
    pub(crate) lean: Vec2,
}

impl PreviewRise {
    /// The same rise for a panel `size` at `at` that stands beside the
    /// bubble (`panel` at `place`): leaning from the bubble's centre, so the
    /// two open as one thing.
    pub(crate) fn beside(self, at: Vec2, size: Vec2, place: Vec2, panel: Vec2) -> Self {
        Self {
            lean: (at + size / 2.0) - (place + panel / 2.0),
            ..self
        }
    }
}

/// A node of a preview the pointer has left, sinking away.
#[derive(Component, Clone, Copy, Debug)]
pub struct PreviewLeave {
    /// 0 → 1 over [`motion::ZOOM_OUT`].
    t: f32,
    toward: Vec2,
    lean: Vec2,
}

/// The node whose shadow is the preview's height over the table: the card's
/// own box inside the bubble.
#[derive(Component)]
pub struct PreviewShade;

/// Which way a preview placed at `place`, `panel` in size, looks to find what
/// it describes: down at the hand, at a card's middle, at the pointer; nowhere
/// for one the keyboard opened.
pub(crate) fn toward(at: crate::hud::hand::PreviewAt, place: Vec2, panel: Vec2) -> Vec2 {
    use crate::hud::hand::PreviewAt;
    let centre = place + panel / 2.0;
    match at {
        PreviewAt::Hand(_) => Vec2::Y,
        PreviewAt::Card(card) => (card.center() - centre).normalize_or_zero(),
        PreviewAt::Pointer(pointer) => (pointer - centre).normalize_or_zero(),
        PreviewAt::Loose => Vec2::ZERO,
    }
}

/// The pose of a node `lean` from the bubble's centre while the bubble is
/// drawn at `scale`, `adrift` pixels short of its place towards `toward`.
fn pose(scale: f32, adrift: f32, toward: Vec2, lean: Vec2) -> (Vec2, Val2) {
    let at = lean * (scale - 1.0) + toward * adrift;
    (Vec2::splat(scale), Val2::px(at.x, at.y))
}

/// How far a preview `t` of the way open has still to travel: an ease-out,
/// so it arrives rather than stops.
fn adrift(t: f32) -> f32 {
    let left = 1.0 - t.clamp(0.0, 1.0);
    RISE * left * left * left
}

/// The transform a node of the standing preview is spawned with and held at
/// while the preview is `t` of the way open.
pub(crate) fn rising(t: f32, rise: PreviewRise) -> UiTransform {
    let (scale, translation) = pose(motion::opening(t), adrift(t), rise.toward, rise.lean);
    UiTransform {
        scale,
        translation,
        ..default()
    }
}

/// The shadow and the light round a card held `height` of the way up: a
/// tight contact shadow that is always there, a wide soft one that is the
/// height, and a breath of moonlight round the edge. Cast upward like every
/// raised thing on this HUD ([`crate::hud::upward_shadow`]), so it lies on
/// the felt and not on the hand under it.
pub(crate) fn shade(height: f32) -> BoxShadow {
    let h = height.clamp(0.0, 1.0);
    BoxShadow(vec![
        ShadowStyle {
            color: palette::SHADOW.with_alpha(0.30 + 0.25 * h),
            x_offset: Val::Px(0.0),
            y_offset: Val::Px(-1.0 - 2.0 * h),
            spread_radius: Val::Px(0.0),
            blur_radius: Val::Px(4.0 + 4.0 * h),
        },
        ShadowStyle {
            color: palette::SHADOW.with_alpha(0.50 * h),
            x_offset: Val::Px(0.0),
            y_offset: Val::Px(-3.0 - 7.0 * h),
            spread_radius: Val::Px(2.0 * h),
            blur_radius: Val::Px(10.0 + 26.0 * h),
        },
        ShadowStyle {
            color: HALO.with_alpha(0.13 * h),
            x_offset: Val::Px(0.0),
            y_offset: Val::Px(0.0),
            spread_radius: Val::Px(0.0),
            blur_radius: Val::Px(4.0 + 16.0 * h),
        },
    ])
}

/// The height a preview `t` of the way open has reached, for [`shade`].
pub(crate) fn height(t: f32) -> f32 {
    let left = 1.0 - t.clamp(0.0, 1.0);
    1.0 - left * left
}

/// Hands the nodes of a preview the pointer has left over to their leaving:
/// they keep their place, lose what made them the standing preview
/// ([`super::PreviewBounds`], the resize handle) and no longer take the
/// pointer. With motion off, or with no clock to leave by, they are gone at
/// once.
pub(crate) fn leave(
    commands: &mut Commands,
    nodes: Vec<(Entity, PreviewRise)>,
    handles: &Query<Entity, With<PreviewResize>>,
    animated: bool,
) {
    if nodes.is_empty() {
        return;
    }
    // The handle is the old preview's: there is one preview, so one handle.
    for handle in handles {
        commands.entity(handle).despawn();
    }
    for (node, rise) in nodes {
        if animated {
            commands
                .entity(node)
                .remove::<(PreviewRise, super::PreviewBounds)>()
                .insert(PreviewLeave {
                    t: 0.0,
                    toward: rise.toward,
                    lean: rise.lean,
                })
                .insert_recursive::<Children>(Pickable::IGNORE);
        } else {
            commands.entity(node).despawn();
        }
    }
}

/// Runs the preview's opening and every leaving preview's sinking, and
/// writes nothing once they are over: a preview at rest is not touched.
pub fn pop_the_preview(
    mut commands: Commands,
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut pop: ResMut<PreviewPop>,
    mut rise: Query<(&PreviewRise, &mut UiTransform), Without<PreviewLeave>>,
    mut leave: Query<(Entity, &mut PreviewLeave, &mut UiTransform), Without<PreviewRise>>,
    mut shades: Query<(&ChildOf, &mut BoxShadow), With<PreviewShade>>,
) {
    let still = prefs.all().reduce_motion;
    let delta = time.delta_secs();
    let opened = pop.opening().then(|| {
        let t = motion::step(pop.t, motion::ZOOM_IN, delta, still);
        pop.t = t;
        t
    });
    if let Some(t) = opened {
        for (node, mut transform) in &mut rise {
            *transform = rising(t, *node);
        }
    }
    for (entity, mut node, mut transform) in &mut leave {
        let t = motion::step(node.t, motion::ZOOM_OUT, delta, still);
        if t >= 1.0 {
            commands.entity(entity).despawn();
            continue;
        }
        node.t = t;
        let (scale, translation) = pose(motion::shutting(t), SINK * t * t, node.toward, node.lean);
        transform.scale = scale;
        transform.translation = translation;
    }
    for (parent, mut shadow) in &mut shades {
        let bubble = parent.parent();
        if let Ok((_, node, _)) = leave.get(bubble) {
            *shadow = shade(1.0 - node.t);
        } else if let Some(t) = opened
            && rise.contains(bubble)
        {
            *shadow = shade(height(t));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subject(n: u32) -> PreviewSubject {
        PreviewSubject {
            object: Some(ObjectId::new(n, 0)),
            art: None,
        }
    }

    /// A preview opens from nothing, is the same preview across a rebuild,
    /// and one handed straight on to the next card does not open again.
    #[test]
    fn a_preview_opens_once_and_a_card_handed_on_does_not_open_again() {
        let mut pop = PreviewPop::default();
        assert!(pop.stand(subject(1), false, false).abs() < f32::EPSILON);
        pop.t = 0.4;
        assert!(
            (pop.stand(subject(1), true, false) - 0.4).abs() < f32::EPSILON,
            "a rebuild of the same preview carries on where it was"
        );
        assert!(
            (pop.stand(subject(2), true, false) - 1.0).abs() < f32::EPSILON,
            "the next card in the same panel stands at once"
        );
        pop.fall();
        assert!(!pop.opening());
        assert!(pop.stand(subject(2), false, false).abs() < f32::EPSILON);
        assert!(pop.opening());
        pop.fall();
        assert!(
            (pop.stand(subject(3), false, true) - 1.0).abs() < f32::EPSILON,
            "with motion off the movement is over on the frame it started"
        );
    }

    /// The opening starts small and short of its place towards the card, and
    /// ends at exactly full size in exactly its place; what stands beside
    /// the bubble grows about the bubble's centre, not about its own.
    #[test]
    fn a_preview_rises_from_the_card_and_ends_exactly_in_place() {
        let rise = PreviewRise {
            toward: Vec2::new(0.0, 1.0),
            lean: Vec2::ZERO,
        };
        let start = rising(0.0, rise);
        assert!((start.scale.x - motion::ZOOM_FROM).abs() < 1e-5);
        assert_eq!(start.translation, Val2::px(0.0, RISE));
        let end = rising(1.0, rise);
        assert_eq!(end.scale, Vec2::ONE);
        assert_eq!(end.translation, Val2::px(0.0, 0.0));
        let beside = PreviewRise {
            toward: Vec2::ZERO,
            lean: Vec2::new(200.0, 0.0),
        };
        let Val::Px(x) = rising(0.0, beside).translation.x else {
            panic!("a pixel translation");
        };
        assert!(
            (x - 200.0 * (motion::ZOOM_FROM - 1.0)).abs() < 1e-3,
            "a node 200 px right of the bubble's centre starts that much nearer it: {x}"
        );
    }

    /// The shadow is the height: tight at the table, wide when held up, and
    /// the light round the card is not there at all until the card is up.
    #[test]
    fn the_shadow_widens_as_the_card_comes_up() {
        let (low, high) = (shade(0.0), shade(1.0));
        assert_eq!(low.0.len(), high.0.len());
        let blur = |shadow: &BoxShadow, layer: usize| match shadow.0[layer].blur_radius {
            Val::Px(px) => px,
            other => panic!("a pixel blur, not {other:?}"),
        };
        assert!(blur(&high, 1) > 2.0 * blur(&low, 1));
        assert!(low.0[1].color.alpha().abs() < f32::EPSILON);
        assert!(low.0[2].color.alpha().abs() < f32::EPSILON);
        assert!(high.0[2].color.alpha() > 0.0);
        assert!((height(1.0) - 1.0).abs() < f32::EPSILON && height(0.0).abs() < f32::EPSILON);
    }

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<crate::prefs::Prefs>()
            .init_resource::<PreviewPop>()
            .add_systems(Update, pop_the_preview);
        app
    }

    fn tick(app: &mut App, secs: f32) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(secs));
        app.update();
    }

    /// The whole opening, run: it moves while it opens, and once it stands
    /// the system writes neither the node's transform nor its shadow again.
    #[test]
    fn a_preview_at_rest_is_not_written() {
        let mut app = app();
        let rise = PreviewRise {
            toward: Vec2::new(0.0, 1.0),
            lean: Vec2::ZERO,
        };
        app.world_mut()
            .resource_mut::<PreviewPop>()
            .stand(subject(1), false, false);
        let bubble = app.world_mut().spawn((rise, rising(0.0, rise))).id();
        let card = app
            .world_mut()
            .spawn((PreviewShade, shade(0.0), ChildOf(bubble)))
            .id();
        tick(&mut app, motion::ZOOM_IN / 2.0);
        let half = *app.world().get::<UiTransform>(bubble).expect("a transform");
        assert!(half.scale.x > motion::ZOOM_FROM, "it has begun to open");
        assert!(
            app.world().get::<BoxShadow>(card).expect("a shadow").0[1]
                .color
                .alpha()
                > 0.0,
            "and to come up off the table"
        );
        tick(&mut app, motion::ZOOM_IN);
        let stood = *app.world().get::<UiTransform>(bubble).expect("a transform");
        assert_eq!(stood.scale, Vec2::ONE);
        assert_eq!(stood.translation, Val2::px(0.0, 0.0));
        // At rest: nothing is written, frame after frame. Read by a system
        // in the same frame, since `App::update` clears the trackers.
        {
            #[derive(Resource, Default)]
            struct Written(usize);
            fn watch(
                mut written: ResMut<Written>,
                transforms: Query<Ref<UiTransform>>,
                shadows: Query<Ref<BoxShadow>>,
            ) {
                written.0 = transforms.iter().filter(DetectChanges::is_changed).count()
                    + shadows.iter().filter(DetectChanges::is_changed).count();
            }
            app.init_resource::<Written>()
                .add_systems(Update, watch.after(pop_the_preview));
            tick(&mut app, 0.016);
            tick(&mut app, 0.016);
            assert_eq!(
                app.world().resource::<Written>().0,
                0,
                "a preview at rest is not touched, nor the shadow under it"
            );
        }
        let _ = card;
    }

    /// A preview the pointer has left is still there while it sinks, smaller
    /// and nearer the card, and gone once [`motion::ZOOM_OUT`] has passed.
    #[test]
    fn a_left_preview_sinks_and_is_then_gone() {
        let mut app = app();
        let node = app
            .world_mut()
            .spawn((
                PreviewLeave {
                    t: 0.0,
                    toward: Vec2::new(0.0, 1.0),
                    lean: Vec2::ZERO,
                },
                UiTransform::default(),
            ))
            .id();
        tick(&mut app, motion::ZOOM_OUT / 2.0);
        let half = *app.world().get::<UiTransform>(node).expect("still there");
        assert!(half.scale.x < 1.0 && half.scale.x > motion::ZOOM_FROM);
        let Val::Px(y) = half.translation.y else {
            panic!("a pixel translation");
        };
        assert!(y > 0.0 && y < SINK, "sinking towards the card: {y}");
        tick(&mut app, motion::ZOOM_OUT);
        assert!(app.world().get_entity(node).is_err(), "and then gone");
    }

    /// With motion off a left preview does not linger for a frame.
    #[test]
    fn with_motion_off_a_left_preview_is_gone_at_once() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<crate::prefs::Prefs>()
            .edit()
            .reduce_motion = true;
        let node = app
            .world_mut()
            .spawn((
                PreviewLeave {
                    t: 0.0,
                    toward: Vec2::ZERO,
                    lean: Vec2::ZERO,
                },
                UiTransform::default(),
            ))
            .id();
        tick(&mut app, 0.001);
        assert!(app.world().get_entity(node).is_err());
    }
}

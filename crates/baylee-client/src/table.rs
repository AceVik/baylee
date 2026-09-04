//! The 2.5D table: a real 3D stage, with flat cards lying on it.
//!
//! # Why 2.5D and not 2D or 3D
//!
//! Cards are flat objects on a flat surface, so modelling them as textured
//! quads costs nothing and buys three things a 2D renderer cannot do cheaply:
//! tapping is a rotation rather than a swapped sprite, the near seat can be
//! given more screen area than the far ones by perspective alone, and focusing
//! an opponent is a camera move instead of a re-layout.
//!
//! Everything a player *reads* — prompt, hand, stack, life totals — stays in
//! the 2D overlay, where text is crisp and layout is predictable. The rule is
//! simply: if it is a card on a battlefield it is in the world; if it is
//! information about the game it is in the overlay.
//!
//! # Redrawing
//!
//! [`sync_scene`] diffs the board model against the entities that exist and
//! touches only what changed. A board that did not change costs one hash lookup
//! per card and no allocation, which is what keeps a 300-permanent commander
//! table at frame rate on a phone.

use crate::Duel;
use crate::cardmat::{CardLook, CardMaterial, material};
use crate::face;
use crate::textures::CardTextures;
use baylee_client_core::board::CardGroup;
use baylee_client_core::images::{FinishTreatment, ImageKey};
use baylee_client_core::layout::{CARD_HEIGHT, CARD_WIDTH, SeatSlot, TableLayout, pack_lane};
use baylee_client_core::tabletop;
use baylee_core::color::ColorSet;
use baylee_core::ids::ObjectId;
use baylee_core::ids::PlayerId;
use bevy::asset::RenderAssetUsages;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Height of the table surface; cards float a hair above it so they never
/// z-fight with the felt.
const TABLE_Y: f32 = 0.0;
/// Vertical gap between the felt and a card.
const CARD_LIFT: f32 = 0.01;
/// Where a seat's mat sits: above the felt, below everything played on it.
const ZONE_LIFT: f32 = 0.002;
/// Where the glow under a mat sits — below the mat, above the felt.
const GLOW_LIFT: f32 = 0.001;
/// Where the centre medallion is inlaid.
const MEDALLION_LIFT: f32 = 0.0015;
/// The felt's extent. Big enough that no seat sees its edge.
const FELT: Vec2 = Vec2::new(60.0, 44.0);
/// Where the lamplight pool lies: on the felt, under everything else.
const HEARTH_LIFT: f32 = 0.0008;
/// How far the lamplight and its inlaid ring reach, in table units.
///
/// It shipped at 34, which put a twenty-unit ring with twenty-four ticks on
/// it across the middle of the felt: the largest, brightest, most detailed
/// thing on screen, and what the eye read was a roulette wheel. The bound is
/// in `docs/design.md` §1.1 and is measured by
/// `camera_tests::the_hearth_ring_is_smaller_than_the_nearest_seats_mat` —
/// the lamp is atmosphere, the mats are where the game is.
const HEARTH_SIZE: f32 = 18.0;
/// How wide the medallion is inlaid, in table units.
const MEDALLION_SIZE: f32 = 9.5;
/// Where the phase wash lies: over the lamplight pool it colours, under the
/// medallion, which it must never touch.
const WASH_LIFT: f32 = 0.0011;
/// How far the phase wash reaches.
///
/// Sized against the *ring*, not against the pool quad it shares a texture
/// with: [`tabletop::hearth`] puts its band at
/// [`tabletop::HEARTH_INNER`]–[`tabletop::HEARTH_OUTER`] of the quad's half
/// width, so the ring a player actually sees is a good deal smaller than
/// [`HEARTH_SIZE`]. A wash wider than that stops reading as a lamp over the
/// middle of the table and starts reading as the table being that colour,
/// which is the version this shipped as first and the reason the number is
/// tied to the ring rather than written down.
const WASH_SIZE: f32 = HEARTH_SIZE * tabletop::HEARTH_OUTER * 1.2;
/// How strong the wash is at [`tabletop::PhaseLight::energy`] 1.0.
const WASH_ALPHA: f32 = 0.24;
/// How fast the wash follows a phase change, per second.
///
/// Slower than a card moves. A step boundary is not an event a player has to
/// catch — it is a condition they should notice having changed — and a wash
/// that snapped would flicker through the four steps of combat.
const WASH_RATE: f32 = 3.0;
/// Margin around a seat's pod, so its mat is a table the cards sit on rather
/// than a box drawn tight around them.
const ZONE_MARGIN: f32 = 0.55;
/// How far past the mat the glow beneath it spreads.
const GLOW_SPREAD: f32 = 2.4;

/// Extra lift per card in a counted stack, so a stack reads as a stack.
const STACK_LIFT: f32 = 0.006;
/// The back of a card: what a stack behind a counted group is made of, and
/// what a card whose art never arrives falls back to.
const BACK_COLOR: Color = Color::srgb(0.12, 0.14, 0.18);
/// How many cards of a group are drawn behind the representative.
const MAX_STACK_DEPTH: usize = 4;
/// Lift and scale for the card under the cursor (subtle — a glance, not a jump).
const HOVER_LIFT: f32 = 0.12;
const HOVER_SCALE: f32 = 1.05;
/// Lift and scale for a card chosen for the pending choice (clearly "in").
const SELECTED_LIFT: f32 = 0.22;
const SELECTED_SCALE: f32 = 1.07;

/// Marks everything spawned for the duel, so closing it is one despawn.
#[derive(Component)]
pub struct DuelStage;

/// The wash of colour over the middle of the table that says which step of
/// the turn this is, and the colour it is currently showing.
///
/// The colour is carried on the component rather than read back out of the
/// material because it is the *eased* value: a material's `base_color` is
/// where it ends up, and easing towards a target needs somewhere to keep
/// where it started.
#[derive(Component)]
pub struct PhaseWash {
    /// The wash's current colour and strength, linear RGBA.
    shown: LinearRgba,
}

/// The table camera.
#[derive(Component)]
pub struct TableCamera;

/// The table camera's state: where it looks, from how far, at which
/// azimuth. Input systems move this; [`apply_camera_rig`] turns it into a
/// transform, so navigation (tabs, keys, drag, gestures) all ends up in
/// one place.
#[derive(Resource, Clone, Copy, PartialEq, Debug)]
pub struct CameraRig {
    /// Look-at point in world space (x/z).
    pub target: Vec2,
    /// Distance from the target (zoom).
    pub distance: f32,
    /// Azimuth around the target (0 = behind the local seat).
    pub yaw: f32,
}

impl Default for CameraRig {
    fn default() -> Self {
        Self {
            target: Vec2::ZERO,
            distance: 20.0,
            yaw: 0.0,
        }
    }
}

impl CameraRig {
    /// Zoom limits.
    pub const MIN_DISTANCE: f32 = 7.0;
    /// Zoom limits.
    pub const MAX_DISTANCE: f32 = 46.0;

    /// Moves the rig so `pod` (a seat's table-space centre) fills the free
    /// canvas area: camera outside the ellipse looking inward, cards
    /// upright with their bottoms toward the screen bottom, the pod
    /// shifted clear of the own-board overlay.
    #[must_use]
    pub fn framing(slot: &SeatSlot, world_center: Vec2) -> Self {
        Self {
            target: world_center * 0.72,
            distance: (slot.half_extent.length() * 2.6).clamp(9.0, Self::MAX_DISTANCE),
            yaw: world_center.y.atan2(world_center.x) + std::f32::consts::FRAC_PI_2,
        }
    }

    /// The whole table, framed inside the part of the window it is actually
    /// seen through.
    ///
    /// This is the shot a duel opens on and the one `navigate_home` returns
    /// to, and it is computed rather than written down because the thing it
    /// has to fit changes: two seats and eight seats are different tables,
    /// and a phone and a monitor leave different amounts of them uncovered.
    /// The hard-coded 20 units it replaced put the local seat's own mat under
    /// the hand bar on every screen — a player could not see their own
    /// creatures, which made every later piece of board legibility moot.
    #[must_use]
    pub fn home(layout: &TableLayout, canvas: Canvas) -> Self {
        let Some((min, max)) = layout.extent() else {
            return Self::default();
        };
        let (min, max) = (min - Vec2::splat(AIR), max + Vec2::splat(AIR));
        let span = max - min;

        // The free band, as normalised device coordinates: +1 is the top of
        // the window, and the tab strip and the hand bar eat inwards.
        let top = 1.0 - 2.0 * canvas.top / canvas.window.y.max(1.0);
        let bottom = -1.0 + 2.0 * canvas.bottom / canvas.window.y.max(1.0);
        let right = 1.0 - 2.0 * canvas.right / canvas.window.x.max(1.0);
        let aspect = canvas.window.x / canvas.window.y.max(1.0);

        // Vertically this is exact: `ground` is linear in the eye distance,
        // so the distance at which the table's far edge lands on `top` and
        // its near edge on `bottom` is one division.
        let deep = span.y / (ground(top) - ground(bottom)).max(1e-3);
        // Horizontally the binding edge is the *near* one. A perspective
        // camera sees less of the felt where the felt is closer, so the band
        // measured at the look plane is not the band the front row has to fit
        // in — measuring there put a four-seat table's outermost mat past the
        // rail. The near edge sits at `min.y`, whose depth is
        // `eye·(1 + k·g_top) − k·span.y` once the far edge is pinned, and
        // that is linear in `eye` too, so requiring the span to fit *there*
        // is still one division rather than a search.
        let g_top = ground(top);
        let k = CAMERA_LEAN / (1.0 + CAMERA_LEAN * CAMERA_LEAN).sqrt();
        let wide = k.mul_add(
            span.y,
            span.x / (half_fov().tan() * aspect * (1.0 + right)).max(1e-3),
        ) / k.mul_add(g_top, 1.0);
        // Clamped *before* the look point is derived from it. Aiming for a
        // camera the clamp then moves is the one way this can put the table
        // off screen while every number above is still right: the far edge
        // would be pinned for an eye that is not there, and land above the
        // tab strip. Clamped first, a table too big for `MAX_DISTANCE` keeps
        // its far edge pinned and overflows at the bottom, which is the
        // graceful direction.
        let lean = (1.0 + CAMERA_LEAN * CAMERA_LEAN).sqrt();
        let eye = deep
            .max(wide)
            .clamp(Self::MIN_DISTANCE * lean, Self::MAX_DISTANCE * lean);

        // The far edge is pinned under the tab strip. Any slack a wide table
        // bought then opens up at the *bottom*, in front of the local seat,
        // which is where a player would rather have it than behind the
        // opponent they are looking at.
        //
        // Sideways the span is centred in the band as it stands at the near
        // edge, for the same reason `wide` was measured there: centring on
        // the look plane's band leaves the front row off-centre, and the rail
        // makes the band asymmetric, so being off-centre costs a whole mat on
        // one side.
        let near = k.mul_add(-span.y, eye * k.mul_add(g_top, 1.0)).max(1e-3);
        let half_band = near * half_fov().tan() * aspect;
        let look = Vec2::new(
            (min.x + max.x).mul_add(0.5, -(half_band * (right - 1.0) * 0.5)),
            max.y - eye * g_top,
        );
        Self {
            // `ground` works from the eye's true distance; the rig stores the
            // height it stands at, which the lean makes shorter.
            distance: eye / lean,
            // Table space to world: `+y` away from the local seat is `-z`.
            target: Vec2::new(look.x, -look.y),
            yaw: 0.0,
        }
    }
}

/// How much bare felt is left around the table when it is framed.
const AIR: f32 = 0.6;

/// Half the camera's vertical field of view.
fn half_fov() -> f32 {
    FOV * 0.5
}

/// Where a point on the felt lands on screen, in one axis, per unit of eye
/// distance.
///
/// Take `s` to be table-space distance from the look point along the
/// screen-vertical, positive away from the local seat, and `q` to be
/// normalised device y. With the eye at distance `D` and the lean written as
/// `L` = [`CAMERA_LEAN`], `C = 1/√(1+L²)`, the camera-space depth and height
/// of that point work out to
///
/// ```text
///     depth(s) = D + L·C·s          height(s) = C·s
/// ```
///
/// — the cross terms cancel, which is the whole reason this is arithmetic and
/// not a projection matrix. So `q = height / (depth · tan(fov/2))`, and solved
/// the other way `s = D · ground(q)`. Being *linear in `D`* is what lets
/// [`CameraRig::home`] invert it with a division instead of a search.
fn ground(q: f32) -> f32 {
    let t = half_fov().tan();
    let c = 1.0 / (1.0 + CAMERA_LEAN * CAMERA_LEAN).sqrt();
    q * t / (c * q.mul_add(-CAMERA_LEAN * t, 1.0))
}

/// The part of the window the table is actually seen through.
///
/// The HUD is not beside the battlefield, it is on top of it: the tab strip,
/// the hand bar and the phase rail are overlays on the same full-window
/// camera. Framing the table against the *window* therefore frames it against
/// a rectangle whose bottom sixth nobody can see.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Canvas {
    /// The window, in logical pixels.
    pub window: Vec2,
    /// Covered at the top: the tab strip.
    pub top: f32,
    /// Covered at the bottom: the hand bar.
    pub bottom: f32,
    /// Covered on the right: the phase rail.
    pub right: f32,
}

impl Canvas {
    /// What the duel HUD covers of a window this size.
    #[must_use]
    pub fn hud(window: Vec2) -> Self {
        Self {
            window,
            top: crate::hud::TAB_H,
            bottom: crate::hud::HAND_BAR_H,
            right: crate::hud::rail::RAIL_W,
        }
    }
}

/// The framing the table currently deserves, and whether the camera is still
/// following it.
///
/// A rig that equals the last framing — or that is still
/// [`CameraRig::default`], which is what the resource starts as and what
/// `navigate_home` asks for, neither of them a place anyone aimed at — is the
/// table's camera and follows the table. One drag, zoom or focus and it is
/// the player's, and a window resize no longer moves it.
#[derive(Resource, Clone, Copy, Default)]
pub struct HomeRig(Option<CameraRig>);

/// Keeps the table framed as seats, focus and window size change.
pub fn frame_table(
    duel: Res<Duel>,
    windows: Query<&Window>,
    mut home: ResMut<HomeRig>,
    mut rig: ResMut<CameraRig>,
) {
    let Some(layout) = duel.layout.as_ref() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let next = CameraRig::home(
        layout,
        Canvas::hud(Vec2::new(window.width(), window.height())),
    );
    let current: CameraRig = *rig;
    let following = home.0.is_none_or(|last| current == last) || current == CameraRig::default();
    if following && current != next {
        *rig = next;
    }
    if home.0 != Some(next) {
        home.0 = Some(next);
    }
}

/// How far the camera stands off to the side, as a fraction of its height.
///
/// The table is read from above, and it has to be: a four-seat pod ring is
/// laid out for a plan view, and the further the camera leans the more a far
/// seat's cards shrink against a near seat's. But a *purely* top-down camera
/// throws away every cue that a card is an object — its edge projects to
/// nothing, its contact shadow hides underneath it, and the board reads as
/// artwork printed into the felt.
///
/// This is the compromise: about 22° off vertical, which is enough that a
/// card's edge and the shadow around it are both visible, and far too little
/// to bring a horizon into frame — which is why there is no sky behind the
/// table and no point drawing one.
const CAMERA_LEAN: f32 = 0.40;

/// The camera's vertical field of view, in radians.
///
/// Shared with [`CameraRig::home`], which inverts the projection to work out
/// how far back the table has to stand — a framing computed against a
/// different angle from the one the camera is set to is a framing that misses.
const FOV: f32 = 0.7;

/// Where the camera actually is, as against where the rig says it should be.
///
/// A second copy rather than smoothing the rig itself, because the rig is
/// *input*: a drag writes it, a zoom writes it, focusing a seat writes it,
/// and every one of those wants to be able to say "there" without having to
/// know that something else is interpolating behind it.
#[derive(Resource, Clone, Copy, Default)]
pub struct ShownRig(Option<CameraRig>);

/// Turns the rig into the camera transform: a near-plan view of the
/// battlefield canvas, leaning by [`CAMERA_LEAN`] so the cards have
/// somewhere to cast a shadow; the rig decides target, zoom, and azimuth.
///
/// The camera follows the rig rather than snapping to it, so tabbing to
/// another seat is a move across the table and not a cut. Yaw is interpolated
/// the short way around, or focusing the seat on your left would spin the
/// table three-quarters of the way round to reach it.
pub fn apply_camera_rig(
    rig: Res<CameraRig>,
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut shown: ResMut<ShownRig>,
    mut cams: Query<&mut Transform, With<TableCamera>>,
) {
    let target = *rig;
    let current = match shown.0 {
        // The first frame is a cut by definition: there is nowhere to come
        // from. So is a table a player has asked to hold still.
        None => target,
        Some(_) if prefs.all().reduce_motion => target,
        Some(current) => {
            let t = 1.0 - (-CAMERA_SETTLE * time.delta_secs()).exp();
            let turn = std::f32::consts::TAU;
            let yaw_delta = (target.yaw - current.yaw + std::f32::consts::PI).rem_euclid(turn)
                - std::f32::consts::PI;
            CameraRig {
                target: current.target.lerp(target.target, t),
                distance: current.distance + (target.distance - current.distance) * t,
                yaw: current.yaw + yaw_delta * t,
            }
        }
    };
    // Nothing moved and nothing was asked for: the camera stands still most
    // of the time and should cost nothing then.
    if shown.0 == Some(current) && !rig.is_changed() {
        return;
    }
    shown.0 = Some(current);

    let horizontal = current.distance * CAMERA_LEAN;
    let height = current.distance;
    let offset = Vec3::new(
        current.yaw.sin() * horizontal,
        height,
        current.yaw.cos() * horizontal,
    );
    let look = Vec3::new(current.target.x, 0.0, current.target.y);
    for mut transform in &mut cams {
        *transform = Transform::from_translation(look + offset).looking_at(look, Vec3::Y);
    }
}

/// How quickly a card settles onto its mark, as a fraction of the remaining
/// distance per second.
///
/// Exponential rather than a fixed duration, because the thing being animated
/// is a *correction*: a card whose lane repacked by half a millimetre and a
/// card that just entered the battlefield are the same code path, and the
/// first must not take as long as the second. At 16 the long move reads as a
/// deal and the short one as a settle, which is what a hand on a real table
/// looks like.
const SETTLE: f32 = 16.0;

/// Below this, a card is simply put on its mark: the last hundredth of a
/// millimetre of an exponential curve is not worth a frame of work, and
/// leaving it unfinished is what makes a "still" board quietly never idle.
const SETTLED: f32 = 0.0008;

/// How far above the table a card appears before dropping onto it.
///
/// Direction-agnostic on purpose. A card could fly in from its owner's hand,
/// and at four seats around a ring that means four different directions and a
/// card that flies *across* two other players' boards to get home. Dropping
/// in reads as "this arrived" from every chair.
const ENTRANCE_RISE: f32 = 1.4;

/// How small a card is when it appears, before it settles to full size.
const ENTRANCE_SCALE: f32 = 0.86;

/// How quickly the camera settles, in the same units as [`SETTLE`].
///
/// Faster than the cards: a drag that lags behind the pointer feels broken,
/// while a card that snaps feels cheap. Same mechanism, different answer.
const CAMERA_SETTLE: f32 = 24.0;

/// A drawn card, and the group it stands for.
#[derive(Component)]
pub struct CardVisual {
    /// The object the card represents and that input reports.
    pub object: ObjectId,
    /// How many permanents it stands for.
    pub count: usize,
}

/// Where a card is going.
///
/// The scene diff writes the *target* and never the transform itself, so
/// every source of movement — a lane repacking, a tap, a hover, a card
/// entering play — arrives through one door and animates for free. It also
/// means the animation cannot desynchronise from the board model: there is
/// nothing to keep in step, because the target is recomputed from the model
/// every frame.
#[derive(Component, Clone, Copy)]
pub struct Motion {
    /// The transform the card belongs at right now.
    pub target: Transform,
}

/// Moves every card towards its mark.
///
/// Frame-rate independent: the fraction covered is `1 - e^(-rate · dt)`, so
/// the same motion plays out identically at 30 and at 144 frames per second.
/// A naive `lerp(0.2)` per frame does not — it makes the whole table twice as
/// fast on a better machine, which is the bug this shape exists to avoid.
pub fn glide(
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut cards: Query<(&Motion, &mut Transform)>,
) {
    let still = prefs.all().reduce_motion;
    let t = 1.0 - (-SETTLE * time.delta_secs()).exp();
    for (motion, mut transform) in &mut cards {
        let there = transform
            .translation
            .distance_squared(motion.target.translation)
            < SETTLED * SETTLED
            && transform.rotation.angle_between(motion.target.rotation) < SETTLED
            && transform.scale.distance_squared(motion.target.scale) < SETTLED * SETTLED;
        if still || there {
            if *transform != motion.target {
                *transform = motion.target;
            }
            continue;
        }
        transform.translation = transform.translation.lerp(motion.target.translation, t);
        transform.rotation = transform.rotation.slerp(motion.target.rotation, t);
        transform.scale = transform.scale.lerp(motion.target.scale, t);
    }
}

/// Where a card is when it first appears: above its mark, and a little small.
fn entrance(target: &Transform) -> Transform {
    let mut start = *target;
    start.translation.y += ENTRANCE_RISE;
    start.scale *= ENTRANCE_SCALE;
    start
}

/// Entities currently drawn, keyed by the object they represent.
#[derive(Resource, Default)]
pub struct SceneIndex {
    cards: HashMap<ObjectId, Entity>,
    /// One material per *look*, shared by every card wearing it — a board of
    /// forty plain Islands is one material, not forty. A foil Island is a
    /// second, and an Island the rules have made indestructible is a third
    /// until it stops being one: those are the differences the shader draws,
    /// so they are exactly the differences the key carries.
    materials: HashMap<CardLook, Handle<CardMaterial>>,
    quad: Option<Handle<Mesh>>,
    blank: Option<Handle<CardMaterial>>,
    /// Text entities of the constructed face, per card currently showing one,
    /// with the snapshot they were built from.
    ///
    /// Held here rather than found by query because the face comes and goes
    /// with a held key: the entities have to be removed as cheaply as they
    /// were made, and a card that no longer wants one must not keep a stale
    /// line of text glued to it. The sequence number is what rebuilds a face
    /// whose card changed (an anthem, a counter, a clone) without rebuilding
    /// every face every frame.
    faces: HashMap<ObjectId, (u64, Vec<Entity>)>,
    /// One material per colour identity and look, for cards drawing their
    /// own face rather than artwork.
    face_materials: HashMap<CardLook, Handle<CardMaterial>>,
    /// A seat's zone: the mat and the glow under it, with the mood they were
    /// last drawn in. Held here for the same reason the cards are — so a
    /// frame in which nothing changed costs a lookup and no allocation.
    zones: HashMap<PlayerId, Zone>,
    /// The two generated images every zone shares: the rounded mat with its
    /// lane bands, and the soft glow. One each for the whole table; the seat
    /// colour is the material's tint, not a second texture.
    mat_image: Option<Handle<Image>>,
    glow_image: Option<Handle<Image>>,
    /// The contact shadow every card sits in: one quad, one material, shared
    /// by the whole table. It is a child of the card, so it follows the tap
    /// rotation and the hover lift with nothing to keep in step.
    shadow_quad: Option<Handle<Mesh>>,
    shadow_material: Option<Handle<StandardMaterial>>,
}

/// One seat's zone on the table.
struct Zone {
    /// The mat itself.
    mat: Entity,
    /// The pool of colour under it.
    glow: Entity,
    /// What the mat was last tinted for.
    mood: Mood,
}

/// What a zone's colour is saying.
///
/// The rim of a seat's mat is the cheapest place to answer "whose turn is
/// it?" and "who is holding everyone up?" — questions a player asks on every
/// single priority pass, and that otherwise cost a trip to the overlay.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Mood {
    /// Whether this is the viewing seat.
    local: bool,
    /// Where the seat stands in the turn.
    standing: Standing,
}

/// What a seat is doing, in the order the zone cares about it.
///
/// Ordered rather than flagged because these do not stack: a seat holding
/// priority is *also* the active seat nine times out of ten, and drawing both
/// would only mean adding two brightnesses together and hoping.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Standing {
    /// Out of the game.
    Lost,
    /// Holding priority — this is the seat everyone else is waiting for.
    Priority,
    /// Their turn, but not currently holding anyone up.
    Active,
    /// Waiting their turn.
    Waiting,
}

impl Mood {
    /// How a seat's pod reads right now.
    fn of(pod: &baylee_client_core::board::SeatPod) -> Self {
        Self {
            local: pod.is_local,
            standing: if pod.has_lost {
                Standing::Lost
            } else if pod.has_priority {
                Standing::Priority
            } else if pod.is_active {
                Standing::Active
            } else {
                Standing::Waiting
            },
        }
    }
}

/// A card's corner radius.
///
/// A real card is 63 mm wide with a 3 mm corner, and this is exactly that —
/// 4.76%, the same `PRINTED_CORNER` the two card shaders cut at. The geometry
/// takes the scanner's white corner away and the shader inks the sliver of
/// pixels the mesh edge antialiases through, which only works while both
/// agree: a mesh cut wider than the print leaves the ink nothing to do, and a
/// mesh cut narrower shows white outside it. It used to be 10%, which removed
/// the white by removing a tenth of the card with it and made every permanent
/// read as a token.
pub const CARD_CORNER: f32 = CARD_WIDTH * 0.0476;

/// How thick a card is, in table units.
///
/// A real card at this scale is about a fiftieth of this — it would be a
/// single pixel at any camera distance a player uses. The point of the
/// thickness is not accuracy but that a card reads as an *object lying on*
/// the table rather than a decal printed into it, so it is exaggerated until
/// the edge is visible and stopped well before a card looks like a tile.
pub const CARD_THICKNESS: f32 = CARD_WIDTH * 0.055;

/// How far past the card its contact shadow spreads, as a fraction of the
/// card's width.
const SHADOW_SPREAD: f32 = 0.22;

/// A card: a rounded rectangle with the printed face on top and a thin wall
/// around its edge.
///
/// The face is UV-mapped exactly like Bevy's `Rectangle` (uv.x left→right,
/// uv.y top→bottom of the printed face) and the wall borrows the UV of the
/// face vertex above it — so a card's edge is whatever colour its border is,
/// which for most cards is the black frame and reads as exactly the right
/// thing. There is no bottom face: the camera rig never goes below the table,
/// and two hundred cards is four hundred triangles worth saving.
fn rounded_card_mesh(width: f32, height: f32, radius: f32) -> Mesh {
    const SEGMENTS: usize = 4; // per corner — plenty at card scale
    let (hw, hh, r) = (width / 2.0, height / 2.0, radius);
    let top = CARD_THICKNESS;
    // Corner arc centres in CCW order with the quarter turn each one sweeps,
    // angles measured the usual way (0° = +x, 90° = +y).
    //
    // Every centre owns the quarter that points *away* from the middle of the
    // card, and it has to: pair a centre with any other quarter and the
    // outline folds back through the centre, so the fan below stitches
    // crossing slivers instead of a card. On a table that reads as a small
    // bright X where a permanent should be.
    let corners: [([f32; 2], f32); 4] = [
        ([hw - r, hh - r], 90.0),
        ([-hw + r, hh - r], 180.0),
        ([-hw + r, -hh + r], 270.0),
        ([hw - r, -hh + r], 360.0),
    ];
    // The outline, with the outward direction at each point — which for an
    // arc point is simply the angle it was drawn at, and is what the wall's
    // normals are.
    let mut outline: Vec<([f32; 2], [f32; 2])> = Vec::new();
    for ([cx, cy], end_deg) in corners {
        let start_deg = end_deg - 90.0;
        for i in 0..=SEGMENTS {
            let a = (start_deg + (end_deg - start_deg) * i as f32 / SEGMENTS as f32).to_radians();
            let (dx, dy) = (a.cos(), a.sin());
            outline.push(([cx + r * dx, cy + r * dy], [dx, dy]));
        }
    }

    // Same mapping as Rectangle: [hw,hh]→[1,0], [-hw,-hh]→[0,1].
    //
    // Clamped, because the outline is built as `centre + r·cos θ`, and at
    // θ = 0 that is `hw - r + r`, which in binary is not always `hw`. A UV a
    // ten-millionth outside the texture samples the wrap or the clamp
    // depending on the backend, so the card would grow a bright thread down
    // one edge on exactly one machine.
    let uv_of = |x: f32, y: f32| {
        [
            f32::midpoint(x / hw, 1.0).clamp(0.0, 1.0),
            ((1.0 - y / hh) * 0.5).clamp(0.0, 1.0),
        ]
    };

    // The face: a centre vertex and the outline, all facing straight up.
    let mut positions: Vec<[f32; 3]> = vec![[0.0, 0.0, top]];
    let mut normals: Vec<[f32; 3]> = vec![[0.0, 0.0, 1.0]];
    let mut uvs: Vec<[f32; 2]> = vec![[0.5, 0.5]];
    for ([x, y], _) in &outline {
        positions.push([*x, *y, top]);
        normals.push([0.0, 0.0, 1.0]);
        uvs.push(uv_of(*x, *y));
    }
    // A triangle fan from the centre, wound counter-clockwise as seen from
    // +z — the side the printed face is on, and the side the camera is on
    // once the card is laid down. The material does not disable back-face
    // culling, so the other winding is an invisible card.
    let m = outline.len();
    let mut indices = Vec::with_capacity(m * 9);
    for i in 1..m {
        indices.extend_from_slice(&[0, i as u32, i as u32 + 1]);
    }
    indices.extend_from_slice(&[0, m as u32, 1]);

    // The wall: the outline again at both heights, with its own normals, so
    // the face above it keeps a clean flat shade.
    let wall = positions.len() as u32;
    for ([x, y], [nx, ny]) in &outline {
        positions.push([*x, *y, top]);
        normals.push([*nx, *ny, 0.0]);
        uvs.push(uv_of(*x, *y));
    }
    for ([x, y], [nx, ny]) in &outline {
        positions.push([*x, *y, 0.0]);
        normals.push([*nx, *ny, 0.0]);
        uvs.push(uv_of(*x, *y));
    }
    for i in 0..m {
        let next = (i + 1) % m;
        let (t0, t1) = (wall + i as u32, wall + next as u32);
        let (b0, b1) = (t0 + m as u32, t1 + m as u32);
        // Down the near edge, along the bottom, back up: that is the order
        // whose normal points away from the card. The other one is a card you
        // can see straight through from the side.
        indices.extend_from_slice(&[t0, b0, b1, t0, b1, t1]);
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_indices(Indices::U32(indices))
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
}

/// Builds the stage: camera, light, and felt.
pub fn spawn_stage(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cards: ResMut<Assets<CardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut index: ResMut<SceneIndex>,
) {
    index.quad = Some(meshes.add(rounded_card_mesh(CARD_WIDTH, CARD_HEIGHT, CARD_CORNER)));

    // The contact shadow: a quad a little larger than a card, carrying a
    // painted halo that is dense under the card and gone by its own edge.
    // Sized from the card so the falloff is the same width on all four sides
    // — a square texture stretched over a card-shaped quad would be wider at
    // the top than at the sides, which is the sort of thing nobody can name
    // but everybody sees.
    let spread = CARD_WIDTH * SHADOW_SPREAD;
    let (shadow_w, shadow_h) = (CARD_WIDTH + 2.0 * spread, CARD_HEIGHT + 2.0 * spread);
    index.shadow_quad = Some(meshes.add(Rectangle::new(shadow_w, shadow_h)));
    #[expect(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "a texture size derived from two positive constants"
    )]
    let shadow_px = (128.0 * shadow_h / shadow_w).round() as u32;
    index.shadow_material = Some(materials.add(StandardMaterial {
        base_color_texture: Some(images.add(image_of(&tabletop::card_shadow(
            128,
            shadow_px,
            spread / shadow_w,
            CARD_CORNER / CARD_WIDTH,
        )))),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    }));
    // The card back: no art, no finish, no glow. It is what the stack behind
    // a counted group is made of, and what a card whose art never arrives
    // falls back to.
    index.blank = Some(cards.add(material(
        CardLook::flat(BACK_COLOR, FinishTreatment::Plain, 0),
        None,
        BACK_COLOR,
    )));

    commands.spawn((
        DuelStage,
        TableCamera,
        Camera3d::default(),
        // Looking down the table from behind the local seat (the default
        // rig; apply_camera_rig owns the transform from here on).
        Transform::from_xyz(0.0, 15.0, 13.2).looking_at(Vec3::ZERO, Vec3::Y),
        Projection::Perspective(PerspectiveProjection {
            fov: FOV,
            ..default()
        }),
        // No tone mapping. Bevy attaches none to a camera by default, so
        // this is belt and braces rather than a fix — but it is the right
        // thing to say out loud: everything in this scene is unlit and
        // display-referred (a generated texture says what the table should
        // *look* like, and a card's art is the same PNG the hand draws
        // unaltered through the UI pass), so a tone mapper reading those
        // numbers as radiance would be wrong. Naming it here stops a future
        // default from quietly doing that.
        Tonemapping::None,
    ));

    // Nothing below is lit, and nothing above it is either: card art must
    // never be tinted by scene lighting, because a player has to be able to
    // read a card's colour identity at a glance. The table gets its depth
    // from painted-in shading instead — which is what `tabletop` generates,
    // and why this stage has no light in it at all.
    index.mat_image = Some(images.add(image_of(&tabletop::seat_mat(512, 256, 0.06, 0.018))));
    index.glow_image = Some(images.add(image_of(&tabletop::glow(128))));

    // The felt.
    commands.spawn((
        DuelStage,
        Mesh3d(meshes.add(Rectangle::new(FELT.x, FELT.y))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(images.add(image_of(&tabletop::felt(1024)))),
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(0.0, TABLE_Y, 0.0)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    // The pool of lamplight over the middle of the table, with the arcane
    // ring inlaid in it. It sits under the seat mats — the mats draw over it
    // where they overlap, so it reads as something the table has and they
    // are lying on rather than as a decal floating above everything.
    commands.spawn((
        DuelStage,
        Mesh3d(meshes.add(Rectangle::new(HEARTH_SIZE, HEARTH_SIZE))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(images.add(image_of(&tabletop::hearth(
                1024,
                tabletop::HEARTH_INNER,
                tabletop::HEARTH_OUTER,
            )))),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(0.0, TABLE_Y + HEARTH_LIFT, 0.0)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    // The phase wash: the same pool of light, coloured by where in the turn
    // we are. It is laid *over* the hearth rather than multiplied into it,
    // because multiplying candlelight by a cold colour gives grey — the
    // usual way a tint like this fails. It starts at nothing and eases to
    // whatever the first view says the step is.
    commands.spawn((
        DuelStage,
        PhaseWash {
            shown: LinearRgba::NONE,
        },
        Mesh3d(meshes.add(Rectangle::new(WASH_SIZE, WASH_SIZE))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::LinearRgba(LinearRgba::NONE),
            base_color_texture: Some(images.add(image_of(&tabletop::glow(256)))),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(0.0, TABLE_Y + WASH_LIFT, 0.0)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    // The medallion inlaid at the centre — the colour wheel every player
    // already has in their head, which is what makes it orientation rather
    // than decoration. It sits in the middle of the table, which is the one
    // patch of felt no seat ever plays on.
    commands.spawn((
        DuelStage,
        Mesh3d(meshes.add(Rectangle::new(MEDALLION_SIZE, MEDALLION_SIZE))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(images.add(image_of(&tabletop::medallion(512)))),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(0.0, TABLE_Y + MEDALLION_LIFT, 0.0)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
}

/// Wraps a generated texture in an `Image` the renderer can bind.
fn image_of(texture: &tabletop::Texture) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: texture.width,
            height: texture.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        texture.rgba.clone(),
        // sRGB: the generator writes what the table should *look* like, not
        // light values, so the samples are display-referred.
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    // The mat and the medallion are stretched over quads much larger than
    // they are; without a linear filter their soft edges come out as stairs.
    image.sampler = bevy::image::ImageSampler::linear();
    image
}

/// The colour a seat's zone is drawn in.
///
/// The viewing seat is gilt, matching the medallion's rings: whatever else is
/// on the table, "mine" is the one edge a player never has to look for. The
/// others take the colours of the pie in ring order, which makes a four-way
/// game four distinguishable places rather than three anonymous opponents.
fn seat_accent(slot: &SeatSlot) -> Color {
    if slot.is_local {
        return Color::srgb(0.78, 0.63, 0.33);
    }
    let hue = tabletop::PIE[(slot.ring_index + 3) % tabletop::PIE.len()];
    Color::srgb(hue[0], hue[1], hue[2])
}

/// How bright a zone's mat is drawn, given what it is saying.
///
/// A seat that has lost fades most of the way out — its permanents are gone
/// and its zone should stop competing for attention — and a seat holding
/// priority is the brightest thing on the felt, because that is the seat
/// everyone else is waiting for.
fn zone_brightness(mood: Mood) -> f32 {
    let base: f32 = if mood.local { 0.95 } else { 0.72 };
    match mood.standing {
        Standing::Lost => 0.22,
        Standing::Priority => (base * 1.38).min(1.6),
        Standing::Active => base * 1.15,
        Standing::Waiting => base,
    }
}

/// Eases the middle of the table towards the colour of the current step.
///
/// The step is on the board model, so this needs no engine and no view of its
/// own; the arithmetic — which colour a step is worth — lives in
/// [`tabletop::phase_light`], where it can be argued with in a test rather
/// than looked at in a screenshot.
///
/// It writes only when the colour actually moves. A wash that reached its
/// target and kept writing would touch a material every frame for the rest of
/// the game, which is exactly the garbage [`sync_zones`] exists to avoid.
pub fn sync_phase(
    time: Res<Time>,
    duel: Res<Duel>,
    prefs: Res<crate::prefs::Prefs>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut washes: Query<(&mut PhaseWash, &MeshMaterial3d<StandardMaterial>)>,
) {
    let Some(board) = duel.board.as_ref() else {
        return;
    };
    let light = tabletop::phase_light(board.step);
    // sRGB in, because the generator and the palette both write what the
    // table should *look* like; the alpha is where the strength lives, so
    // the wash fades out rather than to black.
    let want = Color::srgba(
        light.rgb[0],
        light.rgb[1],
        light.rgb[2],
        light.energy * WASH_ALPHA,
    )
    .to_linear();
    let t = if prefs.all().reduce_motion {
        1.0
    } else {
        1.0 - (-WASH_RATE * time.delta_secs()).exp()
    };
    for (mut wash, handle) in &mut washes {
        let next = LinearRgba {
            red: wash.shown.red + (want.red - wash.shown.red) * t,
            green: wash.shown.green + (want.green - wash.shown.green) * t,
            blue: wash.shown.blue + (want.blue - wash.shown.blue) * t,
            alpha: wash.shown.alpha + (want.alpha - wash.shown.alpha) * t,
        };
        // Below a step this small nothing on screen changes, so stop —
        // otherwise the wash writes a material every frame forever.
        let moved = [
            next.red - wash.shown.red,
            next.green - wash.shown.green,
            next.blue - wash.shown.blue,
            next.alpha - wash.shown.alpha,
        ]
        .iter()
        .any(|d| d.abs() > 1e-4);
        if !moved {
            continue;
        }
        wash.shown = next;
        if let Some(mut material) = materials.get_mut(&handle.0) {
            material.base_color = Color::LinearRgba(next);
        }
    }
}

/// Keeps one mat and one glow per seat in step with the table.
///
/// Zones are spawned when a seat first appears and only ever re-tinted after
/// that: the layout does not move once a game has begun, and a mat rebuilt
/// every frame would be four meshes and four materials of pure garbage per
/// frame for a table nobody is looking at that hard.
pub fn sync_zones(
    mut commands: Commands,
    duel: Res<Duel>,
    mut index: ResMut<SceneIndex>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mats: Query<&MeshMaterial3d<StandardMaterial>>,
) {
    let (Some(board), Some(layout)) = (duel.board.as_ref(), duel.layout.as_ref()) else {
        return;
    };
    let (Some(mat_image), Some(glow_image)) = (index.mat_image.clone(), index.glow_image.clone())
    else {
        return;
    };

    let mut seen: HashSet<PlayerId> = HashSet::new();
    for pod in &board.pods {
        let Some(slot) = layout.slot(pod.player) else {
            continue;
        };
        seen.insert(pod.player);
        let mood = Mood::of(pod);
        let accent = seat_accent(slot);
        let tint = accent.to_linear() * zone_brightness(mood);

        if let Some(zone) = index.zones.get(&pod.player) {
            if zone.mood == mood {
                continue;
            }
            // Only the colour changes, so only the colour is written: the
            // mesh, the transform and the texture all still hold.
            for entity in [zone.mat, zone.glow] {
                if let Ok(handle) = mats.get(entity)
                    && let Some(mut material) = materials.get_mut(&handle.0)
                {
                    let dim = if entity == zone.glow { 0.30 } else { 1.0 };
                    material.base_color = Color::LinearRgba(tint * dim);
                }
            }
            index
                .zones
                .entry(pod.player)
                .and_modify(|zone| zone.mood = mood);
            continue;
        }

        let size = slot.half_extent * 2.0 + Vec2::splat(ZONE_MARGIN * 2.0);
        let flat = Quat::from_rotation_y(-slot.facing)
            * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
        let mat = commands
            .spawn((
                DuelStage,
                Mesh3d(meshes.add(Rectangle::new(size.x, size.y))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::LinearRgba(tint),
                    base_color_texture: Some(mat_image.clone()),
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    ..default()
                })),
                Transform {
                    translation: to_world(slot.center, TABLE_Y + ZONE_LIFT),
                    rotation: flat,
                    scale: Vec3::ONE,
                },
            ))
            .id();
        let glow = commands
            .spawn((
                DuelStage,
                Mesh3d(meshes.add(Rectangle::new(
                    size.x + GLOW_SPREAD * 2.0,
                    size.y + GLOW_SPREAD * 2.0,
                ))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::LinearRgba(tint * 0.30),
                    base_color_texture: Some(glow_image.clone()),
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    ..default()
                })),
                Transform {
                    translation: to_world(slot.center, TABLE_Y + GLOW_LIFT),
                    rotation: flat,
                    scale: Vec3::ONE,
                },
            ))
            .id();
        index.zones.insert(pod.player, Zone { mat, glow, mood });
    }

    // A seat that left the table takes its zone with it.
    index.zones.retain(|player, zone| {
        if seen.contains(player) {
            return true;
        }
        for entity in [zone.mat, zone.glow] {
            commands.entity(entity).despawn();
        }
        false
    });
}

/// Tears the stage down.
pub fn despawn_stage(
    mut commands: Commands,
    stage: Query<Entity, With<DuelStage>>,
    cards: Query<Entity, With<CardVisual>>,
    mut index: ResMut<SceneIndex>,
) {
    for entity in stage.iter().chain(cards.iter()) {
        commands.entity(entity).despawn();
    }
    index.cards.clear();
    index.faces.clear();
    // The zones were spawned with `DuelStage`, so they have just gone with
    // it; what is left is the bookkeeping that would otherwise point at
    // entities that no longer exist.
    index.zones.clear();
}

/// Places table-space coordinates into the world.
///
/// Table space has `+y` running away from the local seat; the world has the
/// camera on `+z`, so the two are mirrored on that axis.
fn to_world(table: Vec2, height: f32) -> Vec3 {
    Vec3::new(table.x, height, -table.y)
}

/// The transform of one card.
fn card_transform(slot: &SeatSlot, position: Vec2, tapped: bool, lift: f32) -> Transform {
    // Lay the quad flat, then turn it so it faces its owner, then tap it.
    let mut rotation =
        Quat::from_rotation_y(-slot.facing) * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    if tapped {
        rotation = Quat::from_rotation_y(-slot.facing)
            * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
            * Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
    }
    Transform {
        translation: to_world(position, TABLE_Y + CARD_LIFT + lift),
        rotation,
        scale: Vec3::ONE,
    }
}

/// Where every group in the current board model belongs.
struct Placement {
    object: ObjectId,
    slot: SeatSlot,
    position: Vec2,
    tapped: bool,
    count: usize,
    art: Option<ImageKey>,
    offer: crate::cardmat::Offer,
    corner: baylee_client_core::cardplate::Corner,
}

/// Computes placements for the whole table.
///
/// Pure geometry over the board model, so the ordering is the model's ordering
/// and therefore stable frame to frame — which is what makes the diff below
/// cheap and stops cards from swapping places when nothing happened.
fn placements(duel: &Duel) -> Vec<Placement> {
    let (Some(board), Some(layout)) = (duel.board.as_ref(), duel.layout.as_ref()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for pod in &board.pods {
        let Some(slot) = layout.slot(pod.player) else {
            continue;
        };
        for lane in &pod.lanes {
            let center = slot.lane_center(lane.kind);
            let packing = pack_lane(lane.groups.len(), slot.lane_width());
            for (group, offset) in lane.groups.iter().zip(packing.offsets.iter()) {
                let along = Vec2::new(slot.facing.cos(), -slot.facing.sin());
                out.push(Placement {
                    object: group.representative,
                    slot: *slot,
                    position: center + along * *offset,
                    tapped: group.status.is_tapped(),
                    count: group.count(),
                    art: group.art,
                    // Resolved here rather than in the sync loop, because
                    // here is where the group's *members* are: a plan taps
                    // one particular Forest, and the card drawn for it may
                    // be standing for four.
                    offer: crate::cardmat::Offer::on(
                        duel.armed.as_ref(),
                        &group.members,
                        group.activatable,
                    ),
                    // Power, toughness, marked damage and the counters — the
                    // rules facts printed on every real card and drawn nowhere
                    // in this client on a card showing art. Resolved here for
                    // the same reason the offer is: the group is here.
                    corner: baylee_client_core::cardplate::Corner::of(group),
                });
            }
        }
    }
    out
}

/// Brings the scene in line with the board model.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)] // the diff loop is one coherent pass
pub fn sync_scene(
    mut commands: Commands,
    duel: Res<Duel>,
    mut index: ResMut<SceneIndex>,
    mut textures: Option<ResMut<CardTextures>>,
    mut card_materials: ResMut<Assets<CardMaterial>>,
    assets: Res<AssetServer>,
    texts: Res<crate::cardtext::CardTexts>,
    mode: Res<crate::face::FaceMode>,
    settings: Res<crate::settings::ClientSettings>,
    fonts: Option<Res<crate::hud::UiFonts>>,
    mut cards: Query<(
        &mut Motion,
        &mut CardVisual,
        &mut MeshMaterial3d<CardMaterial>,
    )>,
) {
    let (Some(statics), Some(textures)) = (duel.statics.as_ref(), textures.as_mut()) else {
        return;
    };
    let Some(quad) = index.quad.clone() else {
        return;
    };
    let blank = index.blank.clone();
    let shadow = index.shadow_quad.clone().zip(index.shadow_material.clone());

    let wanted = placements(&duel);
    let mut live: HashSet<ObjectId> = HashSet::new();

    // Selection state, read once: the keyboard/mouse cursor and the cards
    // already chosen for the pending choice.
    let hovered = duel.hovered;
    let selected: HashSet<ObjectId> = duel
        .interaction
        .as_ref()
        .map(|i| i.selected().iter().copied().collect())
        .unwrap_or_default();

    // The snapshot the faces below were built from: rules text is projected,
    // so a face is only stale when the game state that produced it moved on.
    let seq = duel.view.as_ref().map_or(0, |v| v.seq);

    for placement in &wanted {
        live.insert(placement.object);

        // A card either wears its art or its own text, never both — text on
        // top of artwork is unreadable at any zoom.
        let show_face = face::wants_face(&mode, &settings, textures, placement.art);
        let object = duel
            .view
            .as_ref()
            .and_then(|view| view.object(placement.object));

        // What the card is physically, and what the rules have made it. Both
        // ride on the material, so a foil that gains indestructible becomes a
        // different material and needs no second pass.
        //
        // The finish is a property of the printing, so it comes from the
        // print table — which is per seat, and a printing this seat has not
        // earned reads as plain rather than as a leak.
        let finish = crate::cardmat::finish_of(statics, placement.art);
        // Keywords are what the card is, sickness is what it cannot do this
        // turn, and the offer is what the player could do with it — or has
        // just said they will. All of it rides on the material, so a Forest
        // that becomes tappable becomes a different material and needs no
        // second pass — and stops being one the moment priority moves on.
        let glow = crate::cardmat::glow_of(object, placement.offer);

        let material = if show_face {
            // One material per colour identity, so a mono-green board is one
            // material however many creatures are on it.
            let colors = object.map_or(ColorSet::EMPTY, |o| o.colors);
            let tint = face::table_color(colors);
            let look = CardLook::flat(tint, finish, glow).with_corner(placement.corner);
            if let Some(handle) = index.face_materials.get(&look) {
                handle.clone()
            } else {
                let handle = card_materials.add(material(look, None, tint));
                index.face_materials.insert(look, handle.clone());
                handle
            }
        } else {
            // One material per look, created on first use.
            match placement.art {
                Some(key) => {
                    let look = CardLook::art(key, finish, glow).with_corner(placement.corner);
                    if let Some(handle) = index.materials.get(&look) {
                        handle.clone()
                    } else {
                        let image = textures.get(key, statics, &assets);
                        let handle = card_materials.add(material(look, Some(image), BACK_COLOR));
                        index.materials.insert(look, handle.clone());
                        handle
                    }
                }
                None => blank.clone().unwrap_or_default(),
            }
        };

        let mut transform =
            card_transform(&placement.slot, placement.position, placement.tapped, 0.0);
        // Hover (cursor) lifts the card a touch; a chosen card stays raised
        // until the choice is answered, and so does an armed one — a deed
        // waiting on a second tap is a commitment the player has already
        // made, which is the same claim being selected makes and belongs at
        // the same height. Selected wins over both; the pointer moving away
        // must not put an armed card back down.
        if selected.contains(&placement.object) || placement.offer.armed {
            transform.translation.y += SELECTED_LIFT;
            transform.scale *= SELECTED_SCALE;
        } else if hovered == Some(placement.object) {
            transform.translation.y += HOVER_LIFT;
            transform.scale *= HOVER_SCALE;
        }

        let entity = if let Some(&entity) = index.cards.get(&placement.object) {
            // Existing card: update in place. Touching only what changed is
            // what keeps a large board cheap.
            if let Ok((mut motion, mut visual, mut current_material)) = cards.get_mut(entity) {
                if motion.target != transform {
                    motion.target = transform;
                }
                if visual.count != placement.count {
                    visual.count = placement.count;
                }
                if current_material.0 != material {
                    current_material.0 = material;
                }
            }
            entity
        } else {
            let entity = commands
                .spawn((
                    DuelStage,
                    CardVisual {
                        object: placement.object,
                        count: placement.count,
                    },
                    Mesh3d(quad.clone()),
                    MeshMaterial3d(material),
                    // Appears above its mark and drops onto it; `glide` does
                    // the rest, and a player who has turned motion off gets
                    // the target on the very first frame.
                    entrance(&transform),
                    Motion { target: transform },
                ))
                .id();
            index.cards.insert(placement.object, entity);

            // The contact shadow rides along as a child, which is what keeps
            // it under a tapped card without anything having to rotate it,
            // and what makes it grow out from under a card as that card is
            // lifted. It sits between the felt and the card, and is not
            // pickable — a click near a card's edge means the table.
            if let Some((mesh, material)) = shadow.clone() {
                commands.entity(entity).with_child((
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::from_xyz(0.0, 0.0, -CARD_LIFT * 0.5),
                    Pickable::IGNORE,
                ));
            }

            // A counted group gets a few offset cards behind it so the stack
            // reads as physical depth rather than as a number floating on one
            // card.
            let depth = placement.count.saturating_sub(1).min(MAX_STACK_DEPTH);
            for i in 1..=depth {
                let back = card_transform(
                    &placement.slot,
                    placement.position + Vec2::splat(0.02 * i as f32),
                    placement.tapped,
                    STACK_LIFT * i as f32,
                );
                commands.spawn((
                    DuelStage,
                    Mesh3d(quad.clone()),
                    MeshMaterial3d(blank.clone().unwrap_or_default()),
                    back,
                ));
            }
            entity
        };

        // The text children follow the same decision as the material, and are
        // rebuilt when the snapshot they were made from is no longer current:
        // an anthem, a counter or a clone all change what the face should say.
        let current = index.faces.get(&placement.object).map(|(seq, _)| *seq);
        if show_face && current == Some(seq) {
            continue;
        }
        if let Some((_, previous)) = index.faces.remove(&placement.object) {
            for text in previous {
                commands.entity(text).despawn();
            }
        }
        if !show_face {
            continue;
        }
        let Some((object, fonts)) = object.zip(fonts.as_deref()) else {
            continue;
        };
        let built = face::of_object(object, &texts);
        let spawned = face::spawn_world(&mut commands, entity, &built, fonts);
        index.faces.insert(placement.object, (seq, spawned));
    }

    // Anything no longer on the board leaves the scene.

    let stale: Vec<ObjectId> = index
        .cards
        .keys()
        .copied()
        .filter(|id| !live.contains(id))
        .collect();
    for id in stale {
        if let Some(entity) = index.cards.remove(&id) {
            // Despawning a card takes its text children with it, so the map
            // only has to forget them.
            index.faces.remove(&id);
            commands.entity(entity).despawn();
        }
    }

    // Tell the cache what is on screen so it can evict the rest.
    let visible: Vec<ImageKey> = wanted.iter().filter_map(|p| p.art).collect();
    textures.retain_visible(&visible);
}

/// How a group should be labelled in the overlay, if at all.
#[must_use]
pub fn stack_badge(group: &CardGroup) -> Option<String> {
    group.is_stack().then(|| format!("×{}", group.count()))
}

#[cfg(test)]
mod camera_tests {
    use super::*;
    use baylee_core::ids::PlayerId;

    /// A laptop's window, in logical pixels.
    const WINDOW: Vec2 = Vec2::new(1728.0, 1052.0);

    fn seats(n: u8) -> Vec<PlayerId> {
        (0..n).map(PlayerId::new).collect()
    }

    /// Where a point on the felt lands, in normalised device coordinates.
    ///
    /// Written out forwards on purpose: [`CameraRig::home`] inverts the same
    /// projection, and a test that reused the inverse would agree with it
    /// however wrong both were.
    fn project(rig: CameraRig, canvas: Canvas, table: Vec2) -> Vec2 {
        let lean = CAMERA_LEAN;
        let eye = rig.distance * (1.0 + lean * lean).sqrt();
        let cos = 1.0 / (1.0 + lean * lean).sqrt();
        let t = (FOV * 0.5).tan();
        let aspect = canvas.window.x / canvas.window.y;
        // The rig stores world x/z; `+y` away from the local seat is `-z`.
        let s = table.y - -rig.target.y;
        let depth = lean.mul_add(cos * s, eye);
        Vec2::new(
            (table.x - rig.target.x) / (depth * t * aspect),
            cos * s / (depth * t),
        )
    }

    /// Every corner of every seat's mat, in table space.
    fn corners(layout: &TableLayout) -> Vec<Vec2> {
        let mut out = Vec::new();
        for slot in &layout.slots {
            let (sin, cos) = slot.facing.sin_cos();
            for sx in [-1.0_f32, 1.0] {
                for sy in [-1.0_f32, 1.0] {
                    let local = slot.half_extent * Vec2::new(sx, sy);
                    out.push(
                        slot.center
                            + Vec2::new(
                                cos.mul_add(local.x, sin * local.y),
                                (-sin).mul_add(local.x, cos * local.y),
                            ),
                    );
                }
            }
        }
        out
    }

    /// The bug this whole framing exists for: the table shipped with a
    /// hard-coded 20-unit camera looking at the middle of the felt, and the
    /// local seat's own mat came out *underneath the hand bar*. A player
    /// could not see their own creatures.
    #[test]
    fn the_local_seats_own_mat_is_not_behind_the_hand_bar() {
        let canvas = Canvas::hud(WINDOW);
        let layout = TableLayout::new(&seats(2), 1.78, None);
        let local = layout.local().copied().expect("a local seat");
        let near = local.center.y - local.half_extent.y;

        let bad = project(CameraRig::default(), canvas, Vec2::new(0.0, near));
        let floor = -1.0 + 2.0 * canvas.bottom / canvas.window.y;
        assert!(
            bad.y < floor,
            "the old framing is supposed to be the broken one: {} vs {floor}",
            bad.y
        );

        let good = project(
            CameraRig::home(&layout, canvas),
            canvas,
            Vec2::new(0.0, near),
        );
        assert!(
            good.y >= floor,
            "the near edge of my own mat is still under the hand bar: {} vs {floor}",
            good.y
        );
    }

    #[test]
    fn every_seats_mat_is_inside_the_part_of_the_window_you_can_see() {
        let canvas = Canvas::hud(WINDOW);
        for n in 2..=8 {
            let layout = TableLayout::new(&seats(n), 1.78, None);
            let rig = CameraRig::home(&layout, canvas);
            let top = 1.0 - 2.0 * canvas.top / canvas.window.y;
            let bottom = -1.0 + 2.0 * canvas.bottom / canvas.window.y;
            let right = 1.0 - 2.0 * canvas.right / canvas.window.x;
            for corner in corners(&layout) {
                let at = project(rig, canvas, corner);
                assert!(
                    at.y >= bottom - 1e-3 && at.y <= top + 1e-3,
                    "{n} seats: {corner} lands at y {} , outside {bottom}..{top}",
                    at.y
                );
                assert!(
                    at.x >= -1.0 - 1e-3 && at.x <= right + 1e-3,
                    "{n} seats: {corner} lands at x {} , outside -1..{right}",
                    at.x
                );
            }
        }
    }

    /// A window is not always a laptop's. The framing has to hold for a phone
    /// held upright, where the HUD covers proportionally far more of it.
    #[test]
    fn the_framing_holds_on_a_tall_narrow_window() {
        let canvas = Canvas::hud(Vec2::new(430.0, 932.0));
        let layout = TableLayout::new(&seats(4), 0.46, None);
        let rig = CameraRig::home(&layout, canvas);
        let top = 1.0 - 2.0 * canvas.top / canvas.window.y;
        let bottom = -1.0 + 2.0 * canvas.bottom / canvas.window.y;
        // Both bounds, because only checking the near edge is exactly the
        // hole that let the hand bar bug through in the first place: a shot
        // aimed too far off can satisfy one edge by breaking the other.
        for corner in corners(&layout) {
            let at = project(rig, canvas, corner);
            assert!(
                at.y >= bottom - 1e-3 && at.y <= top + 1e-3,
                "{corner} lands at y {}, outside {bottom}..{top}",
                at.y
            );
        }
        // Horizontally a four-seat table does not fit a phone at any honest
        // distance — at the width this needs, the felt's own edge comes into
        // frame — and no camera can fix that. What fixes it is tranche 4
        // turning the vertical rail into a horizontal strip, which changes
        // `Canvas`. Asserted here as the deliberate gap it is.
        assert!(
            corners(&layout)
                .into_iter()
                .any(|corner| project(rig, canvas, corner).x < -1.0),
            "a phone now fits a four-seat table sideways — tighten this test",
        );
    }

    #[test]
    fn a_table_with_nobody_at_it_frames_nothing_rather_than_dividing_by_zero() {
        let rig = CameraRig::home(&TableLayout::new(&[], 1.78, None), Canvas::hud(WINDOW));
        assert_eq!(rig, CameraRig::default());
        assert!(rig.distance.is_finite());
    }

    /// `docs/design.md` §1.1: the ring is atmosphere, the mats are the game.
    /// The bound is measurable, so it is measured.
    #[test]
    fn the_hearth_ring_is_smaller_than_the_nearest_seats_mat() {
        let layout = TableLayout::new(&seats(2), 1.78, None);
        let local = layout.local().copied().expect("a local seat");
        // `HEARTH_OUTER` is a fraction of the quad's *half* width, so the
        // ring a player sees is that fraction of the quad across.
        let ring = HEARTH_SIZE * tabletop::HEARTH_OUTER;
        assert!(
            ring < local.lane_width(),
            "the eye lands on the ring, not on the board: {ring} vs {}",
            local.lane_width()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(facing: f32) -> SeatSlot {
        SeatSlot {
            player: baylee_core::ids::PlayerId::new(0),
            ring_index: 0,
            angle: facing,
            center: Vec2::ZERO,
            facing,
            half_extent: Vec2::new(6.0, 3.0),
            is_local: true,
        }
    }

    /// Every vertex of the card mesh, as (x, y, z).
    fn points(mesh: &Mesh) -> Vec<(f32, f32, f32)> {
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("card mesh has positions")
        };
        p.iter().map(|v| (v[0], v[1], v[2])).collect()
    }

    /// The outline of the printed face, in order, as (x, y) pairs.
    ///
    /// Vertex 0 is the fan's centre and is not part of the outline; the wall
    /// vertices that follow the face are told apart by their normal, which is
    /// the thing that actually distinguishes them.
    fn rim(mesh: &Mesh) -> Vec<(f32, f32)> {
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(n)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("card mesh has normals")
        };
        let face = n.iter().take_while(|v| v[2] > 0.5).count();
        points(mesh)[1..face].iter().map(|v| (v.0, v.1)).collect()
    }

    /// Triangles as index triples.
    fn triangles(mesh: &Mesh) -> Vec<[u32; 3]> {
        let Some(Indices::U32(idx)) = mesh.indices() else {
            panic!("card mesh is indexed")
        };
        idx.as_chunks::<3>().0.to_vec()
    }

    /// Twice the signed area of a triangle, positive when it is wound
    /// counter-clockwise seen from +z.
    fn cross(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
        (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
    }

    // The card that shipped as a bowtie: every corner arc swept the quarter
    // turn belonging to its neighbour, so the outline crossed itself twice
    // through the middle and a permanent on the battlefield was drawn as a
    // small bright X. `an_untapped_card_lies_flat_on_the_table` passed the
    // whole time — the transform was never the problem — so the mesh needs
    // tests of its own.
    #[test]
    fn the_card_outline_never_folds_through_its_own_middle() {
        let mesh = rounded_card_mesh(CARD_WIDTH, CARD_HEIGHT, CARD_CORNER);
        let rim = rim(&mesh);
        // Walking a convex outline turns the same way at every vertex and
        // comes back around exactly once. A bowtie turns back on itself.
        let mut turn = 0.0_f32;
        for i in 0..rim.len() {
            let (a, b, c) = (rim[i], rim[(i + 1) % rim.len()], rim[(i + 2) % rim.len()]);
            assert!(
                cross(a, b, c) >= 0.0,
                "the outline turns back on itself at vertex {i}: {a:?} {b:?} {c:?}"
            );
            let before = (b.1 - a.1).atan2(b.0 - a.0);
            let after = (c.1 - b.1).atan2(c.0 - b.0);
            let mut delta = after - before;
            while delta > std::f32::consts::PI {
                delta -= std::f32::consts::TAU;
            }
            while delta < -std::f32::consts::PI {
                delta += std::f32::consts::TAU;
            }
            turn += delta;
        }
        assert!(
            (turn - std::f32::consts::TAU).abs() < 1e-3,
            "a closed convex outline turns through exactly one full circle, not {turn}"
        );
    }

    #[test]
    fn the_card_mesh_covers_the_card() {
        let mesh = rounded_card_mesh(CARD_WIDTH, CARD_HEIGHT, CARD_CORNER);
        let rim = rim(&mesh);
        let area: f32 = (0..rim.len())
            .map(|i| {
                let (a, b) = (rim[i], rim[(i + 1) % rim.len()]);
                a.0.mul_add(b.1, -(b.0 * a.1))
            })
            .sum::<f32>()
            / 2.0;
        // The rounded rectangle, minus what the four corner arcs cut away.
        // The arcs are drawn in segments, so the mesh is a hair under.
        let ideal =
            CARD_WIDTH * CARD_HEIGHT - (4.0 - std::f32::consts::PI) * CARD_CORNER * CARD_CORNER;
        assert!(
            area > ideal * 0.99 && area <= ideal,
            "a card of {CARD_WIDTH}×{CARD_HEIGHT} covers about {ideal}, not {area}"
        );
        // And it stays inside the card: no vertex may stick out past an edge.
        for (x, y) in rim {
            assert!(
                x.abs() <= CARD_WIDTH / 2.0 + 1e-5 && y.abs() <= CARD_HEIGHT / 2.0 + 1e-5,
                "({x}, {y}) is outside the card"
            );
        }
    }

    #[test]
    fn every_face_triangle_faces_the_printed_side() {
        let mesh = rounded_card_mesh(CARD_WIDTH, CARD_HEIGHT, CARD_CORNER);
        let points = points(&mesh);
        let outline = rim(&mesh).len();
        // Back-face culling is on, so a triangle wound the other way is an
        // invisible sliver of card.
        for tri in triangles(&mesh) {
            if tri.iter().any(|i| *i as usize > outline) {
                continue; // a wall triangle; checked below
            }
            let flat = |i: u32| (points[i as usize].0, points[i as usize].1);
            let (a, b, c) = (flat(tri[0]), flat(tri[1]), flat(tri[2]));
            assert!(
                cross(a, b, c) > 0.0,
                "face triangle {tri:?} faces away from the camera"
            );
        }
    }

    /// A card is a slab, not a decal: it has a wall around its edge so it
    /// reads as lying *on* the table. Wound the other way, that wall is a
    /// card you can see straight through from the side.
    #[test]
    fn the_card_wall_faces_outwards_and_stands_on_the_table() {
        let mesh = rounded_card_mesh(CARD_WIDTH, CARD_HEIGHT, CARD_CORNER);
        let points = points(&mesh);
        let outline = rim(&mesh).len();
        let mut walls = 0;
        for tri in triangles(&mesh) {
            if tri.iter().all(|i| *i as usize <= outline) {
                continue;
            }
            walls += 1;
            let corner = |i: u32| points[i as usize];
            let (first, second, third) = (corner(tri[0]), corner(tri[1]), corner(tri[2]));
            // The triangle's normal, and the direction away from the card's
            // axis at its centroid. A wall faces out when they agree.
            let edge_a = (second.0 - first.0, second.1 - first.1, second.2 - first.2);
            let edge_b = (third.0 - first.0, third.1 - first.1, third.2 - first.2);
            let normal = (
                edge_a.1 * edge_b.2 - edge_a.2 * edge_b.1,
                edge_a.2 * edge_b.0 - edge_a.0 * edge_b.2,
                edge_a.0 * edge_b.1 - edge_a.1 * edge_b.0,
            );
            let out = (
                (first.0 + second.0 + third.0) / 3.0,
                (first.1 + second.1 + third.1) / 3.0,
            );
            assert!(
                normal.0.mul_add(out.0, normal.1 * out.1) > 0.0,
                "wall triangle {tri:?} faces into the card"
            );
            for point in [first, second, third] {
                assert!(
                    point.2 >= -1e-6 && point.2 <= CARD_THICKNESS + 1e-6,
                    "the wall runs past the card's own thickness at {point:?}"
                );
            }
        }
        assert_eq!(
            walls,
            outline * 2,
            "the wall does not close around the card"
        );
    }

    /// The printed face is what a player looks at, and it has to be the
    /// topmost surface — a face level with the wall would z-fight along every
    /// edge of every card on the table.
    #[test]
    fn the_printed_face_sits_on_top_of_the_slab() {
        let mesh = rounded_card_mesh(CARD_WIDTH, CARD_HEIGHT, CARD_CORNER);
        let points = points(&mesh);
        let outline = rim(&mesh).len();
        for point in &points[..=outline] {
            assert!(
                (point.2 - CARD_THICKNESS).abs() < 1e-6,
                "a face vertex is not on top: {point:?}"
            );
        }
        assert!(
            points.iter().any(|p| p.2.abs() < 1e-6),
            "nothing touches the table, so the card floats"
        );
    }

    #[test]
    fn the_card_face_is_mapped_corner_to_corner() {
        let mesh = rounded_card_mesh(CARD_WIDTH, CARD_HEIGHT, CARD_CORNER);
        let Some(bevy::mesh::VertexAttributeValues::Float32x2(uvs)) =
            mesh.attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!("card mesh has uvs")
        };
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("card mesh has positions")
        };
        // The printed face runs left→right and top→bottom, so the top-left
        // of the card is (0,0) in the image and the bottom-right is (1,1).
        for (p, uv) in pos.iter().zip(uvs) {
            let want_u = f32::midpoint(p[0] / (CARD_WIDTH / 2.0), 1.0);
            let want_v = (1.0 - p[1] / (CARD_HEIGHT / 2.0)) * 0.5;
            assert!((uv[0] - want_u).abs() < 1e-5 && (uv[1] - want_v).abs() < 1e-5);
            assert!((0.0..=1.0).contains(&uv[0]) && (0.0..=1.0).contains(&uv[1]));
        }
    }

    /// The geometry and the two shaders have to round the card at the same
    /// radius, and nothing in the compiler can notice that they do: one is a
    /// Rust constant and the others are text in a `.wgsl` file. So the text
    /// is read.
    ///
    /// Cut the mesh wider than the print and the shader's ink has nothing
    /// left to reach; cut it narrower and a white sliver of the scanner bed
    /// survives outside it. Either way the card stops looking like a card,
    /// which is the entire point of cutting the corner at all.
    #[test]
    fn the_mesh_and_the_shaders_round_the_card_alike() {
        let printed = crate::cardmat::tests::wgsl_const(
            include_str!("shaders/card_common.wgsl"),
            "PRINTED_CORNER",
        );
        assert!(
            (CARD_CORNER / CARD_WIDTH - printed).abs() < 1e-6,
            "the mesh rounds at {} of its width, the shaders at {printed}",
            CARD_CORNER / CARD_WIDTH
        );
    }

    #[test]
    fn table_space_maps_away_from_the_seat_into_the_screen() {
        // +y in table space is away from the local seat, which is -z in the
        // world because the camera sits on +z.
        let world = to_world(Vec2::new(2.0, 5.0), 0.0);
        assert!((world.x - 2.0).abs() < 1e-5);
        assert!((world.z + 5.0).abs() < 1e-5);
    }

    #[test]
    fn an_untapped_card_lies_flat_on_the_table() {
        let t = card_transform(&slot(0.0), Vec2::ZERO, false, 0.0);
        // The quad's normal (+z in local space) should point straight up.
        let normal = t.rotation * Vec3::Z;
        assert!((normal.y - 1.0).abs() < 1e-4, "normal was {normal:?}");
    }

    #[test]
    fn tapping_rotates_a_quarter_turn_but_keeps_the_card_on_the_table() {
        let untapped = card_transform(&slot(0.0), Vec2::ZERO, false, 0.0);
        let tapped = card_transform(&slot(0.0), Vec2::ZERO, true, 0.0);
        assert_ne!(untapped.rotation, tapped.rotation);

        // Still flat: only the in-plane orientation changed.
        let normal = tapped.rotation * Vec3::Z;
        assert!((normal.y - 1.0).abs() < 1e-4, "normal was {normal:?}");

        // The card's long axis has swung to the side.
        let up = tapped.rotation * Vec3::Y;
        assert!(
            up.y.abs() < 1e-4,
            "long axis should now lie across the table"
        );
    }

    #[test]
    fn cards_float_above_the_felt_so_they_never_z_fight() {
        let t = card_transform(&slot(0.0), Vec2::ZERO, false, 0.0);
        assert!(t.translation.y > TABLE_Y);
    }

    #[test]
    fn a_seat_across_the_table_has_its_cards_turned_to_face_it() {
        let near = card_transform(&slot(0.0), Vec2::ZERO, false, 0.0);
        let far = card_transform(&slot(std::f32::consts::PI), Vec2::ZERO, false, 0.0);
        // Both flat, but their in-plane orientation is opposite.
        let near_up = near.rotation * Vec3::Y;
        let far_up = far.rotation * Vec3::Y;
        assert!(
            near_up.dot(far_up) < -0.9,
            "far seat should be turned around"
        );
    }

    #[test]
    fn only_counted_groups_get_a_badge() {
        use baylee_client_core::board::KeywordBadge;
        use baylee_view::ObjectStatus;

        let mut group = CardGroup {
            representative: ObjectId::new(1, 0),
            members: vec![ObjectId::new(1, 0)],
            name: "Soldier".into(),
            power: Some(1),
            toughness: Some(1),
            damage: 0,
            loyalty: None,
            status: ObjectStatus::NONE,
            counters: vec![],
            badges: Vec::<KeywordBadge>::new(),
            art: None,
            is_token: true,
            summoning_sick: false,
            activatable: false,
            individual: None,
        };
        assert_eq!(stack_badge(&group), None);
        group.members.push(ObjectId::new(2, 0));
        group.members.push(ObjectId::new(3, 0));
        assert_eq!(stack_badge(&group).as_deref(), Some("×3"));
    }
    /// The finish is a property of the *printing*, and the print table is per
    /// seat: a printing this seat has not earned reads as plain rather than
    /// as a hole in the hidden-information rule. This pins the lookup that
    /// makes that true, because the alternative — reading a finish off the
    /// card — would be the leak.
    #[test]
    fn a_printing_a_seat_has_not_earned_is_drawn_plain() {
        use baylee_client_core::images::{ArtSize, Face};
        use baylee_core::ids::PrintRef;

        let statics = baylee_view::GameStatic {
            view_version: baylee_view::VIEW_VERSION,
            game_id: String::new(),
            your_seat: baylee_core::ids::PlayerId::new(0),
            seats: Vec::new(),
            prints: vec![
                Some(baylee_view::PrintEntry {
                    scryfall_id: "11111111-2222-3333-4444-555555555555".to_string(),
                    lang: "en".to_string(),
                    finish: baylee_view::Finish::Foil,
                }),
                // Earned by nobody: the seat has not seen this card.
                None,
            ],
        };
        let look = |slot: u16| {
            let key = ImageKey {
                print: PrintRef(slot),
                face: Face::Front,
                size: ArtSize::Normal,
            };
            statics
                .print(key.print)
                .map_or(FinishTreatment::Plain, |entry| entry.finish.into())
        };
        assert_eq!(look(0), FinishTreatment::Foil, "its own deck's printing");
        assert_eq!(look(1), FinishTreatment::Plain, "a hole is not a foil");
    }
}

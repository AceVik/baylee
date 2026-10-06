//! The camera: its rig, the canvas it frames, the lens UI is placed through.

#[allow(clippy::wildcard_imports)] // the table's shared vocabulary
use super::*;

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
    /// Distance from the target.
    ///
    /// Written by the framing and by nothing a hand does: there is no zoom
    /// control any more, so this is an output of [`CameraRig::home`] that a
    /// pan carries along rather than a setting.
    pub distance: f32,
    /// Azimuth around the target (0 = behind the local seat).
    pub yaw: f32,
    /// How far the camera stands off vertical, as a tangent.
    ///
    /// Every shot the client takes is [`CAMERA_LEAN`] and no control changes
    /// it: the lean is a measured trade between a card reading as an object
    /// and the far seat's cards shrinking (that constant carries the
    /// numbers), which is a few degrees wide and nothing a hand aims. It
    /// stays a field rather than moving into the transform because the
    /// framing arithmetic reads it off the rig it is solving for. A tangent
    /// and not an angle, because that is what the transform and the shadow
    /// offsets read.
    pub lean: f32,
}

impl Default for CameraRig {
    fn default() -> Self {
        Self {
            target: Vec2::ZERO,
            distance: 20.0,
            yaw: 0.0,
            lean: CAMERA_LEAN,
        }
    }
}

impl CameraRig {
    /// As close as the framing may pull the camera in: one seat's lane,
    /// filling the screen.
    pub const MIN_DISTANCE: f32 = 12.0;
    /// As far as the framing may push it out.
    ///
    /// It was a limit on the *player's* zoom and is now the only limit there
    /// is: the zoom went at the owner's word (*„Das Zoom in/out sollte eh
    /// weg!"*), so the pair bounds [`CameraRig::home`] and nothing else —
    /// which is why it has headroom over the furthest table there is. Since
    /// every seat at a ring is handed a duel's board (#264), that is eight
    /// seats: 186 units on a laptop, 223 on a phone held upright and 273 on
    /// one turned on its side, where the furthest table used to ask for 81.
    /// A limit sitting just above that would not stop the shot: it would
    /// silently crop it, because a fit refused is a fit that no longer fits.
    ///
    /// Both ends are distances through [`FOV`], so both moved when it did:
    /// the same shot through half the angle stands twice as far off, and a
    /// pair left where they were would have clamped every table on the way
    /// in and every large one on the way out.
    pub const MAX_DISTANCE: f32 = 300.0;

    // `MIN_LEAN` (0.176, about 10° off plan) and `MAX_LEAN` (1.428, about
    // 55°) stood here and are gone with the tilt control they bounded. Their
    // reasons were real and are now [`CAMERA_LEAN`]'s to carry, because the
    // lean is one number the client picks rather than a range a hand moves
    // through: a card is a slab with a wall and a contact shadow and reads
    // as a decal from straight overhead, and a seat looking along its own
    // board sees the backs of its front row.

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
            lean: CAMERA_LEAN,
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
    /// the hand zone on every screen — a player could not see their own
    /// creatures, which made every later piece of board legibility moot.
    #[must_use]
    pub fn home(layout: &TableLayout, canvas: Canvas) -> Self {
        let Some((min, max)) = layout.extent() else {
            return Self::default();
        };
        // A wide duel can show the table's depth without foreshortening side
        // seats. Blend in as the window grows; rings and small windows keep
        // the readable plan view and its breathing room.
        let framing = if layout.slots.len() == 2 && canvas.aspect() >= 1.4 {
            ((canvas.window.x - 800.0) / 480.0).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let tilt = CAMERA_LEAN + (DUEL_LEAN - CAMERA_LEAN) * framing;
        let air = AIR - (AIR - DUEL_AIR) * framing;
        let (min, max) = (min - Vec2::splat(air), max + Vec2::splat(air));
        let span = max - min;

        // The free band, as normalised device coordinates: +1 is the top of
        // the window, and the tab strip and the hand zone eat inwards.
        let top = 1.0 - 2.0 * canvas.top / canvas.window.y.max(1.0);
        let bottom = -1.0 + 2.0 * canvas.bottom / canvas.window.y.max(1.0);
        let right = 1.0 - 2.0 * canvas.right / canvas.window.x.max(1.0);
        let aspect = canvas.window.x / canvas.window.y.max(1.0);

        // Vertically this is exact: `ground` is linear in the eye distance,
        // so the distance at which the table's far edge lands on `top` and
        // its near edge on `bottom` is one division.
        let g_top = ground(top, tilt);
        let g_bottom = ground(bottom, tilt);
        let deep = span.y / (g_top - g_bottom).max(1e-3);
        // Horizontally, every corner is asked, and each one asks about its own
        // depth. A perspective camera sees less of the felt where the felt is
        // closer, so a mat at the near edge needs more room than the same mat
        // across the table — and the corners of the *box* are not on the
        // table at all, they are the bare felt a ring leaves in its corners.
        // Fitting the box is what left a three-seat shot filling 86% of the
        // width it was given and 81% of the height, binding on neither.
        //
        // With the look point centred (below), a corner's depth is
        // `eye·(1 + k·ḡ) + k·(y − ȳ)` — linear in `eye`, which is what keeps
        // this arithmetic. Two corners fit together when the near band holds
        // both, so each *pair* gives a division and the widest pair wins.
        let mean = f32::midpoint(g_top, g_bottom);
        let middle = f32::midpoint(min.y, max.y);
        let k = tilt / (1.0 + tilt * tilt).sqrt();
        let scale = (half_fov().tan() * aspect).max(1e-3);
        let carry = k.mul_add(mean, 1.0);
        let corners = layout.corners(air);
        let mut wide: f32 = 0.0;
        for a in &corners {
            for b in &corners {
                // `a` against the right edge of the band and `b` against the
                // left: the room the two of them need between them, less what
                // their own depths already give, over what a unit of eye buys.
                let held = right * k * (a.y - middle) + k * (b.y - middle);
                wide = wide.max(((a.x - b.x) / scale - held) / (carry * (1.0 + right)));
            }
        }
        // Clamped *before* the look point is derived from it. Aiming for a
        // camera the clamp then moves is the one way this can put the table
        // off screen while every number above is still right: the far edge
        // would be pinned for an eye that is not there, and land above the
        // tab strip. Clamped first, a table too big for `MAX_DISTANCE` keeps
        // its far edge pinned and overflows at the bottom, which is the
        // graceful direction.
        let lean = (1.0 + tilt * tilt).sqrt();
        let eye = deep
            .max(wide)
            .clamp(Self::MIN_DISTANCE * lean, Self::MAX_DISTANCE * lean);

        // The table is **centred** in the band, on both axes. Whichever of
        // the two fits binds, the slack the other one has left over is split
        // evenly instead of being pushed to one edge: the far edge used to be
        // pinned under the tab strip and every spare unit opened up in front
        // of the local seat, which on a duel was a fifth of the window of
        // bare felt below the mats and the whole table riding high.
        //
        // Where a centred look point may stand is an interval — far enough
        // back that the far edge clears `top`, far enough forward that the
        // near edge clears `bottom` — and the middle of it is the shot. A
        // table too big for `MAX_DISTANCE` has no such interval, and there
        // the far edge is pinned again and the overflow goes out of the
        // bottom, which is the graceful direction: a mat behind the tab strip
        // is a mat nobody can see, and one under the hand zone is one the
        // player can pull into view.
        //
        // Sideways the span is centred in the band as it stands at the near
        // edge, for the same reason `wide` is measured there: centring on the
        // look plane's band leaves the front row off-centre, and the rail
        // makes the band asymmetric, so being off-centre costs a whole mat on
        // one side.
        let pinned = max.y - eye * g_top;
        let forward = min.y - eye * g_bottom;
        let along = pinned.max(f32::midpoint(pinned, forward));
        // And sideways, the same interval read off the corners themselves:
        // as far right as the leftmost corner allows, as far left as the
        // rightmost one does, and the middle of that.
        let (mut left, mut right_most) = (f32::NEG_INFINITY, f32::INFINITY);
        for c in &corners {
            let band = (eye + k * (c.y - along)).max(1e-3) * scale;
            left = left.max(c.x - right * band);
            right_most = right_most.min(c.x + band);
        }
        let look = Vec2::new(f32::midpoint(left, right_most), along);
        Self {
            // `ground` works from the eye's true distance; the rig stores the
            // height it stands at, which the lean makes shorter.
            distance: eye / lean,
            // Table space to world: `+y` away from the local seat is `-z`.
            target: Vec2::new(look.x, -look.y),
            yaw: 0.0,
            lean: tilt,
        }
    }
}

/// How much bare felt is left around the table when it is framed.
///
/// Two things live in this number. The first is not optional: what the layout
/// reports is the box the *cards* stand in, and a seat's mat is drawn
/// [`ZONE_MARGIN`] wider than that on every side — so a shot framed on the
/// reported box crops the mat's own printed border, and at the near edge it
/// crops it under the hand zone.
///
/// The rest is the felt itself. A table framed to the last pixel of the band
/// reads as a photograph someone cropped too tightly, whatever the arithmetic
/// says about it fitting; leaving a couple of units of table showing all round
/// is what makes it look like a table being played at rather than a diagram
/// being displayed. It is also roughly where [`GLOW_SPREAD`] fades out, so the
/// halo under an active seat's mat stays in frame with it.
///
/// Wide duels use [`DUEL_AIR`] to spend more of the viewport on cards.
/// Smaller windows and rings retain the full margin
/// to preserve their readability. Everything past [`SLAB_MARGIN`] is sky.
pub(super) const AIR: f32 = 3.5;
/// A wide duel keeps its mat borders plus a small gutter in frame; the
/// decorative outer rail may extend beyond the viewport. Cards use the
/// recovered space without changing their lane geometry or hit regions.
const DUEL_AIR: f32 = 0.65;
const _: () = assert!(DUEL_AIR > ZONE_MARGIN);
const _: () = assert!(AIR > ZONE_MARGIN);

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
/// `L`, `C = 1/√(1+L²)`, the camera-space depth and height
/// of that point work out to
///
/// ```text
///     depth(s) = D + L·C·s          height(s) = C·s
/// ```
///
/// — the cross terms cancel, which is the whole reason this is arithmetic and
/// not a projection matrix. So `q = height / (depth · tan(fov/2))`, and solved
/// the other way `s = D · ground(q, L)`. Being *linear in `D`* is what lets
/// [`CameraRig::home`] invert it with a division instead of a search.
pub(super) fn ground(q: f32, lean: f32) -> f32 {
    let t = half_fov().tan();
    let c = 1.0 / (1.0 + lean * lean).sqrt();
    q * t / (c * q.mul_add(-lean * t, 1.0))
}

/// The part of the window the table is actually seen through.
///
/// The HUD is not beside the battlefield, it is on top of it: the hand zone is
/// an overlay on the same full-window camera. Framing the table against the
/// *window* therefore frames it against a rectangle part of which nobody can
/// see.
///
/// The top used to cost a hundred and ten of those pixels — a strip of seat
/// tabs and a phase rail under it, on every screen and for the whole game.
/// Both are on the table now, written on each seat's own mat, so the top is
/// **zero** and the camera comes in by that much: the framed depth shrinks,
/// so the rig moves closer, so every card is drawn larger. That is the payment
/// the ledge was bought with.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Canvas {
    /// The window, in logical pixels.
    pub window: Vec2,
    /// Covered at the top. Nothing any more — kept because the field is what
    /// a future panel across the top would say so in, and because the framing
    /// arithmetic below is written in terms of all four sides.
    pub top: f32,
    /// Covered at the bottom: the hand zone.
    pub bottom: f32,
    /// Covered on the right. Nothing: the menu pills are a corner rather than
    /// a column, and the stack panel is drawn over the felt on purpose — a
    /// stack lasts a few seconds and the camera must not lurch when one
    /// appears.
    pub right: f32,
}

impl Canvas {
    /// What the duel HUD covers of a window this size.
    #[must_use]
    pub fn hud(window: Vec2) -> Self {
        Self {
            window,
            top: 0.0,
            bottom: crate::hud::HAND_ZONE_H,
            right: 0.0,
        }
    }

    /// The shape of what is left once the HUD has taken its share.
    ///
    /// This is what [`TableLayout::new`] is built against, so the table comes
    /// out the shape of the space it has to fit in. On a 1728×1052 window
    /// with this HUD it is about 2.0 — nothing like the window's 1.64, and
    /// nothing like the `16.0 / 9.0` the layout used to assume.
    #[must_use]
    pub fn aspect(&self) -> f32 {
        let width = (self.window.x - self.right).max(1.0);
        let height = (self.window.y - self.top - self.bottom).max(1.0);
        width / height
    }
}

/// Tells the board model the shape of the space it is drawn in.
///
/// A system of its own because [`crate::rebuild_board`] is called from four
/// places, none of which has a window — it answers a view arriving, a
/// preference changing, a focus moving. The window changes on its own
/// schedule, and this is the one place that notices.
pub fn track_canvas(windows: Query<&Window>, mut duel: ResMut<Duel>) {
    let Ok(window) = windows.single() else {
        return;
    };
    let aspect = Canvas::hud(Vec2::new(window.width(), window.height())).aspect();
    // A resize is a rebuild of the whole layout, so the comparison has to be
    // loose enough that a window nudged by a pixel does not do one per frame.
    if duel
        .canvas_aspect
        .is_none_or(|shown| (shown - aspect).abs() > 0.01)
    {
        duel.canvas_aspect = Some(aspect);
        crate::rebuild_board(&mut duel);
    }
}

/// Tells the board model what the answer being built now proposes.
///
/// A proposal is part of what a stack is merged on
/// (`baylee_client_core::board::Proposal`): three Soldiers declared out of
/// twelve are a card of their own, and so is the Forest a plan would tap
/// out of five. The answer changes through a click, a key, `Esc`, a row in
/// the prompt bar, an armed spell — more doors than
/// [`crate::rebuild_board`] has callers — so the change is noticed here,
/// once a frame, the way [`track_canvas`] notices the window.
pub fn track_proposals(mut duel: ResMut<Duel>) {
    if crate::proposals(&duel) != duel.proposed {
        crate::rebuild_board(&mut duel);
    }
}

/// Keeps the table framed as seats, focus and window size change.
///
/// Whether it may is [`Duel::camera_held`], and that used to be a float
/// comparison: this system kept the framing it had last computed and followed
/// the table only while the rig still *equalled* it. Anything that moved the
/// rig by any amount at all — one pixel of the left-drag orbit that has since
/// been deleted, during an ordinary click on a card — switched the automatic
/// framing off for the rest of the session, silently, and the table then
/// stayed wherever the accident left it through a resize and through a seat
/// joining. Nothing said so and nothing could put it back except a key nobody
/// knew to press.
///
/// Two things put the camera back in the table's hands, and both are about a
/// table rather than about a rig. The **seat count** changing is a different
/// table, so a player aiming at the old one is not aiming at this one. And
/// [`CameraRig::default`] is the one rig that means "nobody aimed this" —
/// [`crate::input::navigate_home`] asks for exactly it, which is how a key, a
/// tab and anything else that cannot reach the flag still comes home.
pub fn frame_table(
    mut duel: ResMut<Duel>,
    windows: Query<&Window>,
    mut seats: Local<usize>,
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
    if *seats != layout.slots.len() {
        *seats = layout.slots.len();
        duel.camera_held = false;
    }
    if *rig == CameraRig::default() {
        duel.camera_held = false;
    }
    if duel.camera_held {
        return;
    }
    if *rig != next {
        *rig = next;
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
/// This is the compromise: about 14° off vertical, which is enough that a
/// card's edge and the shadow around it are both visible.
///
/// It was 22°, and the sentence above about a far seat's cards is why it is
/// not any more. Everything the lean costs is paid by the seat furthest from
/// the camera and collected by the seat nearest it, which on a free-for-all
/// is the player's own: three boards laid out exactly 12.0 units wide each
/// were drawn 450, 381 and 378 pixels wide, and the odd one out was the one
/// the player is looking straight at. A board is the thing a player compares
/// most often, so a table where their own is a fifth larger than everybody
/// else's is a table that has misreported the format.
///
/// Two terms make up that error and the lean drives both. A seat's own width
/// axis is turned away from the camera by its facing, and what is left of it
/// is `√(¼ + ¾cos²)` — pure foreshortening, there at any distance. And the
/// near seat stands `lean · cos · radius` closer to the eye than the ring's
/// centre while the far ones stand half that further away, which is
/// perspective and shrinks as the lens lengthens. Halving the lean and
/// halving the field of view together take a free-for-all from 18.9% to 6.3%,
/// and the seats that gave nothing up are the two opposite ones: it is the
/// player's own board that comes back to the size of theirs.
///
/// It has been 0.40, then 0.24, then 0.27, and is **0.36** — and this last
/// step is the one that cost something, so it is written down rather than
/// buried in a number. The owner has now asked three times for more angle on
/// the table, having been told once what it trades against; that is a
/// decision, not a misunderstanding, and it is theirs to make.
///
/// What it costs, measured rather than argued: at 0.36 the widest board on an
/// eight-seat ring is 10.6% wider than the narrowest, against 6.3% at 0.27.
/// The bound in `every_seat_is_drawn_a_board_of_the_same_width` moved from
/// 1.08 to 1.12 to allow exactly that and no more. The number that matters is
/// still the one the complaint was about — 18.9%, where a player reads their
/// own board as a different format — and 10.6% is well under half of it.
///
/// The lens buys nothing here and was left alone. Tried both ways: 0.34 at a
/// `FOV` of 0.36 spreads 8.2%, and so does 0.33 at 0.42 — the perspective
/// term a longer lens shrinks is the smaller of the two, and the
/// foreshortening term, which is the larger, does not care about the lens at
/// all. So the angle is paid for in width and in nothing else, which is the
/// honest way to sell it.
pub(super) const CAMERA_LEAN: f32 = 0.36;

/// About 32° off vertical for a wide duel; rings retain [`CAMERA_LEAN`].
pub(crate) const DUEL_LEAN: f32 = 0.62;

/// The camera's vertical field of view, in radians.
///
/// Shared with [`CameraRig::home`], which inverts the projection to work out
/// how far back the table has to stand — a framing computed against a
/// different angle from the one the camera is set to is a framing that misses.
///
/// 24° rather than the 40° it was, which is the other half of what
/// [`CAMERA_LEAN`] buys: the same table framed the same way, from twice as
/// far off through half the angle, so every seat sits at nearly the same
/// depth and is drawn at nearly the same size. It is a longer lens than a
/// room is normally seen through, and that is the point — a table is a thing
/// a player reads, not a room they stand in.
pub const FOV: f32 = 0.42;

/// Where the camera actually is, as against where the rig says it should be.
///
/// A second copy rather than smoothing the rig itself, because the rig is
/// *input*: a drag writes it, the framing writes it, focusing a seat writes it,
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
                lean: current.lean + (target.lean - current.lean) * t,
            }
        }
    };
    // Nothing moved and nothing was asked for: the camera stands still most
    // of the time and should cost nothing then.
    if shown.0 == Some(current) && !rig.is_changed() {
        return;
    }
    shown.0 = Some(current);

    let eye = current.eye();
    for mut transform in &mut cams {
        *transform = eye;
    }
}

impl ShownRig {
    /// Where the camera stands *this frame*, or `None` before the first one.
    #[must_use]
    pub fn rig(self) -> Option<CameraRig> {
        self.0
    }
}

impl CameraRig {
    /// The camera transform this rig asks for.
    ///
    /// Extracted from [`apply_camera_rig`] rather than copied, because
    /// [`Lens`] projects with it and anything placed on the table by
    /// projection has to be placed through the *same* eye the camera was set
    /// from. Two derivations of one camera agree until the day one of them is
    /// edited.
    #[must_use]
    pub fn eye(self) -> Transform {
        let horizontal = self.distance * self.lean;
        let offset = Vec3::new(
            self.yaw.sin() * horizontal,
            self.distance,
            self.yaw.cos() * horizontal,
        );
        let look = Vec3::new(self.target.x, 0.0, self.target.y);
        Transform::from_translation(look + offset).looking_at(look, Vec3::Y)
    }
}

/// Where a point on the felt lands in the window, in logical pixels.
///
/// The seat bars are screen-space ink pinned to a rectangle on the table, so
/// something has to answer "where is that rectangle on screen" every frame.
/// It is written here, from a [`CameraRig`], rather than asked of Bevy's
/// `Camera::world_to_viewport`, for a scheduling reason worth stating: a
/// camera's `GlobalTransform` is propagated in `PostUpdate` **after**
/// `UiSystems::Layout` has already run (`bevy_ui` orders its layout
/// `.before(TransformSystems::Propagate)`), so a placement system reading the
/// propagated transform writes a `Node` position the layout will not look at
/// until the next frame. The ink would swim one frame behind the felt for as
/// long as the camera moved. Reading the rig instead, in `Update` and right
/// after [`apply_camera_rig`] has written it, is exact.
#[derive(Clone, Copy)]
pub struct Lens {
    pub(super) clip_from_world: Mat4,
    window: Vec2,
}

impl Lens {
    /// The projection this rig gives onto a window of this size.
    #[must_use]
    pub fn new(rig: CameraRig, window: Vec2) -> Self {
        let aspect = (window.x / window.y.max(1.0)).max(1e-3);
        // `perspective_rh` rather than the reverse-infinite matrix Bevy's
        // `PerspectiveProjection` builds: they differ only in how z is
        // mapped, and nothing here reads z. The near and far planes are
        // therefore arbitrary and only have to bracket the table.
        let clip = Mat4::perspective_rh(FOV, aspect, 0.1, 1000.0);
        Self {
            clip_from_world: clip * rig.eye().to_matrix().inverse(),
            window,
        }
    }

    /// The window this projects onto, in logical pixels.
    #[must_use]
    pub const fn window(&self) -> Vec2 {
        self.window
    }

    /// Where a point on the felt is drawn, or `None` if it is behind the eye.
    #[must_use]
    pub fn project(&self, table: Vec2) -> Option<Vec2> {
        self.project_world(to_world(table, TABLE_Y))
    }

    /// Where a point in the world is drawn, or `None` if it is behind the eye.
    ///
    /// The felt is the common case and [`Self::project`] is the name for it.
    /// This is for what is not a point on the felt: a card is a quad standing
    /// `CARD_LIFT` above it and turned by its own rotation, and its corners
    /// are world points with no table-space spelling. The lift itself turns
    /// out to be worth almost nothing on screen — 0.14 px at a duel, measured
    /// while writing `devctl::card_rect` — because the camera is near enough
    /// overhead that raising something a hundredth of a card barely moves it.
    /// It is the rotation that this is needed for.
    #[must_use]
    pub fn project_world(&self, world: Vec3) -> Option<Vec2> {
        let clip = self.clip_from_world * world.extend(1.0);
        if clip.w <= 1e-4 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        Some(Vec2::new(
            ndc.x.mul_add(0.5, 0.5) * self.window.x,
            0.5f32.mul_add(-ndc.y, 0.5) * self.window.y,
        ))
    }

    /// Intersects a screen point with a horizontal plane in the table scene.
    pub(crate) fn on_plane(&self, screen: Vec2, height: f32) -> Option<Vec3> {
        let ndc = Vec2::new(
            screen.x / self.window.x * 2.0 - 1.0,
            1.0 - screen.y / self.window.y * 2.0,
        );
        // Double precision avoids cancellation between the near/far ray points.
        let inverse = self.clip_from_world.as_dmat4().inverse();
        let ndc = ndc.as_dvec2();
        let near = inverse.project_point3(ndc.extend(-1.0));
        let far = inverse.project_point3(ndc.extend(1.0));
        let direction = far - near;
        if direction.y.abs() < 1e-6 {
            return None;
        }
        let t = (f64::from(height) - near.y) / direction.y;
        (t >= 0.0).then_some((near + direction * t).as_vec3())
    }

    /// Four table points projected, in the order they were given.
    ///
    /// `None` if any of them is behind the eye, because three corners of a
    /// rectangle are not a smaller rectangle — they are a bar drawn somewhere
    /// the shelf is not.
    #[must_use]
    pub fn corners(&self, points: [Vec2; 4]) -> Option<[Vec2; 4]> {
        let mut out = [Vec2::ZERO; 4];
        for (slot, point) in out.iter_mut().zip(points) {
            *slot = self.project(point)?;
        }
        Some(out)
    }
}

/// The screen box one card covers, as its centre and its size in logical
/// pixels.
///
/// The centre is the card's own projected origin rather than the middle of
/// the box: under perspective those differ, and the one worth clicking is the
/// card's.
///
/// It lives here rather than in [`crate::devctl`], where it was written, for
/// the reason the ability sheet needs it: a sheet anchored to a permanent has
/// to be put where that permanent is *drawn*, and the harness had already
/// worked out that a card is four corners turned by its own transform and not
/// an axis-aligned rectangle around its origin.
#[must_use]
pub fn card_box(lens: &Lens, at: &Transform) -> Option<(Vec2, Vec2)> {
    let half = Vec2::new(
        baylee_client_core::layout::CARD_WIDTH,
        baylee_client_core::layout::CARD_HEIGHT,
    ) / 2.0;
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    for corner in [
        Vec2::new(-half.x, -half.y),
        Vec2::new(half.x, -half.y),
        Vec2::new(half.x, half.y),
        Vec2::new(-half.x, half.y),
    ] {
        let drawn = lens.project_world(at.transform_point(corner.extend(0.0)))?;
        min = min.min(drawn);
        max = max.max(drawn);
    }
    Some((lens.project_world(at.translation)?, max - min))
}

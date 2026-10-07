//! The camera: its rig, the canvas it frames, the lens UI is placed through.

#[allow(clippy::wildcard_imports)] // the table's shared vocabulary
use super::*;
use baylee_client_core::tableview::{
    Arrangement, RingLean, VisitCamera, VisitFrame, WindowClass as Frame,
};

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

    /// A camera standing behind `slot` on its own axis, its cards upright,
    /// looking at a point toward the middle of the table.
    ///
    /// Kept for the tests that look at the table from every chair (plates,
    /// shells): they want *a* camera behind each seat, not the visit's fit.
    /// The yaw is the pod's facing and not its bearing from the middle —
    /// `world_center.y.atan2(world_center.x) + π/2` agreed with the facing
    /// on the flanks and gave the home azimuth for the seat across, so the
    /// far board was drawn upside down from behind the local chair
    /// (DESIGN-v7 §2.2).
    #[must_use]
    pub fn framing(slot: &SeatSlot, world_center: Vec2) -> Self {
        Self {
            target: world_center * 0.72,
            distance: (slot.half_extent.length() * 2.6).clamp(9.0, Self::MAX_DISTANCE),
            yaw: behind(slot),
            lean: CAMERA_LEAN,
        }
    }

    /// The whole table, framed inside the part of the window it is actually
    /// seen through, at the recommended shot ([`Shot::default`]).
    #[must_use]
    pub fn home(layout: &TableLayout, canvas: Canvas) -> Self {
        Self::home_shot(layout, canvas, Shot::default()).0
    }

    /// The whole table, framed inside the part of the window it is actually
    /// seen through, and which of the two fits bound it.
    ///
    /// This is the shot a duel opens on and the one `navigate_home` returns
    /// to, and it is computed rather than written down because the thing it
    /// has to fit changes: two seats and eight seats are different tables,
    /// and a phone and a monitor leave different amounts of them uncovered.
    /// The hard-coded 20 units it replaced put the local seat's own mat under
    /// the hand zone on every screen — a player could not see their own
    /// creatures, which made every later piece of board legibility moot.
    ///
    /// A ring (three seats or more) stands closer and steeper than it did
    /// before v7: the device's [`RingLean`] (0.62 by default, D20) and
    /// [`ring_air`] (1.0 on a desktop) instead of [`CAMERA_LEAN`] and
    /// [`AIR`]. Every seat stays whole in the frame; mine, nearest the eye,
    /// collects most of what the steeper angle saves. On a phone the frame is
    /// my own pod and the dial's near half (DESIGN-v7 §1.4): a 266-px band
    /// cannot hold eight readable boards, and a far seat is read by visiting.
    #[must_use]
    pub fn home_shot(layout: &TableLayout, canvas: Canvas, shot: Shot) -> (Self, Binds) {
        match shot.arrangement {
            // The upright ring's flank seats stand square to the table, and
            // at three seats the left one reaches the arrangement pill's
            // corner: this arrangement's home keeps below the pill's line
            // (DESIGN-v8 §2.2's fallback — the arm pays, never the others).
            Arrangement::UprightRing => Self::ring_home(layout, canvas.below_the_pill(), shot),
            // An arrangement not built yet seats the ring, and is shot as one.
            Arrangement::Ring
            | Arrangement::Turntable
            | Arrangement::ArcRail
            | Arrangement::Pods
            | Arrangement::Spotlight
            | Arrangement::TurntableRows
            | Arrangement::FocusRing => Self::ring_home(layout, canvas, shot),
        }
    }

    /// The ring's home shot ([`Self::home_shot`]).
    fn ring_home(layout: &TableLayout, canvas: Canvas, shot: Shot) -> (Self, Binds) {
        let Some((min, max)) = layout.extent() else {
            return (Self::default(), Binds::Deep);
        };
        let class = canvas.class();
        let ring = layout.slots.len() >= 3;
        // A wide duel can show the table's depth without foreshortening side
        // seats. Blend in as the window grows; small windows keep the
        // readable plan view and its breathing room.
        let framing = if layout.slots.len() == 2 && canvas.aspect() >= 1.4 {
            ((canvas.window.x - 800.0) / 480.0).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let (tilt, air) = if class == Frame::Phone && ring {
            (PHONE_LEAN, ring_air(class))
        } else if ring {
            (shot.lean.tangent(), ring_air(class))
        } else {
            (
                CAMERA_LEAN + (DUEL_LEAN - CAMERA_LEAN) * framing,
                AIR - (AIR - DUEL_AIR) * framing,
            )
        };
        if class == Frame::Phone
            && ring
            && let Some(local) = layout.local()
        {
            // My own pod and the dial's near half, in my own (unturned)
            // frame: the local seat's facing is zero.
            let (lo, hi) = pod_box(local, air);
            let lo = lo.min(Vec2::new(-DIAL_REACH, -DIAL_REACH));
            let hi = hi.max(Vec2::new(DIAL_REACH, 0.0));
            let corners = box_corners(lo, hi);
            let fit = fit(lo, hi, &corners, tilt, canvas);
            return (fit.rig(0.0, tilt, |p| p), fit.binds);
        }
        let (min, max) = (min - Vec2::splat(air), max + Vec2::splat(air));
        let corners = layout.corners(air);
        let fit = fit(min, max, &corners, tilt, canvas);
        (fit.rig(0.0, tilt, |p| p), fit.binds)
    }

    /// The shot of one seat's board, the camera standing behind it so its
    /// cards are upright above the hand (DESIGN-v7 §2.2) — or across from it,
    /// by the device's [`VisitCamera`] — and which frame it took.
    ///
    /// **No card moves.** The ring is the one the home shot frames; this only
    /// says where the eye stands. The yaw is the pod's facing (its inward
    /// normal, [`behind`]), not the direction from the middle to the pod: on
    /// a wide ring the two differ, and only the facing draws a pod upright.
    ///
    /// The framed depth runs from the pod's outer edge to a far edge the
    /// [`VisitFrame`] names, measured once per table on the seat **across**
    /// from mine and then held for every seat: my near lane at three and
    /// four seats, the dial from five, the pod alone on a phone. Held, so a
    /// visit draws every seat's board at the same size — the visit is the
    /// equaliser the steeper home shot gave up. A seat further from the
    /// middle than the one across (the ends of a wide ring) keeps that depth
    /// rather than reaching for the dial, so there the dial may fall outside
    /// the frame; `/state.camera.dial_in_frame` says so.
    ///
    /// `None` for a seat that is not at this table.
    #[must_use]
    pub fn visit(
        layout: &TableLayout,
        canvas: Canvas,
        seat: PlayerId,
        shot: Shot,
    ) -> Option<(Self, VisitFrame, Binds)> {
        match shot.arrangement {
            Arrangement::UprightRing => {
                Self::zoom_visit(layout, canvas.below_the_pill(), seat, shot)
            }
            Arrangement::Ring
            | Arrangement::Turntable
            | Arrangement::ArcRail
            | Arrangement::Pods
            | Arrangement::Spotlight
            | Arrangement::TurntableRows
            | Arrangement::FocusRing => Self::ring_visit(layout, canvas, seat, shot),
        }
    }

    /// A visit that is a zoom (DESIGN-v8 §1 rows 2 and 5): the pod is upright
    /// already, so the eye keeps the home azimuth (yaw 0) and only comes in —
    /// the visited pod and its air framed at a wide duel's lean, nothing else.
    ///
    /// Seen from across where the choice says so (an opponent, under
    /// *Automatic*): the eye at yaw π, the board read as a duel opponent's.
    fn zoom_visit(
        layout: &TableLayout,
        canvas: Canvas,
        seat: PlayerId,
        shot: Shot,
    ) -> Option<(Self, VisitFrame, Binds)> {
        let slot = layout.slot(seat)?;
        let class = canvas.class();
        let tilt = if class == Frame::Phone {
            PHONE_LEAN
        } else {
            DUEL_LEAN
        };
        let air = ring_air(class);
        let (lo, hi) = pod_box(slot, air);
        let (lo, hi) = (
            from_pod_frame(slot.facing, lo).min(from_pod_frame(slot.facing, hi)),
            from_pod_frame(slot.facing, lo).max(from_pod_frame(slot.facing, hi)),
        );
        let across = shot.visit.resolve(shot.teammate(seat)) == VisitCamera::Across;
        let (lo, hi) = if across { (-hi, -lo) } else { (lo, hi) };
        let fit = fit(lo, hi, &box_corners(lo, hi), tilt, canvas);
        let yaw = if across { std::f32::consts::PI } else { 0.0 };
        let rig = fit.rig(yaw, tilt, |p| if across { -p } else { p });
        Some((rig, VisitFrame::Pod, fit.binds))
    }

    /// The ring's visit ([`Self::visit`]).
    fn ring_visit(
        layout: &TableLayout,
        canvas: Canvas,
        seat: PlayerId,
        shot: Shot,
    ) -> Option<(Self, VisitFrame, Binds)> {
        let slot = layout.slot(seat)?;
        let class = canvas.class();
        let phone = class == Frame::Phone;
        let tilt = if phone { PHONE_LEAN } else { DUEL_LEAN };
        let air = ring_air(class);
        // *Automatic* answers by the roster: a teammate's board from behind
        // it, an opponent's from across (the owner, 07.10.2026). A phone
        // frames the pod alone either way, from the side the choice says.
        let choice = shot.visit.resolve(shot.teammate(seat));
        let across = choice == VisitCamera::Across;
        // One frame's box and fit: the visited pod in its own frame (`+y`
        // toward the middle), run to the frame's far edge; seen from across,
        // the same box turned half round, the pod at the far side.
        let shoot = |frame: VisitFrame| -> Option<Fit> {
            let reach = visit_reach(layout, frame, air)?;
            let (lo, hi) = pod_box(slot, air);
            let (lo, hi) = (lo, Vec2::new(hi.x, lo.y + reach));
            let (lo, hi) = if across { (-hi, -lo) } else { (lo, hi) };
            Some(fit(lo, hi, &box_corners(lo, hi), tilt, canvas))
        };
        let frame = if choice == VisitCamera::Behind && !phone {
            VisitFrame::of(
                choice,
                shoot(VisitFrame::Lane)?.eye,
                shoot(VisitFrame::Dial)?.eye,
                phone,
            )
        } else {
            VisitFrame::of(choice, 0.0, 0.0, phone)
        };
        let fit = shoot(frame)?;
        let turn = if across { std::f32::consts::PI } else { 0.0 };
        let facing = slot.facing;
        let yaw = (behind(slot) + turn).rem_euclid(std::f32::consts::TAU);
        let rig = fit.rig(yaw, tilt, |p| {
            let p = if across { -p } else { p };
            from_pod_frame(facing, p)
        });
        Some((rig, frame, fit.binds))
    }

    /// Whether the dial's whole disc is inside the band this rig shows.
    #[must_use]
    pub fn sees_the_dial(self, canvas: Canvas) -> bool {
        let lens = Lens::new(self, canvas.window);
        (0..16).all(|k| {
            #[allow(clippy::cast_precision_loss)]
            let at = Vec2::from_angle(std::f32::consts::TAU * k as f32 / 16.0)
                * baylee_client_core::dial::COMPASS_R;
            lens.project(at).is_some_and(|p| {
                p.x >= 0.0
                    && p.x <= canvas.window.x - canvas.right
                    && p.y >= canvas.top
                    && p.y <= canvas.window.y - canvas.bottom
            })
        })
    }
}

/// How a device wants its table shot: the two settings of DESIGN-v7 (D20,
/// D21), read from `ClientSettings::table`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Shot {
    /// How the seats are placed: which home and visit poses apply.
    pub arrangement: Arrangement,
    /// The ring's lean.
    pub lean: RingLean,
    /// Where a visit stands.
    pub visit: VisitCamera,
    /// My teammates, one bit per seat (`1 << PlayerId::get`): who
    /// *Automatic* visits from behind. From the roster's teams
    /// (`GameStatic`), never anything hidden; empty at a table without teams.
    pub teammates: u16,
}

impl Shot {
    /// Whether `seat` is on my team.
    #[must_use]
    pub fn teammate(self, seat: PlayerId) -> bool {
        u32::from(seat.get()) < 16 && self.teammates & (1 << seat.get()) != 0
    }

    /// The teammates of `me` in a roster of `(seat, team)` pairs.
    #[must_use]
    pub fn teammates_of(
        me: PlayerId,
        roster: impl IntoIterator<Item = (PlayerId, Option<u8>)>,
    ) -> u16 {
        let roster: Vec<(PlayerId, Option<u8>)> = roster.into_iter().collect();
        let Some(mine) = roster.iter().find(|(p, _)| *p == me).and_then(|(_, t)| *t) else {
            return 0;
        };
        roster
            .iter()
            .filter(|(p, t)| *p != me && *t == Some(mine) && p.get() < 16)
            .fold(0, |bits, (p, _)| bits | (1 << p.get()))
    }
}

impl From<baylee_client_core::tableview::TableView> for Shot {
    fn from(view: baylee_client_core::tableview::TableView) -> Self {
        Self {
            arrangement: view.arrangement,
            lean: view.lean,
            visit: view.visit,
            teammates: 0,
        }
    }
}

/// Which of the fit's two divisions decided the eye distance: the table's
/// depth against the band's height, or its width against the band's width.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Binds {
    /// The depth bound.
    #[default]
    Deep,
    /// The width bound.
    Wide,
}

impl Binds {
    /// The name `/state.camera.binds` prints.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Deep => "deep",
            Self::Wide => "wide",
        }
    }
}

/// The yaw that stands the eye behind a seat, on its pod's own axis.
///
/// `CameraRig::eye` puts the eye at `(sin yaw, cos yaw)` from the look point
/// in world x/z, which is table `(sin yaw, −cos yaw)`; behind the pod is
/// against its inward normal `(sin f, cos f)`, so `yaw = −facing`. The local
/// seat (facing 0) gives 0, the home azimuth; the seat across gives π.
#[must_use]
pub fn behind(slot: &SeatSlot) -> f32 {
    (-slot.facing).rem_euclid(std::f32::consts::TAU)
}

/// The dial's reach from the middle, its far rim plus half a unit: what a
/// frame that keeps the dial in view runs to.
const DIAL_REACH: f32 = baylee_client_core::firewheel::FLAME_REACH + 0.5;

/// A wide ring's air on a desktop: the mat's printed border and a little
/// felt, not the three and a half units of bare cloth rings used to keep.
pub(super) const RING_AIR: f32 = 1.0;

/// The lean a phone's ring takes: a steeper shot foreshortens a 266-px band
/// too far (DESIGN-v7 §1.4).
pub(super) const PHONE_LEAN: f32 = 0.50;

/// The air a ring keeps, by the window's class (DESIGN-v7 §1.4).
#[must_use]
pub fn ring_air(class: Frame) -> f32 {
    match class {
        // The design said 0.5; the mat's printed border is `ZONE_MARGIN`
        // (0.55) wide, and a frame inside it crops the border on the very
        // screen that can least afford to lose an edge.
        Frame::Phone => PHONE_AIR,
        // The design said 0.8. Under 1.2 a narrow window's flank seat
        // reaches the square beside the report button
        // (`the_corner_beside_the_report_button_lies_on_no_seat_s_place`,
        // 4 seats at 800 x 600), so 1.2 is the measured floor.
        Frame::Narrow => 1.2,
        Frame::Wide => RING_AIR,
    }
}
const _: () = assert!(RING_AIR > ZONE_MARGIN);
/// A phone's ring air: just past the mat's printed border.
const PHONE_AIR: f32 = 0.6;
const _: () = assert!(PHONE_AIR > ZONE_MARGIN);

/// A point in a pod's own frame (`+y` toward the middle, the frame its
/// corners are measured in) turned back into table space.
fn from_pod_frame(facing: f32, p: Vec2) -> Vec2 {
    let (sin, cos) = facing.sin_cos();
    Vec2::new(cos.mul_add(p.x, sin * p.y), (-sin).mul_add(p.x, cos * p.y))
}

/// A table point in a pod's own frame: the inverse of [`from_pod_frame`].
fn into_pod_frame(facing: f32, p: Vec2) -> Vec2 {
    let (sin, cos) = facing.sin_cos();
    Vec2::new(cos.mul_add(p.x, -sin * p.y), sin.mul_add(p.x, cos * p.y))
}

/// A seat's whole footprint plus `air`, as a box in its own frame.
fn pod_box(slot: &SeatSlot, air: f32) -> (Vec2, Vec2) {
    let centre = into_pod_frame(slot.facing, slot.footprint_center());
    let half = slot.footprint() + Vec2::splat(air);
    (centre - half, centre + half)
}

/// How deep a visit's frame runs from the visited pod's outer edge (its air
/// included), measured on the seat across from mine (DESIGN-v7 §2.2).
fn visit_reach(layout: &TableLayout, frame: VisitFrame, air: f32) -> Option<f32> {
    let local = layout.local()?;
    let across = layout.slots.iter().max_by(|a, b| {
        let off =
            |s: &SeatSlot| (s.facing - std::f32::consts::PI).rem_euclid(std::f32::consts::TAU);
        let near = |s: &SeatSlot| off(s).min(std::f32::consts::TAU - off(s));
        near(b).total_cmp(&near(a))
    })?;
    let (lo, hi) = pod_box(across, air);
    let far = match frame {
        VisitFrame::Pod => hi.y - air + 0.5,
        VisitFrame::Dial | VisitFrame::Across => DIAL_REACH,
        VisitFrame::Lane => {
            // My lane nearest the middle, its far side: the lane's centre
            // less half its depth, away from the middle, in my frame; then
            // every corner of it in the across seat's.
            let lane = local.lane_center(baylee_client_core::layout::LaneKind::Creatures);
            let away = Vec2::new(local.facing.sin(), local.facing.cos());
            let side = Vec2::new(local.facing.cos(), -local.facing.sin());
            let back = lane - away * (local.lane_height() * 0.5);
            let half = local.half_extent.x;
            [back + side * half, back - side * half]
                .into_iter()
                .map(|p| into_pod_frame(across.facing, p).y)
                .fold(f32::NEG_INFINITY, f32::max)
                + 0.5
        }
    };
    Some((far - lo.y).max(hi.y - lo.y))
}

/// A box's four corners.
fn box_corners(lo: Vec2, hi: Vec2) -> [Vec2; 4] {
    [lo, Vec2::new(hi.x, lo.y), hi, Vec2::new(lo.x, hi.y)]
}

/// What one fit decided: where the eye looks (in the frame the box was given
/// in) and how far off it stands.
pub(super) struct Fit {
    pub(super) look: Vec2,
    pub(super) eye: f32,
    pub(super) binds: Binds,
}

impl Fit {
    /// The rig for this fit, with `to_table` turning the look point out of
    /// the frame the box was fitted in.
    pub(super) fn rig(&self, yaw: f32, tilt: f32, to_table: impl Fn(Vec2) -> Vec2) -> CameraRig {
        let lean = (1.0 + tilt * tilt).sqrt();
        let look = to_table(self.look);
        CameraRig {
            // `ground` works from the eye's true distance; the rig stores the
            // height it stands at, which the lean makes shorter.
            distance: self.eye / lean,
            // Table space to world: `+y` away from the local seat is `-z`.
            target: Vec2::new(look.x, -look.y),
            yaw,
            lean: tilt,
        }
    }
}

/// Fits a box (`min`..`max`, with `corners` the points that must stay in
/// the band) into the canvas, seen from `-y` at lean `tilt`.
///
/// One fit for every shot: home, a phone's own pod and a visit hand it a
/// box in the frame the camera looks along (`+y` up the screen), and get
/// back a look point in that frame. The arithmetic is the home shot's as it
/// was, unchanged.
pub(super) fn fit(min: Vec2, max: Vec2, corners: &[Vec2], tilt: f32, canvas: Canvas) -> Fit {
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
    let mut wide: f32 = 0.0;
    for a in corners {
        for b in corners {
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
    let eye = deep.max(wide).clamp(
        CameraRig::MIN_DISTANCE * lean,
        CameraRig::MAX_DISTANCE * lean,
    );
    let binds = if deep >= wide {
        Binds::Deep
    } else {
        Binds::Wide
    };

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
    for c in corners {
        let band = (eye + k * (c.y - along)).max(1e-3) * scale;
        left = left.max(c.x - right * band);
        right_most = right_most.min(c.x + band);
    }
    Fit {
        look: Vec2::new(f32::midpoint(left, right_most), along),
        eye,
        binds,
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

    /// The same canvas with its top kept below the line every top-pinned
    /// panel keeps below ([`crate::hud::TOP_CLEAR`]): for an arrangement
    /// whose seats would otherwise reach the switcher's pill.
    #[must_use]
    pub fn below_the_pill(self) -> Self {
        Self {
            top: self.top.max(crate::hud::TOP_CLEAR),
            ..self
        }
    }

    /// The window's size class for the camera (`WindowClass`),
    /// read off the raw window: the table's faces do not follow the shell's
    /// text step, so its width is not divided by one.
    #[must_use]
    pub fn class(&self) -> Frame {
        Frame::of(self.window.x, self.window.y)
    }
}

/// Tells the board model the shape of the space it is drawn in.
///
/// A system of its own because [`crate::rebuild_board`] is called from four
/// places, none of which has a window — it answers a view arriving, a
/// preference changing, a focus moving. The window changes on its own
/// schedule, and this is the one place that notices.
pub fn track_canvas(windows: Query<&Window>, mut duel: ResMut<Duel>) {
    // The arrangement in effect is decided by `arrangement::choose`, which
    // runs just before this and seats the table again when it changes.
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

/// Keeps the table framed as seats, a visit and the window size change.
///
/// The camera has two poses and this is the one door both are computed
/// through: home ([`CameraRig::home_shot`]) while [`Duel::visiting`] is
/// `None`, the visit ([`CameraRig::visit`]) while it names a seat. Both are
/// recomputed on every frame from the layout, the window and this device's
/// [`Shot`], so a resize or a seat joining re-frames whichever pose is
/// standing — a visit re-fits rather than letting go. A visited seat that is
/// no longer at the table sends the camera home.
///
/// What the player chose is the pose, never a rig: there is no orbit, pan or
/// zoom control, so nothing a hand does can leave the camera somewhere this
/// system would have to respect. That is what replaced `camera_held`, a flag
/// set by gestures that no longer exist.
pub fn frame_table(
    mut duel: ResMut<Duel>,
    windows: Query<&Window>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    mut rig: ResMut<CameraRig>,
    mut pose: ResMut<CameraPose>,
) {
    let Some(layout) = duel.layout.as_ref() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let canvas = Canvas::hud(Vec2::new(window.width(), window.height()));
    // The device's lean and visit camera, and the arrangement in effect at
    // this table — not the device's default, which this game's switch, the
    // per-count memory and the offer may all have overruled.
    let teammates = duel.seat().map_or(0, |me| {
        Shot::teammates_of(
            me,
            duel.statics
                .iter()
                .flat_map(|statics| statics.seats.iter().map(|s| (s.player, s.team))),
        )
    });
    let shot = Shot {
        arrangement: duel.arrangement,
        teammates,
        ..settings.map_or_else(Shot::default, |s| Shot::from(s.table))
    };
    let visit = duel
        .camera_visit()
        .and_then(|seat| CameraRig::visit(layout, canvas, seat, shot));
    let (next, frame, binds) = if let Some((rig, frame, binds)) = visit {
        (rig, Some(frame), binds)
    } else {
        let (rig, binds) = CameraRig::home_shot(layout, canvas, shot);
        (rig, None, binds)
    };
    // A seat no longer at the table is home again — for a layout
    // arrangement too, whose interest is a seat it brings across.
    if duel
        .visiting
        .is_some_and(|seat| layout.slot(seat).is_none())
        || (visit.is_none() && duel.camera_visit().is_some())
    {
        duel.visiting = None;
    }
    if *rig != next {
        *rig = next;
    }
    if pose.rig != Some(next) || pose.visiting != duel.visiting {
        *pose = CameraPose {
            visiting: duel.visiting,
            frame,
            binds,
            dial_in_frame: next.sees_the_dial(canvas),
            rig: Some(next),
        };
    }
}

/// What the camera is doing, for `/state.camera` and the dial: the pose it
/// was asked for and what that shot took in.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct CameraPose {
    /// The seat being visited; `None` is home.
    pub visiting: Option<PlayerId>,
    /// A visit's frame; `None` at home.
    pub frame: Option<VisitFrame>,
    /// Which fit bound the shot.
    pub binds: Binds,
    /// Whether the dial's whole disc is in the band.
    pub dial_in_frame: bool,
    /// The rig this was computed for.
    pub rig: Option<CameraRig>,
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
/// *input*: the framing writes it, a visit writes it, and every one of those
/// wants to be able to say "there" without having to know that something
/// else is interpolating behind it.
#[derive(Resource, Clone, Copy, Default)]
pub struct ShownRig {
    rig: Option<CameraRig>,
    /// The pose the camera last settled on or set out for: `None` is home,
    /// a seat a visit. A change is what starts an orbit.
    pose: Option<PlayerId>,
    /// Whether a pose has been seen at all: the first is a cut.
    posed: bool,
    /// The timed move between two poses, while it runs.
    orbit: Option<Orbit>,
}

/// A visit or a return: a timed move, not the exponential settle.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Orbit {
    /// Where the camera was when the move began.
    from: CameraRig,
    /// Seconds into it.
    elapsed: f32,
}

/// How long a visit or a return takes (DESIGN-v7 §2.3): the same for a flank
/// and for the seat across, because yaw, target, distance and lean move
/// together on one clock.
pub const ORBIT_SECS: f32 = 0.55;

/// Ease-in-out cubic: a move that starts and stops gently.
fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0f32).mul_add(t, 2.0).powi(3) / 2.0
    }
}

/// The rig `t` of the way from `from` to `to`, yaw the short way round.
fn between(from: CameraRig, to: CameraRig, t: f32) -> CameraRig {
    let turn = std::f32::consts::TAU;
    let yaw_delta =
        (to.yaw - from.yaw + std::f32::consts::PI).rem_euclid(turn) - std::f32::consts::PI;
    CameraRig {
        target: from.target.lerp(to.target, t),
        distance: from.distance + (to.distance - from.distance) * t,
        yaw: from.yaw + yaw_delta * t,
        lean: from.lean + (to.lean - from.lean) * t,
    }
}

/// Turns the rig into the camera transform: a near-plan view of the
/// battlefield canvas, leaning so the cards have somewhere to cast a shadow;
/// the rig decides target, distance and azimuth.
///
/// Two ways of following it. A **visit or a return** (the pose in
/// [`Duel::visiting`] changed) is a timed orbit of [`ORBIT_SECS`], eased in
/// and out, yaw the short way round, re-aimed every frame at the rig as it
/// stands so a re-fit during the move is followed. Anything else — a resize,
/// a seat joining — is the exponential settle at `CAMERA_SETTLE`, which is
/// right for a few units of re-framing. Under reduced motion both are cuts.
pub fn apply_camera_rig(
    rig: Res<CameraRig>,
    time: Res<Time>,
    duel: Option<Res<Duel>>,
    prefs: Res<crate::prefs::Prefs>,
    mut shown: ResMut<ShownRig>,
    mut cams: Query<&mut Transform, With<TableCamera>>,
) {
    let target = *rig;
    let still = prefs.all().reduce_motion;
    // Only a camera arrangement's visit is a pose: a layout arrangement's
    // interest moves cards, and an orbit started for it would run 0.55 s
    // from the rig to itself.
    let pose = duel.and_then(|duel| duel.camera_visit());
    let mut orbit = shown.orbit;
    if let Some(current) = shown.rig
        && shown.posed
        && shown.pose != pose
        && !still
    {
        orbit = Some(Orbit {
            from: current,
            elapsed: 0.0,
        });
    }
    let current = match (shown.rig, orbit) {
        // The first frame is a cut by definition: there is nowhere to come
        // from. So is a table a player has asked to hold still.
        (None, _) => target,
        (Some(_), _) if still => target,
        (Some(_), Some(mut journey)) => {
            journey.elapsed += time.delta_secs();
            let t = journey.elapsed / ORBIT_SECS;
            orbit = (t < 1.0).then_some(journey);
            if t < 1.0 {
                between(journey.from, target, ease_in_out(t))
            } else {
                target
            }
        }
        (Some(current), None) => {
            let t = 1.0 - (-CAMERA_SETTLE * time.delta_secs()).exp();
            let next = between(current, target, t);
            // The last thousandth of a unit is put on the mark, as a card's
            // is (`glide`'s SETTLED): an exponential left to run converges on
            // the target only to the last bit of an `f32`, and every frame
            // until then writes the camera's transform on a still table.
            if next.close_to(target) { target } else { next }
        }
    };
    if still {
        orbit = None;
    }
    // Nothing moved and nothing was asked for: the camera stands still most
    // of the time and should cost nothing then.
    if shown.rig == Some(current) && shown.pose == pose && shown.orbit == orbit && !rig.is_changed()
    {
        return;
    }
    *shown = ShownRig {
        rig: Some(current),
        pose,
        posed: true,
        orbit,
    };

    let eye = current.eye();
    for mut transform in &mut cams {
        *transform = eye;
    }
}

impl ShownRig {
    /// Where the camera stands *this frame*, or `None` before the first one.
    #[must_use]
    pub fn rig(self) -> Option<CameraRig> {
        self.rig
    }

    /// Whether a visit or a return is under way.
    #[must_use]
    pub fn moving(self) -> bool {
        self.orbit.is_some()
    }
}

impl CameraRig {
    /// Whether this rig is within a thousandth of `other` in every part: on
    /// its mark, for the settle's purpose.
    #[must_use]
    pub fn close_to(self, other: Self) -> bool {
        const NEAR: f32 = 1e-3;
        self.target.abs_diff_eq(other.target, NEAR)
            && (self.distance - other.distance).abs() <= NEAR
            && (self.lean - other.lean).abs() <= NEAR * 1e-1
            && ((self.yaw - other.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI)
                .abs()
                <= NEAR * 1e-1
    }

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

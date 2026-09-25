//! Table geometry: where seats sit, and how a seat's permanents pack into it.
//!
//! # The eight-player problem
//!
//! Two players fit on a screen. Eight do not — a commander table can hold
//! several hundred permanents, and the naive answer (shrink everything) makes
//! the board unreadable exactly when there is most to read. The layout here
//! solves it in three steps, and each is testable arithmetic rather than a
//! render-time accident:
//!
//! 1. **Seats sit on a ring**, with the local seat always at the near edge, and
//!    opponents going clockwise in *turn order* — the player on your left is
//!    the one who acts after you, which is the association a player already has
//!    from a physical table. What divides the ring is a **side** and not a
//!    seat: allies share one and sit along it shoulder to shoulder, because a
//!    partner's board is read as often as one's own and across the table it
//!    is upside down.
//! 2. **Every seat gets the same board**, and the ring grows until that board
//!    is worth playing on. A table where one player's ground is wider than
//!    another's is a table where the wider ground is the one being played on.
//!    The one thing that changes it is a focus — an opponent whose board is
//!    being inspected is enlarged, and what it takes comes from the *other*
//!    opponents rather than from your own.
//! 3. **Lanes fan when they run out of room, and report two thresholds.** A
//!    row that no longer fits overlaps its cards like a physical fan instead
//!    of shrinking them past legibility. [`LanePacking`] names the two points
//!    at which that happens, because they call for different answers:
//!    `fanned` is "the cards would have to overlap", which is the moment
//!    *identical* cards stop being worth drawing separately and the board
//!    model collapses them into a counted stack — a fan of the same card
//!    shows nothing its count does not. `overflowing` is the harder bound,
//!    "even a fan cannot keep them legible", which distinct cards can still
//!    reach after any collapsing is done and which the row has to answer by
//!    scrolling rather than by packing.
//!
//! All coordinates are table-space: `+x` right, `+y` away from the local seat.
//! The renderer maps this onto whatever plane it draws.

use crate::cardplate::PlateRoom;
use baylee_core::ids::PlayerId;
use glam::Vec2;

/// Width of a card in table units. Height follows the real card ratio
/// (63 × 88 mm), so art never has to be letterboxed.
pub const CARD_WIDTH: f32 = 1.0;
/// Height of a card in table units.
pub const CARD_HEIGHT: f32 = 1.397;
/// A card's width divided by its height.
///
/// UI that sizes a card from one dimension needs the other, and a material
/// node — unlike an image node — carries no intrinsic size to fall back on.
pub const CARD_ASPECT: f32 = CARD_WIDTH / CARD_HEIGHT;
/// How far a creature that is fighting stands out of its row, in table units.
///
/// Half a card, along [`SeatSlot::forward`]. It is the gesture a player makes
/// at a real table — the attacker is pushed across the line and the blocker is
/// pushed up to meet it — and the reason it is a *step out of the row* rather
/// than a tilt or a light is that a row with one card out of it can be read
/// from across the table, which is where the player is sitting.
///
/// Half a card and not more, because the step has to say "forward" while the
/// card is still plainly on its own side of the felt. There is room for far
/// more: the narrowest table is a duel, and a creature there can travel 3.53
/// units before its leading edge touches the opponent's ground.
/// `a_staged_creature_never_reaches_another_seats_ground` is that measurement,
/// taken at every seat count, and it is what this number is bounded by.
pub const STAGE_STEP: f32 = CARD_HEIGHT * 0.5;
/// The room one card needs along a lane, however it is turned.
///
/// A card taps by rotating a quarter turn about its own centre (CR 701.26),
/// so the space it claims in a row is its *longest* dimension and not its
/// width. Every cell in a lane is that wide, tapped or not, and the two
/// things that buys are worth the quarter of a card of air around an
/// untapped one: no card can ever overlap its neighbour, and tapping moves
/// nothing — the cell was always the right size and the card turns inside
/// it, on a board where a row that reshuffled itself every time a land paid
/// for something would be unreadable exactly while it is being read.
///
/// Packing to [`CARD_WIDTH`] instead is what put a tapped creature 0.14
/// units into each of its neighbours on a duel's lane that had seventeen
/// units to spare.
pub const CARD_SPAN: f32 = CARD_HEIGHT;
/// Gap between cards in a comfortably filled lane.
pub const CARD_GAP: f32 = 0.12;
/// How much of a card must stay visible when a lane fans: a third (the
/// owner, 25.09: "at least 33% of each card stays visible").
///
/// Below that a row does not fan further but scrolls
/// ([`LanePacking::window`]). It was 0.26, the point where the name and the
/// power/toughness box are both gone, until rows could scroll.
pub const MIN_VISIBLE_FRACTION: f32 = 0.33;

/// Which row of a seat's board a permanent belongs to.
///
/// Splitting by role rather than by play order is what makes an opponent's
/// board readable at a glance: creatures decide combat, lands decide what they
/// can respond with, and everything else is context.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum LaneKind {
    /// Creatures — the combat lane, drawn nearest the middle of the table.
    Creatures,
    /// Artifacts, enchantments, planeswalkers, battles.
    Support,
    /// Lands — drawn at the back, nearest the seat.
    Lands,
}

impl LaneKind {
    /// All lanes, in the order they are drawn from the table centre outwards.
    pub const ALL: [Self; 3] = [Self::Creatures, Self::Support, Self::Lands];

    /// A label for accessibility and for the keyboard zone cycle.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Creatures => "Creatures",
            Self::Support => "Support",
            Self::Lands => "Lands",
        }
    }
}

/// One of the four piles that stand beside a seat's ground.
///
/// These are the zones that are *not* the battlefield, and they are drawn
/// where a player would really have them: on the bare table beside the mat
/// rather than on it. A pile lying on the mat would read as a permanent in
/// play, and whether a creature is in the graveyard or on the battlefield is
/// the one thing about a graveyard that may never be ambiguous.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum PileKind {
    /// The library. Face down always — nobody may look through a library,
    /// its owner included, so it is the one pile with no top card to show
    /// and no way into it.
    Library,
    /// The graveyard, top card up: it is public, and which card died last is
    /// information the game is played on.
    Graveyard,
    /// Public exile.
    Exile,
    /// The command zone — the first commander, plus emblems and companions.
    Command,
    /// The second commander's slot (partners, CR 702.124a).
    ///
    /// A zone drawn as two places rather than one, which is a *drawing*
    /// decision and not a rules one: the command zone is a single zone
    /// (CR 408) and this is where its second commander is put down. Two
    /// slots because a commander is the one thing in that zone a player
    /// looks for by sight — a partner pair on one pile is two cards where
    /// only the top one can be seen, and which of the two is under the other
    /// carries no meaning at all.
    ///
    /// Hidden for every seat that has fewer than two commanders, which is
    /// almost all of them.
    Command2,
}

impl PileKind {
    /// All five, in the order they are laid out.
    pub const ALL: [Self; 5] = [
        Self::Library,
        Self::Graveyard,
        Self::Exile,
        Self::Command,
        Self::Command2,
    ];

    /// Which side of the mat this pile stands on: `1.0` the seat's right
    /// hand, `-1.0` their left.
    ///
    /// Library and graveyard share the right because the two of them are one
    /// motion — a card drawn comes off the top of the first and a card that
    /// dies goes onto the second — and because that is where a player who
    /// holds their hand in the left hand puts them.
    #[must_use]
    pub const fn side(self) -> f32 {
        match self {
            Self::Library | Self::Graveyard | Self::Exile => 1.0,
            Self::Command | Self::Command2 => -1.0,
        }
    }

    /// Which lane row the pile stands level with.
    ///
    /// The right-hand column is a card's life in order of distance from the
    /// hand: the library it is drawn from nearest the seat, the graveyard it
    /// dies into next, and exile — the pile touched least often — furthest
    /// away. The left-hand column is the command zone, one slot per
    /// commander.
    ///
    /// Exile is level with [`LaneKind::Creatures`], and the doc that used to
    /// stand here said no pile ever would: that row is where attackers step
    /// forward and blockers come to meet them, and a pile parked *in* it
    /// would be standing in the only part of the board that moves. It is not
    /// in it. A pile stands `half_extent.x + PILE_REACH` out to the side, on
    /// bare table past the mat's own border — a clearance
    /// [`PILE_REACH`] exists precisely to keep — while a creature declaring
    /// an attack moves *forward*, along the seat's `away`. The guard was
    /// against a pile touching the row, and at this distance it does not.
    #[must_use]
    pub const fn row(self) -> LaneKind {
        match self {
            Self::Library | Self::Command => LaneKind::Lands,
            Self::Graveyard | Self::Command2 => LaneKind::Support,
            Self::Exile => LaneKind::Creatures,
        }
    }

    /// The mark drawn in the middle of this pile's empty place.
    ///
    /// Both command slots carry the crown: they are two places in one zone,
    /// and a second mark would be claiming they are two zones.
    #[must_use]
    pub const fn mark(self) -> crate::tabletop::ZoneMark {
        use crate::tabletop::ZoneMark;
        match self {
            Self::Library => ZoneMark::Library,
            Self::Graveyard => ZoneMark::Graveyard,
            Self::Exile => ZoneMark::Exile,
            Self::Command | Self::Command2 => ZoneMark::Command,
        }
    }

    /// A label for accessibility and for the keyboard zone cycle.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Library => "Library",
            Self::Graveyard => "Graveyard",
            Self::Exile => "Exile",
            Self::Command => "Command",
            Self::Command2 => "Command 2",
        }
    }
}

/// How far the centre of a pile stands out past the edge of the ground it
/// serves.
///
/// Half a card, the mat printed border, and bare table between the two.
/// The bare table is the point of the number: a pile touching the mat reads
/// as part of the board. The border being cleared is
/// [`crate::tabletop::MAT_MARGIN`], and `a_pile_stands_clear_of_the_mat_it_serves`
/// in the renderer fails if the two ever drift apart.
pub const PILE_REACH: f32 = 1.45;

/// How much wider than its playing surface a seat's whole place is, per side.
///
/// The pile strip: [`PILE_REACH`] out to the middle of a pile, and half a
/// card further to its outer edge. It is subtracted inside the ring solve
/// rather than added to the answer afterwards, and that is the whole point of
/// having it as a constant: a table that grows by two strips *after* being
/// fitted to the canvas is a table 15% wider than the screen it is seen
/// through, and the camera pays for it by drawing every card smaller.
const PILE_STRIP: f32 = PILE_REACH + CARD_WIDTH * 0.5;

/// How many cards a hover spreads out of a pile at most.
///
/// Four recent cards keep the preview inside a compact column. The zone
/// browser provides access to the rest. Shared as [`crate::ZonePile::FAN_MAX`].
pub const FAN_MAX: usize = 4;

/// Small lift above the stack; cards remain visually attached to their zone.
pub const FAN_FLOAT: f32 = 0.10;
/// Staircase spacing keeps adjacent card planes separate.
pub const FAN_RISE: f32 = 0.035;
/// Expose a useful strip of each of the four most recent cards.
pub const FAN_STEP: f32 = 0.34;
/// A gentle tilt distinguishes a browsing fan from cards on the battlefield.
pub const FAN_TILT: f32 = 0.35;
/// Restrained rotation keeps the exposed strips easy to point at.
pub const FAN_YAW: f32 = 0.012;
/// Hover feedback must remain smaller than the exposed hit area.
pub const FAN_POP: f32 = 0.035;

/// Where one card of a pile's hover fan stands, in table space.
///
/// Renderer-free like the rest of this module: a pose is four numbers, and
/// turning them into a transform is the renderer's business.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FanPose {
    /// Centre, in table space.
    pub at: Vec2,
    /// Height above the felt.
    pub lift: f32,
    /// Tip towards the camera about the seat's own horizontal axis. Signed
    /// for the *viewer*, so a seat across the table leans towards the one
    /// screen there is rather than away from it.
    pub tilt: f32,
    /// Turn in the card's own plane: one way at one end of the fan and the
    /// other way at the other.
    pub yaw: f32,
}

/// How far a seat has to lean past the side of the ring before the camera is
/// taken to be in front of it rather than behind.
///
/// [`SeatSlot::camera_lies`] is a comparison against zero and a seat at the
/// exact side of the ring is a tie, which in floating point is not a tie at
/// all: `cos(FRAC_PI_2)` is -4.4e-8 and `cos(3·FRAC_PI_2)` is +1.2e-8, so the
/// left flank of a four-seat table would answer one way and the right flank
/// the other. Nothing about the two seats differs, and the answer has to be
/// the same for both.
///
/// It is generous — about 4½° — because a side is placed by walking a
/// polyline of [`RING_STEPS`] steps and lands *near* the side rather than on
/// it, and because there is nothing to lose: the seat closest to a side that
/// is genuinely across the table leans four times this far.
///
/// [`SeatSlot::camera_lies`]: SeatSlot#method.camera_lies
const SIDE_SEAT_TILT: f32 = 0.08;

/// Which of a mat's two long edges carries the seat's shelf: the **outer**
/// one, away from the middle of the table and behind the land row, or the
/// centre-facing one.
///
/// It is the same answer at every seat, which is the whole of the rule:
/// **a seat's ink sits on the edge of its own battlefield nearest the middle
/// of the table — the edge that seat reads as "above".** The three lanes
/// follow it (see [`SeatSlot::lane_center`]): the shelf takes the strip just
/// inside that rim and the board starts a
/// [`MAT_LEDGE`](crate::tabletop::MAT_LEDGE) further back, so no card is ever
/// drawn where the bar is.
///
/// A constant rather than a question asked of each seat, because it stopped
/// being one. The table spent two arrangements answering it per seat, both
/// from the *viewer's* chair: first "above the board it describes on the one
/// screen there is", which sent a seat across the table to its outer edge
/// because its board is drawn upside-down from here; then that, with the
/// local seat pinned to its near edge instead. What both have in common is
/// that an opponent's name and life total were written beyond their far rim,
/// at the top of the screen, as far from their own creatures as the mat
/// allows. The owner asked for the mirror: each seat's ink where *that seat*
/// would read it, which for everyone but the viewer is the lower of their
/// mat's two edges on screen.
///
/// The shader still takes it as a parameter and still draws the band either
/// way ([`crate::tabletop::seat_mat`], and `mat.wgsl` beside it), because the
/// band is a real thing whose end the model chooses; what has gone is the
/// choosing. The only reader that has to agree with this constant is the one
/// that draws the ink, and it agrees by reading
/// [`SeatSlot::ledge_corners`] rather than by knowing the rule.
pub const LEDGE_IS_OUTER: bool = false;

/// One seat's place at the table.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SeatSlot {
    /// Which seat.
    pub player: PlayerId,
    /// Position on the ring: 0 is the local seat, then clockwise in turn order.
    pub ring_index: usize,
    /// Angle on the ring in radians, measured from the near edge.
    pub angle: f32,
    /// Centre of the seat's pod in table space.
    pub center: Vec2,
    /// Rotation that makes this seat's cards face their owner.
    pub facing: f32,
    /// Half the pod's extent.
    pub half_extent: Vec2,
    /// Width reclaimed from an unused command strip; framing keeps its original bounds.
    pub reclaimed: f32,
    /// Whether this is the viewing player's own seat.
    pub is_local: bool,
}

impl SeatSlot {
    /// Give decks without commanders the vacant strip without moving the right-hand piles.
    pub fn reclaim_command_strip(&mut self) {
        if self.reclaimed > 0.0 {
            return;
        }
        self.reclaimed = PILE_STRIP - crate::tabletop::MAT_MARGIN;
        let side = Vec2::new(self.facing.cos(), -self.facing.sin());
        self.center -= side * (self.reclaimed * 0.5);
        self.half_extent.x += self.reclaimed * 0.5;
    }

    /// Centre of the original footprint, including the remaining pile strip.
    #[must_use]
    pub fn footprint_center(&self) -> Vec2 {
        self.center + Vec2::new(self.facing.cos(), -self.facing.sin()) * (self.reclaimed * 0.5)
    }
    /// Usable width of one lane inside this pod.
    #[must_use]
    pub fn lane_width(&self) -> f32 {
        self.half_extent.x * 2.0
    }

    /// The mat's short edge: how deep a seat's ground is, front to back.
    ///
    /// This is the dimension a mat is *smallest* in, and so the one anything
    /// claiming to be smaller than a seat's ground has to be measured
    /// against. [`lane_width`](Self::lane_width) is the long edge, and a
    /// bound written against it passes for an object twice the mat's depth —
    /// which is how the hearth came to fill the gap between two players
    /// while its test went on saying it was smaller than a mat.
    #[must_use]
    pub fn mat_depth(&self) -> f32 {
        self.half_extent.y * 2.0
    }

    /// Height available to a single lane.
    ///
    /// The base row height after reserving the information band and combat
    /// advance. The creature lane additionally owns that advance margin.
    #[must_use]
    pub fn lane_height(&self) -> f32 {
        (self.mat_depth() - crate::tabletop::MAT_LEDGE - STAGE_STEP) / LaneKind::ALL.len() as f32
    }

    /// Where a merged card's count badge stands on this seat's rows: over
    /// the card's top-right corner where the rows leave the felt for it (a
    /// duel's), beside its right edge where they do not (a ring's).
    #[must_use]
    pub fn badge_place(&self) -> crate::cardplate::BadgePlace {
        crate::cardplate::BadgePlace::for_margin((self.lane_height() - CARD_HEIGHT) * 0.5)
    }

    /// Centre of a lane in table space.
    #[must_use]
    pub fn lane_center(&self, lane: LaneKind) -> Vec2 {
        let index = LaneKind::ALL
            .iter()
            .position(|l| *l == lane)
            .unwrap_or_default() as f32;
        let h = self.lane_height();
        // Lane 0 (creatures) sits towards the table centre and lands at the
        // back, at every seat and whichever edge the shelf is on: where a
        // card stands is the seat's own business and does not turn round
        // because the ink moved. So the three lanes are measured from the
        // centre-facing edge, and only a shelf standing *there* pushes them
        // back by its own depth.
        let front = if LEDGE_IS_OUTER {
            0.0
        } else {
            crate::tabletop::MAT_LEDGE
        };
        let offset_from_front = front + STAGE_STEP + (index + 0.5) * h - self.half_extent.y;
        let away = Vec2::new(self.facing.sin(), self.facing.cos());
        self.center - away * offset_from_front
    }

    /// The row whose band of felt `point` lies in, if any: across the lane's
    /// width and within half a lane of its centre, the creature row also
    /// owning the combat step in front of it.
    ///
    /// What a wheel over the felt scrolls, when the row has more cards than
    /// it shows.
    #[must_use]
    pub fn lane_at(&self, point: Vec2) -> Option<LaneKind> {
        let along = Vec2::new(self.facing.cos(), -self.facing.sin());
        let forward = self.forward();
        let h = self.lane_height() * 0.5;
        LaneKind::ALL.iter().copied().find(|&lane| {
            let off = point - self.lane_center(lane);
            let depth = off.dot(forward);
            let front = if lane == LaneKind::Creatures {
                h + STAGE_STEP
            } else {
                h
            };
            off.dot(along).abs() <= self.half_extent.x && depth >= -h && depth <= front
        })
    }

    /// The direction this seat's cards advance in: out of the rows and
    /// towards the middle of the table.
    ///
    /// The same unit vector [`lane_center`](Self::lane_center) measures its
    /// three lanes along, which is what makes "forward" mean the same thing
    /// to a staged attacker as it does to the row it stepped out of. It is
    /// spelled out here rather than left inline there because a second reader
    /// of that vector is a second chance to get its sign wrong, and the sign
    /// is not obvious from the arithmetic: `lane_center` *subtracts* it and
    /// the creature lane's offset is negative, so the two minus signs cancel
    /// and the creature row ends up towards the centre.
    ///
    /// `the_creature_lane_is_forward_of_the_land_lane` is the test, and it is
    /// written as a comparison between the two lanes rather than against a
    /// hand-derived angle: what forward *means* on this table is "the way the
    /// creatures are", at every seat of every ring.
    #[must_use]
    pub fn forward(&self) -> Vec2 {
        Vec2::new(self.facing.sin(), self.facing.cos())
    }

    /// The four corners of this seat's ledge, in table space.
    ///
    /// The band along one long edge of the mat that the seat's bar is
    /// written on. This is what the renderer projects to find where the ink
    /// goes, and it is the *only* thing it needs: the bar is one screen-space
    /// node pinned to this rectangle's projection, never a world-space
    /// object, because there is no text on the 3D table.
    ///
    /// Whichever of the mat's two long edges [`LEDGE_IS_OUTER`] names, so
    /// that every bar at the table is drawn above the board it describes —
    /// above as its *own* seat reads the word.
    ///
    /// Ordered as the seat itself would read them: the two corners on that
    /// outside edge first, left then right in the *seat's* frame, then the
    /// two that meet the lane behind it, right then left. So the four are a
    /// loop, and the first long edge runs the seat's own left to right at
    /// either end of the mat — which is what lets
    /// [`Shelf::of`](crate::seatbar) take the tilt off it without caring
    /// which edge it got.
    #[must_use]
    pub fn ledge_corners(&self) -> [Vec2; 4] {
        let away = Vec2::new(self.facing.sin(), self.facing.cos());
        let side = Vec2::new(self.facing.cos(), -self.facing.sin());
        // `away` points at the middle of the table, so the shelf is measured
        // along it or against it. Everything else about the rectangle — its
        // length, its depth, the order of its corners — is the same either
        // way, which is why this is a sign and not a second branch.
        let reach = if LEDGE_IS_OUTER { -1.0 } else { 1.0 };
        // Out to the edge of the mat as it is *drawn*, which is
        // `MAT_MARGIN` past the playing extent. The shelf and the border at
        // its own end are one band: nothing stands on either, and the mat
        // paints them as one for want of anything else to say about a strip
        // of ground outside the shelf. Measuring to `half_extent` instead is
        // what put a bar 0.46 units inside its own ledge — the ink followed
        // this rectangle to the pixel and this rectangle was not the one on
        // screen.
        let near =
            self.center + away * (reach * (self.half_extent.y + crate::tabletop::MAT_MARGIN));
        let far = self.center + away * (reach * (self.half_extent.y - crate::tabletop::MAT_LEDGE));
        // The length stops at the playing extent even though the band drawn
        // there runs the whole width of the mat, and that is deliberate: the
        // border is a margin for the ink to stop inside, and a bar running
        // out to the corner would be written across the rim that carries the
        // seat's colour.
        let out = side * self.half_extent.x;
        [near - out, near + out, far + out, far - out]
    }

    /// Centre of one of the four piles, in table space.
    ///
    /// Beside the mat rather than on it, level with the lane row it belongs
    /// to. Built from the same frame [`lane_center`](Self::lane_center) uses,
    /// so a pile turns with its seat: at four players the seat on your left
    /// keeps their library at *their* right hand, which is your far side.
    #[must_use]
    pub fn pile_center(&self, pile: PileKind) -> Vec2 {
        // The pod's own sideways direction: `away` turned a quarter to the
        // right, which for a seat facing up the table is the world's `+x`.
        let side = Vec2::new(self.facing.cos(), -self.facing.sin());
        let out = pile.side() * (self.half_extent.x + PILE_REACH);
        self.lane_center(pile.row()) + side * out
    }

    /// Which way along this seat's own depth axis the camera lies: `-1.0` for
    /// the seat's near edge, `1.0` for the table centre.
    ///
    /// `away.y` is the whole test: table `+y` runs away from the camera, so a
    /// seat whose inward normal points up the table has the camera behind it
    /// and a seat across the table has the camera in front. It carries
    /// [`SIDE_SEAT_TILT`] because the two flanks of a ring have to answer
    /// alike, which is the tolerance's whole reason for existing — the shelf
    /// edge used to be the other reader and is now [`LEDGE_IS_OUTER`],
    /// the same answer at every seat.
    ///
    /// The **tie** is the one place this differs, and deliberately. A side
    /// seat's depth axis runs across the screen, so neither end of it is
    /// nearer the camera and the question has no answer; the answer that is
    /// never *wrong* is inwards, over the table, rather than out past its
    /// edge.
    fn camera_lies(&self) -> f32 {
        if self.facing.cos() > SIDE_SEAT_TILT {
            -1.0
        } else {
            1.0
        }
    }

    /// Where one card of a pile's hover fan stands.
    ///
    /// `index` counts from the **top of the pile** — it is the index into
    /// [`crate::ZonePile::fan`] — and it is also the rung, because the fan
    /// opens *backwards*: the top card stands on the pile and does not move,
    /// and each older card steps one [`FAN_STEP`] further from the camera and
    /// one [`FAN_RISE`] higher. So the pointer that opened the pile is still
    /// on the card it opened it to see, and the rest emerge from behind it.
    ///
    /// `under_the_pointer` slides this one card out of the line by
    /// [`FAN_POP`], which is the walk's own feedback.
    ///
    /// An `index` at or past `len` is clamped rather than refused; a fan is a
    /// drawing and the worst a clamp does is stack two cards. The length is
    /// bounded by [`FAN_MAX`] too, so even a caller holding a whole zone's
    /// count cannot spread the fan beyond its strip.
    #[must_use]
    pub fn fan_pose(
        &self,
        pile: PileKind,
        index: usize,
        len: usize,
        under_the_pointer: bool,
    ) -> FanPose {
        let last = len.min(FAN_MAX).saturating_sub(1);
        let rung = index.min(last) as f32;
        let toward = self.camera_lies();
        // The seat's own depth axis, pointing at the table centre — the same
        // vector `lane_center` measures the lanes along — and the sideways
        // one `pile_center` stands the pile out along.
        let away = Vec2::new(self.facing.sin(), self.facing.cos());
        let side = Vec2::new(self.facing.cos(), -self.facing.sin());
        let pop = if under_the_pointer { FAN_POP } else { 0.0 };
        FanPose {
            // Away from the camera, hence the minus, and outwards for the one
            // card the pointer is on — outwards being further from the mat,
            // which is the only direction that is neither the board nor
            // another card of the fan.
            at: self.pile_center(pile) - away * (toward * FAN_STEP * rung)
                + side * (pile.side() * pop),
            lift: FAN_FLOAT + FAN_RISE * rung,
            // A card lies flat facing its owner, and tipping it about that
            // axis raises the edge furthest from the owner. That is towards
            // the camera for the near half of the ring and away from it for
            // the far half, which is why the sign is the viewer's and not the
            // seat's.
            tilt: -toward * FAN_TILT,
            yaw: FAN_YAW * (last as f32 / 2.0 - rung),
        }
    }

    /// The seat's whole footprint: the ground, plus the piles standing beside
    /// it.
    ///
    /// [`half_extent`](Self::half_extent) is the *playing* surface and has to
    /// stay that, because it is what the lanes are packed into. This is what
    /// the camera has to frame. Growing the one to mean the other would let
    /// the board spread into the strips the piles stand in, and the first
    /// wide turn would put a creature on top of the graveyard.
    #[must_use]
    pub fn footprint(&self) -> Vec2 {
        Vec2::new(
            self.half_extent.x + PILE_STRIP - self.reclaimed * 0.5,
            self.half_extent.y,
        )
    }
}

/// Where every seat sits, and how much room each one gets.
#[derive(Clone, PartialEq, Debug)]
pub struct TableLayout {
    /// Seats, index 0 = local, then clockwise in turn order.
    pub slots: Vec<SeatSlot>,
    /// Ring radii; the table is an ellipse so a wide screen is actually used.
    pub radius: Vec2,
}

/// The ring radii for a given `y`, in the shape the canvas asks for.
///
/// `x` is whatever makes the whole footprint — ground and piles together,
/// which is what [`TableLayout::extent`] reports and the camera frames — the
/// same shape as the canvas. A span taller than the canvas wastes its width,
/// a span wider wastes its height, and only a span of the same shape wastes
/// neither.
///
/// `party` is how many seats a side holds *on average*, and it divides the
/// width because a side of two reaches twice as far along itself. Without it
/// a two-headed table came out exactly twice the shape it asked for: the
/// camera fitted it by width, the four boards sat in the top half of the
/// window and the bottom half was bare felt.
///
/// The average and not the busiest side, which is what it was first written
/// as. On a table where every side is a pair the two are the same number, so
/// nothing about a two-headed game changes — but a *mixed* table has sides of
/// one and sides of two, and taking the worst of them halved the whole ring
/// for the sake of one side. Six seats as two pairs and two singles came out
/// on a ring 7.7 by 11.2, an ellipse so much taller than it is wide that four
/// of the six were pushed onto its flanks, and every board at the table
/// collapsed to 3.6 units — a third of what the same six seats get sitting
/// alone. What the ring has to carry is the whole table's demand spread over
/// the sides it has; a side that asks for more than its share takes a longer
/// stretch of its own side, and the compartments already know how to hand
/// that out.
fn ring_for(ry: f32, aspect: f32, half_depth: f32, party: f32) -> Vec2 {
    let rx = (aspect / party)
        .mul_add(ry + half_depth, -half_depth - PILE_STRIP)
        .max(half_depth);
    Vec2::new(rx, ry)
}

/// One side of the table: where a seat, or a team of them, sits on the ring.
#[derive(Clone, Copy, Debug)]
struct Side {
    /// Angle on the ring, measured from the near edge.
    angle: f32,
    /// Where that angle puts the side's middle.
    center: Vec2,
}

/// How finely the ring is walked to place sides at equal distances.
///
/// The perimeter of an ellipse has no closed form, so it is summed off a
/// polyline. At this many steps the error is under a thousandth of the
/// perimeter — a fiftieth of a millimetre on a card — and the whole walk is
/// a few hundred multiplications inside a solve that runs when a seat joins,
/// not every frame.
const RING_STEPS: usize = 256;

/// The sides of a table, in ring order starting at the local seat's own.
///
/// Spaced by **distance along the ring**, not by the angle that parameterises
/// it. Those are the same thing on a circle and nothing like it on a wide
/// ellipse: at six sides on a 19.9 × 11.2 ring the two flank sides sat 11.2
/// apart while the near one had 18.1 to its neighbour, so a table sized to
/// give the flanks a board gave the near seat half again as much and pushed
/// the camera back to frame the result. Equal distances make every side's
/// neighbourhood the same, which is what lets one ring answer for all of
/// them.
///
/// A side's `angle` is then the ring's **inward normal** there, which on an
/// ellipse is not the direction of its own centre: `(sin θ / rx, cos θ / ry)`
/// is perpendicular to the curve, and a mat laid square to anything else is a
/// mat sitting at a slight angle to the table it is drawn on.
fn sides_on(count: usize, radius: Vec2) -> Vec<Side> {
    let point = |t: f32| {
        let (sin, cos) = t.sin_cos();
        Vec2::new(-radius.x * sin, -radius.y * cos)
    };
    let mut arc = [0.0_f32; RING_STEPS + 1];
    let mut previous = point(0.0);
    for k in 1..=RING_STEPS {
        let here = point(core::f32::consts::TAU * k as f32 / RING_STEPS as f32);
        arc[k] = arc[k - 1] + here.distance(previous);
        previous = here;
    }

    let mut sides = Vec::with_capacity(count);
    let mut k = 0;
    for i in 0..count {
        // The walk only ever goes forwards, so the whole ring is read once
        // however many sides ask for a place on it.
        let want = arc[RING_STEPS] * i as f32 / count as f32;
        while k + 1 < RING_STEPS && arc[k + 1] < want {
            k += 1;
        }
        let step = arc[k + 1] - arc[k];
        let part = if step > 1e-6 {
            (want - arc[k]) / step
        } else {
            0.0
        };
        let t = core::f32::consts::TAU * (k as f32 + part) / RING_STEPS as f32;
        let (sin, cos) = t.sin_cos();
        sides.push(Side {
            angle: (sin / radius.x)
                .atan2(cos / radius.y)
                .rem_euclid(core::f32::consts::TAU),
            center: point(t),
        });
    }
    sides
}

/// The lane axis of a side, and the axis across it.
///
/// The same two vectors [`SeatSlot::pile_center`] and [`SeatSlot::lane_center`]
/// are built from, so what is measured here is the box that is drawn there.
fn axes(angle: f32) -> (Vec2, Vec2) {
    let (sin, cos) = angle.sin_cos();
    (Vec2::new(cos, -sin), Vec2::new(sin, cos))
}

/// How wide a compartment every seat at the table owns, from its own middle
/// out along its side's lane — mat, pile strips and the air around them.
///
/// One compartment for the whole table, because a table where one player's
/// board is wider than another's is a table where the wider board is the one
/// being played on. Sides differ in how many seats they hold, so a side's own
/// span is this times its party; what is equal is what a *seat* gets.
///
/// The question is about **rectangles**, and about the right axes. Two boxes
/// miss each other as soon as *some* line separates them, and for two
/// rectangles it is enough to try four: each one's lane axis and each one's
/// depth axis (CR has nothing to say here — this is the separating-axis
/// theorem). On an axis `n` the two together reach
/// `S·(kᵢ|n·alongᵢ| + kⱼ|n·alongⱼ|) + half_depth·(|n·awayᵢ| + |n·awayⱼ|)`,
/// so each axis gives a ceiling on `S` and the pair is happy with the
/// **largest** of the four. A pair that no axis can separate is a pair that
/// touches, and there the smallest ceiling is the answer.
///
/// Trying only the line joining the two middles — which is what this did
/// first — is sound but far too careful: it is one axis, and not one of the
/// four. At four seats it held every board to 10.0 units when the near board
/// could have been 13.7 without coming within a unit and a half of the seat
/// on its left, whose mat lies *across* the table from it and takes up
/// `half_depth` of the width, not its own. The cost was paid twice, because
/// the ring then grew to buy back width that was already there.
///
/// Every other side is asked, not just the two neighbours: a board wide
/// enough now reaches past its neighbour, and the pair that meets first is
/// not always the pair that sits closest.
fn compartment_half(sides: &[Side], party: &[f32], radius: Vec2, half_depth: f32) -> f32 {
    // A compartment nothing bounds still stops at the table's own reach: the
    // board it holds is then exactly `across` wide plus its pile strips,
    // which is the span `ring_for` shapes the ring against. Written as a
    // compartment it has to be the *air included*, or a duel — the one table
    // with no neighbours at all — would come out a seventh narrower than the
    // canvas it was sized for.
    let whole = (radius.x + half_depth + PILE_STRIP) / ARC_SHARE;
    let t = sides.len();
    let mut held = whole;
    for i in 0..t {
        for j in (i + 1)..t {
            let delta = sides[j].center - sides[i].center;
            let (along_i, away_i) = axes(sides[i].angle);
            let (along_j, away_j) = axes(sides[j].angle);
            let (ki, kj) = (party[i], party[j]);
            let on = |n: Vec2| {
                let gap =
                    delta.dot(n).abs() - half_depth * (n.dot(away_i).abs() + n.dot(away_j).abs());
                let reach = ki.mul_add(n.dot(along_i).abs(), kj * n.dot(along_j).abs());
                if reach < 1e-3 {
                    // Neither side grows into this axis at all: two seats
                    // facing each other across the table are as far apart at
                    // any width. The depth has already been paid above.
                    if gap > 0.0 { whole } else { 0.0 }
                } else {
                    (gap / reach).max(0.0)
                }
            };
            let best = on(along_i).max(on(away_i)).max(on(along_j)).max(on(away_j));
            held = held.min(best);
        }
    }
    held
}

/// The ring a table of these sides sits on: the tightest one that still hands
/// every seat the standard board, in the shape that suits the format.
///
/// `even` is what each side asks for, one entry per side, so its length is
/// the number of sides. `alone` says nobody at the table has an ally, which
/// is what makes a round ring worth offering — see [`ROUND_COST`].
/// `standard` is the width every seat is to be handed, which is
/// [`standard_board`]'s to say.
fn ring_that_seats(
    even: &[f32],
    aspect: f32,
    half_depth: f32,
    spread: f32,
    clear: f32,
    alone: bool,
    standard: f32,
) -> Vec2 {
    let t = even.len();
    // What a ring of this shape hands out: where the sides sit on it, and
    // half of one seat's ground inside the compartment it owns.
    let cut_for = |radius: Vec2| {
        let sides = sides_on(t, radius);
        let held = compartment_half(&sides, even, radius, half_depth);
        (sides, pod_half_width(held, radius.x + half_depth))
    };
    let narrowest = |radius: Vec2| cut_for(radius).1 * 2.0;
    // The smallest ring *of a given shape* that still hands every seat the
    // standard board. The shape is a parameter because there are two
    // candidates and they have to be compared on the same terms — each at the
    // tightest it can be, since a ring one unit wider than it needs to be is a
    // unit every card is drawn smaller for.
    let settle = |shape: &dyn Fn(f32) -> Vec2, ceiling: f32| -> Vec2 {
        let (mut lo, mut hi) = (clear, ceiling.max(clear));
        if narrowest(shape(hi)) < standard {
            // Past the cap the camera would have to pull back further than
            // `CameraRig::MAX_DISTANCE`, and a table it cannot frame slides
            // its near mats under the hand bar. Crowded tables live here:
            // they get the biggest ring that can still be seen, and their
            // lanes fan. That is what fanning is for.
            return shape(hi);
        }
        if narrowest(shape(lo)) >= standard {
            return shape(lo);
        }
        for _ in 0..24 {
            let mid = f32::midpoint(lo, hi);
            if narrowest(shape(mid)) >= standard {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        shape(hi)
    };

    let canvas = |ry: f32| ring_for(ry, aspect, half_depth, spread);
    if t < 3 && alone {
        return canvas(clear);
    }
    // The ceiling, in whichever of the two radii binds first on this canvas:
    // `x` is derived from `y` by the aspect, so a cap on `x` is a cap on `y`
    // once it is read back through the same division.
    let by_x = (MAX_RING_X + half_depth + PILE_STRIP) * spread / aspect - half_depth;
    let shaped = settle(&canvas, MAX_RING_Y.min(by_x));
    // Nobody at this table has an ally, so there is a round ring to compare
    // against: it is taken if it still seats everybody at the standard width
    // and the camera can afford to stand where it puts them. Only at three,
    // which is the one table it was ever for — see [`ROUND_COST`].
    (alone && t == 3)
        .then(|| settle(&|ry: f32| Vec2::splat(ry), MAX_RING_Y.min(MAX_RING_X)))
        .filter(|&round| narrowest(round) >= standard)
        .filter(|&round| {
            let (round, shaped) = (cut_for(round), cut_for(shaped));
            reach_of(&round.0, round.1, half_depth, aspect)
                <= reach_of(&shaped.0, shaped.1, half_depth, aspect) * ROUND_COST
        })
        .unwrap_or(shaped)
}

/// What the camera has to swallow to frame a table cut like this, measured in
/// units of canvas *height*.
///
/// The same box [`TableLayout::extent`] reports, read through the canvas so
/// two candidate rings can be compared by the one number that decides how big
/// a card is drawn: a shot is fitted by whichever of the two axes binds, so a
/// span twice as wide as the canvas costs exactly what a span twice as tall
/// does. Written here rather than off a built layout because it is asked
/// *during* the search, before there are any slots to measure.
fn reach_of(sides: &[Side], half_width: f32, half_depth: f32, aspect: f32) -> f32 {
    let mut lo = Vec2::splat(f32::INFINITY);
    let mut hi = Vec2::splat(f32::NEG_INFINITY);
    for side in sides {
        let (sin, cos) = side.angle.sin_cos();
        let (sin, cos) = (sin.abs(), cos.abs());
        let foot = Vec2::new(half_width + PILE_STRIP, half_depth);
        let half = Vec2::new(
            cos.mul_add(foot.x, sin * foot.y),
            sin.mul_add(foot.x, cos * foot.y),
        );
        lo = lo.min(side.center - half);
        hi = hi.max(side.center + half);
    }
    let span = hi - lo;
    if span.x.is_finite() {
        (span.x / aspect).max(span.y)
    } else {
        0.0
    }
}

/// How wide half of one seat's ground is, inside the compartment it owns.
///
/// [`ARC_SHARE`] of the compartment is board and pile strips; the rest is the
/// air that keeps one board from reading as part of the next. It is the same
/// margin between allies on one side as between two sides, because they are
/// the same thing to look at — the compartment is what the geometry hands
/// out, and this is what is drawn inside it.
fn pod_half_width(compartment: f32, across: f32) -> f32 {
    compartment
        .mul_add(ARC_SHARE, -PILE_STRIP)
        .clamp(CARD_WIDTH, across)
}
/// A seat at the table, and who it is allied with.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Seat {
    /// Which seat.
    pub player: PlayerId,
    /// The team it plays for, in formats that have one. Seats sharing a team
    /// share a side of the table.
    pub team: Option<u8>,
}

impl Seat {
    /// A seat playing for itself.
    #[must_use]
    pub const fn alone(player: PlayerId) -> Self {
        Self { player, team: None }
    }

    /// A seat playing for a team, if there is one.
    #[must_use]
    pub const fn on(player: PlayerId, team: Option<u8>) -> Self {
        Self { player, team }
    }
}

/// Groups seats into the sides of the table they sit on, as indices into
/// `seats`.
///
/// Allies share a side and everyone else has one to themselves. Turn order
/// decides the rest: the local seat's side is built first and is therefore
/// the one at the near edge, and the others follow in the order their first
/// member takes a turn. So a team ends up together without any seat moving
/// further round the table than it has to.
fn sides_of(seats: &[Seat]) -> Vec<Vec<usize>> {
    let mut out: Vec<Vec<usize>> = Vec::new();
    for (i, seat) in seats.iter().enumerate() {
        let ally = seat.team.and_then(|team| {
            out.iter()
                .position(|side| seats[side[0]].team == Some(team))
        });
        match ally {
            Some(side) => out[side].push(i),
            None => out.push(vec![i]),
        }
    }
    out
}

impl TableLayout {
    /// Lays out `seats` (in turn order starting with the local seat) on a ring
    /// sized for the canvas it will be seen through.
    ///
    /// Everybody plays for themselves here: each seat is a side of the table
    /// to itself. [`TableLayout::seated`] is the same thing for a format
    /// where seats are allied, and carries the rest of the documentation.
    ///
    /// # Panics
    /// Never — an empty seat list produces an empty layout.
    #[must_use]
    pub fn new(seats: &[PlayerId], aspect: f32, focus: Option<PlayerId>) -> Self {
        let alone: Vec<Seat> = seats.iter().copied().map(Seat::alone).collect();
        Self::seated(&alone, aspect, focus)
    }

    /// The same, for a table where some of the seats are allied.
    ///
    /// `aspect` is the aspect ratio of the part of the window the table is
    /// actually visible in — **not** the window's. The HUD is on top of the
    /// battlefield, not beside it, and it covers about a fifth of the screen;
    /// a layout built against the window is a layout the camera then has to
    /// fit into something else. This used to be a hard-coded `16.0 / 9.0`.
    ///
    /// The ring is sized to waste nothing. `y` is far enough out that the
    /// near and far mats clear the middle and not one unit further, and `x`
    /// is whatever makes the whole table the same shape as the canvas — a
    /// span taller than the canvas wastes its width, a span wider wastes its
    /// height, and only a span of the same shape wastes neither. The camera
    /// fits whatever comes out of here, so a unit of empty table is a unit
    /// every card is drawn smaller for. The one thing worth wasting a unit on
    /// is a table nobody has an ally at: a **free-for-all** is offered a
    /// circle instead, and takes it if it can still be afforded — see
    /// [`ROUND_COST`].
    ///
    /// What divides the ring is a **side**, not a seat: allies share one and
    /// sit along it facing the same way, which is where they sit at a real
    /// table and what a two-headed giant game is unplayable without — your
    /// partner's board is something you read as often as your own, and it was
    /// upside down across the table from you. A table of singles has as many
    /// sides as seats.
    ///
    /// `focus` optionally names an opponent whose board is being inspected;
    /// that pod is enlarged at the expense of the other seats, never of the
    /// local one.
    ///
    /// # Panics
    /// Never — an empty seat list produces an empty layout.
    #[must_use]
    pub fn seated(seats: &[Seat], aspect: f32, focus: Option<PlayerId>) -> Self {
        let n = seats.len();
        // Read before the aspect is narrowed below: the duel it asks about
        // narrows its own, and would otherwise be asked about a canvas it
        // was never going to be drawn on.
        let standard = standard_board(n, aspect);
        // A phone held upright is 0.46 and the tallest thing this has to
        // shape a table for; the floor used to sit above it, so the table was
        // built a seventh wider than the canvas it was going into and the
        // camera had to buy the difference back in distance. It ran out at
        // four seats.
        let aspect = aspect.clamp(0.45, 2.8);
        // A wide duel can spend the unused vertical canvas on taller lanes.
        // Keep the ring layouts unchanged; only the two facing seats benefit.
        let roomy = if n == 2 {
            ((aspect - 1.25) / 0.35).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let aspect = aspect * (1.0 - 0.12 * roomy);
        let half_depth = POD_DEPTH * 0.5 + 0.55 * roomy;
        let parties = sides_of(seats);
        let t = parties.len();

        // How far out the ring has to stand. Two things push it: the mats
        // have to clear the middle of the table, and each seat's share of its
        // side has to be wide enough to play on.
        let clear = half_depth + CENTRE_GAP * 0.5;
        // The second of those is a *search*, and that is why the closed form
        // this used to be is gone. It inverted `2·r·sin(π/n)·ARC_SHARE` for
        // the ring whose *arc* is `MIN_POD_WIDTH` — a closed form for a
        // quantity no seat is ever handed, since what a seat gets is measured
        // between two rectangles and then shared with its allies. It solved a
        // four-seat ring whose arc was the promised 10.0 and whose pods came
        // out 6.10: four cards where the name says seven.
        //
        // The delivered width rises with the ring and the ring is bounded, so
        // bisection answers it in a fixed twenty-four steps of arithmetic —
        // and, unlike an inversion, it asks *the same function the width is
        // read from*, which is the property that was missing.
        // How much of a compartment each seat asks for. One, unless a board
        // is being inspected: that one asks for more and the *other*
        // opponents give it up. Never the local seat — inspecting a board
        // across the table is not a reason to shrink the one being played on,
        // and the near seat used to lose 10.0 down to 8.5 at four seats every
        // time a player looked at somebody else.
        let weights: Vec<f32> = seats
            .iter()
            .map(|seat| {
                if focus == Some(seat.player) {
                    FOCUS_WEIGHT
                } else {
                    1.0
                }
            })
            .collect();
        let among: f32 = weights.iter().skip(1).sum::<f32>().max(1e-3);
        let others = n.saturating_sub(1) as f32;
        let shares: Vec<f32> = (0..n)
            .map(|i| {
                if i == 0 {
                    1.0
                } else {
                    (weights[i] / among * others).clamp(0.55, 2.0).sqrt()
                }
            })
            .collect();
        // What a side asks for is what its seats ask for together.
        let demand = |shares: &[f32]| -> Vec<f32> {
            parties
                .iter()
                .map(|party| party.iter().map(|&i| shares[i]).sum())
                .collect()
        };
        let plain: Vec<f32> = vec![1.0; n];

        let even = demand(&plain);
        // How far along itself a side reaches, in compartments, averaged over
        // the sides there are. It shapes the ring, because a side of two is
        // twice as wide as a side of one on the same table.
        let spread = n.max(1) as f32 / t.max(1) as f32;
        // Two sides of one seat each are a duel, and a duel has nothing to be
        // crowded by: both seats already have the whole table across, and
        // growing the ring would only push the camera back. Sharing a side is
        // crowding, though, so a two-headed table searches like any other.
        let alone = parties.iter().all(|party| party.len() == 1);
        let radius = ring_that_seats(&even, aspect, half_depth, spread, clear, alone, standard);
        if n == 0 {
            return Self {
                slots: Vec::new(),
                radius,
            };
        }

        // Where each side sits and how much of the table it holds — read off
        // the ring the search settled on, through the same functions the
        // search itself asked.
        //
        // The ring was solved for an even table and the compartments are cut
        // for this one. That is deliberate: a player inspecting an opponent
        // should not have the whole table pull away from them, and the bound
        // is asked again with the shares it will actually hand out, so what
        // the focus takes it takes from the table rather than from the gap
        // between two mats.
        let across = radius.x + half_depth;
        let sides = sides_on(t, radius);
        let held = compartment_half(&sides, &demand(&shares), radius, half_depth);

        // Built into the seats' own order, not the ring's: the local seat is
        // `slots[0]` wherever the sides put it, and `ring_index` is what a
        // seat's colour on the felt comes from.
        let mut slots: Vec<Option<SeatSlot>> = vec![None; n];
        for (side, party) in sides.iter().zip(&parties) {
            let (along, _) = axes(side.angle);
            // Allies stand shoulder to shoulder about the side's middle, one
            // compartment each — mat, pile strips and the air between them —
            // so neither their boards nor their graveyards meet.
            let span: f32 = party.iter().map(|&i| shares[i] * held).sum();
            let mut walked = 0.0_f32;
            for &i in party {
                let mine = shares[i] * held;
                let offset = walked + mine - span;
                walked += mine * 2.0;
                slots[i] = Some(SeatSlot {
                    player: seats[i].player,
                    ring_index: i,
                    angle: side.angle,
                    center: along.mul_add(Vec2::splat(offset), side.center),
                    // Cards face their owner: the local seat is upright, the
                    // seat opposite is rotated a half turn, and allies on one
                    // side read theirs the same way up.
                    facing: side.angle,
                    // Width answers to the focus; depth never does. A mat is
                    // as deep as three lanes of cards and no focus makes a
                    // card taller.
                    half_extent: Vec2::new(pod_half_width(mine, across), half_depth),
                    reclaimed: 0.0,
                    is_local: i == 0,
                });
            }
        }

        Self {
            slots: slots.into_iter().flatten().collect(),
            radius,
        }
    }

    /// Every corner of every seat's whole place — ground and piles — in table
    /// space, each grown by `air` on all four sides.
    ///
    /// The box is [`SeatSlot::footprint`] rather than `half_extent`, because
    /// this is what the camera frames and the piles have to be inside it.
    /// They stand outside the playing surface by design, so framing the
    /// playing surface alone would put every graveyard off the screen.
    ///
    /// This and [`extent`](Self::extent) are the same measurement at two
    /// tightnesses, and the difference is a shot. Seats sit on a ring, so the
    /// corners of the *box around them* are bare felt — at three seats the
    /// two that matter are a good four units outside anything anybody plays
    /// on — and a camera that frames the box frames that felt too. It filled
    /// 86% of the width it was given and 81% of the height, binding on
    /// neither.
    #[must_use]
    pub fn corners(&self, air: f32) -> Vec<Vec2> {
        let mut out = Vec::with_capacity(self.slots.len() * 4);
        for slot in &self.slots {
            let (sin, cos) = slot.facing.sin_cos();
            let half = slot.footprint() + Vec2::splat(air);
            for sx in [-1.0_f32, 1.0] {
                for sy in [-1.0_f32, 1.0] {
                    let local = half * Vec2::new(sx, sy);
                    out.push(
                        slot.footprint_center()
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

    /// The rectangle every seat's ground and every seat's piles fit inside,
    /// in table space, as `(min, max)`. `None` for a table with no seats.
    ///
    /// A pod's box is measured in its *own* frame — a seat on the left plays
    /// across the table, not along it — so each one is rotated by its
    /// `facing` before it is taken in.
    #[must_use]
    pub fn extent(&self) -> Option<(Vec2, Vec2)> {
        let mut bounds: Option<(Vec2, Vec2)> = None;
        for slot in &self.slots {
            let (sin, cos) = slot.facing.sin_cos();
            let (sin, cos) = (sin.abs(), cos.abs());
            let footprint = slot.footprint();
            let half = Vec2::new(
                cos.mul_add(footprint.x, sin * footprint.y),
                sin.mul_add(footprint.x, cos * footprint.y),
            );
            let (lo, hi) = (
                slot.footprint_center() - half,
                slot.footprint_center() + half,
            );
            bounds = Some(match bounds {
                None => (lo, hi),
                Some((min, max)) => (min.min(lo), max.max(hi)),
            });
        }
        bounds
    }

    /// The local seat's slot.
    #[must_use]
    pub fn local(&self) -> Option<&SeatSlot> {
        self.slots.first()
    }

    /// The slot belonging to a seat.
    #[must_use]
    pub fn slot(&self, player: PlayerId) -> Option<&SeatSlot> {
        self.slots.iter().find(|s| s.player == player)
    }
}

/// How deep one seat's ground is: the seat's ledge, then three lanes with a
/// card standing in each, and enough air that a lifted card does not overlap
/// the row behind it.
///
/// [`crate::tabletop::MAT_LEDGE`] is *added* rather than taken out of the
/// three lanes, so a lane is exactly as tall as it was before the seat bar
/// existed — the shelf is furniture and a lane is where a card stands.
///
/// A **constant**, and that is the change that made the board fill the
/// screen. While the depth came off `radius.y`, it grew with the ring — so a
/// table laid out for eight seats gave every one of them a deeper mat than a
/// duel did, and a duel, which is what almost every game actually is, got the
/// shallowest board of the lot. A card is the same size at every table, so
/// the ground a card stands on is too.
pub const POD_DEPTH: f32 = CARD_HEIGHT * 3.0 * 1.18 + crate::tabletop::MAT_LEDGE;

/// Clear table kept between the mats, for the medallion and the light pool.
///
/// It was seventeen units. The two mats were 4.4 deep and 17 apart, so four
/// fifths of a duel's screen was empty table — and because the camera fits
/// whatever span the layout reports, every one of those units was a unit the
/// cards were drawn smaller for.
pub const CENTRE_GAP: f32 = 3.4;

/// The width every seat at a table of `seats` is handed: at three and up,
/// what a duel on the same canvas hands each of its two.
///
/// The owner's word (#264, 24.09.): *„Wenn es mehr als 2 Spieler sind, sollen
/// alle Tische so breit sein in etwa wie im 1vs1 Modus, aber dafür wird der
/// Tisch sehr groß und die Kamera zoomt raus."* A ring used to grow only until
/// its narrowest seat had [`MIN_POD_WIDTH`], twelve units, where a duel on a
/// laptop hands each seat 27.4: a player's board at four seats was less than
/// half of the one the same player had in a duel. Now the ring grows until
/// every seat has a duel's board, the table grows with it, and the camera
/// stands back to frame it — on that laptop 44.7 units of distance for a
/// duel, 95 for four seats, 186 for eight, where it was 59 and 81. What a
/// player gives up is how big the whole table is drawn; what they get back
/// is a board as wide as a duel's, one press away on the players' strip
/// (`hud::ledge::players` in the client), which is what the owner asked for
/// the strip to be.
///
/// Asked of [`TableLayout::seated`] itself rather than restated, because a
/// duel's width is not a constant: a duel spends a wide canvas on a wider
/// ring and taller lanes, so it is 27.4 on a laptop, 20.7 at 1.6 and 38.9 on
/// an ultrawide, and "as wide as a duel" is only true on the canvas the duel
/// would have been drawn on. [`MIN_POD_WIDTH`] stays under it as a floor,
/// which is what a phone held upright is handed — its duel is 11.9.
fn standard_board(seats: usize, aspect: f32) -> f32 {
    if seats <= 2 {
        return MIN_POD_WIDTH;
    }
    let duel = [Seat::alone(PlayerId::new(0)), Seat::alone(PlayerId::new(1))];
    let duel = TableLayout::seated(&duel, aspect, None);
    (duel.slots[0].half_extent.x * 2.0).max(MIN_POD_WIDTH)
}

/// The narrowest a seat's lane is allowed to get before the ring grows to
/// make room — eight cards laid side by side — and since #264 the floor under
/// [`standard_board`] rather than the standard itself.
///
/// This is what stops a big table from solving itself by squeezing: more
/// seats get a bigger ring, not a strip of ground too narrow to read. It is
/// the width a pod is *handed*, which is a correction: it used to size the
/// arc, and a pod was then given that arc less a pile strip on each side, so
/// what the name promised and what a player got were four units apart.
///
/// Twelve was chosen as the standard, against a camera that could stand no
/// further back than forty-six units of the lens it then had: five seats was
/// the table with least room, and thirteen would have left it none. The
/// owner then asked for every seat to be as wide as a duel's and for the
/// camera to zoom out to make it so, and it was the ceiling that moved
/// ([`MAX_RING_X`]), not this.
const MIN_POD_WIDTH: f32 = 12.0;

/// The furthest out the ring may stand, whatever the seat count asks for.
///
/// The camera frames whatever [`TableLayout::extent`] reports and clamps at
/// `CameraRig::MAX_DISTANCE`; past that the far edge stays pinned and the
/// near mats slide under the hand bar. So the search that grows the ring
/// needs a ceiling, and it takes **two**, because the two radii are what the
/// camera sees and only one of them is being searched over.
///
/// Set (#264) so that eight seats, the most a table is dealt, get
/// [`standard_board`] on every canvas the layout is shaped for: the widest,
/// 2.8 — an ultrawide, or a phone on its side — needs an `x` of 90.3, and
/// the tallest, 0.46, a phone held upright, a `y` of 33.9. It was 21.0 × 11.2 when
/// the standard was twelve units and the camera stopped at forty-six, and
/// six seats and up lived on it, fanned. A table that wants more room than
/// this does not get it — it gets fanned lanes, which is what fanning is
/// for — and nothing deals one: the gateway seats eight.
const MAX_RING_X: f32 = 92.0;
/// The same ceiling on the other radius; see [`MAX_RING_X`].
const MAX_RING_Y: f32 = 35.0;

/// How much further back the camera may be pushed to seat a free-for-all
/// round, as a multiple of what the same table costs on a ring shaped to the
/// canvas.
///
/// A ring shaped to the canvas wastes nothing, and for two sides or four it
/// also seats them where anyone would sit: opposite each other, or on the
/// four points of a diamond. **Three** is where it comes apart. Equal
/// distances along a 12.0 × 5.7 ellipse put the two opponents at 150° and
/// 210°, which is a mat's width apart at the top of the table with their
/// inner corners nearly touching — a gable, and the same silhouette a 2v1
/// draws, where two allies really do sit shoulder to shoulder. A format is
/// not something a player should have to read off the life totals.
///
/// So a free-for-all is offered a circle, and takes it if the camera can
/// afford it. At three seats it cost a quarter when this was written: 22.3
/// units of reach against 17.9, every card a quarter smaller, and about two
/// fifths of the screen's width left bare — which is what a round table is
/// worth. At four it cost four fifths, for an arrangement that was already
/// a diamond, and at five and six the circle was past the ring's ceiling
/// before it had handed anybody a board. This is the line between those,
/// and it is deliberately nearer the first: 1.25 is bought, 1.43 is not.
///
/// That quarter was the *old* canvas — 110 logical pixels of tab strip and
/// phase rail off the top of the window, an aspect of 2.25, and three seats
/// on an ellipse of 12.95 × 4.99 because the circle asked for more than this
/// allows. With those gone the canvas is 1.97 and the circle costs 9.3%:
/// 8.28 × 8.28, taken, and the one seat count where a taller window made a
/// board smaller (34.9 → 32.0 pixels a table unit). It is bought, not lost.
/// `camera_tests::three_seats_playing_for_themselves_sit_on_a_circle` is
/// what says so out loud, because this filter is one window shape away from
/// flipping back and nothing else at the table would notice.
///
/// Offered at three and nowhere else since #264. Four and up were refused
/// by this price or by the ceiling, and when the ceiling rose to hand every
/// seat a duel's board ([`standard_board`]) a five-seat circle came in under
/// the price — with a seat at 72°, whose lean of 0.31 is neither a flank nor
/// across the table ([`SIDE_SEAT_TILT`]), so the camera's answer at that
/// seat would have been the tolerance's and not the geometry's. Three at a
/// duel's width sit on a circle of 13.5 on the laptop, and it is still taken.
const ROUND_COST: f32 = 1.3;

/// How much of the arc between two neighbours a mat may claim. The rest is
/// the gap that keeps them from touching.
const ARC_SHARE: f32 = 0.86;

/// A focused opponent counts as this many ordinary seats.
const FOCUS_WEIGHT: f32 = 2.6;

/// The pitch after a merged card whose count badge stands beside it
/// ([`BadgePlace::Beside`](crate::cardplate::BadgePlace::Beside)), or a
/// host whose mark may (#305): a whole cell, the room a card turns in.
///
/// Beside a card the badge hangs [`BADGE_REACH`](crate::cardplate::BADGE_REACH)
/// off its right edge, and when it taps the badge turns with it to lie under
/// its right end. The owner's rule is that it lies on no other card's print
/// (25.09), so the card after it may not reach into its cell: that card,
/// tapped, starts half a span from its own centre, and the badge ends short
/// of where it does. Over a card
/// ([`BadgePlace::Above`](crate::cardplate::BadgePlace::Above)) no card of
/// the row reaches the badge, and nothing is held, except before a card with
/// cards tucked under it, whose names peek out where the badge stands.
pub const HELD_PITCH: f32 = CARD_SPAN;
const _: () =
    assert!(CARD_SPAN * 0.5 + CARD_WIDTH * 0.5 + crate::cardplate::BADGE_REACH <= HELD_PITCH);

/// How far each card under a merged pile steps out from the one above it,
/// in card widths: about eight hundredths of a card (the owner, 25.09, who
/// had not noticed the 0.045 it was).
///
/// To the **left**, all of them (the owner, 25.09: "sollte lieber nach
/// links gehen"), in the seat's own frame whether or not the pile is tapped,
/// so a pile is a staircase of edges on the side away from its count badge.
/// Sideways only: a slab standing out at the top would reach towards the
/// band the seat bar writes on, and one at the bottom towards the row behind.
pub const PILE_JOG: f32 = 0.08;

/// The most cards a merged pile shows under its top card: "bis 5 reichen
/// aus" (the owner, 25.09). The count badge says how many there are.
pub const PILE_SLABS: usize = 5;

/// How far the cards under a merged pile of `count` reach out left of its
/// top card, in table units: nothing for a lone card.
#[must_use]
pub fn pile_reach(count: usize) -> f32 {
    count.saturating_sub(1).min(PILE_SLABS) as f32 * PILE_JOG * CARD_WIDTH
}

/// What lies between a card of a row and the next one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Gap {
    /// Nothing: the two fan with the rest of the row.
    Free,
    /// A whole cell, [`HELD_PITCH`]: the card before it may wear a badge
    /// (`board::Lane::gaps`) where the next card would reach it.
    Held,
    /// The two stand in different sections of their row (#263): the card
    /// before it lies whole, and air follows it ([`section_air`]). Wider
    /// than a held gap, so a merged card at the end of a section needs no
    /// more.
    Section,
}

/// The air between two sections of a row with room to spare (#263): four
/// of a comfortable row's gaps, about half a card.
pub const SECTION_AIR: f32 = 4.0 * CARD_GAP;

/// The air a section boundary keeps however tightly its row fans: two gaps.
///
/// Never none: three sections that closed up would read as one row on
/// exactly the crowded board they are there to sort.
pub const SECTION_AIR_MIN: f32 = 2.0 * CARD_GAP;

/// The air after a section when the row fans at `pitch`: [`SECTION_AIR`] at
/// a comfortable pitch, falling in step with the pitch to
/// [`SECTION_AIR_MIN`] at the tightest fan, so a filling row closes up
/// smoothly instead of in a jump where it starts to fan.
#[must_use]
pub fn section_air(pitch: f32) -> f32 {
    let min_pitch = CARD_WIDTH * MIN_VISIBLE_FRACTION;
    let comfortable = CARD_SPAN + CARD_GAP;
    let share = ((pitch - min_pitch) / (comfortable - min_pitch)).clamp(0.0, 1.0);
    SECTION_AIR_MIN + (SECTION_AIR - SECTION_AIR_MIN) * share
}

/// How a lane packed its cards.
#[derive(Clone, PartialEq, Debug)]
pub struct LanePacking {
    /// Horizontal offsets from the lane centre, left to right, of the whole
    /// row: a row that scrolls runs longer than its lane, and
    /// [`window`](Self::window) says which of them are shown and where.
    pub offsets: Vec<f32>,
    /// Distance between successive card centres where nothing holds the gap
    /// open.
    pub pitch: f32,
    /// Whether the cells are tighter than a card's own span, so cards can
    /// overlap.
    ///
    /// "Can", not "do": between [`CARD_WIDTH`] and [`CARD_SPAN`] an untapped
    /// row still has air in it and a tapped one does not, and a lane cannot
    /// know which of its cards will be turned.
    pub fanned: bool,
    /// Whether even a fan cannot show every card legibly, with every held
    /// gap held: the row scrolls, and shows the run of whole cards
    /// [`window`](Self::window) picks.
    pub overflowing: bool,
    /// The lane's usable width, which a scrolled row's window is cut to.
    usable: f32,
    /// How far each card's pile reaches out on its left ([`pile_reach`]),
    /// which the row holds room for as it holds a card.
    reach: Vec<f32>,
}

/// The run of a row's cards that is shown, and how far their offsets move to
/// stand in the lane.
#[derive(Clone, PartialEq, Debug)]
pub struct RowWindow {
    /// The cards shown; the rest of the row is not drawn.
    pub shown: std::ops::Range<usize>,
    /// Added to every offset: a scrolled row's shown run starts at the
    /// lane's left edge.
    pub shift: f32,
}

impl LanePacking {
    /// The cards a row shows when its first shown card is `first`: every
    /// card of a row that fits, else the longest run of whole cards from
    /// `first` that fits the lane, `first` pulled back so the run never
    /// stops short of the row's end.
    #[must_use]
    pub fn window(&self, first: usize) -> RowWindow {
        let n = self.offsets.len();
        if !self.overflowing {
            return RowWindow {
                shown: 0..n,
                shift: 0.0,
            };
        }
        let first = first.min(self.last_first());
        let end = (first..n)
            .take_while(|&i| self.fits(first, i))
            .last()
            .unwrap_or(first)
            + 1;
        RowWindow {
            shown: first..end,
            shift: -self.usable * 0.5 + CARD_SPAN * 0.5 + self.reach[first] - self.offsets[first],
        }
    }

    /// Whether the run of cards `from..=to` fits the lane, the pile under
    /// the first of them too.
    fn fits(&self, from: usize, to: usize) -> bool {
        self.offsets[to] - self.offsets[from] + CARD_SPAN + self.reach[from] <= self.usable + 1e-4
    }

    /// The first card a row may show and still reach its last.
    #[must_use]
    pub fn last_first(&self) -> usize {
        let n = self.offsets.len();
        if n == 0 {
            return 0;
        }
        (0..n).find(|&f| self.fits(f, n - 1)).unwrap_or(n - 1)
    }

    /// The first shown card that brings card `index` into view, moving the
    /// window as little as it can: `first` itself if it is shown already.
    #[must_use]
    pub fn reveal(&self, first: usize, index: usize) -> usize {
        let window = self.window(first);
        if window.shown.contains(&index) || index >= self.offsets.len() {
            return window.shown.start;
        }
        if index < window.shown.start {
            return index;
        }
        (0..=index).find(|&f| self.fits(f, index)).unwrap_or(index)
    }

    /// Whether the card after `index` can lie over it: the next card is
    /// shown and the gap to it, less the pile under it, is tighter than a
    /// card's span.
    #[must_use]
    pub fn covered(&self, index: usize, window: &RowWindow) -> bool {
        window.shown.contains(&(index + 1))
            && self.offsets[index + 1] - self.reach[index + 1] - self.offsets[index]
                < CARD_SPAN - 1e-4
    }

    /// What the row leaves card `index`'s plate ([`PlateRoom`]), `tapped`
    /// and `staged` saying which of the row's cards are turned and which
    /// have stepped out of it into combat ([`STAGE_STEP`]).
    ///
    /// Where the next shown card begins, its pile included, else the lane's
    /// end; and the stretch of air under the card that no shown card's
    /// print reaches into. An untapped card in the row reaches under a
    /// tapped one, a tapped one stays out, and a card that stepped forward
    /// has left the air of the row behind it: a staged card's own air is in
    /// the row, where every card that stayed reaches, and a staged untapped
    /// card beside it too.
    ///
    /// # Panics
    ///
    /// If `tapped` or `staged` do not name the row's cards.
    #[must_use]
    pub fn plate_room(
        &self,
        index: usize,
        window: &RowWindow,
        tapped: &[bool],
        staged: &[bool],
    ) -> PlateRoom {
        let n = self.offsets.len();
        assert!(
            tapped.len() == n && staged.len() == n,
            "a state for every card"
        );
        let half = |j: usize| {
            if tapped[j] {
                CARD_SPAN * 0.5
            } else {
                CARD_WIDTH * 0.5
            }
        };
        let reaches = |j: usize| {
            if staged[index] {
                !staged[j] || !tapped[j]
            } else {
                !staged[j] && !tapped[j]
            }
        };
        let own = self.offsets[index] - CARD_WIDTH * 0.5;
        let end = self.usable * 0.5 - window.shift;
        let start = -self.usable * 0.5 - window.shift;
        let left_of = |j: usize| self.offsets[j] - half(j) - self.reach[j];
        let next = index + 1;
        let right = if window.shown.contains(&next) {
            left_of(next)
        } else {
            end
        };
        let after = window
            .shown
            .clone()
            .filter(|&j| j > index && reaches(j))
            .map(left_of)
            .fold(end, f32::min);
        let before = window
            .shown
            .clone()
            .filter(|&j| j < index && reaches(j))
            .map(|j| self.offsets[j] + half(j))
            .fold(start, f32::max);
        PlateRoom {
            right: (right - own) / CARD_WIDTH,
            below: [(before - own) / CARD_WIDTH, (after - own) / CARD_WIDTH],
        }
    }
}

/// Packs `count` cards into a lane `width` units wide, none of them merged.
#[must_use]
pub fn pack_lane(count: usize, width: f32) -> LanePacking {
    pack_row(&vec![false; count], width)
}

/// Packs a row into a lane `width` units wide, holding the gap after each
/// card `held` names open at [`HELD_PITCH`]: [`pack_gaps`] for a row of one
/// section.
#[must_use]
pub fn pack_row(held: &[bool], width: f32) -> LanePacking {
    let gaps: Vec<Gap> = held
        .iter()
        .map(|&h| if h { Gap::Held } else { Gap::Free })
        .collect();
    pack_gaps(&gaps, &vec![0.0; held.len()], width)
}

/// Packs a row into a lane `width` units wide, `after[i]` saying what lies
/// between card `i` and the next (the last card's entry is not read) and
/// `reach[i]` how far the pile under card `i` stands out on its left
/// ([`pile_reach`]), which the row holds as room of its own: a pile never
/// lies over more of the card before it than that card's own fan allows.
///
/// Cards keep their size and start overlapping once they no longer fit, the
/// way a physical player fans a row. Shrinking instead would trade a readable
/// board for an unreadable one at exactly the moment the board matters most.
/// A fan is only ever of cards over cards within a section: a held gap stays
/// whole, a section boundary keeps its card whole and some air after it, and
/// a row that cannot keep them and still fan legibly scrolls instead
/// (`overflowing`), rather than run past its lane into the piles beside it.
///
/// # Panics
///
/// If `after` and `reach` do not name the same cards.
#[must_use]
pub fn pack_gaps(after: &[Gap], reach: &[f32], width: f32) -> LanePacking {
    assert_eq!(after.len(), reach.len(), "a gap and a reach for every card");
    let count = after.len();
    let usable = width.max(CARD_SPAN);
    let reached: f32 = reach.iter().sum();
    if count <= 1 {
        return LanePacking {
            offsets: vec![reached * 0.5; count],
            pitch: 0.0,
            fanned: false,
            overflowing: false,
            usable,
            reach: reach.to_vec(),
        };
    }

    let gaps = &after[..count - 1];
    let tally = |kind: Gap| gaps.iter().filter(|&&g| g == kind).count() as f32;
    let (free, holds, sections) = (tally(Gap::Free), tally(Gap::Held), tally(Gap::Section));
    let comfortable_pitch = CARD_SPAN + CARD_GAP;
    let min_pitch = CARD_WIDTH * MIN_VISIBLE_FRACTION;
    let comfortable_span = CARD_SPAN
        + reached
        + (free + holds) * comfortable_pitch
        + sections * (CARD_SPAN + SECTION_AIR);

    let (pitch, fanned, overflowing) = if comfortable_span <= usable {
        (comfortable_pitch, false, false)
    } else {
        // Every fanned step is linear in the pitch: a free one is the pitch,
        // a held one does not move, and a section's air follows the pitch
        // down (`section_air`). So the row's span is too, and the pitch that
        // fills the lane is one division.
        let slope = (SECTION_AIR - SECTION_AIR_MIN) / (comfortable_pitch - min_pitch);
        let fixed = CARD_SPAN
            + reached
            + holds * HELD_PITCH
            + sections * (CARD_SPAN + SECTION_AIR_MIN - slope * min_pitch);
        let per_pitch = free + sections * slope;
        if per_pitch > 0.0 {
            let pitch = ((usable - fixed) / per_pitch).min(comfortable_pitch);
            (pitch.max(min_pitch), true, pitch < min_pitch)
        } else {
            (HELD_PITCH, true, usable < fixed)
        }
    };

    // A pile's reach widens the step before it, so the card it lies over
    // keeps what the step would have shown of it.
    let steps: Vec<f32> = gaps
        .iter()
        .zip(&reach[1..])
        .map(|(gap, &reach)| {
            reach
                + match (gap, fanned) {
                    (Gap::Section, false) => CARD_SPAN + SECTION_AIR,
                    (_, false) => comfortable_pitch,
                    (Gap::Free, true) => pitch,
                    (Gap::Held, true) => HELD_PITCH,
                    (Gap::Section, true) => CARD_SPAN + section_air(pitch),
                }
        })
        .collect();
    // The row is centred as a whole: the first card's pile is part of it.
    let span: f32 = steps.iter().sum::<f32>() - reach[0];
    let mut offsets = Vec::with_capacity(count);
    let mut at = -span * 0.5;
    offsets.push(at);
    for step in steps {
        at += step;
        offsets.push(at);
    }

    LanePacking {
        offsets,
        pitch,
        fanned,
        overflowing,
        usable,
        reach: reach.to_vec(),
    }
}

#[cfg(test)]
mod tests;

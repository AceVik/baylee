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
/// The room one card needs along a lane, however it is turned.
///
/// A card taps by rotating a quarter turn about its own centre (CR 701.21),
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
/// How much of a card must stay visible when a lane fans.
///
/// Below roughly a quarter of the card the name and the power/toughness box are
/// both gone, and the fan stops carrying information — that is the point where
/// the board model should group instead.
pub const MIN_VISIBLE_FRACTION: f32 = 0.26;

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
/// The owner's number. A fan is a *glance* — the click-to-browse panel is
/// where a whole zone is read — and seven is about where a spread stops being
/// countable at one look. It stands here with the rest of the fan's shape and
/// is reached as [`crate::ZonePile::FAN_MAX`] everywhere else.
pub const FAN_MAX: usize = 7;

/// How high the lowest card of a hover fan floats above the pile.
///
/// The fan opens **upwards**, and that is forced rather than chosen. Seven
/// cards laid flat and spread sideways want `CARD_WIDTH + 6·visible` inside a
/// strip [`PILE_STRIP`] wide, which leaves 0.16 of a card showing — a border
/// stripe, not a glance — and anything wider spills onto the mat, where a
/// card reads as a permanent in play. That is the one ambiguity a graveyard
/// may never have. Height has no such competition: nothing else on this table
/// stands or floats, so a card in the air is unmistakably not in play.
pub const FAN_FLOAT: f32 = 0.18;

/// How much higher each card of the fan stands than the one under it.
///
/// Seven cards reach `FAN_FLOAT + 6·FAN_RISE` = 1.50 — about a card's height
/// off the felt, and still well under the camera.
pub const FAN_RISE: f32 = 0.22;

/// How far each card of the fan steps towards the camera along the seat's own
/// depth axis.
///
/// Along that axis and never across it: the sideways offset stays exactly
/// [`PILE_REACH`], because across is where the mat is. The step is what makes
/// a fan out of a column — seven cards at one place are one card with six
/// hidden under it — and it passes *over* whichever pile stands next in the
/// column, which at this height reads as held up rather than as lying on it.
pub const FAN_STEP: f32 = 0.12;

/// How far the fan's cards are tipped up to face the camera, in radians.
///
/// About the seat's own horizontal axis, which is the axis a card already
/// lies flat on, so the card stays the right way up for its owner. A seat at
/// the *side* of the ring gets very little from it — that axis runs across
/// the screen there — and its fan is read by lift and overlap alone, the same
/// way [`SeatSlot::ledge_is_outer`] has no "above" to offer a side seat.
pub const FAN_TILT: f32 = 0.55;

/// How far each card of the fan is turned in its own plane, in radians.
///
/// Measured from the middle of the fan, so its two ends are turned
/// `±3·FAN_YAW` opposite ways and it reads as a hand of cards rather than as
/// a staircase. It is the only curve in the shape: the cards themselves stand
/// on a straight line, because what the eye reads is the lift.
pub const FAN_YAW: f32 = 0.035;

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

/// How far a seat has to lean past the side of the ring before its bar
/// changes edges.
///
/// [`SeatSlot::ledge_is_outer`] is a comparison against zero and a seat at
/// the exact side of the ring is a tie, which in floating point is not a
/// tie at all: `cos(FRAC_PI_2)` is -4.4e-8 and `cos(3·FRAC_PI_2)` is
/// +1.2e-8, so the left flank of a four-seat table would take one edge and
/// the right flank the other. Nothing about the two seats differs, and the
/// answer has to be the same for both.
///
/// It is generous — about 4½° — because a side is placed by walking a
/// polyline of [`RING_STEPS`] steps and lands *near* the side rather than on
/// it, and because there is nothing to lose: the seat closest to a side that
/// is genuinely across the table leans four times this far.
const SIDE_SEAT_TILT: f32 = 0.08;

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
    /// Whether this is the viewing player's own seat.
    pub is_local: bool,
}

impl SeatSlot {
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
    /// What the mat has left once the ledge has taken its share, so a lane is
    /// a card tall at every table however the shelf is sized.
    #[must_use]
    pub fn lane_height(&self) -> f32 {
        (self.mat_depth() - crate::tabletop::MAT_LEDGE) / LaneKind::ALL.len() as f32
    }

    /// Which of the mat's two long edges this seat's shelf is written on.
    ///
    /// `true` for the **outer** edge — the one away from the middle of the
    /// table, behind the land row — and `false` for the centre-facing edge.
    ///
    /// The rule is one sentence and it is the viewer's, not the seat's: a
    /// bar is drawn *above* the board it describes, on the screen the local
    /// player is looking at. For the local seat and its near-side
    /// neighbours the edge that reads as "above" is the centre-facing one;
    /// for a seat across the table it is the outer one, because that seat's
    /// board is drawn upside-down from here and its centre-facing edge is at
    /// the bottom of it.
    ///
    /// This was the other way round once — the centre-facing edge for
    /// **every** seat, so a bar always stood between its owner's board and
    /// the hearth. That is the reading from each seat's own chair, and it is
    /// coherent; it is not what anybody sees. Two seats put their bars
    /// back-to-back across the middle of the table and the opponent's sat
    /// under their creatures, which is not "above the battlefield line" for
    /// the one person at the table with a screen.
    ///
    /// `away.y` is the whole test. Table `+y` is away from the camera, so a
    /// seat whose inward normal points up the table has its centre-facing
    /// edge higher on screen. A seat exactly at the side of the ring is a
    /// tie — its mat runs up and down the screen and neither edge is above
    /// anything — and keeps the centre-facing edge, which is the one nearer
    /// the hearth and the one it had before.
    #[must_use]
    pub fn ledge_is_outer(&self) -> bool {
        // With a **tolerance**, and it is load-bearing rather than tidy.
        // `cos(FRAC_PI_2)` is -4.4e-8 in f32 and `cos(3·FRAC_PI_2)` is
        // +1.2e-8, so a bare `< 0.0` sends the left side seat of a four-seat
        // table to one edge and the right one to the other; and `sides_on`
        // places a side by walking a 256-step polyline, so the two are not
        // at 90° to begin with. A side seat has no "above" and both of them
        // have to make the same choice.
        self.facing.cos() < -SIDE_SEAT_TILT
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
        let front = if self.ledge_is_outer() {
            0.0
        } else {
            crate::tabletop::MAT_LEDGE
        };
        let offset_from_front = front + (index + 0.5) * h - self.half_extent.y;
        let away = Vec2::new(self.facing.sin(), self.facing.cos());
        self.center - away * offset_from_front
    }

    /// The four corners of this seat's ledge, in table space.
    ///
    /// The band along one long edge of the mat that the seat's bar is
    /// written on. This is what the renderer projects to find where the ink
    /// goes, and it is the *only* thing it needs: the bar is one screen-space
    /// node pinned to this rectangle's projection, never a world-space
    /// object, because there is no text on the 3D table.
    ///
    /// Whichever of the mat's two long edges [`ledge_is_outer`] names, so
    /// that every bar at the table is drawn above the board it describes.
    ///
    /// Ordered as the seat itself would read them: the two corners on that
    /// outside edge first, left then right in the *seat's* frame, then the
    /// two that meet the lane behind it, right then left. So the four are a
    /// loop, and the first long edge runs the seat's own left to right at
    /// either end of the mat — which is what lets
    /// [`Shelf::of`](crate::seatbar) take the tilt off it without caring
    /// which edge it got.
    ///
    /// [`ledge_is_outer`]: Self::ledge_is_outer
    #[must_use]
    pub fn ledge_corners(&self) -> [Vec2; 4] {
        let away = Vec2::new(self.facing.sin(), self.facing.cos());
        let side = Vec2::new(self.facing.cos(), -self.facing.sin());
        // `away` points at the middle of the table, so the shelf is measured
        // along it or against it. Everything else about the rectangle — its
        // length, its depth, the order of its corners — is the same either
        // way, which is why this is a sign and not a second branch.
        let reach = if self.ledge_is_outer() { -1.0 } else { 1.0 };
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
    /// `away.y` is the whole test, as it is in
    /// [`ledge_is_outer`](Self::ledge_is_outer), and for the same reason:
    /// table `+y` runs away from the camera, so a seat whose inward normal
    /// points up the table has the camera behind it and a seat across the
    /// table has the camera in front. Same tolerance too, because the two
    /// flanks of a ring have to answer alike.
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
    /// [`crate::ZonePile::fan`] — and `len` is how many cards the fan draws.
    /// The rung it stands on is therefore `len - 1 - index`, so the top card
    /// is the highest and the nearest the camera: it is the card a player is
    /// looking for, and the older ones recede underneath it.
    ///
    /// An `index` at or past `len` is clamped rather than refused; a fan is a
    /// drawing and the worst a clamp does is stack two cards.
    #[must_use]
    pub fn fan_pose(&self, pile: PileKind, index: usize, len: usize) -> FanPose {
        let last = len.saturating_sub(1);
        let rung = (last - index.min(last)) as f32;
        let toward = self.camera_lies();
        // The seat's own depth axis, pointing at the table centre — the same
        // vector `lane_center` measures the lanes along.
        let away = Vec2::new(self.facing.sin(), self.facing.cos());
        FanPose {
            at: self.pile_center(pile) + away * (toward * FAN_STEP * rung),
            lift: FAN_FLOAT + FAN_RISE * rung,
            // A card lies flat facing its owner, and tipping it about that
            // axis raises the edge furthest from the owner. That is towards
            // the camera for the near half of the ring and away from it for
            // the far half, which is why the sign is the viewer's and not the
            // seat's — and why it is the *opposite* of the step's: the step
            // walks along the axis, the tilt turns about it.
            tilt: -toward * FAN_TILT,
            yaw: FAN_YAW * (rung - last as f32 / 2.0),
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
        Vec2::new(self.half_extent.x + PILE_STRIP, self.half_extent.y)
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
fn ring_that_seats(
    even: &[f32],
    aspect: f32,
    half_depth: f32,
    spread: f32,
    clear: f32,
    alone: bool,
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
        if narrowest(shape(hi)) < MIN_POD_WIDTH {
            // Past the cap the camera would have to pull back further than
            // `CameraRig::MAX_DISTANCE`, and a table it cannot frame slides
            // its near mats under the hand bar. Crowded tables live here:
            // they get the biggest ring that can still be seen, and their
            // lanes fan. That is what fanning is for.
            return shape(hi);
        }
        if narrowest(shape(lo)) >= MIN_POD_WIDTH {
            return shape(lo);
        }
        for _ in 0..24 {
            let mid = f32::midpoint(lo, hi);
            if narrowest(shape(mid)) >= MIN_POD_WIDTH {
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
    // and the camera can afford to stand where it puts them.
    alone
        .then(|| settle(&|ry: f32| Vec2::splat(ry), MAX_RING_Y.min(MAX_RING_X)))
        .filter(|&round| narrowest(round) >= MIN_POD_WIDTH)
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
        // A phone held upright is 0.46 and the tallest thing this has to
        // shape a table for; the floor used to sit above it, so the table was
        // built a seventh wider than the canvas it was going into and the
        // camera had to buy the difference back in distance. It ran out at
        // four seats.
        let aspect = aspect.clamp(0.45, 2.8);
        let half_depth = POD_DEPTH * 0.5;
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
        let radius = ring_that_seats(&even, aspect, half_depth, spread, clear, alone);
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
            let (lo, hi) = (slot.center - half, slot.center + half);
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

/// The narrowest a seat's lane is allowed to get before the ring grows to
/// make room — eight cards laid side by side.
///
/// This is what stops a big table from solving itself by squeezing: more
/// seats get a bigger ring, not a strip of ground too narrow to read. It is
/// the width a pod is *handed*, which is a correction: it used to size the
/// arc, and a pod was then given that arc less a pile strip on each side, so
/// what the name promised and what a player got were four units apart.
///
/// Ten was too careful once the widths were measured against the right axes.
/// A board is worth what it is worth *against the table around it*, and the
/// number that says so is its share of the span the camera frames: at four
/// seats ten units was 38% of it and twelve is 42%, for two units of camera
/// distance out of the forty-six there are.
///
/// What sets the ceiling on it is not four seats but **five**, which is the
/// table the camera has least room for — at the ring's own ceiling its mats
/// are wider than a six-seat table's and its span is the largest there is.
/// Five costs 39.4 units of distance at a minimum of ten, 44.1 at twelve and
/// 45.8 at thirteen against a `MAX_DISTANCE` of 46, so thirteen would buy a
/// four-seat board two per cent of the screen and leave a five-seat table
/// with no margin at all. Above that it stops paying twice over: seventeen
/// buys 50% at thirty-one units, and every card at the table is drawn a
/// third smaller to read a row nobody fills.
const MIN_POD_WIDTH: f32 = 12.0;

/// The furthest out the ring may stand, whatever the seat count asks for.
///
/// The camera frames whatever [`TableLayout::extent`] reports and clamps at
/// `CameraRig::MAX_DISTANCE`; past that the far edge stays pinned and the
/// near mats slide under the hand bar. So the search that grows the ring
/// needs a ceiling, and it takes **two**, because the two radii are what the
/// camera sees and only one of them is being searched over.
///
/// Measured through `CameraRig::home` on a 1728×1052 window with the duel
/// HUD, whose free area is about 2.01 wide to 1 tall: the camera runs a
/// little under 1.8 units of distance per unit of `x`, so a ring at `x` 23.1
/// needs 46 — exactly the clamp — while 21.0 asks about 42 and leaves the
/// margin standing. `y` binds instead at a narrow canvas, where `x` is small
/// and the table is deep rather than wide.
///
/// A table that wants more room than this does not get it — it gets fanned
/// lanes, which is what fanning is for. Six seats and up live here.
const MAX_RING_X: f32 = 21.0;
/// The same ceiling on the other radius; see [`MAX_RING_X`].
const MAX_RING_Y: f32 = 11.2;

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
/// worth. At four it costs four fifths, for an arrangement that was already
/// a diamond, and at five and six the circle is past [`MAX_RING_Y`] before
/// it has handed anybody a board. This is the line between those, and it is
/// deliberately nearer the first: 1.25 is bought, 1.43 is not.
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
const ROUND_COST: f32 = 1.3;

/// How much of the arc between two neighbours a mat may claim. The rest is
/// the gap that keeps them from touching.
const ARC_SHARE: f32 = 0.86;

/// A focused opponent counts as this many ordinary seats.
const FOCUS_WEIGHT: f32 = 2.6;

/// How a lane packed its cards.
#[derive(Clone, PartialEq, Debug)]
pub struct LanePacking {
    /// Horizontal offsets from the lane centre, left to right.
    pub offsets: Vec<f32>,
    /// Distance between successive card centres.
    pub pitch: f32,
    /// Whether the cells are tighter than a card's own span, so cards can
    /// overlap.
    ///
    /// "Can", not "do": between [`CARD_WIDTH`] and [`CARD_SPAN`] an untapped
    /// row still has air in it and a tapped one does not, and a lane cannot
    /// know which of its cards will be turned.
    pub fanned: bool,
    /// Whether even a fan cannot show every card legibly, so the caller should
    /// group identical cards into counted stacks instead.
    pub overflowing: bool,
}

/// Packs `count` cards into a lane `width` units wide.
///
/// Cards keep their size and start overlapping once they no longer fit, the way
/// a physical player fans a row. Shrinking instead would trade a readable board
/// for an unreadable one at exactly the moment the board matters most.
#[must_use]
pub fn pack_lane(count: usize, width: f32) -> LanePacking {
    if count == 0 {
        return LanePacking {
            offsets: Vec::new(),
            pitch: 0.0,
            fanned: false,
            overflowing: false,
        };
    }
    if count == 1 {
        return LanePacking {
            offsets: vec![0.0],
            pitch: 0.0,
            fanned: false,
            overflowing: false,
        };
    }

    let n = count as f32;
    let comfortable_pitch = CARD_SPAN + CARD_GAP;
    let comfortable_span = comfortable_pitch * (n - 1.0) + CARD_SPAN;
    let usable = width.max(CARD_SPAN);

    let (pitch, fanned) = if comfortable_span <= usable {
        (comfortable_pitch, false)
    } else {
        ((usable - CARD_SPAN) / (n - 1.0), true)
    };

    let min_pitch = CARD_WIDTH * MIN_VISIBLE_FRACTION;
    let overflowing = pitch < min_pitch;
    let pitch = pitch.max(min_pitch);

    let span = pitch * (n - 1.0);
    let offsets = (0..count)
        .map(|i| (i as f32) * pitch - span / 2.0)
        .collect();

    LanePacking {
        offsets,
        pitch,
        fanned,
        overflowing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seats(n: u8) -> Vec<PlayerId> {
        (0..n).map(PlayerId::new).collect()
    }

    #[test]
    fn a_pile_stands_beside_the_ground_and_never_on_it() {
        for n in [2, 3, 4, 6, 8] {
            let layout = TableLayout::new(&seats(n), 2.0, None);
            for slot in &layout.slots {
                let side = Vec2::new(slot.facing.cos(), -slot.facing.sin());
                for pile in PileKind::ALL {
                    let across = (slot.pile_center(pile) - slot.center).dot(side);
                    let near_edge = across.abs() - CARD_WIDTH * 0.5;
                    assert!(
                        near_edge > slot.half_extent.x,
                        "{n} seats: the near edge of the {} is {near_edge} out from \
                         the middle of a mat {} wide — it is lying on the board",
                        pile.label(),
                        slot.half_extent.x
                    );
                    assert!(
                        (across.signum() - pile.side()).abs() < 1e-6,
                        "{n} seats: the {} came out on the seat's other hand",
                        pile.label()
                    );
                }
            }
        }
    }

    /// A hover fan opens straight up out of the pile's own column, at every
    /// seat of every table.
    ///
    /// The claim that matters is the one [`FAN_STEP`]'s doc makes: the fan
    /// steps *along* the seat's depth axis and never across it, because
    /// across is where the mat is and a card on the mat reads as a permanent
    /// in play. So this is the same measurement
    /// `a_pile_stands_beside_the_ground_and_never_on_it` takes, repeated for
    /// every rung of the fan, and it has to come back with the pile's own
    /// number to the last bit.
    #[test]
    fn a_fan_opens_up_the_column_and_never_across_it() {
        for n in [2, 3, 4, 6, 8] {
            let layout = TableLayout::new(&seats(n), 2.0, None);
            for slot in &layout.slots {
                let side = Vec2::new(slot.facing.cos(), -slot.facing.sin());
                for pile in PileKind::ALL {
                    let column = (slot.pile_center(pile) - slot.center).dot(side);
                    for rung in 0..FAN_MAX {
                        let pose = slot.fan_pose(pile, rung, FAN_MAX);
                        let across = (pose.at - slot.center).dot(side);
                        assert!(
                            (across - column).abs() < 1e-4,
                            "{n} seats: card {rung} of the {} fan is {across} out \
                             where the pile is {column} — it has stepped across the \
                             column and onto the mat",
                            pile.label()
                        );
                    }
                }
            }
        }
    }

    /// The top of the pile is the top of the fan: highest, nearest the
    /// camera, and leaning towards it.
    ///
    /// Three claims measured in *table* space, where `+y` runs away from the
    /// camera, and the reason they are one test is that all three hang on the
    /// same sign. Index 0 is the top of the pile, so the rung it stands on is
    /// `len - 1 - index` — pinned here because it is the kind of inversion a
    /// later reader tidies away.
    ///
    /// The side seats are exempt from the leaning half and only from that
    /// half. Their depth axis runs across the screen, so tipping a card about
    /// it barely turns the face towards the camera at all; `cos(facing)` is
    /// what that projection is, and it is what the exemption is written
    /// against rather than a list of seat numbers.
    #[test]
    fn the_top_of_the_pile_is_the_top_of_the_fan() {
        for n in [2, 3, 4, 6, 8] {
            let layout = TableLayout::new(&seats(n), 2.0, None);
            for slot in &layout.slots {
                let top = slot.fan_pose(PileKind::Graveyard, 0, FAN_MAX);
                let bottom = slot.fan_pose(PileKind::Graveyard, FAN_MAX - 1, FAN_MAX);

                assert!(
                    top.lift > bottom.lift,
                    "{n} seats, seat {}: the top card is the lower of the two",
                    slot.ring_index
                );
                assert!(
                    (bottom.lift - FAN_FLOAT).abs() < 1e-6,
                    "the bottom of the fan does not start at the float"
                );
                if slot.facing.cos().abs() > SIDE_SEAT_TILT {
                    assert!(
                        top.at.y < bottom.at.y - 1e-4,
                        "{n} seats, seat {}: the top card stepped away from the camera",
                        slot.ring_index
                    );
                    // The card's normal once it has been laid flat and
                    // tipped: `cos(facing) · sin(tilt)` is how much of it
                    // points at the camera, and it has to be positive.
                    assert!(
                        slot.facing.cos() * top.tilt.sin() > 0.0,
                        "{n} seats, seat {}: the fan leans away from the camera",
                        slot.ring_index
                    );
                } else {
                    // A side seat has no answer to "which end is nearer the
                    // camera" — its depth axis runs across the screen — and
                    // takes the one that is never wrong instead: the fan
                    // steps in over the table rather than out past its edge.
                    let away = Vec2::new(slot.facing.sin(), slot.facing.cos());
                    let inwards = (top.at - slot.pile_center(PileKind::Graveyard)).dot(away);
                    assert!(
                        inwards > 0.0,
                        "{n} seats, seat {}: the side seat's fan stepped off the table",
                        slot.ring_index
                    );
                }
            }
        }
    }

    /// A fan is a hand of cards and not a staircase: its two ends are turned
    /// opposite ways by the same amount, and its middle is not turned at all.
    #[test]
    fn a_fan_is_turned_symmetrically_about_its_middle() {
        let layout = TableLayout::new(&seats(2), 2.0, None);
        let slot = layout.local().expect("a local seat");
        for len in 1..=FAN_MAX {
            let yaws: Vec<f32> = (0..len)
                .map(|i| slot.fan_pose(PileKind::Exile, i, len).yaw)
                .collect();
            let sum: f32 = yaws.iter().sum();
            assert!(
                sum.abs() < 1e-5,
                "a fan of {len} is turned {sum} out of true overall: {yaws:?}"
            );
            if len > 1 {
                assert!(
                    (yaws[0] + yaws[len - 1]).abs() < 1e-6,
                    "a fan of {len} turns its two ends by different amounts"
                );
                assert!(yaws[0] > 0.0, "the top card is turned the wrong way");
            }
        }
    }

    /// An index past the end of the fan is clamped onto the last rung rather
    /// than refused — a fan is a drawing, and the worst a clamp does is put
    /// two cards in one place.
    #[test]
    fn a_card_past_the_end_of_the_fan_lands_on_the_bottom_rung() {
        let layout = TableLayout::new(&seats(2), 2.0, None);
        let slot = layout.local().expect("a local seat");
        let bottom = slot.fan_pose(PileKind::Graveyard, 2, 3);
        for beyond in [3, 4, 99] {
            assert_eq!(slot.fan_pose(PileKind::Graveyard, beyond, 3), bottom);
        }
        // And a fan of nothing is the pile itself, floated.
        let none = slot.fan_pose(PileKind::Graveyard, 0, 0);
        assert_eq!(none.at, slot.pile_center(PileKind::Graveyard));
        assert!((none.lift - FAN_FLOAT).abs() < 1e-6);
    }

    /// The two flanks of a table have to answer alike, and a seat across it
    /// has to answer differently — with room to spare between the two, or
    /// the tolerance that settles the flanks would start deciding real
    /// seats.
    #[test]
    fn the_two_flanks_of_a_table_put_their_bars_on_the_same_edge() {
        let table = TableLayout::new(&seats(4), 1.78, None);
        let local = table.local().expect("a local seat");
        assert!(
            !local.ledge_is_outer(),
            "the seat the camera sits behind reads its own bar above its own \
             creatures, on the edge facing the middle of the table"
        );

        for slot in &TableLayout::new(&seats(3), 1.78, None).slots[1..] {
            assert!(
                slot.ledge_is_outer(),
                "an opponent in a three-way is across the table and its board \
                 is drawn upside-down from here, so its bar belongs on the \
                 outer edge; cos is {}",
                slot.facing.cos()
            );
        }

        // Every case below is also run with a board being inspected, because
        // that is the live call — `TableLayout::new(…, duel.focus)`, and `F`
        // is a key a player presses. A focus reweights the compartments and
        // with them the size of the ring, and a flank that drifted off the
        // side of a ring while somebody looked at an opponent would move its
        // bar to the other edge of its mat for as long as they looked.
        let lookers = [None, Some(PlayerId::new(1)), Some(PlayerId::new(2))];
        for n in [4, 8] {
            for aspect in [1.4_f32, 1.78, 2.25] {
                for focus in lookers {
                    let layout = TableLayout::new(&seats(n), aspect, focus);
                    let flanks: Vec<&SeatSlot> = layout
                        .slots
                        .iter()
                        .filter(|slot| slot.facing.cos().abs() < 0.5)
                        .collect();
                    assert_eq!(
                        flanks.len(),
                        2,
                        "{n} seats at {aspect} with {focus:?} inspected: a table \
                         has two flanks"
                    );
                    for slot in &flanks {
                        assert!(
                            !slot.ledge_is_outer(),
                            "{n} seats at {aspect} with {focus:?} inspected: a \
                             flank has no 'above' and keeps the edge nearer the \
                             hearth; cos is {}",
                            slot.facing.cos()
                        );
                    }
                }
            }
        }

        // And the margin the tolerance is chosen against: nothing at any
        // table sits in the gap between "a flank" and "across from here".
        for n in 2..=8 {
            for aspect in [1.4_f32, 1.78, 2.25] {
                for focus in lookers {
                    for slot in &TableLayout::new(&seats(n), aspect, focus).slots {
                        let lean = slot.facing.cos().abs();
                        assert!(
                            lean < SIDE_SEAT_TILT || lean > SIDE_SEAT_TILT * 4.0,
                            "{n} seats at {aspect} with {focus:?} inspected: seat \
                             {} leans {lean}, which is neither a flank nor plainly \
                             across the table",
                            slot.ring_index
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_four_piles_are_four_places() {
        let layout = TableLayout::new(&seats(2), 2.0, None);
        let slot = layout.local().expect("a local seat");
        for (i, a) in PileKind::ALL.iter().enumerate() {
            for b in &PileKind::ALL[i + 1..] {
                let gap = slot.pile_center(*a).distance(slot.pile_center(*b));
                assert!(
                    gap > CARD_HEIGHT,
                    "the {} and the {} are {gap} apart, and a card is {CARD_HEIGHT} \
                     long — they would be stacked on each other",
                    a.label(),
                    b.label()
                );
            }
        }
    }

    /// At a crowded table the piles are the parts that come nearest the seat
    /// next door, and they are the last thing added to a ring solve that was
    /// written without them.
    /// The four corners of a pile's card, turned to face its seat.
    fn pile_corners(slot: &SeatSlot, pile: PileKind) -> [Vec2; 4] {
        let at = slot.pile_center(pile);
        let side = Vec2::new(slot.facing.cos(), -slot.facing.sin());
        let away = Vec2::new(slot.facing.sin(), slot.facing.cos());
        [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .map(|(x, y)| at + side * (x * CARD_WIDTH * 0.5) + away * (y * CARD_HEIGHT * 0.5))
    }

    /// Whether two convex quads share any area — the separating-axis test,
    /// written out rather than approximated by circles round each box. A
    /// circle bound is *sufficient* and would have failed at seat counts
    /// where nothing actually touches, which makes it useless for telling a
    /// real overlap from a near miss.
    fn quads_overlap(a: [Vec2; 4], b: [Vec2; 4]) -> bool {
        for poly in [a, b] {
            for i in 0..4 {
                let edge = poly[(i + 1) % 4] - poly[i];
                let axis = Vec2::new(-edge.y, edge.x);
                let (pa, pb) = (a.map(|p| axis.dot(p)), b.map(|p| axis.dot(p)));
                let hi = |v: [f32; 4]| v.into_iter().fold(f32::NEG_INFINITY, f32::max);
                let lo = |v: [f32; 4]| v.into_iter().fold(f32::INFINITY, f32::min);
                if hi(pa) < lo(pb) - 1e-6 || hi(pb) < lo(pa) - 1e-6 {
                    return false;
                }
            }
        }
        true
    }

    #[test]
    fn no_two_seats_piles_stand_on_each_other() {
        for n in [3, 4, 5, 6, 7, 8] {
            let layout = TableLayout::new(&seats(n), 2.0, None);
            for (i, a) in layout.slots.iter().enumerate() {
                for b in &layout.slots[i + 1..] {
                    for pa in PileKind::ALL {
                        for pb in PileKind::ALL {
                            assert!(
                                !quads_overlap(pile_corners(a, pa), pile_corners(b, pb)),
                                "{n} seats: seat {:?}'s {} lies on top of seat {:?}'s {}",
                                a.player,
                                pa.label(),
                                b.player,
                                pb.label()
                            );
                        }
                    }
                }
            }
        }
    }

    /// The reason [`SeatSlot::footprint`] exists at all: the piles stand
    /// outside the playing surface, so a camera framed from `half_extent`
    /// puts every one of them off the screen.
    #[test]
    fn every_pile_is_inside_the_rectangle_the_camera_frames() {
        for n in [2, 3, 4, 6, 8] {
            let layout = TableLayout::new(&seats(n), 2.0, None);
            let (min, max) = layout.extent().expect("a table with seats");
            for slot in &layout.slots {
                let (sin, cos) = slot.facing.sin_cos();
                // A card's own box, turned to face its seat, then measured
                // along the table's axes — the same rotation `extent` does.
                let half = Vec2::new(
                    cos.abs()
                        .mul_add(CARD_WIDTH * 0.5, sin.abs() * CARD_HEIGHT * 0.5),
                    sin.abs()
                        .mul_add(CARD_WIDTH * 0.5, cos.abs() * CARD_HEIGHT * 0.5),
                );
                for pile in PileKind::ALL {
                    let at = slot.pile_center(pile);
                    let (lo, hi) = (at - half, at + half);
                    assert!(
                        lo.x >= min.x - 1e-3
                            && lo.y >= min.y - 1e-3
                            && hi.x <= max.x + 1e-3
                            && hi.y <= max.y + 1e-3,
                        "{n} seats: the {} of seat {:?} runs {lo} to {hi}, outside \
                         the framed table {min} to {max} — the camera cuts it off",
                        pile.label(),
                        slot.player
                    );
                }
            }
        }
    }

    #[test]
    fn the_local_seat_is_always_at_the_near_edge() {
        for n in 1..=8u8 {
            let layout = TableLayout::new(&seats(n), 1.78, None);
            let local = layout.local().expect("a local slot");
            assert!(local.is_local);
            assert_eq!(local.ring_index, 0);
            assert!(
                local.center.x.abs() < 1e-4,
                "local seat is centred horizontally for {n} seats"
            );
            assert!(
                local.center.y < 0.0,
                "local seat is on the near side for {n} seats"
            );
        }
    }

    #[test]
    fn two_players_sit_opposite_each_other() {
        let layout = TableLayout::new(&seats(2), 1.78, None);
        let a = layout.slots[0].center;
        let b = layout.slots[1].center;
        assert!(a.y < 0.0 && b.y > 0.0);
        assert!((a.x - b.x).abs() < 1e-4);
    }

    #[test]
    fn every_seat_count_produces_distinct_pod_centres() {
        for n in 2..=8u8 {
            let layout = TableLayout::new(&seats(n), 1.78, None);
            for i in 0..layout.slots.len() {
                for j in (i + 1)..layout.slots.len() {
                    let d = layout.slots[i].center.distance(layout.slots[j].center);
                    assert!(d > 1.0, "seats {i} and {j} of {n} overlap (distance {d})");
                }
            }
        }
    }

    #[test]
    fn seats_are_ordered_clockwise_in_turn_order() {
        let layout = TableLayout::new(&seats(4), 1.78, None);
        // Ring index 1 is the next player in turn order and sits to the left.
        // Clockwise from the near edge of a table *is* the left hand — six
        // o'clock to seven — and it is where Magic's turn order goes, which
        // is the association a player brings with them. This used to read
        // `> 0.0`: seats were laid out anticlockwise while every frame built
        // from `facing` assumed the other way round, so a flank seat's lands
        // stood between it and the middle and its creatures behind its back.
        assert!(layout.slots[1].center.x < 0.0);
        // Ring index 3 is the previous player and sits to the right.
        assert!(layout.slots[3].center.x > 0.0);
        // Angles increase monotonically.
        for w in layout.slots.windows(2) {
            assert!(w[1].angle > w[0].angle);
        }
    }

    #[test]
    fn every_seat_gets_the_same_board() {
        // One board, handed out unchanged to everybody. It was briefly
        // per-side — each side taking what its own neighbours allowed, so a
        // table of three left the near seat half the ring to itself — and a
        // table where one player's board is wider than another's is a table
        // where the wider board is the one being played on.
        for n in 2..=8u8 {
            for aspect in [1.78_f32, 1.0, 0.6] {
                let layout = TableLayout::seated(
                    &seats(n).into_iter().map(Seat::alone).collect::<Vec<_>>(),
                    aspect,
                    None,
                );
                let at_ceiling =
                    layout.radius.x >= MAX_RING_X - 1e-3 || layout.radius.y >= MAX_RING_Y - 1e-3;
                // A duel's ring never grows: two seats have nothing to be
                // crowded by, so their mats sit against the middle and take
                // whatever the canvas leaves. On a square one that is under
                // the minimum, and pushing them apart to reach it would buy
                // width with empty table.
                let at_channel = layout
                    .slots
                    .iter()
                    .map(|slot| slot.center.length() - slot.half_extent.y)
                    .fold(f32::INFINITY, f32::min)
                    <= CENTRE_GAP * 0.5 + 1e-3;
                for slot in &layout.slots {
                    assert!(
                        (slot.half_extent - layout.slots[0].half_extent).length() < 1e-3,
                        "{n} seats at {aspect}: seat {} has {:?} against the local {:?}",
                        slot.ring_index,
                        slot.half_extent,
                        layout.slots[0].half_extent
                    );
                    assert!(
                        at_ceiling || at_channel || slot.lane_width() >= MIN_POD_WIDTH - 1e-2,
                        "{n} seats at {aspect}: a seat plays on {} on a ring that \
                         could still have grown",
                        slot.lane_width()
                    );
                }
            }
        }
    }

    #[test]
    fn focusing_an_opponent_enlarges_it_at_everyone_elses_expense() {
        let players = seats(4);
        let plain = TableLayout::new(&players, 1.78, None);
        let focused = TableLayout::new(&players, 1.78, Some(PlayerId::new(2)));

        let target = focused.slot(PlayerId::new(2)).expect("focused slot");
        let before = plain.slot(PlayerId::new(2)).expect("plain slot");
        assert!(target.half_extent.x > before.half_extent.x);

        let bystander = focused.slot(PlayerId::new(1)).expect("bystander");
        let bystander_before = plain.slot(PlayerId::new(1)).expect("bystander");
        assert!(bystander.half_extent.x < bystander_before.half_extent.x);
    }

    #[test]
    fn an_empty_table_is_handled_without_panicking() {
        let layout = TableLayout::new(&[], 1.78, None);
        assert!(layout.slots.is_empty());
        assert!(layout.local().is_none());
        assert!(layout.extent().is_none());
    }

    #[test]
    fn the_tables_extent_holds_every_seats_mat() {
        for n in 2..=8 {
            let layout = TableLayout::new(&seats(n), 1.78, None);
            let (min, max) = layout.extent().expect("a seated table has an extent");
            for slot in &layout.slots {
                // Whatever a pod's own frame is, its four corners are inside.
                let (sin, cos) = slot.facing.sin_cos();
                for sx in [-1.0_f32, 1.0] {
                    for sy in [-1.0_f32, 1.0] {
                        let local = slot.half_extent * Vec2::new(sx, sy);
                        let corner = slot.center
                            + Vec2::new(
                                cos.mul_add(local.x, sin * local.y),
                                (-sin).mul_add(local.x, cos * local.y),
                            );
                        assert!(
                            corner.x >= min.x - 1e-3
                                && corner.x <= max.x + 1e-3
                                && corner.y >= min.y - 1e-3
                                && corner.y <= max.y + 1e-3,
                            "{n} seats: {corner} escapes {min}..{max}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_seat_across_the_table_is_measured_across_the_table() {
        // The bug this is here for: taking `half_extent` unrotated makes a
        // four-seat table report the side seats as deep and narrow when they
        // are wide and shallow, and the camera then cuts their lands off.
        let layout = TableLayout::new(&seats(4), 1.78, None);
        let side = layout
            .slots
            .iter()
            .find(|s| s.center.x.abs() > s.center.y.abs())
            .copied()
            .expect("a four-seat table has a seat on each side");
        let (min, max) = layout.extent().expect("extent");
        assert!(
            max.x - min.x >= 2.0 * (side.center.x.abs() + side.half_extent.y) - 1e-3,
            "the side seat is laid across the table, not along it"
        );
    }

    #[test]
    fn a_comfortable_lane_does_not_fan() {
        let packing = pack_lane(4, 20.0);
        assert!(!packing.fanned);
        assert!(!packing.overflowing);
        assert_eq!(packing.offsets.len(), 4);
        assert!((packing.pitch - (CARD_SPAN + CARD_GAP)).abs() < 1e-5);
    }

    /// The overlaps the owner saw, and the reason they made no sense: the
    /// lane had room to spare.
    ///
    /// A card taps by turning a quarter of the way round, so it claims
    /// [`CARD_SPAN`] of the row and not [`CARD_WIDTH`]. The lane packed to
    /// the narrower of the two, so a tapped land or an attacking creature sat
    /// 0.14 units inside each of its neighbours — on a duel's lane nearly
    /// twenty units wide holding six cards. Tokens only made it louder: more
    /// cards, tighter pitch, the same fault.
    #[test]
    fn a_row_of_tapped_cards_does_not_overlap_itself() {
        // A duel's own lane is about twenty units across; a four-player pod's
        // is about six. Both, and a deliberately crowded one below them.
        for width in [19.7f32, 6.1, 3.0] {
            for count in 2..=12usize {
                let packing = pack_lane(count, width);
                let step = packing.offsets[1] - packing.offsets[0];
                assert!(
                    (step - packing.pitch).abs() < 1e-4,
                    "the reported pitch is not the step taken"
                );
                if packing.fanned {
                    // A fan is overlap on purpose, and the pitch is already
                    // held above the legibility floor by the case below.
                    continue;
                }
                // `CARD_HEIGHT`, not `CARD_SPAN`: the width a tapped card
                // really occupies is the card's long side, and a test that
                // measured against the constant the packing is written in
                // would agree with it however wrong both were. The first
                // draft of this did exactly that and passed against the code
                // it was written to fail.
                assert!(
                    step >= CARD_HEIGHT,
                    "{count} cards in {width} units: a lane with room to \
                     spare still overlapped when they tapped ({step} apart, \
                     a tapped card being {CARD_HEIGHT} wide)"
                );
            }
        }
    }

    #[test]
    fn a_lane_is_always_centred_on_zero() {
        for count in [1usize, 2, 5, 12, 40] {
            let packing = pack_lane(count, 12.0);
            let sum: f32 = packing.offsets.iter().sum();
            assert!(sum.abs() < 1e-3, "lane of {count} is off-centre by {sum}");
        }
    }

    #[test]
    fn offsets_are_strictly_increasing() {
        let packing = pack_lane(15, 10.0);
        for w in packing.offsets.windows(2) {
            assert!(w[1] > w[0]);
        }
    }

    #[test]
    fn a_crowded_lane_fans_instead_of_shrinking_cards() {
        let packing = pack_lane(15, 10.0);
        assert!(packing.fanned);
        assert!(packing.pitch < CARD_WIDTH, "cards must overlap");
        // Still legible: the fan never hides more than the policy allows.
        assert!(packing.pitch >= CARD_WIDTH * MIN_VISIBLE_FRACTION);
    }

    #[test]
    fn an_unfittable_lane_reports_overflow_so_the_caller_can_group() {
        // Sixty tokens in a narrow opponent pod cannot be fanned legibly.
        let packing = pack_lane(60, 6.0);
        assert!(packing.overflowing);
        assert!(packing.fanned);
        // The pitch is clamped, so the row deliberately runs wider than the
        // pod: the board model is expected to collapse the row instead.
        assert!(packing.pitch >= CARD_WIDTH * MIN_VISIBLE_FRACTION);
    }

    #[test]
    fn empty_and_single_lanes_are_degenerate_but_valid() {
        assert!(pack_lane(0, 10.0).offsets.is_empty());
        assert_eq!(pack_lane(1, 10.0).offsets, vec![0.0]);
    }

    #[test]
    fn lanes_stack_from_the_table_centre_towards_the_seat() {
        let layout = TableLayout::new(&seats(2), 1.78, None);
        let local = layout.local().expect("local");
        let creatures = local.lane_center(LaneKind::Creatures);
        let lands = local.lane_center(LaneKind::Lands);
        // For the near seat, "away from the seat" is +y, so creatures — drawn
        // towards the middle of the table — have the larger y.
        assert!(
            creatures.y > lands.y,
            "creatures {creatures:?} should sit closer to the table centre than lands {lands:?}"
        );

        // At every other seat too, which is the half this test used to miss.
        // Only the near seat was ever asked, and only in a duel — so a flank
        // seat kept its lands towards the middle and its creatures out behind
        // its own back, on every table of three or more, for as long as one
        // could be sat down. `facing` was right and the ring ran the other
        // way, and neither is visible from a seat that sits at angle zero.
        for n in 2..=8u8 {
            for aspect in [1.78_f32, 1.0] {
                for slot in &TableLayout::new(&seats(n), aspect, None).slots {
                    let out = slot.center.length();
                    let creatures = slot.lane_center(LaneKind::Creatures).length();
                    let lands = slot.lane_center(LaneKind::Lands).length();
                    assert!(
                        creatures < out && out < lands,
                        "{n} seats at {aspect}: seat {} has its creatures {creatures:.2} \
                         and its lands {lands:.2} from the middle, sitting at {out:.2}",
                        slot.ring_index
                    );
                }
            }
        }
    }

    #[test]
    fn a_mat_is_the_same_depth_at_every_table() {
        // The bug this replaces: depth came off the ring, so a table laid out
        // for eight seats gave each of them a deeper mat than a duel did — and
        // a duel, which is what almost every game is, got the shallowest board
        // of the lot. A card is the same size at every table.
        for n in 1..=8 {
            for aspect in [0.6_f32, 1.0, 1.78, 2.0, 2.8] {
                for slot in &TableLayout::new(&seats(n), aspect, None).slots {
                    assert!(
                        (slot.half_extent.y * 2.0 - POD_DEPTH).abs() < 1e-3,
                        "{n} seats at {aspect}: mat is {} deep, not {POD_DEPTH}",
                        slot.half_extent.y * 2.0
                    );
                }
            }
        }
    }

    #[test]
    fn the_middle_stays_clear_for_the_table() {
        // The open middle is the negative form of this layout: it is whatever
        // the mats leave. If the mats close in, the medallion has nowhere to
        // sit and a table stops reading as a table; if they drift apart,
        // every card is drawn smaller for the empty felt between them. Both
        // bounds, because the second is the mistake that was actually made.
        for n in 2..=8 {
            let layout = TableLayout::new(&seats(n), 2.0, None);
            let inner = layout
                .slots
                .iter()
                .map(|slot| slot.center.length() - slot.half_extent.y)
                .fold(f32::INFINITY, f32::min);
            assert!(
                inner >= CENTRE_GAP * 0.5 - 1e-3,
                "{n} seats: a mat reaches to {inner} of the middle, inside the {} channel",
                CENTRE_GAP * 0.5
            );

            // And no further out than it has to be. There are exactly three
            // reasons the ring may stand where it does and the mats be as
            // wide as they are, so one of them has to be tight:
            //
            // - the mats are as close to the middle as the open gap allows;
            // - the crowding solve stopped there, a pod being exactly the
            //   board's worth `MIN_POD_WIDTH` promises and one step further
            //   out therefore more than a seat needs;
            // - or the ring is at the ceiling the camera can still frame, and
            //   the pods are narrower than the minimum only because there is
            //   nowhere left to grow.
            //
            // A ring that satisfies none of the three is empty table, and
            // empty table is what every card on it is drawn smaller for.
            //
            // The ellipse used to stand here as a fourth reason — its flanks
            // bring two seats closer together than any circle of the same
            // mean radius would. It is not a separate reason any more, and
            // twice over: `sides_on` spaces sides by distance rather than by
            // angle, so no part of the ring is tighter than another, and what
            // is left of it lives inside `side_half_widths`, which is the
            // function the solve asks. So it comes out as the second bullet
            // like every other way a pod can be limited.
            //
            // It is the *narrowest* pod that has to be tight, because that is
            // what the solve grows the ring for. Some seats then get more
            // than the minimum — a table of three leaves the near seat half
            // the ring to itself — and a wider board than promised is not a
            // reason to push everyone further out.
            let narrowest = layout
                .slots
                .iter()
                .map(|slot| slot.half_extent.x * 2.0)
                .fold(f32::INFINITY, f32::min);
            let at_ceiling =
                layout.radius.x >= MAX_RING_X - 1e-3 || layout.radius.y >= MAX_RING_Y - 1e-3;
            assert!(
                (inner - CENTRE_GAP * 0.5).abs() < 1e-3
                    || (narrowest - MIN_POD_WIDTH).abs() < 1e-2
                    || at_ceiling,
                "{n} seats: mats stop {inner} out and are {narrowest} wide on a ring \
                 {:?} that could still have grown — none of the middle, the \
                 crowding or the ceiling put them there",
                layout.radius
            );
        }
    }

    #[test]
    fn a_duel_comes_out_the_shape_of_its_canvas() {
        // A span taller than the canvas wastes its width, a span wider wastes
        // its height, and the camera fits whatever this reports — so only a
        // span of the canvas's own shape wastes neither. Two seats is the case
        // worth pinning: a ring of six has neighbours to clear and cannot
        // always have it.
        for aspect in [1.0_f32, 1.6, 1.78, 2.0, 2.4] {
            let layout = TableLayout::new(&seats(2), aspect, None);
            let (min, max) = layout.extent().expect("a seated table has an extent");
            let span = max - min;
            let got = span.x / span.y;
            assert!(
                (got - aspect).abs() < 0.05,
                "canvas {aspect}: the table came out {got} ({span:?})"
            );
        }
    }

    /// The shape of the space the duel HUD leaves on a laptop window — what
    /// the layout is actually built against, and nothing like the window's.
    const HUD_ASPECT: f32 = 2.01;

    // `MIN_POD_WIDTH` is the width a seat is *handed*, and for a long time it
    // was the width of an arc a seat was then charged two pile strips out of.
    // Measured at the aspect above: four seats were solved for an arc of 10.0
    // and given 6.10 — four cards on a row the name promises seven to.
    #[test]
    fn a_pod_gets_the_width_its_minimum_promises_or_the_ring_is_at_its_ceiling() {
        for n in [3u8, 4, 5, 6, 8] {
            for aspect in [HUD_ASPECT, 16.0 / 9.0, 1.0] {
                let layout = TableLayout::new(&seats(n), aspect, None);
                let width = layout.slots[0].lane_width();
                let at_ceiling =
                    layout.radius.x >= MAX_RING_X - 0.01 || layout.radius.y >= MAX_RING_Y - 0.01;
                assert!(
                    width >= MIN_POD_WIDTH - 0.01 || at_ceiling,
                    "{n} seats at aspect {aspect:.2}: {width:.2} wide on a ring \
                     ({:.2}, {:.2}) that could still have grown",
                    layout.radius.x,
                    layout.radius.y,
                );
            }
        }
    }

    // The complaint this closes, in the terms it was made in: at four players
    // a seat's row was too narrow. Six cards is an ordinary mid-game board of
    // lands, and they used to overlap on it.
    #[test]
    fn six_cards_lie_side_by_side_at_a_four_seat_table() {
        let layout = TableLayout::new(&seats(4), HUD_ASPECT, None);
        for slot in &layout.slots {
            let packing = pack_lane(6, slot.lane_width());
            assert!(
                !packing.fanned,
                "six cards fan on a {:.2}-wide row",
                slot.lane_width()
            );
        }
    }

    // Growing the ring is only free while the camera can still frame it, so
    // the ceiling exists — and a ceiling that no seat count ever reaches is a
    // ceiling nobody has checked. Six seats and up sit on it.
    #[test]
    fn a_crowded_table_stops_growing_at_the_ceiling() {
        let layout = TableLayout::new(&seats(8), HUD_ASPECT, None);
        assert!(
            layout.radius.x <= MAX_RING_X + 0.01 && layout.radius.y <= MAX_RING_Y + 0.01,
            "the ring outgrew what the camera can frame: {:?}",
            layout.radius
        );
        assert!(
            layout.slots[0].lane_width() < MIN_POD_WIDTH,
            "eight seats reaching the minimum would mean the ceiling is never tested"
        );
    }

    // Seats sharing a ring evenly should get less room as more of them
    // arrive, and the old solve did not: five seats came out *narrower* than
    // six, because the ring was sized against one bound and the width read
    // off another.
    #[test]
    fn more_seats_never_means_a_wider_pod() {
        let mut last = f32::INFINITY;
        for n in [3u8, 4, 5, 6, 8] {
            let width = TableLayout::new(&seats(n), HUD_ASPECT, None).slots[0].lane_width();
            assert!(
                width <= last + 0.01,
                "{n} seats got {width:.2}, wider than the {last:.2} of fewer seats"
            );
            last = width;
        }
    }

    /// Whether two seats' grounds — their mats *and* the pile strips beside
    /// them, which is what [`SeatSlot::footprint`] is — share any table at
    /// all.
    ///
    /// The separating-axis test over the two boxes' own four axes, which is
    /// exact for rectangles: two convex shapes are disjoint exactly when some
    /// axis separates their projections, and for boxes the only candidates
    /// are their edge normals. Written out here because the layout has never
    /// had a way to *ask* — every bound it applies is a bound on the distance
    /// between two centres, and a centre distance says nothing on its own
    /// about two rectangles turned to face different seats.
    fn grounds_overlap(a: &SeatSlot, b: &SeatSlot) -> bool {
        let axes = |s: &SeatSlot| {
            let (sin, cos) = s.facing.sin_cos();
            [Vec2::new(cos, -sin), Vec2::new(sin, cos)]
        };
        let reach = |s: &SeatSlot, u: Vec2| {
            let [along, away] = axes(s);
            let f = s.footprint();
            f.x.mul_add(u.dot(along).abs(), f.y * u.dot(away).abs())
        };
        for u in axes(a).into_iter().chain(axes(b)) {
            if (b.center - a.center).dot(u).abs() > reach(a, u) + reach(b, u) + 1e-4 {
                return false;
            }
        }
        true
    }

    /// Nobody plays on anybody else's table.
    ///
    /// The bound the layout actually applies is on the distance between two
    /// seats' *centres*, which is not the same question: a mat is a rectangle
    /// turned to face its own seat, and two rectangles at a given distance
    /// may or may not meet depending on how they are turned. This asks the
    /// real question of every pair, at every seat count, on three shapes of
    /// canvas.
    #[test]
    fn a_team_sits_along_one_side_of_the_table() {
        // Two-headed giant, seated both ways round: partners next to each
        // other in turn order, and partners alternating with the opposition.
        // Either way a team is one side of the table — a partner's board is
        // read as often as one's own, and across the table it was upside
        // down.
        for order in [[1u8, 1, 2, 2], [1, 2, 1, 2]] {
            let table: Vec<Seat> = order
                .iter()
                .enumerate()
                .map(|(i, &t)| Seat::on(PlayerId::new(i as u8), Some(t)))
                .collect();
            let layout = TableLayout::seated(&table, 1.78, None);
            assert_eq!(layout.slots.len(), 4);

            for (i, slot) in layout.slots.iter().enumerate() {
                let ally = layout
                    .slots
                    .iter()
                    .enumerate()
                    .find(|(j, _)| *j != i && order[*j] == order[i])
                    .expect("a partner")
                    .1;
                assert!(
                    (slot.facing - ally.facing).abs() < 1e-4,
                    "{order:?}: seat {i} faces {} and its partner {}",
                    slot.facing,
                    ally.facing
                );
                // Shoulder to shoulder: one step apart along their own side,
                // and neither of them any nearer the middle than the other.
                let step = (ally.center - slot.center).length();
                let footprint = (slot.half_extent.x + PILE_STRIP) * 2.0;
                assert!(
                    (footprint..footprint * 1.25).contains(&step),
                    "{order:?}: partners stand {step} apart, against a {footprint} board"
                );
                assert!(
                    (slot.center.length() - ally.center.length()).abs() < 1e-3,
                    "{order:?}: one partner sits further out than the other"
                );
                assert!(
                    slot.lane_width() >= MIN_POD_WIDTH - 1e-2,
                    "{order:?}: seat {i} plays on {}",
                    slot.lane_width()
                );
            }

            for i in 0..layout.slots.len() {
                for j in (i + 1)..layout.slots.len() {
                    assert!(
                        !grounds_overlap(&layout.slots[i], &layout.slots[j]),
                        "{order:?}: seats {i} and {j} overlap"
                    );
                }
            }

            // And it is still the shape of the canvas. A side of two reaches
            // twice as far along itself as a side of one, and the ring was
            // shaped as though it did not: the table came out exactly twice
            // as wide as it asked to be, the camera fitted it by width, and
            // all four boards sat in the top half of the window with bare
            // felt under them.
            let (min, max) = layout.extent().expect("a seated table has an extent");
            let span = max - min;
            assert!(
                (span.x / span.y - 1.78).abs() < 0.15,
                "{order:?}: the table came out {} ({span:?})",
                span.x / span.y
            );

            // And the whole point of it: a table of two sides is a smaller
            // table than one of four, so the camera comes in rather than
            // pulling back to frame a ring nobody is sitting on.
            let apart = TableLayout::new(&seats(4), 1.78, None);
            assert!(
                layout.radius.x < apart.radius.x && layout.radius.y < apart.radius.y,
                "{order:?}: two sides want a ring of {:?}, four wanted {:?}",
                layout.radius,
                apart.radius
            );
        }
    }

    /// A table where some seats are partnered and some are not.
    ///
    /// The gateway arranges any of these — `--teams 1,1,2` is a two-on-one,
    /// `1,1,2,2,0` is two pairs and a player on their own — and a side of two
    /// reaches twice as far along itself as a side of one. The board is still
    /// one board: the rule is that every *seat* gets the same one, not every
    /// side, so a lone player's mat is exactly as wide as each half of the
    /// pair across from them. Anything else and the table tells a player
    /// their board is the smaller one before the game has started.
    #[test]
    fn a_mixed_table_still_hands_out_one_board() {
        for order in [
            vec![Some(1u8), Some(1), Some(2)],
            vec![Some(1u8), Some(1), None, None],
            vec![Some(1u8), Some(1), Some(2), Some(2), None],
            vec![Some(1u8), Some(2), Some(1), None, Some(2), None],
        ] {
            let table: Vec<Seat> = order
                .iter()
                .enumerate()
                .map(|(i, &t)| Seat::on(PlayerId::new(i as u8), t))
                .collect();
            for aspect in [1.78_f32, 1.0] {
                let layout = TableLayout::seated(&table, aspect, None);
                assert_eq!(layout.slots.len(), order.len());
                // The same two escapes as `every_seat_gets_the_same_board`:
                // a ring that has stopped growing, and mats already against
                // the centre channel with nothing to be crowded by.
                let at_ceiling =
                    layout.radius.x >= MAX_RING_X - 1e-3 || layout.radius.y >= MAX_RING_Y - 1e-3;
                let at_channel = layout
                    .slots
                    .iter()
                    .map(|slot| slot.center.length() - slot.half_extent.y)
                    .fold(f32::INFINITY, f32::min)
                    <= CENTRE_GAP * 0.5 + 1e-3;
                for slot in &layout.slots {
                    assert!(
                        (slot.half_extent - layout.slots[0].half_extent).length() < 1e-3,
                        "{order:?} at {aspect}: seat {} plays on {:?} against the local \
                         seat's {:?}",
                        slot.ring_index,
                        slot.half_extent,
                        layout.slots[0].half_extent
                    );
                    assert!(
                        at_ceiling || at_channel || slot.lane_width() >= MIN_POD_WIDTH - 1e-2,
                        "{order:?} at {aspect}: a seat plays on {}",
                        slot.lane_width()
                    );
                }
                // Partners still share a side, and a lone seat still has one
                // to itself.
                for (i, slot) in layout.slots.iter().enumerate() {
                    for (j, other) in layout.slots.iter().enumerate().skip(i + 1) {
                        let together = order[i].is_some() && order[i] == order[j];
                        let same_side = (slot.facing - other.facing).abs() < 1e-4;
                        assert_eq!(
                            together, same_side,
                            "{order:?} at {aspect}: seats {i} and {j} face {} and {}",
                            slot.facing, other.facing
                        );
                        assert!(
                            !grounds_overlap(slot, other),
                            "{order:?} at {aspect}: seats {i} and {j} overlap"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn no_two_seats_play_on_the_same_table() {
        for n in 2..=8u8 {
            for aspect in [2.01_f32, 16.0 / 9.0, 1.0, 0.6] {
                // With an opponent under inspection as well: the focus is the
                // one thing that makes two boards different sizes, and the
                // bound it borrows against was measured for boards that are
                // all the same.
                for focus in [None, Some(PlayerId::new(1))] {
                    let layout = TableLayout::new(&seats(n), aspect, focus);
                    for i in 0..layout.slots.len() {
                        for j in (i + 1)..layout.slots.len() {
                            let (a, b) = (&layout.slots[i], &layout.slots[j]);
                            // A mat is never narrower than one card, whatever the
                            // geometry says — a board that cannot hold a single
                            // permanent is not a board. Eight seats on a portrait
                            // canvas reach that floor, and there the mats do meet;
                            // the table has already stopped working by then, and
                            // the honest answer is to seat fewer players, not to
                            // draw a board a card does not fit on.
                            let floored = |s: &SeatSlot| s.lane_width() <= CARD_WIDTH * 2.0 + 1e-3;
                            if floored(a) || floored(b) {
                                continue;
                            }
                            assert!(
                                !grounds_overlap(a, b),
                                "{n} seats at aspect {aspect:.2}, focus {focus:?}: \
                             seats {i} and {j} overlap — {:.2} wide at {:?} \
                             facing {:.2} against {:.2} wide at {:?} facing {:.2}",
                                a.lane_width(),
                                a.center,
                                a.facing,
                                b.lane_width(),
                                b.center,
                                b.facing,
                            );
                        }
                    }
                }
            }
        }
    }

    /// The tightest ring of a given shape that hands every seat the standard
    /// board, and what the camera pays to frame it — found by walking
    /// outwards rather than by bisecting.
    ///
    /// Written out a second time on purpose: a test that asked
    /// [`TableLayout::seated`]'s own search would agree with it however wrong
    /// both were.
    fn tightest(count: usize, aspect: f32, shape: impl Fn(f32) -> Vec2) -> Option<(Vec2, f32)> {
        let half_depth = POD_DEPTH * 0.5;
        let even = vec![1.0; count];
        let floor = half_depth + CENTRE_GAP * 0.5;
        for step in 0..=800_u16 {
            let ry = floor + (MAX_RING_Y - floor) * f32::from(step) / 800.0;
            let radius = shape(ry);
            if radius.x > MAX_RING_X + 1e-3 {
                break;
            }
            let sides = sides_on(count, radius);
            let held = compartment_half(&sides, &even, radius, half_depth);
            let half = pod_half_width(held, radius.x + half_depth);
            if half * 2.0 >= MIN_POD_WIDTH {
                return Some((radius, reach_of(&sides, half, half_depth, aspect)));
            }
        }
        None
    }

    /// What the camera has to swallow to frame a table that was built.
    fn reach_of_layout(layout: &TableLayout, aspect: f32) -> f32 {
        let (lo, hi) = layout.extent().expect("a seated table has an extent");
        let span = hi - lo;
        (span.x / aspect).max(span.y)
    }

    #[test]
    fn three_seats_playing_for_themselves_sit_round_the_table() {
        // A ring shaped to the canvas seats two and four where anybody would
        // sit — opposite each other, or on the four points of a diamond — and
        // three in a gable: the two opponents at 150° and 210°, side by side
        // across the top with their inner corners nearly touching, which is
        // the silhouette a 2v1 draws. Three seats each playing for themselves
        // are a circle, and only a circle says so.
        for aspect in [2.014_f32, 16.0 / 9.0, 1.6, 1.0] {
            let layout = TableLayout::new(&seats(3), aspect, None);
            assert!(
                (layout.radius.x - layout.radius.y).abs() < 1e-3,
                "aspect {aspect:.2}: the ring came out {:?}, which is not round",
                layout.radius
            );
            for (i, slot) in layout.slots.iter().enumerate() {
                let want = core::f32::consts::TAU * i as f32 / 3.0;
                assert!(
                    (slot.facing - want).abs() < 0.01,
                    "aspect {aspect:.2}: seat {i} faces {:.1}°, not the {:.1}° \
                     that would put it a third of the way round",
                    slot.facing.to_degrees(),
                    want.to_degrees()
                );
                assert!(
                    (slot.lane_width() - MIN_POD_WIDTH).abs() < 1e-2,
                    "aspect {aspect:.2}: seat {i} plays on {:.2} units, and the \
                     standard board is {MIN_POD_WIDTH}",
                    slot.lane_width()
                );
            }
            // Taken because it is affordable, and the bound is the one the
            // constant names.
            let shaped = tightest(3, aspect, |ry| ring_for(ry, aspect, POD_DEPTH * 0.5, 1.0))
                .expect("three seats fit on a ring shaped to the canvas");
            assert!(
                reach_of_layout(&layout, aspect) <= shaped.1 * ROUND_COST,
                "aspect {aspect:.2}: sitting round costs {:.1} units of reach \
                 against {:.1} shaped to the canvas, which is past {ROUND_COST}",
                reach_of_layout(&layout, aspect),
                shaped.1
            );
        }
    }

    #[test]
    fn a_table_of_teams_of_one_is_a_free_for_all() {
        // A format that hands every seat a team of its own is not a format
        // with teams in it, and must not be laid out as one.
        for aspect in [2.014_f32, 1.6] {
            let alone = TableLayout::new(&seats(3), aspect, None);
            let labelled = TableLayout::seated(
                &[
                    Seat::on(PlayerId::new(0), Some(1)),
                    Seat::on(PlayerId::new(1), Some(2)),
                    Seat::on(PlayerId::new(2), Some(3)),
                ],
                aspect,
                None,
            );
            assert!(
                (alone.radius - labelled.radius).abs().max_element() < 1e-3,
                "aspect {aspect:.2}: three teams of one came out on {:?}, three \
                 seats alone on {:?}",
                labelled.radius,
                alone.radius
            );
        }
    }

    #[test]
    fn a_bigger_free_for_all_stays_shaped_to_the_canvas() {
        // And it is refused for a reason, not by accident: at four seats and
        // up a round table costs more than [`ROUND_COST`] of what the same
        // seats cost on a ring shaped to the canvas — or it runs past the
        // ring's own ceiling before it has handed anybody a board.
        for n in [4u8, 5, 6, 8] {
            for aspect in [2.014_f32, 16.0 / 9.0, 1.0] {
                let layout = TableLayout::new(&seats(n), aspect, None);
                assert!(
                    (layout.radius.x - layout.radius.y).abs() > 1e-3,
                    "{n} seats at {aspect:.2}: the ring came out round, at {:?}",
                    layout.radius
                );
                let shaped = reach_of_layout(&layout, aspect);
                if let Some((radius, cost)) = tightest(n as usize, aspect, Vec2::splat) {
                    assert!(
                        cost > shaped * ROUND_COST,
                        "{n} seats at {aspect:.2}: a circle of {:.2} would have \
                         cost {cost:.1} units of reach against {shaped:.1}, which \
                         is inside {ROUND_COST} — it should have been taken",
                        radius.x
                    );
                }
            }
        }
    }

    #[test]
    fn a_narrow_canvas_cannot_afford_a_round_table() {
        // The rule is a price and not a seat count, so it answers differently
        // on a canvas taller than it is wide — where a circle is the one
        // shape the camera cannot pay for. A phone held upright is 0.46, the
        // narrowest frame `Metrics::of` draws and the clamp in
        // [`TableLayout::seated`]: a circle wide enough to seat three would
        // cost 56 units of reach against the 22 the table is laid out on. It
        // stays refused all the way up to a canvas nearly square, and three
        // seats on a phone go on sitting where they fit rather than where
        // they would like to.
        for aspect in [0.46_f32, 0.6, 0.75] {
            let layout = TableLayout::new(&seats(3), aspect, None);
            assert!(
                (layout.radius.x - layout.radius.y).abs() > 1e-3,
                "aspect {aspect:.2}: the ring came out round, at {:?}",
                layout.radius
            );
            let shaped = reach_of_layout(&layout, aspect);
            let (radius, cost) =
                tightest(3, aspect, Vec2::splat).expect("a circle seats three at any aspect");
            assert!(
                cost > shaped * ROUND_COST,
                "aspect {aspect:.2}: a circle of {:.2} would have cost {cost:.1} \
                 units of reach against {shaped:.1}, which is inside \
                 {ROUND_COST} — it should have been taken",
                radius.x
            );
        }
    }
}

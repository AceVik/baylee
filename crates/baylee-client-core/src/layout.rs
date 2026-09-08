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
/// where a player would really have them: on the bare timber beside the mat
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
    /// The command zone — commanders, emblems, companions.
    Command,
}

impl PileKind {
    /// All four, in the order they are laid out.
    pub const ALL: [Self; 4] = [Self::Library, Self::Graveyard, Self::Exile, Self::Command];

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
            Self::Library | Self::Graveyard => 1.0,
            Self::Exile | Self::Command => -1.0,
        }
    }

    /// Which lane row the pile stands level with.
    ///
    /// Never [`LaneKind::Creatures`]. That row is the one nearest the middle
    /// of the table, where attackers step forward and blockers come to meet
    /// them; a pile parked at the end of it would be standing in the only
    /// part of the board that moves.
    #[must_use]
    pub const fn row(self) -> LaneKind {
        match self {
            Self::Library | Self::Command => LaneKind::Lands,
            Self::Graveyard | Self::Exile => LaneKind::Support,
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
        }
    }
}

/// How far the centre of a pile stands out past the edge of the ground it
/// serves.
///
/// Half a card, the mat's printed border, and bare timber between the two.
/// The timber is the whole point of the number: a pile touching the mat reads
/// as part of the board. The border being cleared is the client's own
/// `ZONE_MARGIN`, and `a_pile_stands_clear_of_the_mat_it_serves` over there
/// fails if the two ever drift apart.
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
    #[must_use]
    pub fn lane_height(&self) -> f32 {
        self.mat_depth() / LaneKind::ALL.len() as f32
    }

    /// Centre of a lane in table space.
    #[must_use]
    pub fn lane_center(&self, lane: LaneKind) -> Vec2 {
        let index = LaneKind::ALL
            .iter()
            .position(|l| *l == lane)
            .unwrap_or_default() as f32;
        let h = self.lane_height();
        // Lane 0 (creatures) sits towards the table centre, lands at the back.
        let offset_from_front = (index + 0.5) * h - self.half_extent.y;
        let away = Vec2::new(self.facing.sin(), self.facing.cos());
        self.center - away * offset_from_front
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
/// `party` is how many seats the busiest side holds, and it divides the width
/// because a side of two reaches twice as far along itself. Without it a
/// two-headed table came out exactly twice the shape it asked for: the camera
/// fitted it by width, the four boards sat in the top half of the window and
/// the bottom half was bare felt.
fn ring_for(ry: f32, aspect: f32, half_depth: f32, party: usize) -> Vec2 {
    let rx = (aspect / party as f32)
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
    /// every card is drawn smaller for.
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
        // How far along itself the busiest side reaches, in compartments.
        // It shapes the ring, because a side of two is twice as wide as a
        // side of one on the same table.
        let busiest = parties.iter().map(Vec::len).max().unwrap_or(1);
        let narrowest = |ry: f32| {
            let radius = ring_for(ry, aspect, half_depth, busiest);
            let sides = sides_on(t, radius);
            let held = compartment_half(&sides, &even, radius, half_depth);
            pod_half_width(held, radius.x + half_depth) * 2.0
        };
        // Two sides of one seat each are a duel, and a duel has nothing to be
        // crowded by: both seats already have the whole table across, and
        // growing the ring would only push the camera back. Sharing a side is
        // crowding, though, so a two-headed table searches like any other.
        let alone = parties.iter().all(|party| party.len() == 1);
        let ry = if t < 3 && alone {
            clear
        } else {
            // The ceiling, in whichever of the two radii binds first on this
            // canvas: `x` is derived from `y` by the aspect, so a cap on `x`
            // is a cap on `y` once it is read back through the same division.
            let by_x =
                (MAX_RING_X + half_depth + PILE_STRIP) * busiest as f32 / aspect - half_depth;
            let (mut lo, mut hi) = (clear, MAX_RING_Y.min(by_x).max(clear));
            if narrowest(hi) < MIN_POD_WIDTH {
                // Past the cap the camera would have to pull back further
                // than `CameraRig::MAX_DISTANCE`, and a table it cannot frame
                // slides its near mats under the hand bar. Crowded tables
                // live here: they get the biggest ring that can still be
                // seen, and their lanes fan. That is what fanning is for.
                hi
            } else if narrowest(lo) >= MIN_POD_WIDTH {
                lo
            } else {
                for _ in 0..24 {
                    let mid = f32::midpoint(lo, hi);
                    if narrowest(mid) >= MIN_POD_WIDTH {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
                hi
            }
        };
        let radius = ring_for(ry, aspect, half_depth, busiest);
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

/// How deep one seat's ground is: three lanes with a card standing in each,
/// and enough air that a lifted card does not overlap the row behind it.
///
/// A **constant**, and that is the change that made the board fill the
/// screen. While the depth came off `radius.y`, it grew with the ring — so a
/// table laid out for eight seats gave every one of them a deeper mat than a
/// duel did, and a duel, which is what almost every game actually is, got the
/// shallowest board of the lot. A card is the same size at every table, so
/// the ground a card stands on is too.
pub const POD_DEPTH: f32 = CARD_HEIGHT * 3.0 * 1.18;

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
                // crowded by, so their mats sit against the channel and take
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
    fn the_middle_stays_clear_for_the_channel() {
        // The resin channel is the negative form of this layout: it is
        // whatever the mats leave. If the mats close in, there is no channel
        // to draw and the medallion has nowhere to float; if they drift apart,
        // every card is drawn smaller for the empty table between them. Both
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
            // - the mats are as close to the middle as the channel allows;
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
                 {:?} that could still have grown — none of the channel, the \
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
}

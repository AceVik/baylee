//! The firewheel: five flames burning where the colour wheel used to be.
//!
//! The middle of the table carried a medallion — five soft discs of the pie
//! on a ring of worn gold — and the owner's complaint about it was exact:
//! *"Schwarz sieht man hier gar nicht."* A dark disc on a dark table is not
//! a colour, it is a gap, and the wheel was five colours of which one was
//! missing. So the discs are flames now, and the black one is the reason the
//! whole thing is worth doing: a *flame* can be black and still be the most
//! legible thing on the table, because what draws a flame is its edge.
//!
//! Not to be confused with [`crate::tabletop::hearth`], which is the pool of
//! lamplight and the compass ring this wheel stands in. This module draws
//! nothing either: it says where the five stand, what each is made of, how
//! hard each burns given what is on the battlefield, and on what rhythm it
//! flickers. `felt.wgsl` paints it and `table::shader_tests` reads the
//! shader's own text back and fails when the two drift.
//!
//! # Why the flames are painted flat on the cloth
//!
//! The obvious build is five little flames standing up off the table, and it
//! is the wrong one here, for a reason that is measurable rather than
//! aesthetic. The camera never moves and looks down from
//! [`CAMERA_LEAN`](crate::layout) — about 20.6°, or 28.6° at a desktop duel.
//! A *standing* length projects at `sin` of that: a flame 0.72 units tall
//! comes out 11 logical pixels. The same length lying **in the table plane**
//! projects at `cos` of it — 0.67 units, 29 pixels. Flat is 2.7 times the
//! flame for the same number, and at this camera there is nothing on the
//! other side of the trade: a fixed eye gets no parallax from a billboard
//! and no self-occlusion from a raymarched volume, and both of those would
//! pay the same `sin` anyway.
//!
//! What makes a painted flame read as *standing* is a ground cue, which is
//! the argument the falling leaf's shadow already makes in
//! `atmosphere.wgsl`: the **foot** is the brightest and roundest part, the
//! body rises from it along one shared screen-up direction and tapers away,
//! and the **pool** of light is round and centred on the foot rather than on
//! the flame's middle. All five lean the same way, because they are five
//! candles seen from one chair — five flames rising *radially* would read as
//! a sun glyph.
//!
//! # Why light here is painted into the cloth
//!
//! Nothing on this stage is lit. There is no light source, no `Hdr`
//! component on the table camera and no bloom pass, so a value past white is
//! clipped by the 8-bit target rather than blooming — `feltmat::WASH_GAIN`
//! records that mistake in full. So the pools are *added* to the felt inside
//! the felt's own shader, after the sky, exactly where the rail lamp's glow
//! is added. That is also why they are not a quad blended over the table: an
//! added pool lets the weave, the grain and the mineral vein show through
//! it, and that is the whole difference between light and a decal.
//!
//! # The black flame
//!
//! Every other flame is *emitted* and the black one is *subtracted*, and it
//! is otherwise the same object — same silhouette, same foot, same rhythm
//! family. Its body multiplies the cloth down to [`BLACK_BODY`], a hole
//! rather than a colour; the light that says what shape the hole is lives on
//! its [`BLACK_RIM`], which is how a backlit black flame is drawn everywhere
//! it is drawn well; and the light it throws on the table is cold, dim and
//! preceded by a shadow at its foot.
//!
//! The rim is capped rather than pushed: at [`BLACK_RIM`]'s luma it stays
//! under the white flame's body. A brighter violet outline is the first
//! thing anyone reaches for and it is a neon sticker — the opposite of what
//! black needs.

use crate::layout::CENTRE_GAP;

/// How many flames stand on the wheel: one per colour of the pie.
pub const FIRES: usize = 5;

/// White, and the order the other four follow it in.
pub const WHITE: usize = 0;
/// Blue.
pub const BLUE: usize = 1;
/// Black — the one that is subtracted rather than added.
pub const BLACK: usize = 2;
/// Red.
pub const RED: usize = 3;
/// Green.
pub const GREEN: usize = 4;

/// How far out each flame's foot stands from the middle, in table units.
///
/// The medallion's own glows sat here (0.62 of a 1.1-unit half-width) and
/// that is kept: the ring of five is the arrangement every player already
/// has in their head, and moving it would cost the one thing the ornament is
/// actually for. Neighbouring feet come out 0.80 units apart.
pub const FOOT_RADIUS: f32 = 0.68;

/// How far from the middle of the table anything on this wheel may reach.
///
/// The branch in `felt.wgsl` is `length(table) < FLAME_REACH`, so this is
/// both the budget and the optimisation: outside it the cloth is the cloth,
/// and the five flames cost nothing at all over 99% of a thirty-five-unit
/// slab.
pub const FLAME_REACH: f32 = 1.6;

/// Where flame `n`'s foot stands, in table units: `(x, y)` with `y` running
/// **away from the viewer**, which is the table space `felt.wgsl` works in.
///
/// White at the top of the screen and then clockwise, which is the wheel as
/// it is printed on a card.
#[must_use]
pub fn foot_at(n: usize) -> (f32, f32) {
    let at = core::f32::consts::FRAC_PI_2 - core::f32::consts::TAU * (n as f32) / FIRES as f32;
    (at.cos() * FOOT_RADIUS, at.sin() * FOOT_RADIUS)
}

/// How tall a flame at zero stands, in table units — the pilot light.
///
/// A colour nobody is playing still has a place on the wheel. Five flames
/// are a compass and four are a broken one, and a red player looking at
/// their own colour gone out in the middle of the table would be reading
/// something the felt has no business saying. So an unplayed colour banks
/// down — shorter, no core, half the flicker, half the light — and stays
/// alight.
pub const HEIGHT_FLOOR: f32 = 0.40;

/// How much taller than that a flame at full strength stands.
///
/// The ceiling is 0.82 and comes from the layout rather than from taste: the
/// black and red flames rise *toward* the middle, and at 0.82 their tips
/// pass 0.25 units from the blue and green feet —
/// `the_inward_flames_stop_short_of_their_neighbours_feet` is that bound.
pub const HEIGHT_SPAN: f32 = 0.42;

/// Half-width of the pilot light, and how much wider a full flame runs.
pub const WIDTH_FLOOR: f32 = 0.10;
/// How much of a half-width strength adds.
pub const WIDTH_SPAN: f32 = 0.05;

/// How tall flame `n` stands at strength `s`, in table units.
#[must_use]
pub fn height(s: f32) -> f32 {
    HEIGHT_FLOOR + HEIGHT_SPAN * s.clamp(0.0, 1.0)
}

/// How wide flame `n` runs at strength `s` — a half-width, in table units.
#[must_use]
pub fn width(s: f32) -> f32 {
    WIDTH_FLOOR + WIDTH_SPAN * s.clamp(0.0, 1.0)
}

// The three bounds the layout imposes, checked where they are written
// because both sides of each are constants. A later "make them bigger"
// breaks all three at once and silently.
const _: () = assert!(2.0 * FLAME_REACH < CENTRE_GAP);
const _: () = assert!(FOOT_RADIUS + HEIGHT_FLOOR + HEIGHT_SPAN < FLAME_REACH);
const _: () = assert!(FOOT_RADIUS + POOL_OUTER < FLAME_REACH);

/// The hottest part of each flame, low in the body.
///
/// Pale rather than saturated, which is the half of a fire people forget
/// they know: a candle is white in its middle and takes its colour on the
/// way out. Black's is dim violet instead — see the module header.
pub const CORE: [[f32; 3]; FIRES] = [
    [1.00, 0.98, 0.90], // white
    [0.86, 0.94, 1.00], // blue
    [0.42, 0.30, 0.55], // black — the dimmest core on the wheel
    [1.00, 0.90, 0.70], // red
    [0.90, 1.00, 0.82], // green
];

/// The body of each flame: the colour a player reads it as.
///
/// `tabletop::PIE` lifted about 8% so a body sits clear of the felt's own
/// luma — except white's, which is *pulled down*: parchment at 0.91 would be
/// the brightest thing on the table, and this wheel has already been dimmed
/// twice for exactly that.
pub const BODY: [[f32; 3]; FIRES] = [
    [0.94, 0.88, 0.70], // white
    [0.36, 0.62, 0.92], // blue
    [0.00, 0.00, 0.00], // black — not painted at all; see BLACK_BODY
    [0.90, 0.38, 0.22], // red
    [0.40, 0.72, 0.42], // green
];

/// The cooling skirt at each flame's outside.
///
/// It shifts **hue** and not only value, and that is the one thing that
/// stops a flame being a flat coloured blob at thirty pixels: a real flame
/// is a different colour where it is cooling, and a salted one (copper
/// green, potassium lilac) keeps its hue in the body while the skirt falls
/// away from it.
pub const SKIRT: [[f32; 3]; FIRES] = [
    [0.85, 0.55, 0.22], // white — amber, the one honest candle
    [0.18, 0.24, 0.62], // blue  — indigo
    [0.62, 0.52, 0.78], // black — the rim, and its brightest part
    [0.55, 0.10, 0.20], // red   — crimson
    [0.10, 0.36, 0.34], // green — teal
];

/// What the black flame's body multiplies the cloth down to.
///
/// A hole, slightly violet. The felt in the middle of the table sits at
/// about sRGB8 (31, 48, 52); inside the body it goes to (12, 17, 20). Not
/// zero, because a black *disc* is exactly the thing the owner could not
/// see: what is wanted is cloth that has been darkened, so the grain still
/// shows through the middle of the flame and the rim has something to stand
/// against.
pub const BLACK_BODY: [f32; 3] = [0.32, 0.30, 0.38];

/// The black flame's rim: the light that says what shape the hole is.
///
/// This is [`SKIRT`]`[BLACK]`, named because the black flame reverses which
/// stop is brightest — core in the middle, body darkest, skirt brightest —
/// and a reader who assumed the usual order would get it backwards.
pub const BLACK_RIM: [f32; 3] = SKIRT[BLACK];

/// How far down the black flame the rim reaches, as a fraction of its
/// height: nothing at the foot, full at the tip.
pub const BLACK_RIM_FROM: f32 = 0.15;
/// Where the rim is at full strength.
pub const BLACK_RIM_TO: f32 = 0.90;

/// The colour of the little cold light the black flame throws.
pub const BLACK_POOL: [f32; 3] = [0.30, 0.24, 0.42];

/// How much of the others' light the black flame throws.
pub const BLACK_POOL_GAIN: f32 = 0.35;

/// How dark the cloth goes right at the black flame's foot.
///
/// The additive slot cannot subtract, so black's light is two stages: this
/// multiply first, then a pool in [`BLACK_POOL`] added over it.
pub const BLACK_SHADOW: f32 = 0.62;
/// How far that shadow spreads, in table units.
pub const BLACK_SHADOW_REACH: f32 = 0.30;

/// The hot little pool right at a flame's foot: how far it reaches.
///
/// Under 0.25 deliberately. The black and red flames lean toward the middle
/// and their tips pass 0.25 units from the blue and green feet, so a wider
/// hot pool would put blue's light on black's tip.
pub const POOL_INNER: f32 = 0.20;
/// And the wide faint one around it.
pub const POOL_OUTER: f32 = 0.80;

/// How much light the inner pool adds, in **linear** — not display.
///
/// Stated in linear because that is what the felt's last line does: the
/// pools are added after the sky, the way the rail lamp's glow is. A number
/// written as though it were display-referred lands at a different lift on
/// cloth, on worn cloth and in the shadow, and this table has made that
/// mistake before (`felt.wgsl`'s own note: a surface meant for 0.22 measured
/// 0.45).
///
/// It was 0.020 on the first pass, which the brief costed at about +13 sRGB
/// levels — and at +13 levels there was no light on the table at all: the
/// flames stood on bare cloth in the first screenshot and read as stickers.
/// The number that shows is this one, and it is still a light rather than a
/// lamp, because what sells it is the *fall-off* and not the peak.
pub const POOL_INNER_GAIN: f32 = 0.100;
/// And the outer pool's, which is what actually says "there is a fire here".
pub const POOL_OUTER_GAIN: f32 = 0.042;

/// How far the bright foot spreads, in table units: the inner edge and the
/// outer.
///
/// Small, and smaller than it first was. A foot is the hottest part of a
/// flame and the temptation is to make it the *brightest* — which draws a
/// headlight with a flame on top of it. What it has to do is much less than
/// that: be round where the body is not, so the eye reads the flame as
/// standing on something.
pub const SOLE_INNER: f32 = 0.035;
/// Where the foot has faded out entirely.
pub const SOLE_OUTER: f32 = 0.075;
/// How much heat the foot contributes — how white its middle goes.
pub const SOLE_HEAT: f32 = 0.35;

/// How opaque a flame's coolest edge is.
///
/// Below one, and that is what makes a flame a flame rather than a decal
/// cut to the shape of one: real fire is something you see the wall through,
/// and the skirt is the part you see most through. The core stays opaque —
/// `mix(VEIL, 1, heat)` — so the flame is solid where it is hot and thins
/// where it is cooling, which is also the order a painter would put them in.
pub const VEIL: f32 = 0.62;

/// How much the shape noise is stretched before it eats the silhouette.
///
/// Four octaves normalised to 0..1 cluster hard around a half, so
/// `n - EROSION_FLOOR` averages 0.15 and almost never reaches 0.5. Read
/// straight, the erosion took a sixth of the tip and the flames came out as
/// gel capsules — the first screenshot is exactly that. Doubling the
/// deviation is what turns it into a lick.
pub const EROSION_GAIN: f32 = 2.0;

/// How many sources of one colour, per seat, bring a flame to full height.
///
/// The curve is `(sources / (PER_SEAT * seats)).powf(0.6)`, clamped — so
/// eight sources at a duel is a full flame and a ring of eight needs
/// proportionally more, which is what stops a big table saturating every
/// flame on turn three.
///
/// **Concave on purpose**: the exponent is below one, so the first source of
/// a colour is the one that shows. One Forest at a duel already lifts the
/// pilot to 0.36, four reach 0.66. A linear ramp would leave the first half
/// of a game with five flames that all look unplayed.
pub const PER_SEAT: f32 = 4.0;

/// The shape of that curve.
pub const SHARPNESS: f32 = 0.6;

/// Turn a tally of coloured sources into how hard each flame burns, 0 to 1.
///
/// The tally is what `baylee-client::manasources::table_mana` counts:
/// permanents that can make a colour, every seat's, a source of several
/// colours split between them.
///
/// **Sources and not devotion**, which is a choice worth recording because
/// the alternative is defensible. Devotion (CR 700.5 — the coloured mana
/// symbols on the permanents a player controls) moves with what is being
/// *played* rather than with what is being tapped, and it separates two
/// colours that a source count reads as identical from turn three. What
/// decides it here is that the owner asked for the other one in so many
/// words — *"Umso mehr Mana einer bestimmten Farbe auf dem Feld liegt"* —
/// and that devotion is not on the wire at all: `PublicObject` carries
/// `colors` and `mana_value` but no pips, so it would cost a view field and
/// a `VIEW_VERSION` bump to answer honestly. If it is ever wanted, the
/// counter changes and this curve does not.
///
/// Turn count was the owner's other suggestion and is rejected as the
/// *height* source: it raises all five together, and five equal flames at
/// any height look like the same table. It arrives anyway, through the door
/// that makes sense — a game that has run longer has more lands on it.
#[must_use]
pub fn strength(tally: [f32; FIRES], seats: usize) -> [f32; FIRES] {
    let full = PER_SEAT * (seats.max(1) as f32);
    let mut out = [0.0_f32; FIRES];
    for (slot, sources) in out.iter_mut().zip(tally) {
        *slot = (sources.max(0.0) / full).clamp(0.0, 1.0).powf(SHARPNESS);
    }
    out
}

/// How fast each flame gutters, in Hz — its own rhythm, and its character.
///
/// Five rates that share no common multiple, because five flames on one
/// clock are a strobe: the eye finds the beat and then cannot let it go.
/// They also *say* something — blue is gas and barely moves, black is smoky
/// and lazy, red is restless.
///
/// All five are under 1.5 Hz, which is a bound and not an accident. This
/// table refuses motion a card-reader catches sideways, and a flicker
/// between two battlefields is the one place that rule is being asked to
/// bend. What carries "alive" is the noise scrolling *inside* a silhouette
/// 13 by 31 pixels across, not the envelope.
/// Written to three places, and that is the finding rather than a fussy
/// spelling. The first draft of this table was `[0.9, 1.4, 0.6, 1.2, 0.8]`
/// — five characters, correctly ranked, and every one of them a multiple of
/// a tenth, so the whole wheel came back to exactly where it started every
/// **ten seconds** and did it for as long as the game lasted. The test below
/// measured it as a repeat to within nothing at all. Numerators that are
/// mutually prime put that back beyond a thousand seconds.
pub const RATE: [f32; FIRES] = [0.907, 1.433, 0.581, 1.193, 0.769];

/// How much of its height each flame's gutter takes, at full strength.
///
/// Ten percent at the most, for the reason above.
pub const DEPTH: [f32; FIRES] = [0.05, 0.03, 0.10, 0.09, 0.06];

/// How fast the shape noise scrolls up each flame, in table units a second.
///
/// Blue's is faster and its envelope is nearly flat: a gas flame hisses
/// rather than gutters, and the difference between the two is exactly which
/// of these two numbers carries it.
pub const SCROLL: [f32; FIRES] = [1.6, 2.2, 1.6, 1.6, 1.6];

/// How hard the noise eats into each flame from the top down.
///
/// This is what makes a lick. Where a high band of noise crosses the
/// silhouette at about seven tenths of its height with a low band above it,
/// the shape pinches and a tongue comes off — and at thirty pixels that is
/// the *one* thing that reads as fire rather than as a coloured leaf. Black
/// erodes harder than the rest, so its tip breaks into smoke.
pub const EROSION: [f32; FIRES] = [2.2, 2.2, 2.8, 2.2, 2.2];

/// How much of the silhouette survives the erosion on average.
pub const EROSION_FLOOR: f32 = 0.35;

/// The draught in the room, in seconds: every flame leans into it together.
///
/// Shared, and that is the point — five independent flames are five looping
/// gifs, five flames that swell together are five flames in one room. Nine
/// and a half seconds against the seat mat's seven and eleven, so nothing on
/// this table ever comes back into step with anything else on it.
pub const DRAUGHT_PERIOD: f32 = 9.5;
/// How much of a flame's height the draught takes.
pub const DRAUGHT_DEPTH: f32 = 0.05;

/// How often red flares, in seconds — and how often black sheds its tip.
///
/// Two events, each on its own period, each shaped by `mark_event` over a
/// small fraction of it. They are what stop the wheel being five sine waves:
/// something happens, rarely, and it is different for each colour.
pub const FLARE_PERIOD: f32 = 7.3;
/// How often the black flame pinches off a wisp of smoke, in seconds.
pub const PINCH_PERIOD: f32 = 8.3;
/// How often the white flame gutters down and recovers, in seconds.
pub const GUTTER_PERIOD: f32 = 13.7;

/// Where the clock is parked when the player has asked for no motion.
///
/// A chosen pose rather than zero: at `t = 0` every noise field is at its
/// own origin, which is a shape nobody picked. This is a still flame with a
/// good tip on it.
pub const STILL_AT: f32 = 2.3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unplayed_colour_banks_down_to_a_pilot_light_and_never_goes_out() {
        let dark = strength([0.0; FIRES], 2);
        for (n, s) in dark.iter().enumerate() {
            assert!(s.abs() < 1e-6, "flame {n} is at {s} on an empty table");
        }
        assert!(
            height(0.0) > 0.0 && width(0.0) > 0.0,
            "a wheel with a hole in it is the old medallion again"
        );
    }

    #[test]
    fn a_flame_climbs_with_its_colour_and_tops_out() {
        let mut last = 0.0;
        for sources in [1.0_f32, 2.0, 4.0, 8.0] {
            let mut tally = [0.0; FIRES];
            tally[RED] = sources;
            let s = strength(tally, 2)[RED];
            assert!(s > last, "{sources} sources did not out-burn {last}");
            last = s;
        }
        assert!(
            (last - 1.0).abs() < 1e-6,
            "eight sources reached only {last}"
        );
        // And nothing past it climbs any further, or a ramping deck would
        // grow a flame out of the wheel.
        let over = strength([0.0, 0.0, 0.0, 40.0, 0.0], 2)[RED];
        assert!((over - 1.0).abs() < 1e-6, "forty sources reached {over}");
    }

    /// The concave curve, stated as the thing it is for: the first source of
    /// a colour has to be visible, or half a game is five pilot lights.
    #[test]
    fn the_first_source_of_a_colour_is_the_one_that_shows() {
        let one = strength([1.0, 0.0, 0.0, 0.0, 0.0], 2)[WHITE];
        let linear = 1.0 / (PER_SEAT * 2.0);
        assert!(
            one > linear * 2.0,
            "one source lifts the flame to {one}, barely past the linear {linear}"
        );
    }

    /// A bigger table needs more of a colour for the same flame, or a ring
    /// of eight saturates every one of them on turn three.
    #[test]
    fn a_bigger_table_asks_for_more_of_a_colour() {
        let duel = strength([4.0, 0.0, 0.0, 0.0, 0.0], 2)[WHITE];
        let ring = strength([4.0, 0.0, 0.0, 0.0, 0.0], 8)[WHITE];
        assert!(
            ring < duel,
            "eight seats burnt as hard as two: {ring} vs {duel}"
        );
        // Four times the seats, four times the sources, the same flame.
        let fed = strength([16.0, 0.0, 0.0, 0.0, 0.0], 8)[WHITE];
        assert!((fed - duel).abs() < 1e-5, "{fed} against {duel}");
    }

    #[test]
    fn a_negative_tally_cannot_put_a_flame_below_its_floor() {
        let s = strength([-3.0, 0.0, 0.0, 0.0, 0.0], 2)[WHITE];
        assert!(s.abs() < 1e-6, "a negative count burnt at {s}");
        assert!((height(s) - HEIGHT_FLOOR).abs() < 1e-6);
    }

    /// The five stand on the wheel every player already knows, white at the
    /// top of the screen and the rest clockwise from it.
    #[test]
    fn the_five_stand_on_the_wheel_every_player_already_knows() {
        let (x, y) = foot_at(WHITE);
        assert!(x.abs() < 1e-6, "white is not at the top: {x}");
        assert!((y - FOOT_RADIUS).abs() < 1e-6, "white is off the rim: {y}");
        for n in 0..FIRES {
            let (x, y) = foot_at(n);
            assert!(
                ((x * x + y * y).sqrt() - FOOT_RADIUS).abs() < 1e-5,
                "flame {n} is off the ring"
            );
        }
        // Clockwise on screen: blue to white's right, and lower.
        let (bx, by) = foot_at(BLUE);
        assert!(
            bx > 0.0 && by < FOOT_RADIUS,
            "blue is not clockwise of white"
        );
        // And the two that lean inward are the bottom pair.
        assert!(foot_at(BLACK).1 < 0.0 && foot_at(RED).1 < 0.0);
    }

    /// The bound that a later "make them bigger" would break in silence.
    ///
    /// The black and red flames rise toward the middle of the wheel, so
    /// their tips travel *at* the blue and green feet. What keeps the hot
    /// pool off another flame's tip is that this gap stays wider than
    /// [`POOL_INNER`] — and the gap is a function of [`HEIGHT_SPAN`], which
    /// is the number somebody will reach for first.
    #[test]
    fn the_inward_flames_stop_short_of_their_neighbours_feet() {
        let tip_y = foot_at(BLACK).1 + height(1.0);
        let tip = (foot_at(BLACK).0, tip_y);
        let neighbour = foot_at(BLUE);
        let gap = ((tip.0 - neighbour.0).powi(2) + (tip.1 - neighbour.1).powi(2)).sqrt();
        assert!(
            gap > POOL_INNER,
            "black's tip comes within {gap} of blue's foot, inside a {POOL_INNER} pool"
        );
    }

    /// Five rhythms that share no multiple, measured rather than asserted
    /// about — and **with the finding it can see injected**, because the
    /// interesting number is not zero. Flames that are genuinely unrelated
    /// do occasionally all reach the top together; what must not happen is
    /// that they do it on a beat.
    #[test]
    fn the_five_gutter_together_only_by_accident() {
        fn together(rates: [f32; FIRES], seeds: [f32; FIRES]) -> usize {
            (0..120 * 60)
                .filter(|frame| {
                    let t = *frame as f32 / 60.0;
                    rates.iter().zip(seeds).all(|(rate, seed)| {
                        // Each flame's noise is read at its own seed, so the
                        // phases are offset as well as the rates.
                        let phase = (t * rate + seed).fract();
                        phase < 0.08 || phase > 0.92
                    })
                })
                .count()
        }
        let seeds = [0.0, 0.37, 0.74, 1.11, 1.48];
        let real = together(RATE, seeds);
        assert!(
            real * 500 < 120 * 60,
            "the five came to the top together on {real} frames in two minutes"
        );
        // The finding this can see, injected: five flames on one rate and
        // one seed, which is what a first draft writes.
        let locked = together([RATE[0]; FIRES], [0.0; FIRES]);
        assert!(
            locked * 500 >= 120 * 60,
            "one shared rate only strobed {locked} frames — the check cannot \
             see the thing it is for"
        );
    }

    /// And the whole wheel does not come back round inside a game.
    #[test]
    fn the_rhythm_does_not_repeat_inside_a_game() {
        let mut closest = 1.0_f32;
        // From five seconds on: every cycle is at its start at zero, and
        // that is the beginning rather than a repeat.
        for frame in (5 * 120)..=(600 * 120) {
            let t = frame as f32 / 120.0;
            let apart = RATE
                .iter()
                .map(|rate| {
                    let phase = (t * rate).fract();
                    phase.min(1.0 - phase)
                })
                .fold(0.0_f32, f32::max);
            closest = closest.min(apart);
        }
        assert!(
            closest > 0.02,
            "the wheel repeats to within {closest} of a cycle inside ten minutes"
        );
    }

    /// The draught is shared, so it may not be a multiple of anything a
    /// single flame does — that is what would make all five breathe on one
    /// flame's beat.
    #[test]
    fn the_draught_is_not_in_step_with_any_one_flame() {
        for (n, rate) in RATE.iter().enumerate() {
            let cycles = DRAUGHT_PERIOD * rate;
            let off = (cycles - cycles.round()).abs();
            assert!(
                off > 0.1,
                "flame {n} fits {cycles} gutters into one draught, which is a beat"
            );
        }
        for (name, period) in [
            ("flare", FLARE_PERIOD),
            ("pinch", PINCH_PERIOD),
            ("gutter", GUTTER_PERIOD),
        ] {
            let ratio = period / DRAUGHT_PERIOD;
            assert!(
                (ratio - ratio.round()).abs() > 0.1,
                "the {name} event lands on the draught every {ratio} of it"
            );
        }
    }

    /// Black is the one flame whose brightest part is its outside, and the
    /// rim is capped rather than pushed.
    #[test]
    fn the_black_flame_is_drawn_by_its_edge_and_does_not_shout() {
        fn luma(c: [f32; 3]) -> f32 {
            0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
        }
        assert!(
            luma(BLACK_RIM) > luma(CORE[BLACK]),
            "the black flame is brighter in its middle than at its edge"
        );
        assert!(
            luma(BLACK_RIM) < luma(BODY[WHITE]),
            "the violet rim ({}) out-shouts the white flame's body ({})",
            luma(BLACK_RIM),
            luma(BODY[WHITE])
        );
        // The body is a multiply and every channel of it has to darken.
        for (n, channel) in BLACK_BODY.iter().enumerate() {
            assert!(
                (0.0..1.0).contains(channel),
                "channel {n} of the black body is {channel}, which brightens the cloth"
            );
        }
        assert!((0.0..1.0).contains(&BLACK_SHADOW));
        // Every other flame is painted; black's body is not.
        for (n, body) in BODY.iter().enumerate() {
            let brightest = body.iter().copied().fold(0.0_f32, f32::max);
            if n == BLACK {
                assert!(brightest < 1e-6, "the black body is painted at {brightest}");
            } else {
                assert!(
                    brightest > 0.3,
                    "flame {n}'s body is not a colour: {brightest}"
                );
            }
        }
    }

    /// White is the one colour of the pie that has to be *pulled down*.
    #[test]
    fn the_white_flame_is_not_the_brightest_thing_on_the_table() {
        fn luma(c: [f32; 3]) -> f32 {
            0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
        }
        assert!(
            luma(BODY[WHITE]) < luma(crate::tabletop::PIE[WHITE]),
            "white's body was taken straight from the pie"
        );
        // And every skirt is darker than its body, which is what a skirt is.
        for n in 0..FIRES {
            if n == BLACK {
                continue;
            }
            assert!(
                luma(SKIRT[n]) < luma(BODY[n]),
                "flame {n}'s skirt is brighter than its body"
            );
        }
    }
}

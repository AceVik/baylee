// The table's skirt: the ground the hand is held against, and the rail it
// hangs from.
//
// The felt is an altar (`felt.wgsl`: "an altar's edge, not a tray's") and the
// actions row is its front edge. What hangs from an altar's front edge is a
// frontal — an antependium — and this is that cloth **let down flat**: one
// dark gauze from the shelf's lip to the bottom of the window and off the
// frame, sheer enough that the sky is still seen through it, with the
// player's cards held up in front of it.
//
// It paints **two** nodes, not one, and that is the whole shape of this file.
// The actions row and the hand zone are siblings rather than parent and child
// because the row has to stand *over* the zone dialog's veil and the zone
// under it (`hud::Z_LEDGE` against `hud::Z_HAND`), so one node cannot be
// both. They share this shader, one pair of clocks and one fold field indexed
// on **window** x, which is what makes them read as one piece of cloth with a
// rail across the top rather than as two surfaces that happen to match.
//
// Nothing here is a light, and nothing here is a surface a card lies *on*. It
// is cloth in the shelf's shadow, and the brightest pixel on it stays under
// everything on a card and under the question's ink.
//
// # Why it is a ground at all, and the scar it has to stay clear of
//
// This strip was once an opaque 88 % black panel the width of the window, and
// the owner had it removed: a hand of cards laid on a black strip is a hand of
// cards in a *panel*, which is the one thing on this screen that must not read
// as an interface. What went with it was the dark ground each card's glow was
// read against, and the glow was then reported as missing. Two promises hold
// or this is a mistake: **the table still shows through**, and **a card's glow
// still has something to be read against**.
//
// The second one is what the swags this shader used to draw could not keep.
// Measured in the running client against a day sky: the sheer body left the
// bottom of the window reading (138, 153, 162) against a sky of (185, 213,
// 236), so two thirds of the zone simply *were* the sky and the cards floated
// on it. The owner's word for that is "not complete", and his instruction is
// a container. The first promise is what the density stops short of: at 0.82
// the day sky's clouds stay legible through the cloth, and over felt or at
// night the ground is (26, 25, 19) — `palette::DIALOG` itself, which is where
// the dialog register the owner asked for comes from.
//
// # Why there is no edge anywhere but the top
//
// No hem, no scallop, no floor line. The zone runs off the window on three
// sides, so a drawn bottom would be a tray's inner wall. What makes it read
// as a container is two things: the rail is the top edge of one continuous
// surface, and the ground runs unbroken off the bottom of the frame instead
// of dissolving into sky halfway down.
//
// The **top two corners** are the exception, and were not one until the owner
// asked for them on 14.09.2026. The paragraph they overrule said a rounded
// corner would be a claim that a 3200-pixel band is an object; the top edge is
// the only one of the four that is an edge at all, and rounding where it meets
// the frame is what keeps the whole controls bar reading as a surface laid
// against the bottom of the window. `frontal::CORNER` carries the rest.
//
// # Why the folds are one-dimensional, and straight
//
// A hanging fold is vertical over its whole length, and this surface is 190
// pixels tall and up to 3200 wide. An isotropic field in a strip that thin
// reads as a flat wash: there is no room across it for a two-dimensional
// feature to be seen. So the folds are `fbm1` over x alone — and straight,
// with no `y` term at all, because a fold that leaned differently at
// different depths would meet the seam between the two nodes at two different
// places and tear the one field in half.
//
// # The clocks
//
// Two, and both of them already on this screen: `mat.wgsl` breathes a seat's
// rim every 7 seconds and travels a swell round it every 11. Borrowing those
// two adds no new tempo — a screen with one tempo is a room and a screen with
// six is a fairground. The 7 drives one factor that deepens the folds, dyes
// them and darkens the rail *together*, so the cloth breathes once rather
// than in three places; the 11 leans the fold field, outward from the middle
// so the owner's mirror symmetry survives. Nothing travels *along* the rail:
// a lit feature crossing behind the buttons would read as passing through
// them, and 3200 pixels in 11 seconds is a chase rather than a breath.
//
// `energy` scales every sine and nothing else, so `reduce_motion` leaves the
// cloth at the middle of its movement rather than wherever a frozen sine
// happened to stop it: the end of the movement and not its absence, as
// `hud::motion` puts it.

#import bevy_render::globals::Globals
#import bevy_ui::ui_vertex_output::UiVertexOutput
#import "embedded://baylee_client/shaders/noise.wgsl"::fbm1

struct FrontalParams {
    /// The one dye, in **linear** light; `w` is how far the folds move it.
    dye: vec4<f32>,
    /// The lip's dye, also linear; `w` is how many pixels of it there are,
    /// and zero is a surface with no lip.
    lip: vec4<f32>,
    /// Where the ground stops being flat (`x`, pixels from this node's top),
    /// how dense it is above that (`y`), how dense at the window's bottom
    /// edge (`z`), and how far the whole surface breathes (`w`).
    ramp: vec4<f32>,
    /// How far the folds move the density.
    grain: f32,
    /// How far the two movements travel. 0 leaves the cloth still.
    energy: f32,
    /// Width over height of the node, so the folds are pixels and not a
    /// fraction of the window.
    aspect: f32,
    /// The node's height in logical pixels — every number below is in them.
    height: f32,
    /// How far the **top two** corners are rounded, in pixels.
    corner: f32,
}

@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> params: FrontalParams;

const TAU: f32 = 6.2831853;

/// How wide a fold is.
///
/// Deliberately sharing no small multiple with `hud::hand_layout`'s card step,
/// which caps at 100: a fold that lined up with the cards would read as
/// scaffolding behind them rather than as cloth they are held in front of.
/// 100 / 56 is 1.79, which is about as far from a whole number as it gets.
const FOLD_PX: f32 = 56.0;

/// How the density falls from the shoulder to the window's bottom edge.
///
/// Above one, so the change is spent in the last third — where the card
/// bottoms are, and where there is least table left to see through the cloth.
const RAMP: f32 = 2.2;

/// The breath, in seconds, and how far it swings the fold depth.
///
/// The factor runs 0.5 to 1.5, so the folds surface and sink rather than
/// merely brightening: at the trough there is half the texture there is at
/// rest and at the peak half again as much.
const BREATH_SECONDS: f32 = 7.0;
const BREATH: f32 = 0.5;

/// The swell, in seconds, how far it leans the fold field, and over how many
/// pixels its phase turns.
///
/// The phase is on `abs(x)`, which keeps the cloth mirror-symmetric about the
/// window's middle — the owner asked for symmetry, and symmetry that is
/// arithmetic cannot drift — and makes it stir outwards from the centre
/// instead of sliding sideways as one sheet.
const SWELL_SECONDS: f32 = 11.0;
const SWELL_PX: f32 = 6.0;
const SWELL_SPREAD: f32 = 1400.0;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let h = params.height;
    // Pixels from the **window's** middle, and pixels down from this node's
    // own top. The first is `(uv.x - 0.5) * aspect * height`, which is
    // `(uv.x - 0.5) * width` — the same number in both nodes at the same
    // place on screen, whatever their heights, which is what lets one fold
    // field run out of the rail into the skirt without a seam.
    let x = (in.uv.x - 0.5) * params.aspect * h;
    let y = in.uv.y * h;
    let t = globals.time;

    // The breath and the swell: one factor and one offset, shared by
    // everything that moves below.
    let wave = sin(TAU * t / BREATH_SECONDS) * params.energy;
    let breath = 1.0 + BREATH * wave;
    let lean = SWELL_PX
        * sin(TAU * t / SWELL_SECONDS + abs(x) / SWELL_SPREAD)
        * params.energy;
    let fold = fbm1((x + lean) / FOLD_PX) - 0.5;

    // The ground: flat down to the shoulder, then falling away to the foot.
    // On the rail the shoulder is its whole height, so the ramp never starts
    // and the two ends are the same number.
    let fall = clamp((y - params.ramp.x) / max(h - params.ramp.x, 1.0), 0.0, 1.0);
    var alpha = params.ramp.y + (params.ramp.z - params.ramp.y) * pow(fall, RAMP);
    alpha = alpha + params.ramp.w * wave;
    alpha = alpha + 2.0 * fold * params.grain * breath;

    // The weave. A **multiply**, and zero-mean, so the folds surface and sink
    // without the ground's own colour ever moving: a ground whose mean walked
    // about under a card would read as the card's own glow pulsing, and that
    // glow is a rules statement.
    let dye = params.dye.rgb * (1.0 + 2.0 * fold * params.dye.w * breath);

    // The **top two** corners, rounded, and not the bottom two: the owner
    // asked for them on 14.09.2026, and the cloth runs off the bottom of the
    // window, where a rounded corner would be a notch cut out of the frame.
    // Cut here rather than left to `BorderRadius`, because a `MaterialNode`
    // is drawn by this shader and what the UI pass does with a radius on one
    // is a question — the same reason the lip is painted here.
    //
    // The centre is clamped along the top edge, so `out` is the distance from
    // one circle between the corners and from a straight line between them:
    // for `across` in `[r, w - r]` the centre sits directly above at
    // `(across, r)` and `out` collapses to `r - y`. That is what lets one
    // number serve both the cut and the lip below.
    let w = params.aspect * h;
    let across = in.uv.x * w;
    let r = params.corner;
    let rounding = r > 0.0 && y < r;
    let centre = vec2<f32>(clamp(across, r, max(w - r, r)), r);
    let out = distance(vec2<f32>(across, y), centre);
    // Half a device pixel, in this shader's own units. Taken here and not
    // inside the branch below: a derivative asked for under non-uniform
    // control flow is undefined, and `rounding` is per-fragment.
    let aa = max(0.5 * fwidth(out), 0.0001);

    // How far under the surface's own top edge this fragment lies — measured
    // from the **rounded** edge where there is one, which is the whole point.
    // `y` alone is the distance from the node's top *row*, and a lip drawn on
    // that is a straight band the corner then cuts off flat: the bright line
    // stopped dead about five pixels short of each end and left a blunt
    // corner behind it, which is the "seltsame Kante" the owner reported on
    // 14.09.2026. Measured before the change at 3008 x 1630: the lip ran from
    // x = 16 to the far edge and the arc left of it carried none of it.
    let depth = select(y, r - out, rounding);

    // The lip: the one line on either surface, and the place the table stops.
    // Opaque on purpose, and painted here rather than left to `BorderColor`
    // because a `MaterialNode` is what draws this node and whether a border
    // survives on one is a question rather than a given.
    // `depth >= 0.0` is what keeps this off the skirt, whose `lip.w` is zero
    // and whose `lip.rgb` is therefore black: without it every fragment
    // *outside* the arc would satisfy `depth < 0` and the corner's own
    // anti-aliased rim would be painted black instead of dyed.
    let on_the_lip = select(0.0, 1.0, depth >= 0.0 && depth < params.lip.w);
    alpha = mix(alpha, 1.0, on_the_lip);

    // And the cut comes **after** the lip, because the lip writes an opaque
    // alpha: cutting first and lighting second would hand the corner's own
    // anti-aliasing back to the band that follows it round.
    //
    // The band is **one device pixel**, taken from the derivative rather than
    // written as a number, and that is what makes the lip one line rather than
    // two weights. It was `smoothstep(r - 1.0, r + 0.5, out)` — one and a half
    // *logical* pixels, three device pixels on this screen — and the straight
    // top edge is entirely inside that band, so the row the lip is drawn on
    // was faded to about half before it ever reached the frame. Measured at
    // 3008 x 1630: the lip along the straight edge composited to (35, 30, 19)
    // against `DIALOG_LINE`'s (55, 48, 31), while the same lip round the
    // corner — whose fragments are fully covered — came out at the full
    // (55, 48, 31). A corner brighter than the line it ends is the artefact
    // over again, so the fade is narrowed to where a fragment really does
    // straddle the edge.
    if rounding {
        alpha = alpha * (1.0 - smoothstep(r - aa, r + aa, out));
    }

    return vec4<f32>(
        mix(dye, params.lip.rgb, on_the_lip),
        clamp(alpha, 0.0, 1.0),
    );
}

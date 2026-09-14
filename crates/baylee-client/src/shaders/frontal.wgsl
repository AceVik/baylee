// The cloth that hangs from the actions row, over the hand.
//
// The felt is an altar (`felt.wgsl`: "an altar's edge, not a tray's") and the
// actions row is its front edge. What hangs from an altar's front edge is a
// **frontal** — an antependium — and that is what this is: a dark gauze,
// gathered along the whole length of the shelf and falling in a chain of
// shallow symmetric swags, sheer enough that the table is seen through it,
// with the player's cards held up in front of it. The node it paints has been
// called the veil since it was a gradient; this gives the word its fiction.
//
// Nothing here is a light, and nothing here is a surface a card lies *on*. It
// is cloth in the shelf's shadow, and the brightest pixel on it stays under
// everything on a card and under the question's ink.
//
// # Why it exists at all, and the scar it has to stay clear of
//
// This strip was once an opaque 88 % black panel the width of the window, and
// the owner had it removed: a hand of cards laid on a black strip is a hand of
// cards in a *panel*, which is the one thing on this screen that must not read
// as an interface. What went with it was the dark ground each card's glow was
// read against, and the glow was then reported as missing. A gradient brought
// the ground back; this brings it back as something to look at. Both promises
// hold or this is a mistake: **the table still shows through**, and **a card's
// glow still has something to be read against**.
//
// # The shape
//
// Tacks every `PITCH` pixels, placed so that a whole swag is centred on the
// window's middle and the chain is mirror-symmetric by construction — the
// owner asked for symmetric curves, and symmetry that is arithmetic cannot
// drift. Between two tacks the cloth's lower edge is a catenary, which is the
// curve a hanging chain actually takes; `SAG` deep at the middle, `TACK` deep
// at the tack. Inside that band the cloth is dense at the shelf and thins to
// `BAND_EDGE` at its own lower edge, so the scallop reads as gathered cloth
// rather than as a shape someone cut. Below it the body is sheer and carries
// only the folds.
//
// A straight-edged dense band would be the shelf grown twenty pixels taller —
// the removed panel at a new height. The scallop and the sheer edge are the
// whole difference.
//
// # Why the folds are one-dimensional
//
// A hanging fold is vertical over its whole length, and this surface is 150
// pixels tall and up to 3200 wide. An isotropic field in a strip that thin
// reads as a flat wash: there is no room across it for a two-dimensional
// feature to be seen. So the folds are `fbm1` over x alone, warped so they
// diverge from the tacks and gather into the sag bottoms.
//
// # The clocks
//
// Two, and both of them already on this screen: `mat.wgsl` breathes a seat's
// rim every 7 seconds and travels a swell round it every 11. Borrowing those
// two adds no new tempo — a screen with one tempo is a room and a screen with
// six is a fairground. `energy` scales the two amplitudes and nothing else, so
// `reduce_motion` leaves the chain exactly where it was designed rather than
// somewhere a frozen sine happened to leave it: the end of the movement and
// not its absence, as `hud::motion` puts it.

#import bevy_render::globals::Globals
#import bevy_ui::ui_vertex_output::UiVertexOutput
#import "embedded://baylee_client/shaders/noise.wgsl"::fbm1

struct FrontalParams {
    /// The gathered hem at the shelf, and how dense it is there (`a`).
    hem: vec4<f32>,
    /// The sheer body, and how dense it is at the window's edge (`a`).
    body: vec4<f32>,
    /// The gathering stitch at each tack, and how far it shows (`a`).
    thread: vec4<f32>,
    /// How far the two movements travel. 0 leaves the cloth hanging still.
    energy: f32,
    /// Width over height of the node, so the swags are pixels and not a
    /// fraction of the window.
    aspect: f32,
    /// The node's height in logical pixels — every number below is in them.
    height: f32,
    /// How dense the body is where it leaves the band, before it falls away
    /// towards `body.a`.
    shoulder: f32,
}

@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> params: FrontalParams;

const TAU: f32 = 6.2831853;

/// How far apart the cloth is tacked to the shelf.
///
/// Deliberately sharing no small multiple with `hud::hand_layout`'s card step,
/// which caps at 100: a swag that lined up with the cards would read as
/// scaffolding behind them rather than as cloth they are held in front of. It
/// gives 5.3 swags at 1280, 7.2 at 1728 and 13.3 at 3200 — a wider window
/// gets *more* cloth, never stretched cloth.
const PITCH: f32 = 240.0;

/// How deep the cloth hangs at a swag's middle, and at a tack.
const SAG: f32 = 22.0;
const TACK: f32 = 5.0;

/// The catenary's shape. Larger is a deeper, squarer bow; 1.6 is the curve of
/// a chain hung slack, which is what a valance is.
const CATENARY: f32 = 1.6;

/// How far the band's lower edge is feathered into the body.
const EDGE_SOFT: f32 = 5.0;

/// How dense the cloth still is where the band ends.
///
/// Not zero and not one: a sag bottom is *sheer* and a tack is dense, and
/// that difference is what makes the scallop read as gathering rather than as
/// a cut edge.
const BAND_EDGE: f32 = 0.75;

/// Where the dye stops being the shelf's and starts being the ground's.
const WARM_FROM: f32 = 8.0;
const WARM_TO: f32 = 44.0;

/// How wide a fold is, how much it is worth in the band and in the body.
const FOLD_PX: f32 = 48.0;
const FOLD_BAND: f32 = 0.10;
const FOLD_BODY: f32 = 0.07;

/// How far the folds fan out from the tacks by the window's edge.
const SPREAD: f32 = 0.35;

/// The two clocks, in seconds, and how far each travels.
const BREATH_SECONDS: f32 = 11.0;
const BREATH_PX: f32 = 1.5;
const SWAY_SECONDS: f32 = 7.0;
const SWAY_PX: f32 = 3.0;

/// How long the gathering stitch shows below the shelf.
const THREAD_PX: f32 = 5.0;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let h = params.height;
    // Pixels from the window's middle, and pixels down from the shelf. Every
    // constant above is in these, which is the whole reason `height` and
    // `aspect` are uniforms: a field authored in `uv` is a field that is
    // stretched on the next window.
    let x = (in.uv.x - 0.5) * params.aspect * h;
    let y = in.uv.y * h;
    let t = globals.time;

    // Which swag, and where inside it. The half turns the tacks to `x = ±P/2
    // ± nP`, which puts a whole swag on the middle and makes the chain
    // mirror-symmetric about it — including the partial swag at each window
    // edge, which is cloth continuing past the frame.
    let s = x / PITCH + 0.5;
    let swag = floor(s);
    let u = s - swag;
    let bow = 1.0 - (cosh(CATENARY * (2.0 * u - 1.0)) - 1.0) / (cosh(CATENARY) - 1.0);

    // The breath: the chain ripples outward from the middle, and is exactly
    // still at every tack, because a valance is fixed where it is tacked.
    let breath = BREATH_PX * bow
        * sin(TAU * t / BREATH_SECONDS + 0.5 * abs(swag))
        * params.energy;
    let depth = TACK + SAG * bow + breath;

    // The gathered band: dense at the shelf, `BAND_EDGE` at its own lower
    // edge, feathered from there into the body.
    let down = clamp(y / max(depth, 1.0), 0.0, 1.0);
    let band = mix(params.hem.a, BAND_EDGE, down);
    let gathered = 1.0 - smoothstep(depth, depth + EDGE_SOFT, y);

    // The body: sheer where it leaves the band and falling away to the
    // window's edge. The power is what keeps it flat across the cards and
    // spends its darkening in the last third, where the card bottoms are and
    // where there is no table left to see through it.
    let free = max(0.0, y - depth);
    let fall = free / max(h - depth, 1.0);
    let sheer = params.shoulder + (params.body.a - params.shoulder) * pow(fall, 2.2);

    // The folds. Warped so they fan out of the tacks and gather into the sag
    // bottoms — one continuous odd term rather than a fan from the *nearest*
    // tack, whose sign would flip at every swag middle and tear the field.
    let sway = SWAY_PX * pow(fall, 1.5)
        * sin(TAU * t / SWAY_SECONDS + abs(x) / 900.0)
        * params.energy;
    let fan = SPREAD * (PITCH / TAU) * sin(TAU * s) * fall;
    let fold = fbm1((x + fan + sway) / FOLD_PX) - 0.5;

    var alpha = mix(sheer, band, gathered);
    alpha = alpha + 2.0 * fold * mix(FOLD_BODY, FOLD_BAND, gathered);

    // The dye travels from the shelf's own colour to the ground's. It is
    // worth almost nothing against a bright neutral sky — a dark dye is
    // hue-neutral at any partial coverage — and everything against the felt
    // and at night, which is where it can be seen.
    let dye = mix(params.hem.rgb, params.body.rgb, smoothstep(WARM_FROM, WARM_TO, y));

    // The gathering stitch: a thread at each tack, and the first thing to go
    // if this surface ever has to be quieter.
    let to_tack = min(u, 1.0 - u) * PITCH;
    let stitch = params.thread.a
        * (1.0 - smoothstep(0.5, 1.5, to_tack))
        * (1.0 - smoothstep(THREAD_PX - 2.0, THREAD_PX, y));

    return vec4<f32>(mix(dye, params.thread.rgb, stitch), clamp(alpha, 0.0, 1.0));
}

//! Shared midnight furniture: the hand well, action rail and seat cartouches.
//!
//! The shader is `shaders/frontal.wgsl` and the whole argument is written
//! there. This is the half that has to exist in Rust: the material, the
//! **ten** handles it is ever minted on, and the system that keeps each
//! surface's idea of its own size and of the motion preference in step with
//! the node it is on.
//!
//! It is shaped like [`crate::ambience`] on purpose — the same `UiMaterial`,
//! the same guard on there being a render world at all, the same
//! aspect-written-from-`ComputedNode`. What it does **not** share is the
//! field: a full-screen aurora with a vignette in its middle is authored for
//! a screen, and these two nodes are 40 and 190 pixels tall and up to 3200
//! wide.
//!
//! # Why two handles and not one node
//!
//! The actions row has to stand *over* the zone dialog's table veil and the
//! hand zone *under* it — `hud::Z_LEDGE` against `hud::Z_HAND`, because a
//! question may never be dimmed and the hand it is asking about may — so they
//! are siblings and one node cannot be both. A `UiMaterial` carries one
//! uniform block per handle and [`hang`] writes each node's own height and
//! aspect into it, so two nodes of different heights need two handles. They
//! share the shader, the clocks and a fold field indexed on **window** x,
//! which is what makes them one piece of cloth rather than two surfaces that
//! happen to match.
//!
//! # Why the handles live in a resource
//!
//! [`crate::hud::sync_overlay`] rebuilds the whole hand zone whenever
//! [`crate::hud::HudRevision`] changes, and that counter **counts the hover**
//! — so the nodes this material is on are respawned on every pointer move
//! across a card. A material minted in the builder would be a fresh asset per
//! mouse move: a leak, and a pipeline specialisation each time. So they are
//! minted once into [`Cloth`] and handed out by clone, which is what
//! `UiCardMaterials` does for every card in the hand for the same reason.
//!
//! The seat cache adds eight fixed slots, indexed by roster order rather
//! than game identity so opening another duel cannot grow it. The virtual
//! clock and motion preference are shared by every surface; no new plugin
//! wiring is required beyond the existing `FrontalPlugin`.

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use bevy::ui::ComputedNode;

/// Everything a surface of cloth reads.
///
/// The colours are **linear**, because that is what the UI material pipeline
/// hands to the target — `ambience` passes `LinearRgba` for the same reason,
/// and mixing two dyes in linear space is also the only mix that means
/// anything.
#[derive(Clone, Copy, ShaderType, Debug)]
pub struct FrontalParams {
    /// The one dye; `w` is how far the folds move it.
    pub dye: Vec4,
    /// The lip's dye; `w` is how many pixels of it there are, and zero is a
    /// surface with no lip.
    pub lip: Vec4,
    /// Where the ground stops being flat (`x`, pixels from the node's top),
    /// how dense it is above that (`y`), how dense at the window's bottom
    /// edge (`z`), and how far the whole surface breathes (`w`).
    pub ramp: Vec4,
    /// How far the folds move the density.
    pub grain: f32,
    /// How far the two movements travel. Zero leaves the cloth still.
    pub energy: f32,
    /// Width over height of the node.
    pub aspect: f32,
    /// The node's height in logical pixels.
    pub height: f32,
    /// How far the top two corners are rounded, in pixels.
    pub corner: f32,
    /// Virtual seconds, surface kind (skirt / rail / seat), reserved.
    pub surface: Vec4,
    /// Stationary WUBRG inlays in linear light.
    pub inlays: [Vec4; 5],
}

/// The cloth, as a UI material.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct FrontalMaterial {
    /// Everything the shader reads.
    #[uniform(0)]
    pub params: FrontalParams,
}

impl UiMaterial for FrontalMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/frontal.wgsl".into()
    }
}

/// A node wearing the cloth, so its size can be written back into its
/// material.
#[derive(Component, Clone, Copy, Default)]
pub struct Hanging;

/// Two dock handles and at most eight seat handles, retained across rebuilds.
#[derive(Resource, Default)]
pub struct Cloth {
    skirt: Option<Handle<FrontalMaterial>>,
    rail: Option<Handle<FrontalMaterial>>,
    seats: [Option<Handle<FrontalMaterial>>; 8],
}

impl Cloth {
    /// The skirt: the whole hand zone, running behind the rail at the top and
    /// off the bottom of the window.
    ///
    /// `None` when there is no render world — a headless app has no
    /// `Assets<FrontalMaterial>`, and the hand zone falls back to a flat
    /// ground of the same dye rather than growing a second code path.
    pub fn skirt(
        &mut self,
        assets: Option<&mut Assets<FrontalMaterial>>,
    ) -> Option<Handle<FrontalMaterial>> {
        let assets = assets?;
        if let Some(handle) = self.skirt.as_ref() {
            return Some(handle.clone());
        }
        let handle = assets.add(FrontalMaterial {
            params: FrontalParams {
                dye: dye(SKIRT_WEAVE),
                lip: trim(0.0),
                // The shoulder is the rail's own height: above it the ground
                // is behind an opaque row of buttons and a ramp there would
                // be spent where nobody can see it.
                ramp: Vec4::new(crate::hud::LEDGE_H, SKIRT, FOOT, 0.0),
                grain: SKIRT_FOLD,
                energy: 1.0,
                // Both are written by `hang` on the first frame the node has
                // a size; until then a square node is the least wrong guess.
                aspect: 1.0,
                height: crate::hud::HAND_ZONE_H,
                corner: CORNER,
                surface: Vec4::ZERO,
                inlays: inlays(),
            },
        });
        self.skirt = Some(handle.clone());
        Some(handle)
    }

    /// The rail: the actions row, the top edge of the same cloth.
    pub fn rail(
        &mut self,
        assets: Option<&mut Assets<FrontalMaterial>>,
    ) -> Option<Handle<FrontalMaterial>> {
        let assets = assets?;
        if let Some(handle) = self.rail.as_ref() {
            return Some(handle.clone());
        }
        let handle = assets.add(FrontalMaterial {
            params: FrontalParams {
                dye: dye(RAIL_WEAVE),
                lip: trim(crate::hud::LEDGE_LIP),
                // Its shoulder is its whole height, so the ramp never starts
                // and the rail is one density all the way down.
                ramp: Vec4::new(crate::hud::LEDGE_H, RAIL, RAIL, RAIL_BREATH),
                grain: RAIL_FOLD,
                energy: 1.0,
                aspect: 1.0,
                height: crate::hud::LEDGE_H,
                corner: CORNER,
                surface: Vec4::new(0.0, 1.0, 0.0, 0.0),
                inlays: inlays(),
            },
        });
        self.rail = Some(handle.clone());
        Some(handle)
    }

    /// One handle per visible seat, reused across revisions and duels. A
    /// malformed ninth seat falls back to flat paint rather than allocating.
    pub fn seat(
        &mut self,
        index: usize,
        assets: Option<&mut Assets<FrontalMaterial>>,
    ) -> Option<Handle<FrontalMaterial>> {
        let slot = self.seats.get_mut(index)?;
        let assets = assets?;
        if let Some(handle) = slot.as_ref() {
            return Some(handle.clone());
        }
        let handle = assets.add(FrontalMaterial {
            params: FrontalParams {
                dye: dye(0.16),
                lip: trim(0.65),
                ramp: Vec4::new(0.0, 0.96, 0.96, 0.0),
                grain: 0.0,
                energy: 0.0,
                aspect: 1.0,
                height: 48.0,
                corner: 4.0,
                surface: Vec4::new(0.0, 2.0, 0.0, 0.0),
                inlays: inlays(),
            },
        });
        *slot = Some(handle.clone());
        Some(handle)
    }
}

fn trim(width: f32) -> Vec4 {
    let colour = LinearRgba::from(crate::hud::palette::DOCK_EDGE);
    Vec4::new(colour.red, colour.green, colour.blue, width)
}

fn inlays() -> [Vec4; 5] {
    crate::hud::palette::DOCK_INLAYS.map(|colour| {
        let colour = LinearRgba::from(colour);
        Vec4::new(colour.red, colour.green, colour.blue, 1.0)
    })
}

/// One mineral-leather dye for the dock and seat furniture. Dialogs keep
/// their own warm register; these surfaces belong to the midnight table.
fn dye(weave: f32) -> Vec4 {
    let dye = LinearRgba::from(crate::hud::palette::DOCK_GROUND);
    Vec4::new(dye.red, dye.green, dye.blue, weave)
}

/// How far the **top two** corners of the controls bar are rounded.
///
/// The owner asked for it on 14.09.2026 — "gib der Actions-Bar oben rechts und
/// links dem Border kleinen radius" — which reverses this file's shader header
/// and `ax-design.md` §3.3. Their argument was that an edge running the full
/// width of the window has no corners to round; what it missed is that the top
/// edge is the only one of the four that is an edge at all. The other three
/// run off the frame, so rounding these two says the thing the design was
/// after in the first place: the cloth is a surface laid against the bottom of
/// the window, not a band painted across it.
///
/// **Both** surfaces carry it, not the rail alone. The skirt starts at the
/// same top edge and runs behind the rail, so a radius on one of them would
/// cut two notches out of the bar and show the other's square corner through
/// them.
///
/// 10 against `hud::LEDGE_H`'s 40 is a quarter of the bar's height: "klein" at
/// the scale a 3200-pixel bar is read at, and still unmistakably there.
pub(crate) const CORNER: f32 = 10.0;

/// How dense the skirt is where it leaves the rail, and at the window's
/// bottom edge.
///
/// **The number the whole container is bought with, and it is bounded on both
/// sides.** Under it is the scar this surface exists to stay clear of: the
/// removed 88 % panel measured (70, 82, 92) against this window's own sky.
/// Over it is the thing the owner reported as missing — the sheer cloth this
/// replaces left the bottom of the zone reading (138, 153, 162) against a sky
/// of (185, 213, 236), which is to say the bottom two thirds of the zone
/// simply *were* the sky and the cards floated on it.
///
/// The owner then asked for the zone to be **less** transparent still, in the
/// same breath as asking for the rail to be more ("Mach die Hand-Zone weniger
/// transparent", 14.09.2026), and the two move together: the rail is only
/// affordable *because* the skirt behind it got denser. 0.88 lands at (74, 84,
/// 92) at the shoulder and 0.92 at (63, 71, 76) at the foot — past the removed
/// panel in luminance and unlike it in every other way, because it is a dye
/// rather than black, it moves, and it still passes the day sky's clouds. Over
/// felt or at night it is (26, 25, 19), which is `palette::DIALOG` itself.
///
/// The arithmetic is **linear**, where an alpha buys far less darkening than
/// sRGB predicts; every figure above was composited that way and then read
/// back off the screen.
pub(crate) const SKIRT: f32 = 0.88;
const FOOT: f32 = 0.92;

/// Both ends of that, as §3.2 states every pair, and at compile time because
/// both sides are constants: under the first of them the gauze is back and
/// the sky reads through the zone, over the second the table is closed off
/// and the panel is back. The third is the cloth's own sense: it gathers
/// weight as it falls and never loses it.
const _: () = assert!(SKIRT >= 0.76);
const _: () = assert!(FOOT <= 0.94);
const _: () = assert!(FOOT >= SKIRT);

/// How dense the rail is, and how far that breathes on the 7-second clock.
///
/// The rail is no longer opaque — the owner asked for "a little bit
/// transparent" on 14.09.2026, which reverses `ax-design.md` §3.3 — and the
/// transparency is only affordable because the skirt now runs *behind* it.
/// Over the bare sky a rail at 0.96 already takes the quietest ink on it from
/// 4.69 : 1 to 3.70, under the 4.5 a reader needs; over the skirt, 0.86 keeps
/// it at 4.86 at the worst pose the two clocks can make.
/// `crate::hud::palette::LEDGE_SOFT` is the other half of that trade, and
/// `the_rail_stays_readable_at_every_pose_the_cloth_can_take` is what holds
/// the pair of them.
pub(crate) const RAIL: f32 = 0.86;
const RAIL_BREATH: f32 = 0.02;

/// How far the folds move the density, and the dye, on each surface.
///
/// The rail's are about a third of the skirt's, for the reason the shader
/// gives about nothing travelling along it: the row is read, and texture
/// behind a line of writing is noise under it. The skirt is looked *at*, so
/// it carries the cloth.
const SKIRT_FOLD: f32 = 0.04;
const SKIRT_WEAVE: f32 = 0.30;
const RAIL_FOLD: f32 = 0.012;
const RAIL_WEAVE: f32 = 0.20;

/// Keeps each surface's idea of its own size, and of the motion preference,
/// in step with the node it is on.
///
/// The same job `ambience::breathe` does and the same trap: a UI node has no
/// size until layout has run, so the first frame after a respawn reports
/// zero — and these nodes are respawned on every pointer move. A division
/// there is a NaN aspect for one frame, every time.
fn hang(
    nodes: Query<(&ComputedNode, &MaterialNode<FrontalMaterial>), With<Hanging>>,
    materials: Option<ResMut<Assets<FrontalMaterial>>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    time: Res<Time<Virtual>>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    for (computed, handle) in &nodes {
        let size = computed.size() * computed.inverse_scale_factor;
        if size.y <= 0.0 {
            continue;
        }
        let Some(mut material) = materials.get_mut(&handle.0) else {
            continue;
        };
        material.params.aspect = size.x / size.y;
        material.params.height = size.y;
        material.params.energy = f32::from(u8::from(!still));
        material.params.surface.x = if still { 0.0 } else { time.elapsed_secs() };
    }
}

/// Installs the cloth, if there is anything to draw it on.
pub struct FrontalPlugin;

impl Plugin for FrontalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Cloth>();
        // The same guard `ambience` carries: embedding a shader needs an
        // asset server and a UI material needs a render world, and a headless
        // test app has neither. `Cloth`'s getters then answer `None` and
        // the hand zone draws a flat ground of the same dye.
        if !app.world().contains_resource::<AssetServer>()
            || app.get_sub_app(bevy::render::RenderApp).is_none()
        {
            return;
        }
        embedded_asset!(app, "shaders/noise.wgsl");
        embedded_asset!(app, "shaders/frontal.wgsl");
        app.add_plugins(UiMaterialPlugin::<FrontalMaterial>::default())
            .add_systems(Update, hang);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The WGSL is parsed and validated with the same front end wgpu uses.
    ///
    /// The alternative is finding a typo when a pipeline is built — in a
    /// browser, where there is no filesystem to look at. The grain is pasted
    /// in ahead of it because `check_wgsl` strips `#import` lines, so an
    /// imported function is an unknown identifier to it.
    #[test]
    fn the_frontal_shader_compiles() {
        let prelude = format!(
            "\
struct UiVertexOutput {{
    @location(0) uv: vec2<f32>,
    @location(1) border_widths: vec4<f32>,
    @location(2) border_radius: vec4<f32>,
    @location(3) @interpolate(flat) size: vec2<f32>,
    @builtin(position) position: vec4<f32>,
}};
struct Globals {{ time: f32 }};
{}",
            include_str!("shaders/noise.wgsl")
        );
        crate::cardmat::tests::check_wgsl(include_str!("shaders/frontal.wgsl"), &prelude);
    }

    /// The quietest ink on the rail is readable at **every** pose the two
    /// clocks can put the cloth in, over the brightest ground there is.
    ///
    /// This is the bound the owner's "a little bit transparent" is spent
    /// against, and it is the reason `palette::LEDGE_SOFT` exists: with
    /// `DIALOG_SOFT` the same stack reads 4.07 : 1. The ground is the sky
    /// recovered from the running client behind the bottom row of the hand
    /// zone — (185, 213, 236) — which is the brightest thing this surface is
    /// ever composited over.
    ///
    /// The scan is over *achievable* poses and not over the independent
    /// extremes of each term, which is the distinction that matters: one
    /// `fold` value drives the density **and** the dye, and it lightens one
    /// while it darkens the other, so the two worst cases cannot happen at
    /// once. Taking them independently gives 4.43 and would fail a surface
    /// that is fine.
    #[test]
    fn the_rail_stays_readable_at_every_pose_the_cloth_can_take() {
        let shader = include_str!("shaders/frontal.wgsl");
        let swing: f32 = number(shader, "const BREATH: f32 = ");
        // `fbm1` is four octaves at 0.5, 0.25, 0.125, 0.0625, so it cannot
        // leave [0, 0.9375] and the shader's `fbm1(..) - 0.5` cannot leave
        // [-0.5, 0.4375]. The bound is the field's, not a sample of it.
        let (fold_lo, fold_hi) = (-0.5_f32, 0.4375_f32);
        let sky = LinearRgba::from(Color::srgb_u8(185, 213, 236));
        let dialog = LinearRgba::from(crate::hud::palette::DOCK_GROUND);
        // Both sides are already linear here, which is the whole point: the
        // UI pipeline composites in linear light, and an alpha there buys far
        // less darkening than sRGB arithmetic predicts.
        let luma =
            |c: LinearRgba| 0.2126f32.mul_add(c.red, 0.7152f32.mul_add(c.green, 0.0722 * c.blue));
        let ink = luma(LinearRgba::from(crate::hud::palette::LEDGE_SOFT));

        let mut worst = f32::MAX;
        for i in -100_i16..=100 {
            let wave = f32::from(i) / 100.0;
            let breath = 1.0 + swing * wave;
            for j in 0_i16..=200 {
                let fold = fold_lo + (fold_hi - fold_lo) * f32::from(j) / 200.0;
                let over = |dense: f32, weave: f32, breath_of: f32, under: LinearRgba| {
                    let a = (dense + 2.0 * fold * breath_of).clamp(0.0, 1.0);
                    let lit = 1.0 + 2.0 * fold * weave * breath;
                    LinearRgba::new(
                        dialog.red * lit * a + under.red * (1.0 - a),
                        dialog.green * lit * a + under.green * (1.0 - a),
                        dialog.blue * lit * a + under.blue * (1.0 - a),
                        1.0,
                    )
                };
                let skirt = over(SKIRT, SKIRT_WEAVE, SKIRT_FOLD * breath, sky);
                let rail = over(
                    RAIL + RAIL_BREATH * wave,
                    RAIL_WEAVE,
                    RAIL_FOLD * breath,
                    skirt,
                );
                let ground = luma(rail);
                worst = worst.min((ink.max(ground) + 0.05) / (ink.min(ground) + 0.05));
            }
        }
        assert!(
            worst >= 4.5,
            "the quietest ink on the rail reads at {worst} : 1 at its worst pose"
        );
    }

    /// One cloth, and the seam between the two nodes is not a seam.
    ///
    /// The fold field is indexed on the window's own x and carries no `y`
    /// term at all, so the fold leaving the bottom of the rail is the same
    /// fold entering the top of the skirt. A lean that varied with depth —
    /// which is what this shader used to do — would meet the seam at two
    /// different places on the two nodes and tear the field in half.
    #[test]
    fn one_fold_field_runs_through_both_surfaces() {
        let shader = include_str!("shaders/frontal.wgsl");
        assert!(
            shader.contains("let x = (in.uv.x - 0.5) * params.aspect * h;"),
            "the fold field is indexed on the window's x, not on the node's"
        );
        let lean = shader
            .split_once("let lean = ")
            .expect("the swell is still called a lean")
            .1
            .split_once("let fold")
            .expect("and the fold is computed right after it")
            .0;
        // The two ways a depth term has ever been written here: `fall`, which
        // is the ramp's own coordinate, and a bare `y`. `params.energy`
        // contains a `y` and is not one, so the second is checked as an
        // identifier rather than as a letter.
        let depth = lean.contains("fall")
            || lean
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .any(|word| word == "y");
        assert!(
            !depth,
            "the swell leans with depth, which tears the seam: {lean}"
        );
        assert!(
            shader.contains("let fold = fbm1((x + lean) / FOLD_PX) - 0.5;"),
            "and the fold itself is the lean and nothing else"
        );
    }

    /// A card's step and the cloth's folds must not line up.
    ///
    /// `hand_layout` caps the step at `HAND_CARD_W + 8`, and folds falling on
    /// the gaps between cards would read as scaffolding the cards were hung
    /// on rather than as cloth they are held in front of.
    #[test]
    fn the_folds_do_not_line_up_with_the_cards() {
        let fold: f32 = number(
            include_str!("shaders/frontal.wgsl"),
            "const FOLD_PX: f32 = ",
        );
        let step = crate::hud::HAND_CARD_W + 8.0;
        let ratio = step / fold;
        assert!(
            (ratio - ratio.round()).abs() > 0.15,
            "the fold is {fold} against a card step of {step}, which is \
             {ratio} folds — near enough to a whole one to read as a grid"
        );
    }

    /// The cloth speaks the shelf's register and the ground's, and nothing a
    /// card says.
    ///
    /// The shelf has `ledge.rs::the_ledge_speaks_only_dialog` for this; the
    /// cloth hangs off the shelf and is read in the same glance, so it is
    /// held to the same list. `ACTIVATABLE`, `ARMED` and `WILL_TAP` are rules
    /// statements on cards and nothing in the interface may borrow them.
    #[test]
    fn the_cloth_borrows_no_light_that_means_something() {
        let source = include_str!("frontal.rs")
            .split_once("#[cfg(test)]")
            .expect("the tests are still where they were")
            .0;
        let shader = include_str!("shaders/frontal.wgsl");
        for forbidden in [
            "ACTIVATABLE",
            "WILL_TAP",
            "palette::ARMED",
            "palette::BRASS",
            "palette::ACCENT",
            "palette::PARCHMENT",
            "palette::CANDLE",
        ] {
            assert!(
                !source.contains(forbidden),
                "the cloth writes with `{forbidden}`, which belongs to a card"
            );
            assert!(
                !shader.contains(forbidden),
                "the cloth's shader names `{forbidden}`"
            );
        }
    }

    /// Reads a `const` back out of the shader.
    fn number(shader: &str, name: &str) -> f32 {
        shader
            .split_once(name)
            .unwrap_or_else(|| panic!("`{name}` is no longer in the shader"))
            .1
            .split(';')
            .next()
            .expect("a const ends in a semicolon")
            .trim()
            .parse()
            .expect("and is a number")
    }
}

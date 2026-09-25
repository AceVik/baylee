//! The keyword strip's material: a card's marks as an object lying on it
//! (#274), and since #298 its label — the chip, the moon, the crests. The
//! plate lies beside it as an object of its own ([`crate::platemat`]).
//!
//! The marks rode the card's own material as a rail along the print's bottom
//! edge until #274, over the artist and the copyright line. They are an
//! object now — a dark strip with its own contact shadow, standing on the
//! seam between the art and the type line — so they are a quad and a material
//! of their own, and the card's material has no dimension for them at all.
//! `baylee_client_core::cardrail` says where the quad lies on the card;
//! `card_common.wgsl`'s `label_strip` draws inside it, for both shaders here.
//!
//! **The material key is the strip and nothing else.** Every strip is the
//! same quad, and the shader sizes the strip inside it from its words
//! (`cardrail::Strip`), so a lane of twelve Soldiers saying the same thing is
//! one material however long it is, and a table has at most one strip
//! material per distinct thing its strips say. The clock is on the
//! material rather than in the key, for the reason
//! [`CardParams::motion`](crate::cardmat::CardParams::motion) gives.

use baylee_client_core::cardrail;
use bevy::asset::embedded_asset;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use crate::cardmat::motion_of;

/// What the strip's shader reads.
///
/// Thirty-two bytes, deliberately: a uniform block under the GL backend is
/// laid out `std140`, which rounds it up to a multiple of sixteen, and a
/// buffer shorter than the block it feeds would not bind. The plate's word
/// left for its own material, and its four bytes are padding now.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub struct MarksParams {
    /// The marks, one bit each in `cardrail::MARK_ORDER`'s order.
    pub bits: u32,
    /// The clock: [`MOVING`](crate::cardmat::MOVING) or
    /// [`STILL`](crate::cardmat::STILL).
    pub motion: f32,
    /// The quad's size in card widths, [`cardrail::quad_rect`]'s: what turns
    /// the quad's UV into the lengths the strip is laid out in.
    pub quad: Vec2,
    /// The chip, [`cardrail::Strip::swing`].
    pub swing: u32,
    /// The moon and the crests, [`cardrail::Strip::label`].
    pub label: u32,
    /// The block's last eight bytes.
    pub pad: UVec2,
}

impl MarksParams {
    /// The shader's words for `strip`, on the clock `motion`.
    #[must_use]
    pub fn new(strip: cardrail::Strip, motion: f32) -> Self {
        Self {
            bits: strip.marks,
            motion,
            quad: quad_size(),
            swing: strip.swing,
            label: strip.label,
            pad: UVec2::ZERO,
        }
    }
}

/// What the strip over one permanent's art says, read off the object itself:
/// the preview's strip, which has no board group behind it.
///
/// The table builds the same strip from the permanent's group, and both come
/// from the same doors — [`cardrail::mark_bits`], `Corner::of_object`,
/// [`board::asleep`](baylee_client_core::board::asleep) and
/// [`cardcrest::marks`](baylee_client_core::cardcrest::marks) — so a card
/// held up in the preview says what it says on the table. The preview shows
/// the whole print, so the chip is on it only where the plate is: where the
/// print cannot say the numbers.
#[must_use]
pub fn strip_of(object: &baylee_view::PublicObject) -> cardrail::Strip {
    use baylee_client_core::{board, cardcrest, cardplate::Corner};
    let corner = Corner::of_object(object);
    let provenance = board::provenance_of(object, crate::cardart::registry());
    cardrail::Strip::new(
        cardrail::mark_bits(object.keywords),
        corner.shows_plate(true, false).then_some(corner),
        board::asleep(object),
        cardcrest::marks(provenance, object.commander),
    )
}

/// The strip's quad, `[width, height]` in card widths.
#[must_use]
pub fn quad_size() -> Vec2 {
    let [x0, y0, x1, y1] = cardrail::quad_rect();
    Vec2::new(x1 - x0, y1 - y0)
}

/// The strip on a card lying on the table.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct MarksMaterial {
    /// The strip and the clock.
    #[uniform(0)]
    pub params: MarksParams,
    /// The glyph atlas the marks, the chip's numerals and the crests are
    /// drawn from, baked at startup.
    ///
    /// Always [`crate::markatlas::MARKS`], which is why it is not an
    /// `Option`: the handle is filled with a blank field before any material
    /// is built, so there is no moment at which a strip could be asked to
    /// bind an image that does not exist — and a `None` here would bind the
    /// fallback white texture, which decodes as a distance of -0.25
    /// everywhere and floods every glyph with ink.
    #[texture(1)]
    #[sampler(2)]
    pub marks: Handle<Image>,
}

impl MarksMaterial {
    /// The material for `strip`.
    #[must_use]
    pub fn new(strip: cardrail::Strip, motion: f32) -> Self {
        Self {
            params: MarksParams::new(strip, motion),
            marks: crate::markatlas::MARKS,
        }
    }
}

impl Material for MarksMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/marks.wgsl".into()
    }

    /// The strip's edge is antialiased and its shadow is soft, so it blends.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    /// Where a strip sits in the transparent pass: at the height it lies at,
    /// on a card's face. See [`table::sort_bias`](crate::table::sort_bias).
    fn depth_bias(&self) -> f32 {
        crate::table::sort_bias(crate::table::STRIP_RUNG)
    }
}

/// The same strip, on a card drawn as a UI node: the preview.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct MarksUiMaterial {
    /// The strip and the clock.
    #[uniform(0)]
    pub params: MarksParams,
    /// The glyph atlas; see [`MarksMaterial::marks`].
    #[texture(1)]
    #[sampler(2)]
    pub marks: Handle<Image>,
}

impl UiMaterial for MarksUiMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/marks_ui.wgsl".into()
    }
}

/// UI strip materials, one per strip.
///
/// The preview is rebuilt whenever the game moves, so a material minted per
/// rebuild would grow `Assets` for as long as the duel lasted; one per strip
/// is at most one per distinct thing a previewed strip has said. Holding still
/// is rewritten in place, the way
/// [`UiCardMaterials::set_still`](crate::cardmat::UiCardMaterials::set_still)
/// does it and for its reasons.
#[derive(Resource, Default)]
pub struct UiMarksMaterials {
    made: HashMap<cardrail::Strip, Handle<MarksUiMaterial>>,
    still: bool,
}

impl UiMarksMaterials {
    /// The material for `strip`, made once.
    pub fn get(
        &mut self,
        strip: cardrail::Strip,
        assets: &mut Assets<MarksUiMaterial>,
    ) -> Handle<MarksUiMaterial> {
        let motion = motion_of(self.still);
        self.made
            .entry(strip)
            .or_insert_with(|| {
                assets.add(MarksUiMaterial {
                    params: MarksParams::new(strip, motion),
                    marks: crate::markatlas::MARKS,
                })
            })
            .clone()
    }

    /// Forgets every material, when the duel closes.
    pub fn clear(&mut self) {
        self.made.clear();
    }

    fn set_still(&mut self, still: bool, assets: &mut Assets<MarksUiMaterial>) {
        if self.still == still {
            return;
        }
        self.still = still;
        let motion = motion_of(still);
        for handle in self.made.values() {
            if let Some(mut material) = assets.get_mut(handle) {
                material.params.motion = motion;
            }
        }
    }
}

/// The preview's strip follows the preference, as its card does.
fn track_motion(
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut cache: ResMut<UiMarksMaterials>,
    mut assets: ResMut<Assets<MarksUiMaterial>>,
) {
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    cache.set_still(still, &mut assets);
}

/// Registers both strip materials and ships their shaders in the binary, for
/// the reason [`CardMaterialPlugin`](crate::cardmat::CardMaterialPlugin)
/// embeds its own. `card_common.wgsl`, which both import, is registered
/// there.
pub struct MarksMaterialPlugin;

impl Plugin for MarksMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/marks.wgsl");
        embedded_asset!(app, "shaders/marks_ui.wgsl");
        app.add_plugins(MaterialPlugin::<MarksMaterial>::default())
            .add_plugins(UiMaterialPlugin::<MarksUiMaterial>::default())
            .init_resource::<UiMarksMaterials>()
            .add_systems(Update, track_motion);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cardmat::tests::check_wgsl;

    /// The strip's table shader, parsed and validated against the same stubs
    /// the card shader is.
    #[test]
    fn the_strip_shader_compiles() {
        let prelude = "\
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};
struct Globals { time: f32 };
@group(0) @binding(11) var<uniform> globals: Globals;
";
        check_wgsl(
            include_str!("shaders/marks.wgsl"),
            &format!("{prelude}{}", include_str!("shaders/card_common.wgsl")),
        );
    }

    /// And its UI twin, with the UI card shader's stubs.
    #[test]
    fn the_ui_strip_shader_compiles() {
        let prelude = "\
struct UiVertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) border_widths: vec4<f32>,
    @location(2) border_radius: vec4<f32>,
    @location(3) @interpolate(flat) size: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
struct Globals { time: f32 };
";
        check_wgsl(
            include_str!("shaders/marks_ui.wgsl"),
            &format!("{prelude}{}", include_str!("shaders/card_common.wgsl")),
        );
    }

    /// The uniform is one whole `std140` block, which is what the note on
    /// [`MarksParams`] claims and the GL backend needs.
    #[test]
    fn the_strip_uniform_is_one_block() {
        assert_eq!(MarksParams::min_size().get(), 32);
    }

    /// The two shaders declare the uniform in the order Rust lays it out.
    ///
    /// Nothing compares a WGSL struct with a Rust one: a field swapped on one
    /// side reads the word as the clock and draws no marks, with every
    /// shader still compiling.
    #[test]
    fn both_shaders_read_the_uniform_rust_writes() {
        for (which, src) in [
            ("marks.wgsl", include_str!("shaders/marks.wgsl")),
            ("marks_ui.wgsl", include_str!("shaders/marks_ui.wgsl")),
        ] {
            let open = src.find("struct MarksParams {").expect("the uniform");
            let body = &src[open..open + src[open..].find('}').expect("it closes")];
            let fields: Vec<&str> = body
                .lines()
                .map(str::trim)
                .filter(|l| !l.starts_with("//") && l.contains(':'))
                .collect();
            assert_eq!(
                fields,
                [
                    "bits: u32,",
                    "motion: f32,",
                    "quad: vec2<f32>,",
                    "swing: u32,",
                    "label: u32,",
                    "pad: vec2<u32>,"
                ],
                "{which} lays the uniform out differently from `MarksParams`"
            );
        }
    }

    /// One material per strip, and holding still is rewritten on the ones
    /// already made rather than minting new ones.
    #[test]
    fn a_strip_is_one_material_and_holds_still_in_place() {
        let mut assets = Assets::<MarksUiMaterial>::default();
        let mut cache = UiMarksMaterials::default();
        let strip = |marks, sick| cardrail::Strip::new(marks, None, sick, [None, None]);
        let flying = cache.get(strip(1, false), &mut assets);
        assert_eq!(
            cache.get(strip(1, false), &mut assets),
            flying,
            "the same strip, the same material"
        );
        let other = cache.get(strip(1 | 1 << 8, false), &mut assets);
        assert_ne!(other, flying, "another word, another material");
        let asleep = cache.get(strip(1, true), &mut assets);
        assert_ne!(asleep, flying, "the moon is part of the key");
        assert_eq!(
            assets.get(&asleep).map(|m| m.params.label),
            Some(cardrail::label::MOON)
        );
        assert_eq!(assets.len(), 3);

        cache.set_still(true, &mut assets);
        assert_eq!(assets.len(), 3, "holding still made materials");
        for handle in [&flying, &other, &asleep] {
            let made = assets.get(handle).expect("still there");
            assert!(
                made.params.motion.abs() < f32::EPSILON,
                "a strip still moving"
            );
        }
        assert_eq!(
            assets.get(&flying).map(|m| m.params.quad),
            Some(quad_size()),
            "the quad the shader lays the strip out in"
        );
    }
}

//! The keyword strip's material: a card's marks as an object lying on it
//! (#274).
//!
//! The marks rode the card's own material as a rail along the print's bottom
//! edge until #274, over the artist and the copyright line. They are an
//! object now — a dark strip with its own contact shadow, standing on the
//! seam between the art and the type line — so they are a quad and a material
//! of their own, and the card's material has no dimension for them at all.
//! `baylee_client_core::cardrail` says where the quad lies on the card;
//! `card_common.wgsl`'s `marks_strip` draws inside it, for both shaders here.
//!
//! **The material key is the word and nothing else.** Every strip is the same
//! quad, and the shader sizes the strip inside it from the twelve-bit word
//! (`cardrail::mark_bits`), so a lane of twelve Soldiers with the same
//! keywords is one material however long it is, and a table has at most one
//! strip material per distinct set of keywords on it. The clock is on the
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
/// Sixteen bytes, deliberately: a uniform block under the GL backend is laid
/// out `std140`, which rounds it up to sixteen, and a buffer that bound
/// eight would be shorter than the block it feeds.
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
}

impl MarksParams {
    /// The strip for `bits`, on the clock `motion`.
    #[must_use]
    pub fn new(bits: u32, motion: f32) -> Self {
        Self {
            bits,
            motion,
            quad: quad_size(),
        }
    }
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
    /// The word and the clock.
    #[uniform(0)]
    pub params: MarksParams,
    /// The glyph atlas, always [`crate::markatlas::MARKS`] — see
    /// [`CardMaterial::marks`](crate::cardmat::CardMaterial::marks) for why
    /// it is not an `Option`.
    #[texture(1)]
    #[sampler(2)]
    pub marks: Handle<Image>,
}

impl MarksMaterial {
    /// The strip for `bits`.
    #[must_use]
    pub fn new(bits: u32, motion: f32) -> Self {
        Self {
            params: MarksParams::new(bits, motion),
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
    /// The word and the clock.
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

/// UI strip materials, one per word.
///
/// The preview is rebuilt whenever the game moves, so a material minted per
/// rebuild would grow `Assets` for as long as the duel lasted; one per word
/// is at most one per distinct set of keywords ever previewed. Holding still
/// is rewritten in place, the way
/// [`UiCardMaterials::set_still`](crate::cardmat::UiCardMaterials::set_still)
/// does it and for its reasons.
#[derive(Resource, Default)]
pub struct UiMarksMaterials {
    made: HashMap<u32, Handle<MarksUiMaterial>>,
    still: bool,
}

impl UiMarksMaterials {
    /// The material for `bits`, made once.
    pub fn get(
        &mut self,
        bits: u32,
        assets: &mut Assets<MarksUiMaterial>,
    ) -> Handle<MarksUiMaterial> {
        let motion = motion_of(self.still);
        self.made
            .entry(bits)
            .or_insert_with(|| {
                assets.add(MarksUiMaterial {
                    params: MarksParams::new(bits, motion),
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
        assert_eq!(MarksParams::min_size().get(), 16);
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
                ["bits: u32,", "motion: f32,", "quad: vec2<f32>,"],
                "{which} lays the uniform out differently from `MarksParams`"
            );
        }
    }

    /// One material per word, and holding still is rewritten on the ones
    /// already made rather than minting new ones.
    #[test]
    fn a_word_is_one_material_and_holds_still_in_place() {
        let mut assets = Assets::<MarksUiMaterial>::default();
        let mut cache = UiMarksMaterials::default();
        let flying = cache.get(1, &mut assets);
        assert_eq!(
            cache.get(1, &mut assets),
            flying,
            "the same word, the same material"
        );
        let other = cache.get(1 | 1 << 8, &mut assets);
        assert_ne!(other, flying, "another word, another material");
        assert_eq!(assets.len(), 2);

        cache.set_still(true, &mut assets);
        assert_eq!(assets.len(), 2, "holding still made materials");
        for handle in [&flying, &other] {
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

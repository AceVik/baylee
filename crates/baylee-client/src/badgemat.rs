//! The count badge's material: how many permanents a merged card stands for,
//! as an object at its top-right corner (#261, #274).
//!
//! The count was a pill painted into the card's own material, over the
//! printed cost, until #274 moved everything this client says off the print.
//! The owner's placement is an object: "ganz oben links an der Ecke, leicht
//! überragend mit elevation shadow". So it is a quad and a material of its
//! own, like the keyword strip ([`crate::marksmat`]), and the card's material
//! has no dimension for it at all. Since #298 it is the figures over their
//! own drop shadow with no plate behind them, and since the owner's word of
//! 25.09 it stands at the card's right edge: over the top-right corner where
//! the rows leave the room, beside the right edge where they do not
//! ([`cardplate::BadgePlace`]). `baylee_client_core::cardplate` says where
//! the quad lies ([`cardplate::badge_quad_rect`]) and where the count lies
//! in it; `card_common.wgsl`'s `count_badge` draws it, for both shaders
//! here.
//!
//! **The material key is the count, the place and which way up it reads.**
//! Every badge is the same quad, and the shader sizes the body inside it from
//! the count, so a table has at most one badge material per distinct count,
//! place and turn on it. There is no clock: a badge does not move on its own.
//! A badge whose card is seen from its far side is drawn a half turn round
//! ([`BadgeParams::turned`]), so its count reads upright to the one looking.

use baylee_client_core::cardplate::{self, BadgePlace};
use bevy::asset::embedded_asset;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

/// What the badge's shader reads.
///
/// Thirty-two bytes: a uniform block under the GL backend is laid out
/// `std140`, the rectangle takes the first sixteen, and the count, the
/// body's corner and the turn take the next sixteen.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub struct BadgeParams {
    /// The quad, [`cardplate::badge_quad_rect`]: `[x0, y0, x1, y1]` in card
    /// widths from the card's top-left corner, which is what turns the quad's
    /// UV into the point on the card the badge is laid out at.
    pub quad: Vec4,
    /// How many permanents the card stands for,
    /// [`cardplate::count_word`]: below two, nothing is drawn. Or, with
    /// [`cardplate::BADGE_ATTACHED`] set, how many cards lie folded under
    /// a host ([`cardplate::attached_word`], #305), from one.
    pub count: u32,
    /// Where the count's right end stands and its top, in card widths
    /// ([`cardplate::badge_rect`]): in the material and not the shader, so
    /// `badge_rect` is what moves the badge.
    pub right: f32,
    /// See [`Self::right`].
    pub top: f32,
    /// [`cardplate::PLATE_TURNED`] where the one looking sees the card from
    /// its far side: the figures are drawn a half turn round their box's
    /// middle, which covers what the box covered. Zero otherwise.
    pub turned: u32,
}

impl BadgeParams {
    /// The badge saying `count`, standing at `place`, upright on its card.
    #[must_use]
    pub fn new(count: u32, place: BadgePlace) -> Self {
        let [_, top, right, _] = cardplate::badge_rect(count, place);
        Self {
            quad: Vec4::from_array(cardplate::badge_quad_rect(place)),
            count,
            right,
            top,
            turned: 0,
        }
    }
}

/// Where the preview stands its badge: over the card, as a duel does, since
/// the preview has no row to be crowded by.
pub const PREVIEW: BadgePlace = BadgePlace::Above;

/// The badge's quad, `[width, height]` in card widths: one size at either
/// place.
#[must_use]
pub fn quad_size() -> Vec2 {
    let [x0, y0, x1, y1] = cardplate::badge_quad_rect(BadgePlace::Above);
    Vec2::new(x1 - x0, y1 - y0)
}

/// The badge on a card lying on the table.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct BadgeMaterial {
    /// The count and the quad.
    #[uniform(0)]
    pub params: BadgeParams,
    /// The glyph atlas the plate's numerals are drawn from, always
    /// [`crate::markatlas::MARKS`] — see
    /// [`MarksMaterial::marks`](crate::marksmat::MarksMaterial::marks) for why
    /// it is not an `Option`.
    #[texture(1)]
    #[sampler(2)]
    pub marks: Handle<Image>,
}

impl BadgeMaterial {
    /// The badge saying `count`, standing at `place`, and `turned` a half
    /// turn round for one seeing its card from the far side.
    #[must_use]
    pub fn new(count: u32, place: BadgePlace, turned: bool) -> Self {
        Self {
            params: BadgeParams {
                turned: if turned { cardplate::PLATE_TURNED } else { 0 },
                ..BadgeParams::new(count, place)
            },
            marks: crate::markatlas::MARKS,
        }
    }
}

impl Material for BadgeMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/badge.wgsl".into()
    }

    /// The badge's edge is antialiased and its shadow is soft, so it blends.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    /// Where a badge sits in the transparent pass: at the height the strip
    /// lies at, on a card's face. See [`table::sort_bias`](crate::table::sort_bias).
    fn depth_bias(&self) -> f32 {
        crate::table::sort_bias(crate::table::STRIP_RUNG)
    }
}

/// The same badge, on a card drawn as a UI node: the preview.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct BadgeUiMaterial {
    /// The count and the quad.
    #[uniform(0)]
    pub params: BadgeParams,
    /// The glyph atlas; see [`BadgeMaterial::marks`].
    #[texture(1)]
    #[sampler(2)]
    pub marks: Handle<Image>,
}

impl UiMaterial for BadgeUiMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/badge_ui.wgsl".into()
    }
}

/// UI badge materials, one per count.
///
/// The preview is rebuilt whenever the game moves, so a material minted per
/// rebuild would grow `Assets` for as long as the duel lasted; one per count
/// is at most one per distinct count ever previewed.
#[derive(Resource, Default)]
pub struct UiBadgeMaterials {
    made: HashMap<u32, Handle<BadgeUiMaterial>>,
}

impl UiBadgeMaterials {
    /// The material for `count`, made once.
    pub fn get(
        &mut self,
        count: u32,
        assets: &mut Assets<BadgeUiMaterial>,
    ) -> Handle<BadgeUiMaterial> {
        self.made
            .entry(count)
            .or_insert_with(|| {
                assets.add(BadgeUiMaterial {
                    params: BadgeParams::new(count, PREVIEW),
                    marks: crate::markatlas::MARKS,
                })
            })
            .clone()
    }

    /// Forgets every material, when the duel closes.
    pub fn clear(&mut self) {
        self.made.clear();
    }
}

/// Registers both badge materials and ships their shaders in the binary, for
/// the reason [`CardMaterialPlugin`](crate::cardmat::CardMaterialPlugin)
/// embeds its own. `card_common.wgsl`, which both import, is registered
/// there.
pub struct BadgeMaterialPlugin;

impl Plugin for BadgeMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/badge.wgsl");
        embedded_asset!(app, "shaders/badge_ui.wgsl");
        app.add_plugins(MaterialPlugin::<BadgeMaterial>::default())
            .add_plugins(UiMaterialPlugin::<BadgeUiMaterial>::default())
            .init_resource::<UiBadgeMaterials>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cardmat::tests::check_wgsl;

    /// The badge's table shader, parsed and validated against the vertex
    /// output the strip's is given; it reads no clock.
    #[test]
    fn the_badge_shader_compiles() {
        let prelude = "\
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};
";
        check_wgsl(
            include_str!("shaders/badge.wgsl"),
            &format!("{prelude}{}", include_str!("shaders/card_common.wgsl")),
        );
    }

    /// And its UI twin, with the UI card shader's stubs.
    #[test]
    fn the_ui_badge_shader_compiles() {
        let prelude = "\
struct UiVertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) border_widths: vec4<f32>,
    @location(2) border_radius: vec4<f32>,
    @location(3) @interpolate(flat) size: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
";
        check_wgsl(
            include_str!("shaders/badge_ui.wgsl"),
            &format!("{prelude}{}", include_str!("shaders/card_common.wgsl")),
        );
    }

    /// The uniform is two whole `std140` blocks, which is what the note on
    /// [`BadgeParams`] claims and the GL backend needs.
    #[test]
    fn the_badge_uniform_is_two_blocks() {
        assert_eq!(BadgeParams::min_size().get(), 32);
    }

    /// The two shaders declare the uniform in the order Rust lays it out.
    ///
    /// Nothing compares a WGSL struct with a Rust one: the count read as the
    /// rectangle's first corner draws the badge off the quad, with every
    /// shader still compiling.
    #[test]
    fn both_shaders_read_the_uniform_rust_writes() {
        for (which, src) in [
            ("badge.wgsl", include_str!("shaders/badge.wgsl")),
            ("badge_ui.wgsl", include_str!("shaders/badge_ui.wgsl")),
        ] {
            let open = src.find("struct BadgeParams {").expect("the uniform");
            let body = &src[open..open + src[open..].find('}').expect("it closes")];
            let fields: Vec<&str> = body
                .lines()
                .map(str::trim)
                .filter(|l| !l.starts_with("//") && l.contains(':'))
                .collect();
            assert_eq!(
                fields,
                [
                    "quad: vec4<f32>,",
                    "count: u32,",
                    "right: f32,",
                    "top: f32,",
                    "turned: u32,"
                ],
                "{which} lays the uniform out differently from `BadgeParams`"
            );
        }
    }

    /// One material per count, carrying the quad the shader lays the badge
    /// out in.
    #[test]
    fn a_count_is_one_material() {
        let mut assets = Assets::<BadgeUiMaterial>::default();
        let mut cache = UiBadgeMaterials::default();
        let four = cache.get(4, &mut assets);
        assert_eq!(
            cache.get(4, &mut assets),
            four,
            "the same count, the same material"
        );
        let five = cache.get(5, &mut assets);
        assert_ne!(five, four, "another count, another material");
        assert_eq!(assets.len(), 2);
        let made = assets.get(&four).expect("still there");
        assert_eq!(made.params.count, 4);
        assert_eq!(
            made.params.quad,
            Vec4::from_array(cardplate::badge_quad_rect(PREVIEW)),
            "the quad the shader lays the badge out in"
        );
        let [_, top, right, _] = cardplate::badge_rect(4, PREVIEW);
        assert_eq!(
            (made.params.right, made.params.top),
            (right, top),
            "where the shader stands the body"
        );
    }
}

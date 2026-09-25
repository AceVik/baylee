//! The plate's material: a creature's power and toughness, a planeswalker's
//! loyalty or a saga's chapter, as an object at its card's bottom right.
//!
//! The plate was the keyword strip's first cell (#298) until the owner,
//! 25.09, read the numbers sitting with the marks as the fault. So it is a
//! quad and a material of its own, like the count badge
//! ([`crate::badgemat`]): beside the printed box over the black border where
//! the row leaves it the room, on its own card's foot, right-aligned to the
//! part of the card in sight, where the next card reaches that far, and
//! upright under a tapped card. `baylee_client_core::cardplate` says where
//! ([`cardplate::plate_rect`]) and how big the quad is
//! ([`cardplate::plate_quad`]); `card_common.wgsl`'s `plate_object` draws
//! it, for both shaders here. The chip stays on the strip.
//!
//! **The material key is the two words and nothing else.** Every plate is
//! the same quad, and the shader lays the body out flush right inside it,
//! so where a plate stands is its transform's and a lane of twelve 2/2
//! Soldiers is one material. There is no clock: a plate does not move on
//! its own. The ink word also says which way up it reads: a plate whose card
//! is seen from its far side, as an opponent's across the table, is drawn a
//! half turn round ([`PlateWords::turned`]), so its numbers read upright to
//! the one looking (the PO, 25.09).

use baylee_client_core::cardplate;
use bevy::asset::embedded_asset;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

/// What the plate's shader reads.
///
/// Sixteen bytes: one `std140` block, which the GL backend lays a uniform
/// out in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ShaderType)]
pub struct PlateWords {
    /// The plate, [`cardplate::Plate::packed`].
    pub word: u32,
    /// Its ink, the second of [`cardplate::Corner::plate_words`]: the tone,
    /// [`cardplate::PLATE_NIGHT`] on a sleeping creature, and
    /// [`cardplate::PLATE_TURNED`] where its card is seen from the far side.
    pub ink: u32,
}

impl PlateWords {
    /// The words for `corner`'s plate, `night` on a creature that cannot
    /// attack yet.
    #[must_use]
    pub fn of(corner: cardplate::Corner, night: bool) -> Self {
        let [word, ink] = corner.plate_words(night);
        Self { word, ink }
    }

    /// The same words, drawn a half turn round ([`cardplate::PLATE_TURNED`])
    /// where `turned`: for one seeing the card from its far side.
    #[must_use]
    pub fn turned(self, turned: bool) -> Self {
        Self {
            ink: if turned {
                self.ink | cardplate::PLATE_TURNED
            } else {
                self.ink & !cardplate::PLATE_TURNED
            },
            ..self
        }
    }
}

/// The uniform: the quad's size and the words.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub struct PlateParams {
    /// The quad's size in card widths, [`quad_size`]: what turns the quad's
    /// UV into the lengths the plate is laid out in.
    pub quad: Vec2,
    /// The plate.
    pub word: u32,
    /// Its ink.
    pub ink: u32,
}

impl PlateParams {
    /// The shader's words for `words`.
    #[must_use]
    pub fn new(words: PlateWords) -> Self {
        Self {
            quad: quad_size(),
            word: words.word,
            ink: words.ink,
        }
    }
}

/// The plate's quad, `[width, height]` in card widths: one size for every
/// plate, wherever it stands.
#[must_use]
pub fn quad_size() -> Vec2 {
    let [x0, y0, x1, y1] =
        cardplate::plate_quad([0.0, 0.0, cardplate::PLATE_W, cardplate::PLATE_H]);
    Vec2::new(x1 - x0, y1 - y0)
}

/// Where the preview stands a plate of `kind`: beside the printed box, over
/// the black border, as a row with room to spare stands it — the preview has
/// no row to be crowded by. The quad, [`cardplate::plate_quad`].
#[must_use]
pub fn preview_quad(kind: u32) -> [f32; 4] {
    cardplate::plate_rect(kind, false, cardplate::PlateRoom::OPEN, None)
        .map_or([0.0; 4], |(body, _)| cardplate::plate_quad(body))
}

/// What the plate over one permanent says, read off the object itself: the
/// preview's, which has no board group behind it.
///
/// [`crate::marksmat::strip_of`]'s doors — `Corner::of_object` and
/// [`board::asleep`](baylee_client_core::board::asleep) — so a card held up
/// in the preview says what it says on the table; `None` where the print
/// says the numbers itself, as the preview shows the whole print.
#[must_use]
pub fn words_of(object: &baylee_view::PublicObject) -> Option<PlateWords> {
    let corner = cardplate::Corner::of_object(object);
    corner
        .shows_plate(true, false)
        .then(|| PlateWords::of(corner, baylee_client_core::board::asleep(object)))
}

/// The plate on a card lying on the table.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct PlateMaterial {
    /// The words and the quad.
    #[uniform(0)]
    pub params: PlateParams,
    /// The glyph atlas the numerals are drawn from, always
    /// [`crate::markatlas::MARKS`] — see
    /// [`MarksMaterial::marks`](crate::marksmat::MarksMaterial::marks) for why
    /// it is not an `Option`.
    #[texture(1)]
    #[sampler(2)]
    pub marks: Handle<Image>,
}

impl PlateMaterial {
    /// The plate saying `words`.
    #[must_use]
    pub fn new(words: PlateWords) -> Self {
        Self {
            params: PlateParams::new(words),
            marks: crate::markatlas::MARKS,
        }
    }
}

impl Material for PlateMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/plate.wgsl".into()
    }

    /// The plate's edge is antialiased and its shadow is soft, so it blends.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    /// Where a plate sits in the transparent pass: at the height the strip
    /// and the badge lie at. See [`table::sort_bias`](crate::table::sort_bias).
    fn depth_bias(&self) -> f32 {
        crate::table::sort_bias(crate::table::STRIP_RUNG)
    }
}

/// The same plate, on a card drawn as a UI node: the preview.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct PlateUiMaterial {
    /// The words and the quad.
    #[uniform(0)]
    pub params: PlateParams,
    /// The glyph atlas; see [`PlateMaterial::marks`].
    #[texture(1)]
    #[sampler(2)]
    pub marks: Handle<Image>,
}

impl UiMaterial for PlateUiMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/plate_ui.wgsl".into()
    }
}

/// UI plate materials, one per thing a plate says.
///
/// [`crate::badgemat::UiBadgeMaterials`]'s reason: the preview is rebuilt
/// whenever the game moves, and a material minted per rebuild would grow
/// `Assets` for as long as the duel lasted.
#[derive(Resource, Default)]
pub struct UiPlateMaterials {
    made: HashMap<PlateWords, Handle<PlateUiMaterial>>,
}

impl UiPlateMaterials {
    /// The material for `words`, made once.
    pub fn get(
        &mut self,
        words: PlateWords,
        assets: &mut Assets<PlateUiMaterial>,
    ) -> Handle<PlateUiMaterial> {
        self.made
            .entry(words)
            .or_insert_with(|| {
                assets.add(PlateUiMaterial {
                    params: PlateParams::new(words),
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

/// Registers both plate materials and ships their shaders in the binary, for
/// the reason [`CardMaterialPlugin`](crate::cardmat::CardMaterialPlugin)
/// embeds its own. `card_common.wgsl`, which both import, is registered
/// there.
pub struct PlateMaterialPlugin;

impl Plugin for PlateMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/plate.wgsl");
        embedded_asset!(app, "shaders/plate_ui.wgsl");
        app.add_plugins(MaterialPlugin::<PlateMaterial>::default())
            .add_plugins(UiMaterialPlugin::<PlateUiMaterial>::default())
            .init_resource::<UiPlateMaterials>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cardmat::tests::check_wgsl;
    use baylee_client_core::cardplate::{Corner, Plate};

    /// The plate's table shader, parsed and validated against the vertex
    /// output the strip's is given.
    #[test]
    fn the_plate_shader_compiles() {
        let prelude = "\
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};
";
        check_wgsl(
            include_str!("shaders/plate.wgsl"),
            &format!("{prelude}{}", include_str!("shaders/card_common.wgsl")),
        );
    }

    /// And its UI twin, with the UI card shader's stubs.
    #[test]
    fn the_ui_plate_shader_compiles() {
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
            include_str!("shaders/plate_ui.wgsl"),
            &format!("{prelude}{}", include_str!("shaders/card_common.wgsl")),
        );
    }

    /// The uniform is one whole `std140` block, which is what the note on
    /// [`PlateParams`] claims and the GL backend needs.
    #[test]
    fn the_plate_uniform_is_one_block() {
        assert_eq!(PlateParams::min_size().get(), 16);
    }

    /// The two shaders declare the uniform in the order Rust lays it out.
    ///
    /// Nothing compares a WGSL struct with a Rust one: the ink read as the
    /// plate draws no plate at all, with every shader still compiling.
    #[test]
    fn both_shaders_read_the_uniform_rust_writes() {
        for (which, src) in [
            ("plate.wgsl", include_str!("shaders/plate.wgsl")),
            ("plate_ui.wgsl", include_str!("shaders/plate_ui.wgsl")),
        ] {
            let open = src.find("struct PlateParams {").expect("the uniform");
            let body = &src[open..open + src[open..].find('}').expect("it closes")];
            let fields: Vec<&str> = body
                .lines()
                .map(str::trim)
                .filter(|l| !l.starts_with("//") && l.contains(':'))
                .collect();
            assert_eq!(
                fields,
                ["quad: vec2<f32>,", "word: u32,", "ink: u32,"],
                "{which} lays the uniform out differently from `PlateParams`"
            );
        }
    }

    /// The quad the shader lays the plate out in is the one the table and
    /// the preview draw: `plate_quad` of any body, since every body is
    /// flush right in the widest plate's quad.
    #[test]
    fn every_plate_is_one_quad() {
        use baylee_client_core::cardplate::{KIND_LORE, PlateRoom, plate_body_width, plate_rect};
        for kind in [cardplate::KIND_FIGHT, cardplate::KIND_LOYALTY, KIND_LORE] {
            for tapped in [false, true] {
                let (body, _) = plate_rect(kind, tapped, PlateRoom::OPEN, None).expect("room");
                assert!((body[2] - body[0] - plate_body_width(kind)).abs() < 1e-6);
                let [x0, y0, x1, y1] = cardplate::plate_quad(body);
                assert!((Vec2::new(x1 - x0, y1 - y0) - quad_size()).length() < 1e-6);
            }
        }
    }

    /// One material per thing a plate says, carrying the quad the shader
    /// lays the plate out in; a sleeping creature's plate is another.
    #[test]
    fn a_plate_is_one_material() {
        let mut assets = Assets::<PlateUiMaterial>::default();
        let mut cache = UiPlateMaterials::default();
        let corner = Corner {
            plate: Plate::Fight {
                power: 2,
                toughness: 2,
                damage: 0,
            },
            ..Corner::default()
        };
        let awake = cache.get(PlateWords::of(corner, false), &mut assets);
        assert_eq!(cache.get(PlateWords::of(corner, false), &mut assets), awake);
        let asleep = cache.get(PlateWords::of(corner, true), &mut assets);
        assert_ne!(asleep, awake, "a sleeping creature's plate is moon-grey");
        assert_eq!(assets.len(), 2);
        let made = assets.get(&awake).expect("still there");
        assert_eq!(made.params.quad, quad_size());
        assert_eq!(made.params.word, corner.plate.packed());
    }
}

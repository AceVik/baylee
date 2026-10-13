//! The shells round a permanent on its preview (the PM, 25.09): a card held
//! up to the light wears what it wears on the felt, its keywords' border
//! and summoning sickness's wave.
//!
//! The table's shells are meshes with a vertex stage of their own, which
//! follows the real camera's ray to the card's face ([`crate::shellmat`]);
//! the preview is a UI node with no camera to follow. So this is the table
//! shader's twin, as [`crate::platemat`]'s UI plate is the plate's:
//! `shell_ui.wesl` draws each shell as it stands, seen from straight over the
//! card, which is what lines it up with the flat print, with the layers of
//! `shell_common.wesl`, which the table draws with too. It always stands:
//! the preview has air round it, so nothing lies down as a ring.
//!
//! Which shells a card wears is [`Shells::of`](crate::shellmat::Shells::of)'s,
//! the door the table's rows ask as well; what the preview draws is
//! [`Shells::standing`](crate::shellmat::Shells::standing), one node per
//! look. **A new shell is a look there and an arm in `shell_ui.wesl`**, and
//! the preview asks nothing else.
//!
//! **Nothing but the wave lies on the print** (`docs/legal.md` §3): from
//! straight over the card a point's ray meets the face right under it, so
//! the mask is the card's own outline, and every colour the shader returns
//! but the wave's carries it (`every_colour_but_the_waves_carries_the_mask`).

use baylee_client_core::cardrail::CARD_TALL;
use bevy::asset::embedded_asset;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use crate::cardmat::motion_of;
use crate::shellmat::{ShellLook, border_margin};

/// How much further than the furthest shell the preview's node reaches, in
/// card widths: room for the last pixel of each edge's smoothing.
const AIR: f32 = 0.02;
/// How far past the card's sides and foot the node reaches: the widest
/// border's foot.
pub const PREVIEW_SIDE: f32 = 0.11 + AIR;
/// How far past the card's top edge it reaches: as far, every border being
/// the same all round.
pub const PREVIEW_TOP: f32 = PREVIEW_SIDE;
/// The node every shell of the preview is drawn in, `[x0, y0, x1, y1]` in
/// card widths from the card's top-left corner, `y` down it: the card grown
/// by as far as any shell reaches past each edge.
pub const PREVIEW_QUAD: [f32; 4] = [
    -PREVIEW_SIDE,
    -PREVIEW_TOP,
    1.0 + PREVIEW_SIDE,
    CARD_TALL + PREVIEW_SIDE,
];

/// A shell on the preview: a marker, so one can be found and counted
/// without being taken for the card, its strip, its plate or its badge.
#[derive(Component)]
pub struct PreviewShell;

/// What the preview's shell shader reads.
///
/// Three whole `std140` rows: the node; the kind, the clock and the
/// keywords; the border's margin.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub struct ShellUiParams {
    /// The node, [`PREVIEW_QUAD`]: what turns its UV into the point on the
    /// card the shell is laid out at.
    pub quad: Vec4,
    /// A [`ShellKind`](crate::shellmat::ShellKind).
    pub kind: u32,
    /// The clock: [`MOVING`](crate::cardmat::MOVING) or
    /// [`STILL`](crate::cardmat::STILL).
    pub motion: f32,
    /// The keywords ([`crate::shellmat::Ward`]).
    pub ward: u32,
    /// How many layers they make.
    pub layers: u32,
    /// How far past the card's edge the border's foot stands
    /// ([`border_margin`]).
    pub margin: f32,
    /// Nothing: the third row's rest.
    pub spare: f32,
    /// See [`Self::spare`].
    pub rest: Vec2,
}

impl ShellUiParams {
    /// What the shader is told to draw `look` on the clock `motion`.
    #[must_use]
    pub fn new(look: ShellLook, motion: f32) -> Self {
        let layers = look.ward.layers();
        Self {
            quad: Vec4::from_array(PREVIEW_QUAD),
            kind: look.kind as u32,
            motion,
            ward: look.ward.0,
            layers,
            margin: border_margin(layers),
            spare: 0.0,
            rest: Vec2::ZERO,
        }
    }
}

/// One shell on the preview.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct ShellUiMaterial {
    /// The look, the node and the clock.
    #[uniform(0)]
    pub params: ShellUiParams,
}

impl UiMaterial for ShellUiMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/shell_ui.wesl".into()
    }
}

/// The preview's shell materials, one per look.
///
/// [`crate::badgemat::UiBadgeMaterials`]'s reason: the preview is rebuilt
/// whenever the game moves, and a material minted per rebuild would grow
/// `Assets` for as long as the duel lasted. Holding still is rewritten in
/// place, as [`crate::marksmat::UiMarksMaterials`] does it.
#[derive(Resource, Default)]
pub struct UiShellMaterials {
    made: HashMap<ShellLook, Handle<ShellUiMaterial>>,
    still: bool,
}

impl UiShellMaterials {
    /// The material for `look`, made once.
    pub fn get(
        &mut self,
        look: ShellLook,
        assets: &mut Assets<ShellUiMaterial>,
    ) -> Handle<ShellUiMaterial> {
        let motion = motion_of(self.still);
        self.made
            .entry(look)
            .or_insert_with(|| {
                assets.add(ShellUiMaterial {
                    params: ShellUiParams::new(look, motion),
                })
            })
            .clone()
    }

    /// Forgets every material, when the duel closes.
    pub fn clear(&mut self) {
        self.made.clear();
    }

    fn set_still(&mut self, still: bool, assets: &mut Assets<ShellUiMaterial>) {
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

/// The preview's shells follow the preference, as its card does.
fn track_motion(
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut cache: ResMut<UiShellMaterials>,
    mut assets: ResMut<Assets<ShellUiMaterial>>,
) {
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    cache.set_still(still, &mut assets);
}

/// Registers the preview's shell material and ships its shader in the
/// binary, for the reason
/// [`CardMaterialPlugin`](crate::cardmat::CardMaterialPlugin) embeds its own.
/// `card_common.wesl` and `shell_common.wesl`, which it imports, are
/// registered with the card's material and the table's shells.
pub struct ShellUiPlugin;

impl Plugin for ShellUiPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/shell_ui.wesl");
        app.add_plugins(UiMaterialPlugin::<ShellUiMaterial>::default())
            .init_resource::<UiShellMaterials>()
            .add_systems(Update, track_motion);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cardmat::tests::{check_wgsl, wgsl_const};
    use crate::shellmat::MAX_LAYERS;

    const SHADER: &str = include_str!("shaders/shell_ui.wesl");
    const COMMON: &str = include_str!("shaders/shell_common.wesl");

    /// The shader, parsed and validated with the UI card shader's stubs and
    /// both files it imports.
    #[test]
    fn the_ui_shell_shader_compiles() {
        let prelude = "\
struct UiVertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) border_widths: vec4<f32>,
    @location(2) border_radius_x: vec4<f32>,
    @location(3) border_radius_y: vec4<f32>,
    @location(4) @interpolate(flat) size: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
struct Globals { time: f32 };
";
        check_wgsl(
            SHADER,
            &format!(
                "{prelude}{}{COMMON}",
                include_str!("shaders/card_common.wesl")
            ),
        );
    }

    /// Three whole `std140` rows.
    #[test]
    fn the_uniform_is_three_whole_rows() {
        assert_eq!(ShellUiParams::min_size().get(), 48);
    }

    /// The shader declares the uniform in the order Rust lays it out.
    #[test]
    fn the_shader_reads_the_uniform_rust_writes() {
        let open = SHADER.find("struct ShellUiParams {").expect("the uniform");
        let body = &SHADER[open..open + SHADER[open..].find('}').expect("it closes")];
        let fields: Vec<&str> = body
            .lines()
            .map(str::trim)
            .filter(|l| !l.starts_with("//") && l.contains(':'))
            .collect();
        assert_eq!(
            fields,
            [
                "quad: vec4<f32>,",
                "kind: u32,",
                "motion: f32,",
                "ward: u32,",
                "layers: u32,",
                "margin: f32,",
                "spare: f32,",
                "rest: vec2<f32>,"
            ],
            "shell_ui.wesl lays the uniform out differently from `ShellUiParams`"
        );
    }

    /// The duel's lean the preview lights the wave at is the table's.
    #[test]
    fn the_shader_leans_as_a_duel_does() {
        let theirs = wgsl_const(SHADER, "LEAN");
        assert!((theirs - crate::table::DUEL_LEAN).abs() < 1e-4);
    }

    /// Every colour the shader returns but the wave's has the mask as the
    /// last factor of its alpha: a `return` that forgot it would be the
    /// border drawn over the preview's print.
    #[test]
    fn every_colour_but_the_waves_carries_the_mask() {
        let open = SHADER.find("fn fragment(").expect("the fragment stage");
        let body = &SHADER[open..];
        let branch = "if params.kind == SHELL_WAVE {";
        let start = body.find(branch).expect("the branch");
        let end = start + body[start..].find("\n    }").expect("it closes");
        let own = &body[start..end];
        assert_eq!(own.matches("return").count(), 1, "`{branch}` returns once");
        assert!(own.contains("return vec4<f32>(MOONLIGHT, glow);"));
        let mut at = 0;
        let mut returns = Vec::new();
        for line in body.split_inclusive('\n') {
            let here = at;
            at += line.len();
            if !(start..end).contains(&here) && line.trim().starts_with("return") {
                returns.push(line.trim());
            }
        }
        assert!(!returns.is_empty(), "the fragment stage returns too little");
        for line in returns {
            let alpha = line
                .strip_suffix(");")
                .and_then(|l| l.rsplit_once(','))
                .map(|(_, alpha)| alpha.trim());
            assert!(
                alpha.is_some_and(|a| a == "clear" || a.ends_with("* clear")),
                "`{line}` returns an alpha the mask has not touched"
            );
        }
    }

    /// The node holds the widest border the preview draws, short of the
    /// node's edge by the smoothing's room.
    #[test]
    fn the_node_holds_every_shell() {
        assert!(border_margin(MAX_LAYERS) + AIR <= PREVIEW_SIDE + 1e-6);
        assert!(border_margin(MAX_LAYERS) + AIR <= PREVIEW_TOP + 1e-6);
    }
}

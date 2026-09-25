//! The shells round a permanent on its preview (the PM, 25.09): a card held
//! up to the light wears what it wears on the felt, indestructible's
//! darksteel rim, hexproof's or shroud's dome and defender's wall.
//!
//! The table's shells are meshes with a vertex stage of their own, which
//! follows the real camera's ray to the card's face ([`crate::shellmat`]);
//! the preview is a UI node with no camera to follow. So this is the table
//! shader's twin, as [`crate::platemat`]'s UI plate is the plate's:
//! `shell_ui.wgsl` draws each shell as it stands, seen from straight over the
//! card, which is what lines it up with the flat print, with the steel, the
//! glass and the brick of `shell_common.wgsl`, which the table draws with
//! too. It always stands: the preview has air round it, so nothing lies down
//! as a ring and nothing casts a shadow on a felt that is not there.
//!
//! Which shells a card wears is [`Shells::of`]'s, the door the table's rows
//! ask as well; what the preview draws is [`Shells::standing`], one node per
//! look. **A new shell is a look there and an arm in `shell_ui.wgsl`**, and
//! the preview asks nothing else.
//!
//! **Nothing but a dome lies on the print** (`docs/legal.md` §3): from
//! straight over the card a point's ray meets the face right under it, so
//! the mask is the card's own outline, and every colour the shader returns
//! but the dome's carries it (`every_colour_but_the_domes_carries_the_mask`).

use baylee_client_core::cardrail::CARD_TALL;
use bevy::asset::embedded_asset;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use crate::cardmat::motion_of;
use crate::shellmat::{
    DOME_MARGIN, Dome, RIM_MARGIN, ShellLook, WALL_BATTER, WALL_BULGE, WALL_NEAR, WALL_OVERHANG,
    WALL_THICK,
};

/// How much further than the furthest shell the preview's node reaches, in
/// card widths: room for the last pixel of each edge's smoothing.
const AIR: f32 = 0.02;
/// How far past the card's sides and foot the node reaches: a dome's foot,
/// which stands furthest out there.
pub const PREVIEW_SIDE: f32 = DOME_MARGIN + AIR;
/// How far past the card's top edge it reaches: the wall's face towards the
/// table's middle, at its middle, where it bows furthest out.
pub const PREVIEW_TOP: f32 = WALL_NEAR + WALL_BATTER + WALL_THICK + WALL_BULGE + AIR;
/// The node every shell of the preview is drawn in, `[x0, y0, x1, y1]` in
/// card widths from the card's top-left corner, `y` down it: the card grown
/// by as far as any shell reaches past each edge.
pub const PREVIEW_QUAD: [f32; 4] = [
    -PREVIEW_SIDE,
    -PREVIEW_TOP,
    1.0 + PREVIEW_SIDE,
    CARD_TALL + PREVIEW_SIDE,
];

const _: () = assert!(RIM_MARGIN < DOME_MARGIN && WALL_OVERHANG < DOME_MARGIN);

/// A shell on the preview: a marker, so one can be found and counted
/// without being taken for the card, its strip, its plate or its badge.
#[derive(Component)]
pub struct PreviewShell;

/// What the preview's shell shader reads.
///
/// Three whole `std140` rows: the colour, the node, and the kind, the clock
/// and a dome's row.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub struct ShellUiParams {
    /// A dome's colour, linear; the steel and the wall ignore it.
    pub tint: Vec4,
    /// The node, [`PREVIEW_QUAD`]: what turns its UV into the point on the
    /// card the shell is laid out at.
    pub quad: Vec4,
    /// A [`ShellKind`](crate::shellmat::ShellKind).
    pub kind: u32,
    /// The clock: [`MOVING`](crate::cardmat::MOVING) or
    /// [`STILL`](crate::cardmat::STILL).
    pub motion: f32,
    /// A dome's row, [`crate::shellmat::DomeRow`]: its foot past the card's
    /// edge and its crown over the face. Zero for the steel and the wall.
    pub dome: Vec2,
}

impl ShellUiParams {
    /// What the shader is told to draw `look` on the clock `motion`.
    #[must_use]
    pub fn new(look: ShellLook, motion: f32) -> Self {
        let row = look.dome.map(Dome::row);
        Self {
            tint: Vec3::from_array(row.map_or([0.0; 3], |row| row.tint)).extend(1.0),
            quad: Vec4::from_array(PREVIEW_QUAD),
            kind: look.kind as u32,
            motion,
            dome: row.map_or(Vec2::ZERO, |row| Vec2::new(row.margin, row.height)),
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
        "embedded://baylee_client/shaders/shell_ui.wgsl".into()
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
/// `card_common.wgsl` and `shell_common.wgsl`, which it imports, are
/// registered with the card's material and the table's shells.
pub struct ShellUiPlugin;

impl Plugin for ShellUiPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/shell_ui.wgsl");
        app.add_plugins(UiMaterialPlugin::<ShellUiMaterial>::default())
            .init_resource::<UiShellMaterials>()
            .add_systems(Update, track_motion);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cardmat::tests::{check_wgsl, wgsl_const};
    use crate::shellmat::DOME_CROWN;

    const SHADER: &str = include_str!("shaders/shell_ui.wgsl");
    const COMMON: &str = include_str!("shaders/shell_common.wgsl");

    /// The shader, parsed and validated with the UI card shader's stubs and
    /// both files it imports.
    #[test]
    fn the_ui_shell_shader_compiles() {
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
            SHADER,
            &format!(
                "{prelude}{}{COMMON}",
                include_str!("shaders/card_common.wgsl")
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
                "tint: vec4<f32>,",
                "quad: vec4<f32>,",
                "kind: u32,",
                "motion: f32,",
                "dome: vec2<f32>,"
            ],
            "shell_ui.wgsl lays the uniform out differently from `ShellUiParams`"
        );
    }

    /// Every number the shader and `shellmat` share is the same number in
    /// both: a wall drawn off the arc the table builds it on is another wall.
    #[test]
    fn the_shader_draws_the_shells_shellmat_builds() {
        for (name, ours) in [
            ("RIM_MARGIN", RIM_MARGIN),
            ("DOME_CROWN", DOME_CROWN),
            ("DOME_ROUNDING", crate::shellmat::DOME_ROUNDING),
            ("CAP_ROUND", crate::shellmat::CAP_ROUND),
            ("WALL_NEAR", WALL_NEAR),
            ("WALL_BULGE", WALL_BULGE),
            ("WALL_THICK", WALL_THICK),
            ("WALL_BATTER", WALL_BATTER),
            ("WALL_OVERHANG", WALL_OVERHANG),
            ("LEAN", crate::table::DUEL_LEAN),
        ] {
            let theirs = wgsl_const(SHADER, name);
            assert!(
                (theirs - ours).abs() < 1e-4,
                "{name}: {ours} in Rust, {theirs} in shell_ui.wgsl"
            );
        }
        #[expect(clippy::cast_precision_loss)] // five merlons
        let merlons = crate::shellmat::WALL_MERLONS as f32;
        assert!((wgsl_const(SHADER, "WALL_MERLONS") - merlons).abs() < f32::EPSILON);
    }

    /// Every colour the shader returns but the dome's has the mask as the
    /// last factor of its alpha: a `return` that forgot it would be the rim
    /// or the wall drawn over the preview's print.
    #[test]
    fn every_colour_but_the_domes_carries_the_mask() {
        let open = SHADER.find("fn fragment(").expect("the fragment stage");
        let body = &SHADER[open..];
        let dome = body
            .find("if params.kind == SHELL_DOME {")
            .expect("the dome's branch");
        let dome_end = dome + body[dome..].find("\n    }").expect("it closes");
        let glass = &body[dome..dome_end];
        assert_eq!(
            glass.matches("return").count(),
            1,
            "the dome's branch returns once"
        );
        assert!(glass.contains("return vec4<f32>(colour, glass);"));
        let returns: Vec<&str> = body[..dome]
            .lines()
            .chain(body[dome_end..].lines())
            .map(str::trim)
            .filter(|l| l.starts_with("return"))
            .collect();
        assert!(returns.len() >= 2, "the fragment stage returns too little");
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

    /// The node holds every shell the preview draws: a dome's foot of every
    /// row, the rim's foot and the wall's far face at its middle, each short
    /// of the node's edge by the smoothing's room.
    #[test]
    fn the_node_holds_every_shell() {
        for dome in Dome::ALL {
            assert!(dome.row().margin + AIR <= PREVIEW_SIDE + 1e-6, "{dome:?}");
        }
        const { assert!(RIM_MARGIN + AIR <= PREVIEW_SIDE) };
        let (middle, out, _) = crate::shellmat::wall_arc(0.5);
        let far =
            middle.y + out.y * WALL_THICK / 2.0 - baylee_client_core::layout::CARD_HEIGHT / 2.0;
        assert!(
            far + AIR <= PREVIEW_TOP + 1e-6,
            "the wall reaches {far} past the card"
        );
        for u in [0.0, 1.0] {
            let (end, _, _) = crate::shellmat::wall_arc(u);
            assert!(end.x.abs() - 0.5 + AIR <= PREVIEW_SIDE + 1e-6);
        }
    }
}

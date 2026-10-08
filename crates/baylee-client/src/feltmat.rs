//! Smoked glass over animated elemental strata, with the dial in the middle.
//! Analytic reflections and caustics keep the surface in one opaque pass;
//! card art stays unlit and independent of the eased day/night exposure.

use baylee_client_core::tabletop;
use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

/// Everything the felt shader needs.
#[derive(Clone, Copy, ShaderType, Debug, Default)]
pub struct FeltParams {
    /// The phase lamp: `rgb` its colour, `w` how much of it there is.
    ///
    /// Straight out of [`tabletop::phase_light`], eased rather than snapped —
    /// the rail runs all the way round the table and a step boundary that
    /// changed it in one frame would read as a flash.
    pub wash: Vec4,
    /// Where that light enters the rail: `xy` a point on the active seat's
    /// own edge in table space, `zw` the direction it travels from there.
    ///
    /// This is what keeps the signal honest at more than two seats. "Warm at
    /// one end, cool at the other" cannot be a property of a *ring*; it is a
    /// property of whose turn it is, and it moves.
    pub source: Vec4,
    /// The light the room is in: `rgb` a multiplier on the table's own
    /// colour, `w` how much of it arrives.
    ///
    /// Written by [`crate::sky::light_the_table`] out of the same eased
    /// [`SkyPhase`](baylee_client_core::sky::SkyPhase) the sky itself is
    /// drawn from, which is what makes the day/night change one movement:
    /// nothing here knows a transition is happening, and the table follows
    /// the sky because both read the same number.
    pub ambient: Vec4,
    /// How hard the first four flames of the firewheel burn, 0 to 1: white,
    /// blue, black, red.
    ///
    /// Out of [`baylee_client_core::firewheel::strength`], eased rather than
    /// snapped — a flame that grows over a second and a half is a fire being
    /// fed, and one that jumps is a signal.
    pub flames: Vec4,
    /// `x` green's strength; `yz` **screen-up in table space**; `w` spare.
    ///
    /// Screen-up is one direction for all five and not each flame's own
    /// radius, which is the difference between five candles seen from one
    /// chair and a sun glyph. It is the local seat's inward direction, so it
    /// is right at four seats and at eight as well as at a duel.
    pub flames_tail: Vec4,
    /// The slab's world size, which is how the shader turns a point on the
    /// table into a point in the cloth.
    ///
    /// The shader works from the world position and this, rather than from
    /// the mesh's own uv: a uv origin is a convention of whichever builder
    /// made the mesh, and guessing it wrong mirrors the whole field — which a
    /// duel, symmetric about both axes, would not show.
    pub span: Vec2,
    /// The corner radius the mesh was cut with, so the rail follows the same
    /// racetrack the slab does.
    pub corner: f32,
    /// How wide the padded rail runs, in table units.
    pub rail: f32,
    /// The clock the pulse runs on: [`MOVING`](crate::cardmat::MOVING) or
    /// [`STILL`](crate::cardmat::STILL).
    ///
    /// The same two values the cards use, and deliberately the same
    /// constants: a table that kept moving while every card on it held still
    /// would make the setting look broken.
    pub motion: f32,
    /// How hard the lamp burns at full energy.
    pub gain: f32,
    /// How thick the slab is, so the apron can be shaded down its height.
    pub thickness: f32,
    /// How many seats stand on the dial (0 to 8): the jewels drawn.
    pub seats: f32,
    /// Per-duel domain offset, orientation, and scale of the vascular pattern.
    pub pattern: Vec4,
    /// Where each cellular field's cells sit in [`FeltMaterial::veins`]: the
    /// texel offset of the trunk field in `xy`, of the capillary field in
    /// `zw` (`baylee_client_core::feltveins::VeinTable::offsets`).
    pub veins: Vec4,
    /// The dial's two hands as table-space vectors (DESIGN-v7 §3.2): the turn
    /// hand in `xy`, the priority hand in `zw`. Written by
    /// [`crate::dial::turn_the_dial`] only while one moves.
    pub hands: Vec4,
    /// `x` the priority hand's length (0 retracted), `y` whether it points at
    /// me, `z`/`w` when the turn and the priority hand last arrived, on the
    /// shader's clock (the tip lights fade from there).
    pub dial: Vec4,
    /// `x` when the hub last pulsed, `y` which light: 1 ivory (the turn came
    /// to me), 2 teal (priority did), 0 none.
    pub pulse: Vec4,
    /// Every seat's jewel direction, two per vector (`xy`, `zw`).
    pub jewels: [Vec4; 4],
    /// Every seat's jewel colour in `rgb` (display-referred); `a` 1 at the
    /// table, 0.4 left the game, plus 2 while choosing an opening hand.
    pub tints: [Vec4; 8],
    /// A team's colour round the jewel; `a` 1 where the seat has a team.
    pub teams: [Vec4; 8],
    /// The tear (DESIGN-v8, the owner's of 07.10.2026), on a piece of the
    /// tearing table (`table::pieces`): `y` the jagged line's seed, `z` the
    /// molten seam's brightness, `w` the piece's side (-1 mine, 1 far). The
    /// whole slab holds zero.
    pub rift: Vec4,
    /// Where the baked slow fields lie ([`FeltMaterial::warp`],
    /// `baylee_client_core::feltwarp::WarpField::rect`); zero until the grid
    /// for this cut has been computed, and the shader works them out itself.
    pub warp: Vec4,
}

/// Random presentation seed, sampled once when a duel is created.
/// Kept on `Duel`, so resizing, reconnecting, and card updates retain the surface.
#[derive(Clone, Copy, Debug)]
pub struct TablePattern(
    /// Bounded shader domain parameters: offset, angle, and scale.
    pub Vec4,
);

impl TablePattern {
    fn from_seed(seed: u64) -> Self {
        let bytes = seed.to_le_bytes();
        let part = |i| f32::from(u16::from_le_bytes([bytes[i], bytes[i + 1]])) / 256.0;
        Self(Vec4::new(part(0), part(2), part(4), part(6)))
    }
}

impl Default for TablePattern {
    fn default() -> Self {
        Self::from_seed(crate::host::fresh_seed())
    }
}

/// How bright the rail burns at the top of combat.
///
/// Above 1.0 deliberately, and what that buys is **saturation, not bloom**.
/// This comment used to say the camera was HDR and a value past white would
/// bloom; it is not and it does not. In bevy 0.19 an HDR intermediate is the
/// opt-in `Hdr` *component*, the table camera carries `Tonemapping::None`
/// and nothing else, and there is no bloom pass anywhere in this client — so
/// everything past 1.0 is clipped by the 8-bit target. The mistake is worth
/// recording rather than quietly deleting, because it is the mechanism the
/// next person designing light on this table will reach for: there is no
/// bloom to reach for, and a glow here has to be *painted* — an additive
/// term on the cloth, which is exactly what `felt.wgsl` does with this
/// number.
///
/// What the overshoot does do is hold the middle of the rail at white while
/// the ends still fall off, which is what a lamp looks like. It is safe here
/// in a way it would not be anywhere else on this table: the cards use their
/// own unlit shader and take no light from the scene, so the glow spreads
/// over the felt and stops at the cardboard.
pub const WASH_GAIN: f32 = 1.80;

/// The slab.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct FeltMaterial {
    /// Everything the shader computes from. The cloth itself is arithmetic
    /// (see the module header); the one texture below is a table of numbers
    /// the arithmetic would otherwise redo on every pixel.
    #[uniform(0)]
    pub params: FeltParams,
    /// Every vein cell's point, computed once per cut
    /// (`baylee_client_core::feltveins`, [`vein_points`]).
    #[texture(1)]
    pub veins: Handle<Image>,
    /// The cloth's slow fields — the warp, the heat, the silt — baked once
    /// per cut off the main thread ([`BakeWarp`]); a one-texel stand-in
    /// until then, which the shader does not read ([`FeltParams::warp`]).
    #[texture(2)]
    #[sampler(3)]
    pub warp: Handle<Image>,
}

/// The baked fields as the texture the felt samples: four half floats a
/// texel, filtered linearly, clamped at the edges. `RENDER_WORLD` only, like
/// the vein table.
#[must_use]
pub fn warp_image(field: &baylee_client_core::feltwarp::WarpField) -> Image {
    use bevy::asset::RenderAssetUsages;
    use bevy::image::ImageSampler;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let mut bytes = Vec::with_capacity(field.texels.len() * 8);
    for texel in &field.texels {
        for value in texel {
            bytes.extend_from_slice(&half::f16::from_f32(*value).to_le_bytes());
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: field.width,
            height: field.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        bytes,
        TextureFormat::Rgba16Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::linear();
    image
}

/// The stand-in a slab is cut with until its fields are baked: one texel,
/// of the format the shader's binding expects, never read.
#[must_use]
pub fn no_warp() -> Image {
    warp_image(&baylee_client_core::feltwarp::WarpField {
        width: 1,
        height: 1,
        origin: [0.0, 0.0],
        size: [1.0, 1.0],
        texels: vec![[0.0; 4]],
    })
}

/// The slow fields of a cut being baked off the main thread, on the slab
/// they are for: the texture and the rect it covers.
#[derive(Component)]
pub struct BakeWarp(bevy::tasks::Task<(Image, Vec4)>);

impl BakeWarp {
    /// Starts baking the fields for a slab `span` across under `pattern`.
    #[must_use]
    pub fn start(span: Vec2, pattern: Vec4) -> Self {
        let (span, pattern) = (span.to_array(), pattern.to_array());
        Self(bevy::tasks::AsyncComputeTaskPool::get().spawn(async move {
            let field = baylee_client_core::feltwarp::WarpField::for_table(span, pattern);
            (warp_image(&field), Vec4::from_array(field.rect()))
        }))
    }
}

/// Puts a finished bake on its slab's material: the shader reads the grid
/// from the next frame on. A slab cut again meanwhile has a new bake on it,
/// and the old one was dropped with its component, unfinished.
pub fn install_the_warp(
    mut commands: Commands,
    mut bakes: Query<(Entity, &mut BakeWarp, &MeshMaterial3d<FeltMaterial>)>,
    mut materials: ResMut<Assets<FeltMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    for (entity, mut bake, material) in &mut bakes {
        let Some((image, rect)) =
            bevy::tasks::block_on(bevy::tasks::futures_lite::future::poll_once(&mut bake.0))
        else {
            continue;
        };
        commands.entity(entity).remove::<BakeWarp>();
        if let Some(mut material) = materials.get_mut(&material.0) {
            // The old texture goes with its last handle, not before: a
            // tearing table's pieces may still be drawn with a copy of it.
            material.warp = images.add(image);
            material.params.warp = rect;
        }
    }
}

/// The vein table for a slab `span` across under `pattern`: the image the
/// felt reads its cell points from, and the offsets that go in
/// [`FeltParams::veins`].
///
/// `RENDER_WORLD` only: nothing on the CPU reads it again, so its bytes leave
/// the main world once uploaded. A duel's table is a few kilobytes.
#[must_use]
pub fn vein_points(span: Vec2, pattern: Vec4) -> (Image, Vec4) {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let table =
        baylee_client_core::feltveins::VeinTable::for_table(span.to_array(), pattern.to_array());
    let (width, height, texels) = if table.texels.is_empty() {
        (1, 1, vec![0, 0])
    } else {
        (table.width, table.height, table.texels)
    };
    let image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        texels,
        TextureFormat::Rg8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    #[allow(clippy::cast_precision_loss)]
    let offsets = Vec4::new(
        table.offsets[0][0] as f32,
        table.offsets[0][1] as f32,
        table.offsets[1][0] as f32,
        table.offsets[1][1] as f32,
    );
    (image, offsets)
}

impl Material for FeltMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/felt.wgsl".into()
    }

    /// The slab is the floor of the scene: opaque, and the only thing under
    /// the cards that is. Everything painted on top of it — the mats, the
    /// medallion, the glows — blends over it, which is what makes them read
    /// as lying on a table rather than as being the table.
    ///
    /// It is opaque *and* cut to shape, which is the point of the mesh being
    /// a racetrack: what shows outside the table is the sky, and it shows
    /// because there is no table there, not because the table is see-through.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
}

/// The lamp a step calls for, packed for the shader.
#[must_use]
pub fn wash_of(step: baylee_view::Step) -> Vec4 {
    let light = tabletop::phase_light(step);
    Vec4::new(light.rgb[0], light.rgb[1], light.rgb[2], light.energy)
}

/// Installs the felt material and its shader.
pub struct FeltMaterialPlugin;

impl Plugin for FeltMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/felt.wgsl");
        app.add_plugins(MaterialPlugin::<FeltMaterial>::default());
    }
}

#[cfg(test)]
mod pattern_tests {
    use super::*;

    #[test]
    fn distinct_seeds_produce_stable_bounded_shader_domains() {
        let a = TablePattern::from_seed(0).0;
        let b = TablePattern::from_seed(u64::MAX).0;
        assert_ne!(a, b);
        assert!(b.cmpge(Vec4::ZERO).all() && b.cmplt(Vec4::splat(256.0)).all());
        assert_eq!(a, TablePattern::from_seed(0).0);
        assert_eq!(b, TablePattern::from_seed(u64::MAX).0);
    }
}

#[cfg(test)]
mod warp_tests {
    use super::*;

    /// A bake started on a slab ends on its material: the grid's texture in
    /// place of the stand-in, and the rect that tells the shader to read it
    /// — the same rect the grid itself names. Until then the material says
    /// zero, and the shader computes.
    #[test]
    fn a_finished_bake_is_put_on_its_slab() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<FeltMaterial>>()
            .add_systems(Update, install_the_warp);
        let (span, pattern) = (Vec2::new(6.0, 4.0), Vec4::new(3.0, 5.0, 64.0, 128.0));
        let stand_in = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(no_warp());
        let material = app
            .world_mut()
            .resource_mut::<Assets<FeltMaterial>>()
            .add(FeltMaterial {
                params: FeltParams::default(),
                veins: Handle::default(),
                warp: stand_in.clone(),
            });
        let slab = app
            .world_mut()
            .spawn((
                MeshMaterial3d(material.clone()),
                BakeWarp::start(span, pattern),
            ))
            .id();
        for _ in 0..2000 {
            app.update();
            if app.world().get::<BakeWarp>(slab).is_none() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(
            app.world().get::<BakeWarp>(slab).is_none(),
            "the bake finished"
        );
        let materials = app.world().resource::<Assets<FeltMaterial>>();
        let felt = materials.get(&material).expect("the material");
        let field =
            baylee_client_core::feltwarp::WarpField::for_table(span.to_array(), pattern.to_array());
        assert_eq!(felt.params.warp, Vec4::from_array(field.rect()));
        assert_ne!(felt.warp, stand_in, "the stand-in was replaced");
        let image = app
            .world()
            .resource::<Assets<Image>>()
            .get(&felt.warp)
            .expect("the grid's texture");
        assert_eq!((image.width(), image.height()), (field.width, field.height));
    }

    /// The texture holds the grid's numbers as half floats, texel for texel.
    #[test]
    fn the_texture_holds_the_grid() {
        let field =
            baylee_client_core::feltwarp::WarpField::for_table([3.0, 2.0], [0.0, 0.0, 0.0, 0.0]);
        let image = warp_image(&field);
        let bytes = image.data.as_ref().expect("bytes");
        assert_eq!(bytes.len(), field.texels.len() * 8);
        for (i, texel) in field.texels.iter().enumerate().step_by(7) {
            for (c, value) in texel.iter().enumerate() {
                let at = (i * 4 + c) * 2;
                let read = half::f16::from_le_bytes([bytes[at], bytes[at + 1]]).to_f32();
                assert!((read - value).abs() < 1e-3, "texel {i} channel {c}");
            }
        }
    }
}

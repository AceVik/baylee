//! The light on a primary button (owner, 09.10.2026: "an animated shader or
//! similar"): a quiet glow from the top edge at rest, and while the pointer
//! is on the button a soft band of light sweeping across it.
//!
//! One small UI material per primary button (`shaders/kit_sheen.wgsl`,
//! uniforms only), laid over the button's ground and under its words. The
//! band moves on `globals.time`, never on a time written into the material;
//! the material is written only while the pointer's warmth on the button
//! changes ([`Feel`]), and at rest the shader reads no clock at all, so an
//! idle screen draws the same pixels and asks for no frame (the rest pacing,
//! `quality.rs`). Under `reduce_motion` the band holds still in the middle.
//!
//! A test app has no render world: [`install`] registers the material only
//! where an asset server exists, and [`dress`] does nothing without one, the
//! same shape as `ambience`.

use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use super::metrics::px_fixed;
use super::tokens;
use crate::ambience::Feel;

/// A button that wears the light (the kit puts it on `Weight::Primary`).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Sheen;

/// The light laid on a [`Sheen`] button: its material, which [`drive`]
/// writes.
#[derive(Component, Clone, Debug)]
pub struct SheenSurface(pub Handle<SheenMaterial>);

/// What the shader reads.
#[derive(Clone, Copy, ShaderType, Debug, PartialEq)]
pub struct SheenParams {
    /// The light's colour (linear).
    pub light: Vec4,
    /// How far the pointer is on the button, 0 to 1.
    pub hover: f32,
    /// The resting glow's strength.
    pub glow: f32,
    /// 1 moves the band, 0 holds it (`reduce_motion`).
    pub sweep: f32,
    /// Padding to the 16-byte boundary.
    pub pad: f32,
}

/// The light, as a UI material.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct SheenMaterial {
    /// Everything the shader reads.
    #[uniform(0)]
    pub params: SheenParams,
}

impl UiMaterial for SheenMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/kit_sheen.wgsl".into()
    }
}

/// The resting glow: enough to read as light on the blue face, not as a
/// second colour.
pub const GLOW: f32 = 0.06;

/// The light's colour: the primary face's rim, lifted towards white.
fn light() -> Vec4 {
    crate::ambience::lighter(tokens::PRIMARY_EDGE, 0.55)
        .to_linear()
        .to_vec4()
}

/// What the material says for a button the pointer warms by `warmth` (the
/// [`Feel`]'s: negative under a press), with motion allowed or not.
#[must_use]
pub fn params(warmth: f32, still: bool) -> SheenParams {
    SheenParams {
        light: light(),
        hover: warmth.clamp(0.0, 1.0),
        glow: GLOW,
        sweep: if still { 0.0 } else { 1.0 },
        pad: 0.0,
    }
}

/// A [`Sheen`] button not yet wearing its light.
type Undressed = (With<Sheen>, Without<SheenSurface>);

/// Lays the light on every [`Sheen`] button that has none yet.
pub(crate) fn dress(
    mut commands: Commands,
    faces: Query<(Entity, &Feel), Undressed>,
    materials: Option<ResMut<Assets<SheenMaterial>>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    for (face, feel) in &faces {
        let handle = materials.add(SheenMaterial {
            params: params(feel.warmth, still),
        });
        let surface = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px_fixed(0.0),
                    right: px_fixed(0.0),
                    top: px_fixed(0.0),
                    bottom: px_fixed(0.0),
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL - 1.0)),
                    ..default()
                },
                MaterialNode(handle.clone()),
                Pickable::IGNORE,
            ))
            .id();
        commands
            .entity(face)
            .insert(SheenSurface(handle))
            .insert_children(0, &[surface]);
    }
}

/// Writes a button's light where the pointer's warmth on it changed, or the
/// motion preference did; never at rest.
pub(crate) fn drive(
    faces: Query<(Ref<Feel>, &SheenSurface)>,
    materials: Option<ResMut<Assets<SheenMaterial>>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let switched = prefs.as_ref().is_some_and(Res::is_changed);
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    for (feel, surface) in &faces {
        if !feel.is_changed() && !switched {
            continue;
        }
        let want = params(feel.warmth, still);
        // Read before written: a write uploads the block again.
        if materials.get(&surface.0).is_some_and(|m| m.params == want) {
            continue;
        }
        if let Some(mut material) = materials.get_mut(&surface.0) {
            material.params = want;
        }
    }
}

/// Registers the material and its shader where there is a renderer, and
/// the two systems everywhere (they do nothing without the material).
pub(super) fn install(app: &mut App) {
    app.add_systems(Update, (dress, drive.after(crate::ambience::feel)));
    if !app.world().contains_resource::<AssetServer>() {
        return;
    }
    // The shader is filed by `ambience`, whose file sits beside `shaders/`
    // (`embedded_asset!` names a shader by the path of the file it is in).
    app.add_plugins(UiMaterialPlugin::<SheenMaterial>::default());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// At rest the band is off and the glow is the resting one; the pointer
    /// lights it, a press does not (it sinks); `reduce_motion` holds the band.
    #[test]
    fn the_light_follows_the_pointer_and_holds_still_when_asked() {
        let rest = params(0.0, false);
        assert!(rest.hover.abs() < f32::EPSILON && (rest.glow - GLOW).abs() < f32::EPSILON);
        assert!((params(1.0, false).hover - 1.0).abs() < f32::EPSILON);
        assert!(
            params(-1.0, false).hover.abs() < f32::EPSILON,
            "a press is no hover"
        );
        assert!(
            params(1.0, true).sweep.abs() < f32::EPSILON,
            "reduce_motion"
        );
        assert!((params(1.0, false).sweep - 1.0).abs() < f32::EPSILON);
    }

    /// The material is written while the warmth changes and not at rest.
    #[test]
    fn the_material_is_written_only_while_the_warmth_moves() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<SheenMaterial>()
            .add_systems(Update, drive);
        let handle = app
            .world_mut()
            .resource_mut::<Assets<SheenMaterial>>()
            .add(SheenMaterial {
                params: params(0.0, false),
            });
        let face = app
            .world_mut()
            .spawn((Feel::new(tokens::PRIMARY), SheenSurface(handle.clone())))
            .id();
        app.update();
        app.update();
        let written = |app: &mut App| {
            app.world_mut()
                .resource_mut::<Messages<AssetEvent<SheenMaterial>>>()
                .drain()
                .filter(|e| matches!(e, AssetEvent::Modified { .. }))
                .count()
        };
        let _ = written(&mut app);
        app.update();
        app.update();
        assert_eq!(written(&mut app), 0, "a button at rest wrote its light");
        app.world_mut()
            .get_mut::<Feel>(face)
            .expect("a feel")
            .warmth = 0.5;
        app.update();
        assert!(
            written(&mut app) > 0,
            "the pointer's warmth reached the light"
        );
        let hover = app
            .world()
            .resource::<Assets<SheenMaterial>>()
            .get(&handle)
            .map(|m| m.params.hover);
        assert_eq!(hover, Some(0.5));
    }
}

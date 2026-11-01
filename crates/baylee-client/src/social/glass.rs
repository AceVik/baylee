//! The chat's light (owner, 29.10.2026: "make the chat input and the chat
//! window more attractive, with light shader animations").
//!
//! One small UI material (`shaders/chat_glass.wesl`, uniforms only) in two
//! looks, each laid as light over a chat surface and never over the middle
//! of what is read:
//!
//! - **the window**: a rim of light just inside the panel's edge, brightest
//!   along the top, and a band that climbs the sides for 0.9 s when a chat
//!   line arrives (`list::Arrive` on a `ChatRow`);
//! - **the field**: the composer's box, whose rim lights and carries a
//!   travelling glint while it has the caret.
//!
//! A surface asks for its light with a [`Glass`] node; [`dress`] lays the
//! material on it, so the window and the composer, rebuilt as they are, know
//! nothing of materials and a test app without a renderer draws them bare.
//! Two materials serve every surface ([`Lights`]), written only when the
//! caret comes or goes, `reduce_motion` changes, or a line arrives
//! ([`light`]); the band and the glint move on `globals.time`, so a resting
//! window writes nothing (`/changed`), and with no band running and no
//! caret the shader draws the same pixels every frame.

use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use super::Field;
use super::rows::ChatRow;
use super::window::list::Arrive;

/// Which look a [`Glass`] node wears.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Glass {
    /// Over the whole window, above its parts.
    Window,
    /// Inside the composer's box.
    Field,
}

/// What the shader reads.
#[derive(Clone, Copy, ShaderType, Debug, PartialEq)]
pub struct GlassParams {
    /// The light's colour (linear).
    pub light: Vec4,
    /// The field has the caret: 0 or 1.
    pub focus: f32,
    /// `Time::elapsed_secs_wrapped` (the clock `globals.time` is written
    /// from) when the last line arrived; [`NEVER`] for none.
    pub arrived: f32,
    /// 1 moves, 0 holds still (`reduce_motion`).
    pub motion: f32,
    /// 0 the window, 1 the field.
    pub kind: f32,
}

/// The light, as a UI material.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct GlassMaterial {
    /// Everything the shader reads.
    #[uniform(0)]
    pub params: GlassParams,
}

impl UiMaterial for GlassMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/chat_glass.wesl".into()
    }
}

/// An arrival long enough ago that its band has run.
pub const NEVER: f32 = -1.0e6;

/// The two materials every chat surface shares, made once.
#[derive(Resource, Default)]
pub(crate) struct Lights {
    window: Option<Handle<GlassMaterial>>,
    field: Option<Handle<GlassMaterial>>,
}

/// The light's colour: the primary's rim lifted towards white, the light
/// the shell's primary buttons wear (`shellkit::sheen`).
fn colour() -> Vec4 {
    crate::ambience::lighter(crate::shellkit::tokens::PRIMARY_EDGE, 0.55)
        .to_linear()
        .to_vec4()
}

/// What the material says for `glass`, with the caret in the field or not,
/// the last arrival at `arrived`, and motion allowed or not.
#[must_use]
pub fn params(glass: Glass, focus: bool, arrived: f32, still: bool) -> GlassParams {
    GlassParams {
        light: colour(),
        focus: if focus { 1.0 } else { 0.0 },
        arrived,
        motion: if still { 0.0 } else { 1.0 },
        kind: match glass {
            Glass::Window => 0.0,
            Glass::Field => 1.0,
        },
    }
}

/// A node of light filling its parent, for `glass`.
pub(crate) fn layer(glass: Glass, radius: Val) -> impl Bundle {
    (
        glass,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            border_radius: BorderRadius::all(radius),
            ..default()
        },
        // Over the window's parts, which stand at 1.
        ZIndex(2),
        Pickable::IGNORE,
    )
}

/// Registers the material and its systems (which do nothing without a
/// renderer, the shape of `shellkit::sheen`).
pub(crate) fn install(app: &mut App) {
    app.init_resource::<Lights>()
        .add_systems(Update, (dress, light).chain());
    if !app.world().contains_resource::<AssetServer>() {
        return;
    }
    // Filed by `ambience`, beside `shaders/`, as the sheen's is.
    app.add_plugins(UiMaterialPlugin::<GlassMaterial>::default());
}

/// Lays the light on every [`Glass`] node that has none yet, making the two
/// materials the first time one is needed.
fn dress(
    mut commands: Commands,
    nodes: Query<(Entity, &Glass), Without<MaterialNode<GlassMaterial>>>,
    materials: Option<ResMut<Assets<GlassMaterial>>>,
    mut lights: ResMut<Lights>,
    ui: Res<super::ChatUi>,
    prefs: Option<Res<crate::prefs::Prefs>>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    if nodes.is_empty() {
        return;
    }
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let focus = ui.typing && ui.field == Field::Composer;
    for (node, glass) in &nodes {
        let slot = match glass {
            Glass::Window => &mut lights.window,
            Glass::Field => &mut lights.field,
        };
        let handle = slot
            .get_or_insert_with(|| {
                materials.add(GlassMaterial {
                    params: params(*glass, focus, NEVER, still),
                })
            })
            .clone();
        commands.entity(node).try_insert(MaterialNode(handle));
    }
}

/// Writes the two lights when the caret comes or goes, `reduce_motion`
/// changes, or a chat line arrives; never at rest. A game log line arriving
/// in the table's window moves its row but lights nothing: at a busy table
/// that would be a band every few frames.
fn light(
    time: Res<Time>,
    ui: Res<super::ChatUi>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    lights: Res<Lights>,
    arrivals: Query<(), (Added<Arrive>, With<ChatRow>)>,
    materials: Option<ResMut<Assets<GlassMaterial>>>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let focus = ui.typing && ui.field == Field::Composer;
    let arrived = (!arrivals.is_empty()).then(|| time.elapsed_secs_wrapped());
    for (glass, handle) in [
        (Glass::Window, &lights.window),
        (Glass::Field, &lights.field),
    ] {
        let Some(handle) = handle else {
            continue;
        };
        let Some(was) = materials.get(handle).map(|m| m.params) else {
            continue;
        };
        // Only the window carries an arrival; the field never moves for one.
        let since = match glass {
            Glass::Window => arrived.unwrap_or(was.arrived),
            Glass::Field => was.arrived,
        };
        let want = params(glass, focus, since, still);
        // Read before written: a write uploads the block again.
        if want != was
            && let Some(mut material) = materials.get_mut(handle)
        {
            material.params = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The field lights only with the caret, a window carries the arrival,
    /// and `reduce_motion` stills both.
    #[test]
    fn the_light_follows_the_caret_and_holds_still_when_asked() {
        let rest = params(Glass::Field, false, NEVER, false);
        assert!(rest.focus.abs() < f32::EPSILON && (rest.kind - 1.0).abs() < f32::EPSILON);
        assert!((params(Glass::Field, true, NEVER, false).focus - 1.0).abs() < f32::EPSILON);
        let window = params(Glass::Window, true, 12.5, false);
        assert!(window.kind.abs() < f32::EPSILON);
        assert!((window.arrived - 12.5).abs() < f32::EPSILON);
        assert!(params(Glass::Window, false, NEVER, true).motion.abs() < f32::EPSILON);
    }

    /// A resting chat writes nothing: the system run twice with nothing
    /// changed leaves both materials as they were (`/changed`'s promise),
    /// and an arrival is the one thing besides the caret that writes.
    #[test]
    fn a_resting_chat_writes_no_light() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<GlassMaterial>()
            .init_resource::<super::super::ChatUi>()
            .init_resource::<Lights>()
            .add_systems(Update, (dress, light).chain());
        app.world_mut().spawn(layer(Glass::Window, Val::Px(6.0)));
        app.world_mut().spawn(layer(Glass::Field, Val::Px(4.0)));
        app.update();
        app.update();
        let modified = |app: &mut App| {
            app.world_mut()
                .resource_mut::<Messages<AssetEvent<GlassMaterial>>>()
                .drain()
                .filter(|e| matches!(e, AssetEvent::Modified { .. }))
                .count()
        };
        modified(&mut app);
        app.update();
        app.update();
        assert_eq!(modified(&mut app), 0, "a resting chat wrote its light");

        // A game log line arriving lights nothing; a chat line does.
        app.world_mut().spawn(Arrive::default());
        app.update();
        assert_eq!(modified(&mut app), 0, "a log line started the band");
        app.world_mut().spawn((
            Arrive::default(),
            ChatRow {
                key: baylee_client_core::social::LineKey {
                    conv: baylee_protocol::social::Conv::Group { id: "g".into() },
                    id: "1".into(),
                },
            },
        ));
        app.update();
        assert_eq!(modified(&mut app), 1, "an arrival starts the window's band");
        app.world_mut()
            .resource_mut::<super::super::ChatUi>()
            .typing = true;
        app.update();
        assert!(modified(&mut app) >= 1, "the caret lights the field");
    }
}

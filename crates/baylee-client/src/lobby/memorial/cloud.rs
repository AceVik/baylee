//! A shared material: the approved portrait stays intact; light moves around it.
use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

#[derive(Component)]
pub(in crate::lobby) struct Cloud {
    pub front: bool,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub(crate) struct CloudMaterial {
    #[uniform(0)]
    feel: Vec4,
    #[texture(1)]
    #[sampler(2)]
    portrait: Handle<Image>,
    #[uniform(3)]
    pointer: Vec4,
}
impl UiMaterial for CloudMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/memorial.wesl".into()
    }
}
#[derive(Resource)]
pub(crate) struct CloudArt(pub Handle<CloudMaterial>);
pub(crate) struct CloudPlugin;
impl Plugin for CloudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, super::album::reading_keys);
        if !app.world().contains_resource::<AssetServer>() {
            return;
        }
        app.add_plugins(UiMaterialPlugin::<CloudMaterial>::default());
        let portrait = app
            .world()
            .resource::<AssetServer>()
            .load("scenes/baylee-cloud.png");
        let material = app
            .world_mut()
            .resource_mut::<Assets<CloudMaterial>>()
            .add(CloudMaterial {
                feel: Vec4::ZERO,
                pointer: Vec4::ZERO,
                portrait,
            });
        app.insert_resource(CloudArt(material))
            .add_systems(Update, (layout, animate));
    }
}

#[allow(clippy::too_many_arguments)] // One retained material, input and the existing motion policy.
fn animate(
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    quality: Option<Res<crate::quality::InUse>>,
    resting: Option<Res<crate::quality::Resting>>,
    focus: Res<InputFocus>,
    ring: Res<InputFocusVisible>,
    clouds: Query<(Entity, &Hovered, &bevy::ui::RelativeCursorPosition), With<Cloud>>,
    art: Res<CloudArt>,
    mut materials: ResMut<Assets<CloudMaterial>>,
    mut clock: Local<f32>,
    mut glow: Local<f32>,
) {
    let active = clouds
        .iter()
        .any(|(e, h, _)| h.0 || (ring.0 && focus.get() == Some(e)));
    let still = crate::quality::ambient_still(prefs.all().reduce_motion, quality.as_deref());
    let target = if active { 1.0 } else { 0.0 };
    *glow = if still || (*glow - target).abs() < 0.001 {
        target
    } else {
        *glow + (target - *glow) * (1.0 - (-7.5 * time.delta_secs()).exp())
    };
    if !still && !clouds.is_empty() && !resting.is_some_and(|r| r.0) {
        *clock += time.delta_secs();
    }
    let feel = Vec4::new(
        *glow,
        if still { 0.0 } else { *clock },
        if still { 0.0 } else { 1.0 },
        0.0,
    );
    let pointer = clouds
        .iter()
        .find_map(|(_, h, p)| {
            h.0.then_some(p.normalized)
                .flatten()
                .map(|v| Vec4::new(v.x + 0.5, v.y + 0.5, 1.0, 0.0))
        })
        .unwrap_or(Vec4::ZERO);
    if materials
        .get(&art.0)
        .is_some_and(|m| m.feel != feel || m.pointer != pointer)
        && let Some(mut material) = materials.get_mut(&art.0)
    {
        material.feel = feel;
        material.pointer = pointer;
    }
}

/// Use the actual panel edges, including text scaling. A resized window never
/// leaves Baylee over an input, nor lets her dedication fall below the viewport.
fn layout(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    scale: Option<Res<UiScale>>,
    frames: Query<(
        &crate::vista::Framed,
        &ComputedNode,
        &bevy::ui::UiGlobalTransform,
    )>,
    mut clouds: Query<(&Cloud, &mut Node)>,
) {
    let Ok(window) = windows.single() else { return };
    let (width, height) = crate::hud::scale::desktop_dimensions(window, scale.as_deref());
    #[allow(clippy::cast_precision_loss)] // Display dimensions are small integers.
    let pixels = window.physical_width() as f32 / width;
    for (cloud, mut node) in &mut clouds {
        let kind = if cloud.front {
            crate::vista::Vista::Front
        } else {
            crate::vista::Vista::Interior
        };
        let panel = frames
            .iter()
            .find(|(f, n, _)| f.0 == kind && n.size().y > 0.0);
        let (left, top, size) = if cloud.front {
            let margin = panel.map_or(width * 0.2, |(_, n, p)| {
                (p.translation.x - n.size().x * 0.5) / pixels
            });
            let size = (margin - 32.0).min(width * 0.25).clamp(80.0, 600.0);
            (12.0, (height * 0.37).min(height - size / 1.5 - 48.0), size)
        } else if let Some((_, n, p)) = panel {
            let bottom = (p.translation.y + n.size().y * 0.5) / pixels;
            let size = ((height - bottom - 52.0) * 1.5)
                .min(n.size().x / pixels)
                .clamp(80.0, 600.0);
            (
                p.translation.x / pixels - size * 0.5,
                height - size / 1.5 - 44.0,
                size,
            )
        } else {
            let size = (width * 0.22).min(height * 0.35).min(600.0);
            (12.0, height - size / 1.5 - 44.0, size)
        };
        let wanted = (Val::Px(left), Val::Px(top), Val::Px(size));
        if (node.left, node.top, node.width) != wanted {
            node.left = wanted.0;
            node.top = wanted.1;
            node.width = wanted.2;
        }
    }
}

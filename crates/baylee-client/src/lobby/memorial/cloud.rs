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
}
impl UiMaterial for CloudMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/memorial.wesl".into()
    }
}
#[derive(Resource)]
pub(crate) struct CloudArt {
    pub cloud: Handle<CloudMaterial>,
    pub logo: Handle<CloudMaterial>,
}
pub(crate) struct CloudPlugin;
impl Plugin for CloudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CloudGeometry>()
            .add_systems(Update, super::album::reading_keys);
        if !app.world().contains_resource::<AssetServer>() {
            return;
        }
        app.add_plugins(UiMaterialPlugin::<CloudMaterial>::default());
        let assets = app.world().resource::<AssetServer>();
        let portrait = assets.load("scenes/baylee-cloud.png");
        let logo = assets.load("brand/baylee-logo.png");
        let mut materials = app.world_mut().resource_mut::<Assets<CloudMaterial>>();
        let cloud = materials.add(CloudMaterial {
            feel: Vec4::ZERO,
            portrait,
        });
        let logo = materials.add(CloudMaterial {
            feel: Vec4::ZERO,
            portrait: logo,
        });
        app.insert_resource(CloudArt { cloud, logo })
            .add_systems(Update, animate)
            .add_systems(PostUpdate, layout.after(bevy::ui::UiSystems::Layout));
    }
}

#[allow(clippy::too_many_arguments)] // One retained material, input and the existing motion policy.
fn animate(
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    state: Res<crate::lobby::LobbyState>,
    quality: Option<Res<crate::quality::InUse>>,
    resting: Option<Res<crate::quality::Resting>>,
    focus: Res<InputFocus>,
    ring: Res<InputFocusVisible>,
    mut clouds: Query<(Entity, &Hovered, &Cloud, Option<&mut UiTransform>)>,
    art: Res<CloudArt>,
    mut materials: ResMut<Assets<CloudMaterial>>,
    mut glow: Local<f32>,
) {
    let active = !state.memorial.open
        && clouds
            .iter()
            .any(|(e, h, _, _)| h.0 || (ring.0 && focus.get() == Some(e)));
    let still = crate::quality::ambient_still(prefs.all().reduce_motion, quality.as_deref());
    let target = if active { 1.0 } else { 0.0 };
    *glow = if still || (*glow - target).abs() < 0.001 {
        target
    } else {
        *glow + (target - *glow) * (1.0 - (-7.5 * time.delta_secs()).exp())
    };
    for (_, _, cloud, transform) in &mut clouds {
        if cloud.front
            && let Some(mut transform) = transform
        {
            transform.set_if_neq(UiTransform {
                translation: bevy::ui::Val2::px(0.0, -2.5 * *glow),
                scale: Vec2::splat(1.0 + 0.04 * *glow),
                ..default()
            });
        }
    }
    let moving = !still && !resting.is_some_and(|r| r.0);
    let motion = if moving { 1.0 } else { 0.0 };
    for (handle, is_logo) in [(&art.cloud, 0.0), (&art.logo, 1.0)] {
        let feel = Vec4::new(*glow, is_logo, motion, 0.0);
        if materials.get(handle).is_some_and(|m| m.feel != feel)
            && let Some(mut material) = materials.get_mut(handle)
        {
            material.feel = feel;
        }
    }
}

/// Keep the interior cloud in the bottom-left footer strip without letting her
/// dedication fall below the viewport. The front-door logo is sized by its flex
/// column (`front/mod.rs`).
fn layout(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    scale: Option<Res<UiScale>>,
    mut clouds: Query<(&Cloud, &mut Node, &mut Visibility)>,
    mut geometry: ResMut<CloudGeometry>,
) {
    let Ok(window) = windows.single() else { return };
    let (width, height) = crate::hud::scale::desktop_dimensions(window, scale.as_deref());
    for (cloud, mut node, mut visible) in &mut clouds {
        if cloud.front {
            continue;
        }
        let size = ((super::CLOUD_FOOTER - 44.0) * 1.5).min(width - 24.0);
        let (left, top) = (12.0, height - size / 1.5 - 44.0);
        let wanted = (Val::Px(left), Val::Px(top), Val::Px(size));
        if geometry.0 != Some(wanted) {
            geometry.0 = Some(wanted);
        }
        if (node.left, node.top, node.width) == wanted {
            visible.set_if_neq(Visibility::Inherited);
        }
        if (node.left, node.top, node.width) != wanted {
            node.left = wanted.0;
            node.top = wanted.1;
            node.width = wanted.2;
        }
    }
}

/// Rebuilds start at the last measured position, avoiding a one-frame jump
/// through percentage-based fallback geometry whenever the focus changes.
#[derive(Resource, Default)]
struct CloudGeometry(Option<(Val, Val, Val)>);
#[allow(dead_code)]
pub(super) fn restore_layout(commands: &mut Commands, entity: Entity) {
    commands.queue(move |world: &mut World| {
        let position = world.get_resource::<CloudGeometry>().and_then(|g| g.0);
        if let Some((left, top, width)) = position
            && let Ok(mut cloud) = world.get_entity_mut(entity)
        {
            if let Some(mut node) = cloud.get_mut::<Node>() {
                node.left = left;
                node.top = top;
                node.width = width;
            }
            cloud.insert(Visibility::Inherited);
        }
    });
}

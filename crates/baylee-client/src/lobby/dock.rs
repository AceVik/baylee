//! The hand HUD's actual leather, tooling and mineral inlays, reused by the lobby.
use crate::frontal::{Cloth, FrontalMaterial, Hanging};
use bevy::prelude::*;

/// The leather's corner radius: a lobby panel's (`front::CARD_RADIUS`, the
/// same 14 as `ui::surface`), less the one-pixel border the ground sits
/// inside.
pub(super) const GROUND_RADIUS: f32 = super::front::CARD_RADIUS - 1.0;

/// How dense the leather is behind a lobby panel: even, and not quite
/// opaque. The front door fades a panel by lowering it (`front::fade_front`).
pub(super) const GROUND_DENSITY: f32 = 0.98;

/// How far above the tooled line a lobby panel's five inlays sit, in pixels:
/// on the hand they sit on it, and on a panel standing on the page they
/// would be read as studs on its border.
pub(super) const INLAY_LIFT: f32 = 10.0;

/// A bounded material slot: each differently sized panel needs its own uniforms.
#[derive(Component, Clone, Copy)]
pub(crate) struct Dock(pub u8);

#[derive(Resource, Default)]
pub(super) struct Surfaces(std::collections::BTreeMap<u8, Handle<FrontalMaterial>>);

pub(super) fn materialize(
    mut commands: Commands,
    panels: Query<(Entity, &Dock), Added<Dock>>,
    mut cache: ResMut<Surfaces>,
    mut cloth: Option<ResMut<Cloth>>,
    materials: Option<ResMut<Assets<FrontalMaterial>>>,
) {
    let (Some(cloth), Some(mut materials)) = (cloth.as_mut(), materials) else {
        return;
    };
    for (parent, slot) in &panels {
        let handle = if let Some(handle) = cache.0.get(&slot.0) {
            handle.clone()
        } else {
            let Some(source) = cloth.skirt(Some(&mut materials)) else {
                continue;
            };
            let Some(mut material) = materials.get(&source).cloned() else {
                continue;
            };
            // Keep the hand's full tooling and five inlays, with even opacity
            // behind a reading surface rather than the hand's bottom fade.
            material.params.ramp = Vec4::new(36.0, GROUND_DENSITY, GROUND_DENSITY, 0.0);
            material.params.surface = Vec4::new(0.0, 0.0, INLAY_LIFT, 0.0);
            // A lobby panel stands on the page, so all four corners are cut,
            // one pixel inside the panel's own radius and border. The hand's
            // cloth runs off the window and cuts only its top two.
            material.params.corner = GROUND_RADIUS;
            material.params.foot_corner = GROUND_RADIUS;
            let night = Color::srgb(0.026, 0.052, 0.10).to_linear();
            material.params.dye.x = night.red;
            material.params.dye.y = night.green;
            material.params.dye.z = night.blue;
            let handle = materials.add(material);
            cache.0.insert(slot.0, handle.clone());
            handle
        };
        let ground = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    top: px(0),
                    right: px(0),
                    bottom: px(0),
                    ..default()
                },
                MaterialNode(handle),
                Hanging,
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(parent).insert_children(0, &[ground]);
    }
}

//! The hand HUD's actual leather, tooling and mineral inlays, reused by the lobby.
use crate::frontal::{Cloth, FrontalMaterial, Hanging};
use bevy::prelude::*;

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
            material.params.ramp = Vec4::new(36.0, 0.98, 0.98, 0.0);
            material.params.surface = Vec4::ZERO;
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

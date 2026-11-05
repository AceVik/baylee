//! Shader-animated achievement & favourite frame for deck tiles in the Lobby.
//!
//! Each deck earns a rank tier (`0..=11`) from its account-bound play and win
//! statistics (`baylee_client_core::prefs::DeckPlayStats::rank_tier`). Higher
//! tiers render progressively more ornate metallic, crystalline, and aurora
//! perimeter frames via `shaders/deck_frame.wesl`, while favourite decks
//! (`★`) receive a warm golden rim.

use bevy::picking::hover::PickingInteraction;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use bevy::ui::Val::Percent;

/// GPU uniform parameters for `deck_frame.wesl`.
#[derive(ShaderType, Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct DeckFrameParams {
    /// Earned achievement frame tier (`0..=11`).
    pub tier: u32,
    /// `1` when the deck is marked as a favourite (`★`), `0` otherwise.
    pub favourite: u32,
    /// `1.0` normally, `0.0` when `Preferences::reduce_motion` is active.
    pub motion: f32,
    /// Smoothed hover intensity (`0.0..=1.0`).
    pub hover: f32,
}

/// UI material for the shader-animated deck achievement frame.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug, Default)]
pub struct DeckFrameUiMaterial {
    /// Shader uniform parameters.
    #[uniform(0)]
    pub params: DeckFrameParams,
}

impl UiMaterial for DeckFrameUiMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/deck_frame.wesl".into()
    }
}

/// Links a frame overlay node to the interactive tile wrapper entity so its
/// shader hover intensity tracks pointer hover/press smoothly.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct DeckFrameOverlay {
    pub(crate) target: Entity,
    pub(crate) tier: u8,
    pub(crate) favourite: bool,
    pub(crate) hover: f32,
}

/// Attaches a shader-animated frame overlay to `tile_face`, tracking pointer
/// hover on `hit_target`. Safe in headless tests where `Assets<DeckFrameUiMaterial>`
/// is absent.
pub(crate) fn attach(
    commands: &mut Commands,
    tile_face: Entity,
    hit_target: Entity,
    tier: u8,
    favourite: bool,
) {
    let overlay = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Percent(100.0),
                height: Percent(100.0),
                border_radius: BorderRadius::all(Val::Px(10.0)),
                ..default()
            },
            DeckFrameOverlay {
                target: hit_target,
                tier,
                favourite,
                hover: 0.0,
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(tile_face).add_child(overlay);
    commands.queue(move |world: &mut World| {
        let motion = world
            .get_resource::<crate::prefs::Prefs>()
            .map_or(1.0, |p| if p.all().reduce_motion { 0.0 } else { 1.0 });
        let Some(mut store) = world.get_resource_mut::<Assets<DeckFrameUiMaterial>>() else {
            return;
        };
        let handle = store.add(DeckFrameUiMaterial {
            params: DeckFrameParams {
                tier: u32::from(tier),
                favourite: u32::from(favourite),
                motion,
                hover: 0.0,
            },
        });
        if let Ok(mut entity) = world.get_entity_mut(overlay) {
            entity.insert(MaterialNode(handle));
        }
    });
}

/// Smoothly updates `hover` and `motion` on active deck frame materials.
pub(crate) fn animate_deck_frames(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    store: Option<ResMut<Assets<DeckFrameUiMaterial>>>,
    interactions: Query<&PickingInteraction>,
    mut overlays: Query<(&mut DeckFrameOverlay, &MaterialNode<DeckFrameUiMaterial>)>,
) {
    let Some(mut store) = store else {
        return;
    };
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let motion = if still { 0.0 } else { 1.0 };
    let step = if still {
        1.0
    } else {
        1.0 - (-18.0 * time.delta_secs()).exp()
    };

    for (mut overlay, mat_node) in &mut overlays {
        let want = match interactions.get(overlay.target).ok() {
            Some(PickingInteraction::Pressed) => 1.0,
            Some(PickingInteraction::Hovered) => 0.85,
            _ => 0.0,
        };
        let next = if (overlay.hover - want).abs() < 0.004 {
            want
        } else {
            overlay.hover + (want - overlay.hover) * step
        };
        if (overlay.hover - next).abs() > f32::EPSILON {
            overlay.hover = next;
        }
        let new_params = DeckFrameParams {
            tier: u32::from(overlay.tier),
            favourite: u32::from(overlay.favourite),
            motion,
            hover: next,
        };
        let needs_update = store
            .get(&mat_node.0)
            .is_some_and(|mat| mat.params != new_params);
        if needs_update && let Some(mut mat) = store.get_mut(&mat_node.0) {
            mat.params = new_params;
        }
    }
}

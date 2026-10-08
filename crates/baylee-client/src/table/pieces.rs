//! The tearing table's pieces (the owner, 07.10.2026: *"each seat owns its
//! own segment of the table … the targeted player's segment rotates over as
//! one body and docks"*, and his and the coordinator's refinements the same
//! day).
//!
//! At rest the table is the one slab it always was — no seam can show,
//! because there is none. While a tear runs (`Duel::tear`,
//! `layout::transition`) the slab stands down and the tear's own parts
//! stand in for it: three pieces cut along the jagged line
//! ([`geometry::piece_mesh`]: mine, the far piece leaving and the far piece
//! arriving), each carried by `glide` to its piece's pose; the void under
//! the table they sink into and rise out of ([`geometry::void_mesh`]); and
//! the dial, lifted whole off the tear ([`geometry::dial_mesh`]). Every one
//! is drawn by `felt.wgsl` with a copy of the slab's material, `rift.w`
//! saying which it is. Over, they stand exactly where the slab does and go,
//! and the slab comes back: one frame, nothing moved.

pub mod geometry;

#[allow(clippy::wildcard_imports)] // the table's shared vocabulary
use super::*;
use baylee_client_core::layout::transition::{
    Piece, Pose, VOID, dial_lift, seam, shake, spill, weld,
};
pub use geometry::{DIAL_R, DRIP_DEPTH, Outline, dial_mesh, piece_mesh, void_mesh};

/// A piece of a tearing table.
#[derive(Component, Clone, Copy, Debug)]
pub struct TablePiece(pub Piece);

/// The void under a tearing table.
#[derive(Component, Clone, Copy, Debug)]
pub struct TearVoid;

/// The dial, lifted off a tearing table.
#[derive(Component, Clone, Copy, Debug)]
pub struct FloatingDial;

/// What `felt.wgsl` draws, by `rift.w`: a piece of the table.
pub const RIFT_PIECE: f32 = 1.0;
/// … the void under it.
pub const RIFT_VOID: f32 = 2.0;
/// … the floating dial.
pub const RIFT_DIAL: f32 = 3.0;

/// How far in front of the tear's line the void reaches toward me: past the
/// line's reach and the opening, so the gap is floored from every side.
const VOID_FROM: f32 = -(baylee_client_core::layout::transition::LINE_REACH
    + baylee_client_core::layout::transition::OPENING * 0.5
    + 1.5);

/// How far the floating dial tilts with the shake, radians per unit of it.
const DIAL_WOBBLE: f32 = 0.3;

/// The world transform of a piece in `pose`, the slab standing at `slab`:
/// turned about its pivot, then slid along the table and lifted.
#[must_use]
pub fn piece_transform(pose: Pose, slab: Transform) -> Transform {
    let pivot = slab.translation + Vec3::new(pose.pivot.x, 0.0, -pose.pivot.y);
    Transform::from_matrix(
        Mat4::from_translation(Vec3::new(0.0, pose.lift, -pose.shift))
            * Mat4::from_translation(pivot)
            * Mat4::from_rotation_y(pose.turn)
            * Mat4::from_translation(-pivot)
            * slab.to_matrix(),
    )
}

/// Where the floating dial stands at `t`: lifted over the slab's middle and
/// tilting a little with the shake.
#[must_use]
pub fn dial_transform(t: f32, slab: Transform) -> Transform {
    let wobble = shake(t) * DIAL_WOBBLE;
    let centre = slab.translation;
    Transform::from_matrix(
        Mat4::from_translation(Vec3::new(0.0, dial_lift(t) + 0.003, 0.0))
            * Mat4::from_translation(centre)
            * Mat4::from_quat(Quat::from_euler(EulerRot::XYZ, wobble, 0.0, wobble * 0.6))
            * Mat4::from_translation(-centre)
            * slab.to_matrix(),
    )
}

/// The rift uniform a piece is drawn with: how far its veins spill, its
/// line's seed, the seam's heat (mine carries the seam) and what it is.
fn rift_of(piece: Piece, seed: f32, t: f32) -> Vec4 {
    let hot = if piece == Piece::Near {
        seam(weld(t))
    } else {
        0.0
    };
    Vec4::new(spill(t), seed, hot, RIFT_PIECE)
}

/// The slab, as the tear sees it.
type SlabParts<'a> = (
    &'a MeshMaterial3d<FeltMaterial>,
    &'a Transform,
    &'a mut Visibility,
);

/// The tear's parts, as this system moves them.
type PartParts<'a> = (
    Entity,
    Option<&'a TablePiece>,
    Option<&'a FloatingDial>,
    &'a MeshMaterial3d<FeltMaterial>,
    Option<&'a mut Motion>,
    &'a mut Visibility,
);

/// The filter that finds the tear's parts.
type IsPart = (
    Without<Slab>,
    Or<(With<TablePiece>, With<TearVoid>, With<FloatingDial>)>,
);

/// Stands the tear's parts in for the slab while a tear runs, poses them,
/// spills and welds them, and gives the slab back when it is over. Writes
/// nothing on a table that is not tearing.
#[allow(clippy::needless_pass_by_value)] // a Bevy system
pub fn tear_the_slab(
    mut commands: Commands,
    duel: Res<Duel>,
    mut slabs: Query<SlabParts, (With<Slab>, Without<TablePiece>)>,
    mut parts: Query<PartParts, IsPart>,
    mut materials: ResMut<Assets<FeltMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Ok((slab_material, slab_at, mut slab_seen)) = slabs.single_mut() else {
        return;
    };
    let Some(tear) = duel.tear.as_ref() else {
        if !parts.is_empty() {
            for (entity, _, _, material, _, _) in &parts {
                materials.remove(&material.0);
                commands.entity(entity).despawn();
            }
            slab_seen.set_if_neq(Visibility::Inherited);
        }
        return;
    };
    if parts.is_empty() {
        let Some(base) = materials.get(&slab_material.0).cloned() else {
            return;
        };
        stand_in(
            &mut commands,
            tear,
            *slab_at,
            &base,
            &mut materials,
            &mut meshes,
        );
        slab_seen.set_if_neq(Visibility::Hidden);
        return;
    }
    for (_, piece, dial, material, motion, mut seen) in &mut parts {
        let (target, shown, want) = match (piece, dial) {
            (Some(piece), _) => {
                let pose = tear.plan.pose(piece.0, tear.t);
                (
                    Some(piece_transform(pose, *slab_at)),
                    pose.shown,
                    rift_of(piece.0, tear.seed, tear.t),
                )
            }
            (None, Some(_)) => (
                Some(dial_transform(tear.t, *slab_at)),
                true,
                Vec4::new(0.0, tear.seed, 0.0, RIFT_DIAL),
            ),
            (None, None) => (
                None,
                true,
                Vec4::new(spill(tear.t), tear.seed, 0.0, RIFT_VOID),
            ),
        };
        if let (Some(target), Some(mut motion)) = (target, motion)
            && motion.target != target
        {
            motion.target = target;
        }
        seen.set_if_neq(if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if materials
            .get(&material.0)
            .is_some_and(|m| (m.params.rift - want).abs().max_element() > 1e-4)
            && let Some(mut m) = materials.get_mut(&material.0)
        {
            m.params.rift = want;
        }
    }
}

/// Spawns the tear's parts where the slab stands: the three pieces, the
/// void and the floating dial, each with its own copy of the slab's
/// material.
fn stand_in(
    commands: &mut Commands,
    tear: &crate::Tear,
    slab_at: Transform,
    base: &FeltMaterial,
    materials: &mut Assets<FeltMaterial>,
    meshes: &mut Assets<Mesh>,
) {
    let outline = Outline {
        span: base.params.span,
        corner: base.params.corner,
    };
    let mut material_for = |rift: Vec4| {
        let mut material = base.clone();
        material.params.rift = rift;
        materials.add(material)
    };
    let near_mesh = meshes.add(piece_mesh(outline, TABLE_THICKNESS, tear.seed, true));
    let far_mesh = meshes.add(piece_mesh(outline, TABLE_THICKNESS, tear.seed, false));
    for piece in [Piece::Near, Piece::Leaving, Piece::Arriving] {
        let pose = tear.plan.pose(piece, tear.t);
        // Mine and the leaving piece start where the slab stands and
        // glide apart; the arriving one waits, hidden, where it rises
        // from.
        let at = if piece == Piece::Arriving {
            piece_transform(pose, slab_at)
        } else {
            slab_at
        };
        let shape = if piece == Piece::Near {
            near_mesh.clone()
        } else {
            far_mesh.clone()
        };
        commands.spawn((
            DuelStage,
            TablePiece(piece),
            Mesh3d(shape),
            MeshMaterial3d(material_for(rift_of(piece, tear.seed, tear.t))),
            at,
            Motion {
                target: piece_transform(pose, slab_at),
            },
            if pose.shown {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
            Pickable::IGNORE,
        ));
    }
    commands.spawn((
        DuelStage,
        TearVoid,
        Mesh3d(meshes.add(void_mesh(outline, VOID_FROM))),
        MeshMaterial3d(material_for(Vec4::new(
            spill(tear.t),
            tear.seed,
            0.0,
            RIFT_VOID,
        ))),
        Transform::from_translation(Vec3::new(0.0, -VOID, 0.0)) * slab_at,
        Visibility::Inherited,
        Pickable::IGNORE,
    ));
    commands.spawn((
        DuelStage,
        FloatingDial,
        Mesh3d(meshes.add(dial_mesh(outline, base.params.pulse.z.max(1.0)))),
        MeshMaterial3d(material_for(Vec4::new(0.0, tear.seed, 0.0, RIFT_DIAL))),
        slab_at,
        Motion {
            target: dial_transform(tear.t, slab_at),
        },
        Visibility::Inherited,
        Pickable::IGNORE,
    ));
}

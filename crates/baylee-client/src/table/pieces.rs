//! The tearing table's pieces (the owner, 07.10.2026: *"each seat owns its
//! own segment of the table … the targeted player's segment rotates over as
//! one body and docks"*).
//!
//! At rest the table is the one slab it always was — no seam can show,
//! because there is none. While a tear runs (`Duel::tear`,
//! `layout::transition`) the slab stands down and three pieces stand in for
//! it: copies of its mesh and its material, each drawing only its own side
//! of the tear's jagged line (`felt.wgsl`, `rift.w` the side), each carried
//! by `glide` to its piece's pose — mine toward me, the leaving far piece
//! turning out to its seat's bearing, the arriving one turning in from its
//! own. Over, the pieces stand exactly where the slab does and go, and the
//! slab comes back: one frame, nothing moved.

#[allow(clippy::wildcard_imports)] // the table's shared vocabulary
use super::*;
use baylee_client_core::layout::transition::{Piece, Pose, seam, weld};

/// A piece of a tearing table.
#[derive(Component, Clone, Copy, Debug)]
pub struct TablePiece(pub Piece);

/// Which side of the tear's line a piece draws (`felt.wgsl`'s `rift.w`):
/// mine the near side, both far pieces the far side.
fn side(piece: Piece) -> f32 {
    match piece {
        Piece::Near => -1.0,
        Piece::Leaving | Piece::Arriving => 1.0,
    }
}

/// The world transform of a piece in `pose`, the slab standing at `slab`:
/// slid along the table, lifted, then turned about the table's middle.
#[must_use]
pub fn piece_transform(pose: Pose, slab: Transform) -> Transform {
    Transform::from_matrix(
        Mat4::from_rotation_y(pose.turn)
            * Mat4::from_translation(Vec3::new(0.0, pose.lift, -pose.shift))
            * slab.to_matrix(),
    )
}

/// The rift uniform a piece is drawn with: its line's seed, the seam's
/// brightness (the near and the arriving piece weld; the leaving one has
/// gone by then) and its side.
fn rift_of(piece: Piece, seed: f32, t: f32) -> Vec4 {
    let hot = if piece == Piece::Leaving {
        0.0
    } else {
        seam(weld(t))
    };
    Vec4::new(0.0, seed, hot, side(piece))
}

/// The slab, as the pieces see it.
type SlabParts<'a> = (
    &'a Mesh3d,
    &'a MeshMaterial3d<FeltMaterial>,
    &'a Transform,
    &'a mut Visibility,
);

/// The pieces, as this system moves them.
type PieceParts<'a> = (
    Entity,
    &'a TablePiece,
    &'a MeshMaterial3d<FeltMaterial>,
    &'a mut Motion,
    &'a mut Visibility,
);

/// Stands the pieces in for the slab while a tear runs, poses them, welds
/// their seam, and gives the slab back when it is over. Writes nothing on
/// a table that is not tearing.
#[allow(clippy::needless_pass_by_value)] // a Bevy system
pub fn tear_the_slab(
    mut commands: Commands,
    duel: Res<Duel>,
    mut slabs: Query<SlabParts, (With<Slab>, Without<TablePiece>)>,
    mut pieces: Query<PieceParts, Without<Slab>>,
    mut materials: ResMut<Assets<FeltMaterial>>,
) {
    let Ok((mesh, slab_material, slab_at, mut slab_seen)) = slabs.single_mut() else {
        return;
    };
    let Some(tear) = duel.tear.as_ref() else {
        if !pieces.is_empty() {
            for (entity, _, material, _, _) in &pieces {
                materials.remove(&material.0);
                commands.entity(entity).despawn();
            }
            slab_seen.set_if_neq(Visibility::Inherited);
        }
        return;
    };
    if pieces.is_empty() {
        let Some(base) = materials.get(&slab_material.0).cloned() else {
            return;
        };
        for piece in [Piece::Near, Piece::Leaving, Piece::Arriving] {
            let pose = tear.plan.pose(piece, tear.t);
            let mut material = base.clone();
            material.params.rift = rift_of(piece, tear.seed, tear.t);
            // The near and the leaving piece start where the slab stands and
            // glide apart; the arriving one waits, hidden, where it comes from.
            let at = if piece == Piece::Arriving {
                piece_transform(pose, *slab_at)
            } else {
                *slab_at
            };
            commands.spawn((
                DuelStage,
                TablePiece(piece),
                Mesh3d(mesh.0.clone()),
                MeshMaterial3d(materials.add(material)),
                at,
                Motion {
                    target: piece_transform(pose, *slab_at),
                },
                if pose.shown {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                },
                Pickable::IGNORE,
            ));
        }
        slab_seen.set_if_neq(Visibility::Hidden);
        return;
    }
    for (_, piece, material, mut motion, mut seen) in &mut pieces {
        let pose = tear.plan.pose(piece.0, tear.t);
        let target = piece_transform(pose, *slab_at);
        if motion.target != target {
            motion.target = target;
        }
        seen.set_if_neq(if pose.shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        let want = rift_of(piece.0, tear.seed, tear.t);
        if materials
            .get(&material.0)
            .is_some_and(|m| (m.params.rift - want).abs().max_element() > 1e-4)
            && let Some(mut m) = materials.get_mut(&material.0)
        {
            m.params.rift = want;
        }
    }
}

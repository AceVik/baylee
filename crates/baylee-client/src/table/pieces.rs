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
use baylee_client_core::layout::transition::{Piece, Pose, seam, spill, tear_line, weld};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};

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

/// The rift uniform a piece is drawn with: how far its veins spill, its
/// line's seed, the seam's brightness (the near and the arriving piece weld;
/// the leaving one has gone by then) and its side.
fn rift_of(piece: Piece, seed: f32, t: f32) -> Vec4 {
    let hot = if piece == Piece::Leaving {
        0.0
    } else {
        seam(weld(t))
    };
    Vec4::new(spill(t), seed, hot, side(piece))
}

/// How far below the slab the spilled veins may hang, in slab thicknesses
/// (the cut face's mesh reaches this far; the shader draws only the drips).
pub const DRIP_DEPTH: f32 = 1.6;

/// The step the cut face is meshed at along the line, table units: finer
/// than the tear's teeth (about a unit apart), coarse enough to be a few
/// hundred quads on the widest table.
const CUT_STEP: f32 = 0.06;

/// A piece's cut face (the owner's of 07.10.2026: *at the torn edges you see
/// the slab's cross-section … and the veins spill out*): a strip down the
/// tear's jagged line from the slab's top to its bottom and on below it for
/// the drips, in the slab mesh's own frame (`x`, `y` table, `z` up), facing
/// the gap — `+y` on my piece, `-y` on a far one. Its uv says what it is to
/// `felt.wgsl`: `u` from 2 to 3 across the table (a slab's own uv stays in
/// 0 to 1), `v` 0 at the top, 1 at the slab's bottom, past 1 the drips.
#[must_use]
pub fn cut_face(span: Vec2, thickness: f32, seed: f32, facing: f32) -> Mesh {
    let half = span.x * 0.5;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let steps = ((span.x / CUT_STEP).ceil() as usize).max(2);
    let mut positions = Vec::with_capacity((steps + 1) * 3);
    let mut normals = Vec::with_capacity((steps + 1) * 3);
    let mut uvs = Vec::with_capacity((steps + 1) * 3);
    for i in 0..=steps {
        #[allow(clippy::cast_precision_loss)]
        let u = i as f32 / steps as f32;
        let x = -half + span.x * u;
        let y = tear_line(x, seed);
        for (z, v) in [
            (0.0, 0.0),
            (-thickness, 1.0),
            (-thickness * (1.0 + DRIP_DEPTH), 1.0 + DRIP_DEPTH),
        ] {
            positions.push([x, y, z]);
            normals.push([0.0, facing, 0.0]);
            uvs.push([2.0 + u, v]);
        }
    }
    let mut indices = Vec::with_capacity(steps * 12);
    for i in 0..steps {
        #[allow(clippy::cast_possible_truncation)]
        let at = |col: usize, row: usize| (col * 3 + row) as u32;
        for row in 0..2 {
            let (a, b, c, d) = (
                at(i, row),
                at(i + 1, row),
                at(i, row + 1),
                at(i + 1, row + 1),
            );
            // Wound counter-clockwise as seen from `facing` (bevy's front
            // face): the cross-section is seen from the gap and from nowhere
            // else. The drips hang free in the gap and are seen from both
            // sides, so their strip is wound both ways.
            let toward = [a, b, c, b, d, c];
            let away = [a, c, b, b, c, d];
            if facing > 0.0 {
                indices.extend_from_slice(&toward);
            } else {
                indices.extend_from_slice(&away);
            }
            if row == 1 {
                indices.extend_from_slice(if facing > 0.0 { &away } else { &toward });
            }
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_indices(Indices::U32(indices))
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
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
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Ok((mesh, slab_material, slab_at, mut slab_seen)) = slabs.single_mut() else {
        return;
    };
    let Some(tear) = duel.tear.as_ref() else {
        if !pieces.is_empty() {
            for (entity, _, material, _, _) in &pieces {
                materials.remove(&material.0);
                // The cut face goes with its piece (a child).
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
            let span = base.params.span;
            let handle = materials.add(material);
            let cut = meshes.add(cut_face(span, TABLE_THICKNESS, tear.seed, -side(piece)));
            commands
                .spawn((
                    DuelStage,
                    TablePiece(piece),
                    Mesh3d(mesh.0.clone()),
                    MeshMaterial3d(handle.clone()),
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
                ))
                .with_child((
                    // Its cut face, in the piece's own frame: carried with
                    // it, drawn with its material.
                    Mesh3d(cut),
                    MeshMaterial3d(handle),
                    Transform::IDENTITY,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The cut face's cross-section is wound to face the gap it is seen
    /// from (counter-clockwise, bevy's front face, as seen from `facing`),
    /// and the drips under it both ways. A face wound the other way is a
    /// cross-section culled from the only side anybody looks at it from —
    /// which is what the first one was, measured live.
    #[test]
    fn the_cut_face_faces_the_gap() {
        for facing in [1.0_f32, -1.0] {
            let mesh = cut_face(Vec2::new(12.0, 6.0), 0.9, 3.7, facing);
            let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!("positions");
            };
            let Some(Indices::U32(index)) = mesh.indices() else {
                panic!("indices");
            };
            let (mut toward, mut away) = (0, 0);
            for tri in index.chunks(3) {
                let [a, b, c] = [tri[0], tri[1], tri[2]].map(|i| Vec3::from(p[i as usize]));
                let normal = (b - a).cross(c - a);
                // The cross-section rows only: their vertices are at or
                // above the slab's bottom.
                if a.z.min(b.z).min(c.z) >= -0.9 - 1e-4 {
                    if normal.y * facing > 0.0 {
                        toward += 1;
                    } else {
                        away += 1;
                    }
                }
            }
            assert!(toward > 0, "facing {facing}: none toward the gap");
            assert_eq!(
                away, 0,
                "facing {facing}: {away} cross-section triangles face away"
            );
        }
    }
}

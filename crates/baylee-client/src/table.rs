//! The 2.5D table: a real 3D stage, with flat cards lying on it.
//!
//! # Why 2.5D and not 2D or 3D
//!
//! Cards are flat objects on a flat surface, so modelling them as textured
//! quads costs nothing and buys three things a 2D renderer cannot do cheaply:
//! tapping is a rotation rather than a swapped sprite, the near seat can be
//! given more screen area than the far ones by perspective alone, and focusing
//! an opponent is a camera move instead of a re-layout.
//!
//! Everything a player *reads* — prompt, hand, stack, life totals — stays in
//! the 2D overlay, where text is crisp and layout is predictable. The rule is
//! simply: if it is a card on a battlefield it is in the world; if it is
//! information about the game it is in the overlay.
//!
//! # Redrawing
//!
//! [`sync_scene`] diffs the board model against the entities that exist and
//! touches only what changed. A board that did not change costs one hash lookup
//! per card and no allocation, which is what keeps a 300-permanent commander
//! table at frame rate on a phone.

use crate::Duel;
use crate::textures::CardTextures;
use baylee_client_core::board::CardGroup;
use baylee_client_core::images::ImageKey;
use baylee_client_core::layout::{CARD_HEIGHT, CARD_WIDTH, SeatSlot, pack_lane};
use baylee_core::ids::ObjectId;
use bevy::light::NotShadowCaster;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;

/// Height of the table surface; cards float a hair above it so they never
/// z-fight with the felt.
const TABLE_Y: f32 = 0.0;
/// Vertical gap between the felt and a card.
const CARD_LIFT: f32 = 0.01;
/// Extra lift per card in a counted stack, so a stack reads as a stack.
const STACK_LIFT: f32 = 0.006;
/// How many cards of a group are drawn behind the representative.
const MAX_STACK_DEPTH: usize = 4;

/// Marks everything spawned for the duel, so closing it is one despawn.
#[derive(Component)]
pub struct DuelStage;

/// The table camera.
#[derive(Component)]
pub struct TableCamera;

/// A drawn card, and the group it stands for.
#[derive(Component)]
pub struct CardVisual {
    /// The object the card represents and that input reports.
    pub object: ObjectId,
    /// How many permanents it stands for.
    pub count: usize,
}

/// Entities currently drawn, keyed by the object they represent.
#[derive(Resource, Default)]
pub struct SceneIndex {
    cards: HashMap<ObjectId, Entity>,
    /// One material per texture, shared by every card using it — a board of
    /// forty Islands is one material, not forty.
    materials: HashMap<ImageKey, Handle<StandardMaterial>>,
    quad: Option<Handle<Mesh>>,
    blank: Option<Handle<StandardMaterial>>,
}

/// Builds the stage: camera, light, and felt.
pub fn spawn_stage(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut index: ResMut<SceneIndex>,
) {
    index.quad = Some(meshes.add(Rectangle::new(CARD_WIDTH, CARD_HEIGHT)));
    index.blank = Some(materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.14, 0.18),
        unlit: true,
        ..default()
    }));

    commands.spawn((
        DuelStage,
        TableCamera,
        Camera3d::default(),
        // Looking down the table from behind the local seat. The angle is
        // shallow enough that opponent pods stay readable but steep enough
        // that the near seat's own board is not foreshortened away.
        Transform::from_xyz(0.0, 15.0, 13.0).looking_at(Vec3::new(0.0, 0.0, -1.0), Vec3::Y),
        Projection::Perspective(PerspectiveProjection {
            fov: 0.7,
            ..default()
        }),
    ));

    commands.spawn((
        DuelStage,
        DirectionalLight {
            illuminance: 6_000.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(0.0, 20.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // The felt. Unlit so card art is never tinted by scene lighting: a player
    // must be able to tell a card's colour identity at a glance.
    commands.spawn((
        DuelStage,
        Mesh3d(meshes.add(Rectangle::new(60.0, 44.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.07, 0.09, 0.11),
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(0.0, TABLE_Y, 0.0)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        NotShadowCaster,
    ));
}

/// Tears the stage down.
pub fn despawn_stage(
    mut commands: Commands,
    stage: Query<Entity, With<DuelStage>>,
    cards: Query<Entity, With<CardVisual>>,
    mut index: ResMut<SceneIndex>,
) {
    for entity in stage.iter().chain(cards.iter()) {
        commands.entity(entity).despawn();
    }
    index.cards.clear();
}

/// Places table-space coordinates into the world.
///
/// Table space has `+y` running away from the local seat; the world has the
/// camera on `+z`, so the two are mirrored on that axis.
fn to_world(table: Vec2, height: f32) -> Vec3 {
    Vec3::new(table.x, height, -table.y)
}

/// The transform of one card.
fn card_transform(slot: &SeatSlot, position: Vec2, tapped: bool, lift: f32) -> Transform {
    // Lay the quad flat, then turn it so it faces its owner, then tap it.
    let mut rotation =
        Quat::from_rotation_y(-slot.facing) * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    if tapped {
        rotation = Quat::from_rotation_y(-slot.facing)
            * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
            * Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
    }
    Transform {
        translation: to_world(position, TABLE_Y + CARD_LIFT + lift),
        rotation,
        scale: Vec3::ONE,
    }
}

/// Where every group in the current board model belongs.
struct Placement {
    object: ObjectId,
    slot: SeatSlot,
    position: Vec2,
    tapped: bool,
    count: usize,
    art: Option<ImageKey>,
}

/// Computes placements for the whole table.
///
/// Pure geometry over the board model, so the ordering is the model's ordering
/// and therefore stable frame to frame — which is what makes the diff below
/// cheap and stops cards from swapping places when nothing happened.
fn placements(duel: &Duel) -> Vec<Placement> {
    let (Some(board), Some(layout)) = (duel.board.as_ref(), duel.layout.as_ref()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for pod in &board.pods {
        let Some(slot) = layout.slot(pod.player) else {
            continue;
        };
        for lane in &pod.lanes {
            let center = slot.lane_center(lane.kind);
            let packing = pack_lane(lane.groups.len(), slot.lane_width());
            for (group, offset) in lane.groups.iter().zip(packing.offsets.iter()) {
                let along = Vec2::new(slot.facing.cos(), -slot.facing.sin());
                out.push(Placement {
                    object: group.representative,
                    slot: *slot,
                    position: center + along * *offset,
                    tapped: group.status.is_tapped(),
                    count: group.count(),
                    art: group.art,
                });
            }
        }
    }
    out
}

/// Brings the scene in line with the board model.
#[allow(clippy::too_many_arguments)]
pub fn sync_scene(
    mut commands: Commands,
    duel: Res<Duel>,
    mut index: ResMut<SceneIndex>,
    mut textures: Option<ResMut<CardTextures>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>,
    mut cards: Query<(
        &mut Transform,
        &mut CardVisual,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    let (Some(statics), Some(textures)) = (duel.statics.as_ref(), textures.as_mut()) else {
        return;
    };
    let Some(quad) = index.quad.clone() else {
        return;
    };
    let blank = index.blank.clone();

    let wanted = placements(&duel);
    let mut live: HashSet<ObjectId> = HashSet::new();

    for placement in &wanted {
        live.insert(placement.object);

        // One material per texture, created on first use.
        let material = match placement.art {
            Some(key) => {
                if let Some(handle) = index.materials.get(&key) {
                    handle.clone()
                } else {
                    let image = textures.get(key, statics, &assets);
                    let handle = materials.add(StandardMaterial {
                        base_color_texture: Some(image),
                        unlit: true,
                        ..default()
                    });
                    index.materials.insert(key, handle.clone());
                    handle
                }
            }
            None => blank.clone().unwrap_or_default(),
        };

        let transform = card_transform(&placement.slot, placement.position, placement.tapped, 0.0);

        if let Some(&entity) = index.cards.get(&placement.object) {
            // Existing card: update in place. Touching only what changed is
            // what keeps a large board cheap.
            if let Ok((mut current, mut visual, mut current_material)) = cards.get_mut(entity) {
                if *current != transform {
                    *current = transform;
                }
                if visual.count != placement.count {
                    visual.count = placement.count;
                }
                if current_material.0 != material {
                    current_material.0 = material;
                }
            }
            continue;
        }

        let entity = commands
            .spawn((
                DuelStage,
                CardVisual {
                    object: placement.object,
                    count: placement.count,
                },
                Mesh3d(quad.clone()),
                MeshMaterial3d(material),
                transform,
                NotShadowCaster,
            ))
            .id();
        index.cards.insert(placement.object, entity);

        // A counted group gets a few offset cards behind it so the stack reads
        // as physical depth rather than as a number floating on one card.
        let depth = placement.count.saturating_sub(1).min(MAX_STACK_DEPTH);
        for i in 1..=depth {
            let back = card_transform(
                &placement.slot,
                placement.position + Vec2::splat(0.02 * i as f32),
                placement.tapped,
                STACK_LIFT * i as f32,
            );
            commands.spawn((
                DuelStage,
                Mesh3d(quad.clone()),
                MeshMaterial3d(blank.clone().unwrap_or_default()),
                back,
                NotShadowCaster,
            ));
        }
    }

    // Anything no longer on the board leaves the scene.
    let stale: Vec<ObjectId> = index
        .cards
        .keys()
        .copied()
        .filter(|id| !live.contains(id))
        .collect();
    for id in stale {
        if let Some(entity) = index.cards.remove(&id) {
            commands.entity(entity).despawn();
        }
    }

    // Tell the cache what is on screen so it can evict the rest.
    let visible: Vec<ImageKey> = wanted.iter().filter_map(|p| p.art).collect();
    textures.retain_visible(&visible);
}

/// How a group should be labelled in the overlay, if at all.
#[must_use]
pub fn stack_badge(group: &CardGroup) -> Option<String> {
    group.is_stack().then(|| format!("×{}", group.count()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(facing: f32) -> SeatSlot {
        SeatSlot {
            player: baylee_core::ids::PlayerId::new(0),
            ring_index: 0,
            angle: facing,
            center: Vec2::ZERO,
            facing,
            half_extent: Vec2::new(6.0, 3.0),
            is_local: true,
        }
    }

    #[test]
    fn table_space_maps_away_from_the_seat_into_the_screen() {
        // +y in table space is away from the local seat, which is -z in the
        // world because the camera sits on +z.
        let world = to_world(Vec2::new(2.0, 5.0), 0.0);
        assert!((world.x - 2.0).abs() < 1e-5);
        assert!((world.z + 5.0).abs() < 1e-5);
    }

    #[test]
    fn an_untapped_card_lies_flat_on_the_table() {
        let t = card_transform(&slot(0.0), Vec2::ZERO, false, 0.0);
        // The quad's normal (+z in local space) should point straight up.
        let normal = t.rotation * Vec3::Z;
        assert!((normal.y - 1.0).abs() < 1e-4, "normal was {normal:?}");
    }

    #[test]
    fn tapping_rotates_a_quarter_turn_but_keeps_the_card_on_the_table() {
        let untapped = card_transform(&slot(0.0), Vec2::ZERO, false, 0.0);
        let tapped = card_transform(&slot(0.0), Vec2::ZERO, true, 0.0);
        assert_ne!(untapped.rotation, tapped.rotation);

        // Still flat: only the in-plane orientation changed.
        let normal = tapped.rotation * Vec3::Z;
        assert!((normal.y - 1.0).abs() < 1e-4, "normal was {normal:?}");

        // The card's long axis has swung to the side.
        let up = tapped.rotation * Vec3::Y;
        assert!(
            up.y.abs() < 1e-4,
            "long axis should now lie across the table"
        );
    }

    #[test]
    fn cards_float_above_the_felt_so_they_never_z_fight() {
        let t = card_transform(&slot(0.0), Vec2::ZERO, false, 0.0);
        assert!(t.translation.y > TABLE_Y);
    }

    #[test]
    fn a_seat_across_the_table_has_its_cards_turned_to_face_it() {
        let near = card_transform(&slot(0.0), Vec2::ZERO, false, 0.0);
        let far = card_transform(&slot(std::f32::consts::PI), Vec2::ZERO, false, 0.0);
        // Both flat, but their in-plane orientation is opposite.
        let near_up = near.rotation * Vec3::Y;
        let far_up = far.rotation * Vec3::Y;
        assert!(
            near_up.dot(far_up) < -0.9,
            "far seat should be turned around"
        );
    }

    #[test]
    fn only_counted_groups_get_a_badge() {
        use baylee_client_core::board::KeywordBadge;
        use baylee_view::ObjectStatus;

        let mut group = CardGroup {
            representative: ObjectId::new(1, 0),
            members: vec![ObjectId::new(1, 0)],
            name: "Soldier".into(),
            power: Some(1),
            toughness: Some(1),
            damage: 0,
            status: ObjectStatus::NONE,
            counters: vec![],
            badges: Vec::<KeywordBadge>::new(),
            art: None,
            is_token: true,
            summoning_sick: false,
            individual: None,
        };
        assert_eq!(stack_badge(&group), None);
        group.members.push(ObjectId::new(2, 0));
        group.members.push(ObjectId::new(3, 0));
        assert_eq!(stack_badge(&group).as_deref(), Some("×3"));
    }
}

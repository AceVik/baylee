//! The shells a protected permanent wears.

#[allow(clippy::wildcard_imports)] // the table's shared vocabulary
use super::*;

/// Which part of a permanent's shell an entity is ([`shellmat`]).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellPart {
    /// The rim standing round the card.
    Rim,
    /// The ring on the felt the rim lies down to where it has no room.
    Ring,
    /// A dome standing over the card.
    Dome,
    /// The ring on the felt a dome lies down to.
    DomeRing,
    /// Defender's wall, on the felt past the card's top edge.
    Wall,
    /// A standing dome's shadow on the felt, or the wall's.
    Shade,
    /// Summoning sickness's wave, over the card's face.
    Wave,
}

/// One shell round a card: what stands, the ring it lies down to, and how
/// it stands as [`fit_the_shells`] last found: its step of
/// [`shellmat::DOME_STEPS`] (a rim has only the first), or `None` lying.
/// A dome's shadow on the felt is a child of what stands.
#[derive(Clone, Copy, Debug)]
struct Layer {
    stand: Entity,
    lie: Entity,
    step: Option<usize>,
    shade: Option<Entity>,
}

/// The shells round one card, inside out: see [`SceneIndex::shells`].
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Shell {
    /// Indestructible's steel.
    steel: Option<Layer>,
    /// Hexproof's or shroud's dome, and which.
    dome: Option<(shellmat::Dome, Layer)>,
    /// Defender's wall, which always stands.
    wall: Option<Entity>,
    /// Summoning sickness's wave, and whether it was drawn last frame.
    wave: Option<(Entity, bool)>,
}

impl Shell {
    fn layers(self) -> impl Iterator<Item = Layer> {
        self.steel
            .into_iter()
            .chain(self.dome.map(|(_, layer)| layer))
    }

    pub(super) fn despawn(self, commands: &mut Commands) {
        for layer in self.layers() {
            commands.entity(layer.stand).despawn();
            commands.entity(layer.lie).despawn();
        }
        if let Some(wall) = self.wall {
            commands.entity(wall).despawn();
        }
        if let Some((wave, _)) = self.wave {
            commands.entity(wave).despawn();
        }
    }

    fn is_empty(self) -> bool {
        self.layers().next().is_none() && self.wall.is_none() && self.wave.is_none()
    }
}

/// The material for `look`, made the first time a table asks for it.
fn shell_material(
    index: &mut SceneIndex,
    materials: &mut Assets<ShellMaterial>,
    look: ShellLook,
    motion: f32,
) -> Handle<ShellMaterial> {
    index
        .shell_materials
        .entry(look)
        .or_insert_with(|| materials.add(ShellMaterial::new(look, motion)))
        .clone()
}

/// Spawns one shell round `card`, hidden: what stands, at `stand_at` in the
/// card's space, and the ring it lies down to.
fn spawn_layer(
    commands: &mut Commands,
    card: Entity,
    stand: (ShellPart, Handle<Mesh>, Handle<ShellMaterial>),
    lie: (ShellPart, Handle<Mesh>, Handle<ShellMaterial>),
    step: Option<usize>,
) -> Layer {
    let [stand, lie] = [(stand, true), (lie, false)].map(|((part, mesh, material), up)| {
        commands
            .spawn((
                part,
                Mesh3d(mesh),
                MeshMaterial3d(material),
                // What stands has its origin on the card's face, which is the
                // plane its mask measures the print on; a ring is laid on
                // the felt by `fit_the_shells`.
                if up {
                    Transform::from_xyz(0.0, 0.0, CARD_THICKNESS)
                } else {
                    Transform::default()
                },
                Visibility::Hidden,
                Pickable::IGNORE,
            ))
            .id()
    });
    commands.entity(card).add_children(&[stand, lie]);
    Layer {
        stand,
        lie,
        step,
        shade: None,
    }
}

/// Spawns a shadow on the felt under `parent`, hidden or not as `shown`:
/// it lies in `parent`'s own space.
fn spawn_shade(
    commands: &mut Commands,
    index: &mut SceneIndex,
    materials: &mut Assets<ShellMaterial>,
    parent: Entity,
    mesh: Handle<Mesh>,
    motion: f32,
    shown: Visibility,
) -> Entity {
    let material = shell_material(index, materials, ShellLook::steel(ShellKind::Shade), motion);
    let shade = commands
        .spawn((
            ShellPart::Shade,
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::default(),
            shown,
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(parent).add_child(shade);
    shade
}

/// Puts a permanent's shells round it, or takes them away: indestructible's
/// steel, hexproof's or shroud's dome, defender's wall and summoning
/// sickness's wave.
///
/// Each is spawned hidden, as children of the card, and [`fit_the_shells`]
/// shows it standing or lying on the same frame: which depends on where
/// every card is once they have all moved, which is not known here. A card
/// standing in a pile's hover fan has none: it is not on the battlefield.
pub(super) fn sync_shell(
    commands: &mut Commands,
    index: &mut SceneIndex,
    materials: &mut Assets<ShellMaterial>,
    card: Entity,
    placement: &Placement,
    motion: f32,
) {
    let on_table = placement.fan.is_none();
    let steel = placement.indestructible && on_table;
    let dome = placement.dome.filter(|_| on_table);
    let wall = placement.defender && on_table;
    let wave = placement.sick && on_table;
    let mut shell = index
        .shells
        .get(&placement.object)
        .copied()
        .unwrap_or_default();
    if shell.steel.is_some() == steel
        && shell.dome.map(|(d, _)| d) == dome
        && shell.wall.is_some() == wall
        && shell.wave.is_some() == wave
    {
        return;
    }
    let (Some(rim_mesh), Some(ring_mesh)) = (
        index.rim_mesh.clone(),
        index.ring_meshes.get(&Band::Whole).cloned(),
    ) else {
        return;
    };
    if shell.steel.is_some() != steel {
        if let Some(layer) = shell.steel.take() {
            Shell {
                steel: Some(layer),
                ..Shell::default()
            }
            .despawn(commands);
        } else {
            let rim = shell_material(index, materials, ShellLook::steel(ShellKind::Rim), motion);
            let ring = shell_material(index, materials, ShellLook::steel(ShellKind::Ring), motion);
            // The half of the band it takes under a dome's ring, ready for
            // the frame both lie down.
            let mut inner = ShellLook::steel(ShellKind::Ring);
            inner.band = Band::Inner;
            shell_material(index, materials, inner, motion);
            shell.steel = Some(spawn_layer(
                commands,
                card,
                (ShellPart::Rim, rim_mesh, rim),
                (ShellPart::Ring, ring_mesh.clone(), ring),
                None,
            ));
        }
    }
    if shell.dome.map(|(d, _)| d) != dome {
        if let Some(gone) = shell.dome.take() {
            Shell {
                dome: Some(gone),
                ..Shell::default()
            }
            .despawn(commands);
        }
        shell.dome = dome.and_then(|which| {
            spawn_dome(commands, index, materials, card, which, ring_mesh, motion)
        });
    }
    if shell.wall.is_some() != wall {
        shell.wall = sync_wall(commands, index, materials, card, shell.wall, motion);
    }
    if shell.wave.is_some() != wave {
        shell.wave = sync_wave(commands, index, materials, card, shell.wave, motion);
    }
    if shell.is_empty() {
        index.shells.remove(&placement.object);
    } else {
        index.shells.insert(placement.object, shell);
    }
}

/// Builds hexproof's or shroud's dome round `card`, hidden: what stands,
/// the ring it lies down to, the materials of both bands that ring may lie
/// in, and its shadow on the felt. [`fit_the_shells`] shows them.
fn spawn_dome(
    commands: &mut Commands,
    index: &mut SceneIndex,
    materials: &mut Assets<ShellMaterial>,
    card: Entity,
    which: shellmat::Dome,
    ring_mesh: Handle<Mesh>,
    motion: f32,
) -> Option<(shellmat::Dome, Layer)> {
    let look = |kind, band| ShellLook {
        kind,
        dome: Some(which),
        band,
    };
    let standing = shell_material(index, materials, look(ShellKind::Dome, Band::Whole), motion);
    let lying = shell_material(
        index,
        materials,
        look(ShellKind::DomeRing, Band::Whole),
        motion,
    );
    shell_material(
        index,
        materials,
        look(ShellKind::DomeRing, Band::Outer),
        motion,
    );
    let mesh = index.dome_meshes.get(&(which, 0)).cloned()?;
    let shade = index.dome_shades.get(&(which, 0)).cloned()?;
    let mut layer = spawn_layer(
        commands,
        card,
        (ShellPart::Dome, mesh, standing),
        (ShellPart::DomeRing, ring_mesh, lying),
        None,
    );
    layer.shade = Some(spawn_shade(
        commands,
        index,
        materials,
        layer.stand,
        shade,
        motion,
        Visibility::Hidden,
    ));
    Some((which, layer))
}

/// Takes defender's wall away if `had` one, or builds one, hidden, as a
/// child of `card`: [`fit_the_shells`] stands it on the felt.
fn sync_wall(
    commands: &mut Commands,
    index: &mut SceneIndex,
    materials: &mut Assets<ShellMaterial>,
    card: Entity,
    had: Option<Entity>,
    motion: f32,
) -> Option<Entity> {
    if let Some(gone) = had {
        commands.entity(gone).despawn();
        return None;
    }
    let mesh = index.wall_mesh.clone()?;
    let shade = index.wall_shade.clone()?;
    let material = shell_material(index, materials, ShellLook::steel(ShellKind::Wall), motion);
    let wall = commands
        .spawn((
            ShellPart::Wall,
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::default(),
            Visibility::Hidden,
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(card).add_child(wall);
    spawn_shade(
        commands,
        index,
        materials,
        wall,
        shade,
        motion,
        Visibility::Inherited,
    );
    Some(wall)
}

/// Takes summoning sickness's wave away if `had` one, or lays one, hidden,
/// on `card`'s face: [`fit_the_shells`] shows it where it lands on no other
/// card's print.
fn sync_wave(
    commands: &mut Commands,
    index: &mut SceneIndex,
    materials: &mut Assets<ShellMaterial>,
    card: Entity,
    had: Option<(Entity, bool)>,
    motion: f32,
) -> Option<(Entity, bool)> {
    if let Some((gone, _)) = had {
        commands.entity(gone).despawn();
        return None;
    }
    let mesh = index.wave_mesh.clone()?;
    let material = shell_material(index, materials, ShellLook::steel(ShellKind::Wave), motion);
    let wave = commands
        .spawn((
            ShellPart::Wave,
            Mesh3d(mesh),
            MeshMaterial3d(material),
            // On the card's face, which its sheet is drawn from.
            Transform::from_xyz(0.0, 0.0, CARD_THICKNESS),
            Visibility::Hidden,
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(card).add_child(wave);
    Some((wave, false))
}

/// Where a ring lies under the card at `at`, in the card's own space: flat on
/// the felt at [`shellmat::RING_RUNG`] under the card's middle, unbanked,
/// the way [`ground_the_shadows`] holds a flier's shadow. It grows with the
/// card, since it is drawn round it.
pub(super) fn on_the_felt(at: &Transform) -> Transform {
    let ground = Vec3::new(
        at.translation.x,
        TABLE_Y + shellmat::RING_RUNG,
        at.translation.z,
    );
    let right = at.rotation * Vec3::X;
    let yaw = (-right.z).atan2(right.x);
    Transform {
        translation: at.to_matrix().inverse().transform_point3(ground),
        rotation: at.rotation.inverse()
            * Quat::from_rotation_y(yaw)
            * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        scale: Vec3::ONE,
    }
}

/// Where defender's wall stands for the card at `at`, in the card's own
/// space: on the felt under the card's middle, turned as the card is and
/// unbanked, at the height a resting card's face has, which is the plane
/// its mesh and its mask are drawn from ([`shellmat::wall_mesh`]). Never
/// lifted or grown with the card, which would carry it over its
/// neighbours' faces.
pub fn wall_pose(at: &Transform) -> Transform {
    let mut pose = on_the_felt(at);
    let foot = Vec3::new(
        at.translation.x,
        TABLE_Y + shellmat::RIM_DROP,
        at.translation.z,
    );
    pose.translation = at.to_matrix().inverse().transform_point3(foot);
    pose.scale = Vec3::ONE / at.scale;
    pose
}

/// Stands each protected permanent's shells, or lays them on the felt as
/// rings, and keeps the rings flat on the felt under their card.
///
/// After the glide, because the question is about where every card *is* this
/// frame: a card gliding in, a hover lifting one, a flier on its bob. It asks
/// [`shellmat::rim_stands`] for the steel and [`shellmat::dome_step`] for the
/// dome, against every card on the table, departing ones included, from
/// where the camera is. A dome stands at the tallest step that fits. Where
/// the steel and the dome both lie, the steel takes the band's inner half
/// and the dome its outer, nested as they stand. Defender's wall always
/// stands, on the felt ([`wall_pose`]). Summoning sickness's wave is drawn
/// where [`shellmat::wave_stands`] finds it lands on no other print, and
/// comes back only with a flier's bob of headroom, as a ring stands up.
#[allow(clippy::type_complexity)] // four disjoint views of one table
#[allow(clippy::too_many_lines)] // four shells fitted in one pass over the table
pub fn fit_the_shells(
    mut index: ResMut<SceneIndex>,
    cards: Query<
        (Entity, &Transform, &Visibility),
        (Or<(With<CardVisual>, With<Departing>)>, Without<ShellPart>),
    >,
    camera: Query<&Transform, (With<TableCamera>, Without<ShellPart>)>,
    mut parts: Query<
        (
            &mut Visibility,
            &mut Transform,
            &mut Mesh3d,
            &mut MeshMaterial3d<ShellMaterial>,
        ),
        (With<ShellPart>, Without<TableCamera>),
    >,
) {
    if index.shells.is_empty() {
        return;
    }
    let Ok(eye) = camera.single() else {
        return;
    };
    let eye = eye.translation;
    // Only the cards that are drawn: a scrolled row's hidden cards stand
    // past the lane's end, where nothing is to be kept clear of them.
    let faces: Vec<(Entity, shellmat::Footprint)> = cards
        .iter()
        .filter(|(_, _, seen)| **seen != Visibility::Hidden)
        .map(|(entity, at, _)| (entity, shellmat::Footprint::of(at)))
        .collect();
    let SceneIndex {
        shells,
        cards: drawn,
        ring_meshes,
        dome_meshes,
        dome_shades,
        shell_materials,
        ..
    } = &mut *index;
    for (object, shell) in shells.iter_mut() {
        let Some(&card) = drawn.get(object) else {
            continue;
        };
        let Ok((_, at, _)) = cards.get(card) else {
            continue;
        };
        if let Some(wall) = shell.wall
            && let Ok((mut shown, mut transform, _, _)) = parts.get_mut(wall)
        {
            shown.set_if_neq(Visibility::Inherited);
            transform.set_if_neq(wall_pose(at));
        }
        let me = shellmat::Footprint::of(at);
        let others = faces
            .iter()
            .filter(move |(entity, _)| *entity != card)
            .map(|(_, face)| *face);
        if let Some((wave, standing)) = &mut shell.wave {
            let headroom = if *standing {
                0.0
            } else {
                shellmat::STAND_AGAIN
            };
            *standing = shellmat::wave_stands(&me, others.clone(), eye, headroom);
            if let Ok((mut shown, _, _, _)) = parts.get_mut(*wave) {
                shown.set_if_neq(if *standing {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
            }
        }
        if let Some(steel) = &mut shell.steel {
            let headroom = if steel.step.is_some() {
                0.0
            } else {
                shellmat::STAND_AGAIN
            };
            steel.step = shellmat::rim_stands(&me, others.clone(), eye, headroom).then_some(0);
        }
        if let Some((dome, layer)) = &mut shell.dome {
            layer.step = shellmat::dome_step(dome.row(), layer.step, &me, others, eye);
        }
        let both_lie =
            shell.layers().all(|layer| layer.step.is_none()) && shell.layers().count() == 2;
        let lying = on_the_felt(at);
        let mut show = |layer: Layer, stand_mesh: Option<Handle<Mesh>>, ring: ShellLook| {
            if let Ok((mut shown, _, mut mesh, _)) = parts.get_mut(layer.stand) {
                shown.set_if_neq(if layer.step.is_some() {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
                if let Some(wanted) = stand_mesh
                    && mesh.0 != wanted
                {
                    mesh.0 = wanted;
                }
            }
            if let Ok((mut shown, mut transform, mut mesh, mut material)) = parts.get_mut(layer.lie)
            {
                shown.set_if_neq(if layer.step.is_some() {
                    Visibility::Hidden
                } else {
                    Visibility::Inherited
                });
                transform.set_if_neq(lying);
                if let Some(wanted) = ring_meshes.get(&ring.band)
                    && mesh.0 != *wanted
                {
                    mesh.0 = wanted.clone();
                }
                if let Some(wanted) = shell_materials.get(&ring)
                    && material.0 != *wanted
                {
                    material.0 = wanted.clone();
                }
            }
        };
        if let Some(steel) = shell.steel {
            let mut ring = ShellLook::steel(ShellKind::Ring);
            if both_lie {
                ring.band = Band::Inner;
            }
            show(steel, None, ring);
        }
        if let Some((dome, layer)) = shell.dome {
            let ring = ShellLook {
                kind: ShellKind::DomeRing,
                dome: Some(dome),
                band: if both_lie { Band::Outer } else { Band::Whole },
            };
            let mesh = layer
                .step
                .and_then(|step| dome_meshes.get(&(dome, step)).cloned());
            show(layer, mesh, ring);
            // Its shadow, at the steps it stands on the felt at.
            let shade = layer
                .step
                .and_then(|step| dome_shades.get(&(dome, step)).cloned());
            if let Some(entity) = layer.shade
                && let Ok((mut shown, _, mut mesh, _)) = parts.get_mut(entity)
            {
                shown.set_if_neq(if shade.is_some() {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
                if let Some(wanted) = shade
                    && mesh.0 != wanted
                {
                    mesh.0 = wanted;
                }
            }
        }
    }
}

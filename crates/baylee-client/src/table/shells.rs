//! The shells a protected or summoning-sick permanent wears.

#[allow(clippy::wildcard_imports)] // the table's shared vocabulary
use super::*;

/// Which part of a permanent's shell an entity is ([`shellmat`]).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellPart {
    /// Its keywords' border standing round the card.
    Border,
    /// The ring on the felt the border lies down to where it has no room.
    Ring,
    /// Summoning sickness's wave, over the card's face.
    Wave,
}

/// A card's border: what stands, the ring it lies down to, which keywords
/// it says, and whether it stood when [`fit_the_shells`] last looked.
#[derive(Clone, Copy, Debug)]
struct Layer {
    ward: shellmat::Ward,
    stand: Entity,
    lie: Entity,
    standing: bool,
}

/// The shells round one card: see [`SceneIndex::shells`].
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Shell {
    /// Its keywords' border.
    border: Option<Layer>,
    /// Summoning sickness's wave, and whether it was drawn last frame.
    wave: Option<(Entity, bool)>,
}

impl Shell {
    pub(super) fn despawn(self, commands: &mut Commands) {
        if let Some(layer) = self.border {
            commands.entity(layer.stand).despawn();
            commands.entity(layer.lie).despawn();
        }
        if let Some((wave, _)) = self.wave {
            commands.entity(wave).despawn();
        }
    }

    fn is_empty(self) -> bool {
        self.border.is_none() && self.wave.is_none()
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

/// Puts a permanent's shells round it, or takes them away: its keywords'
/// border and summoning sickness's wave.
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
    let ward = if on_table {
        placement.ward
    } else {
        shellmat::Ward::NONE
    };
    let wave = placement.sick && on_table;
    let mut shell = index
        .shells
        .get(&placement.object)
        .copied()
        .unwrap_or_default();
    if shell.border.map_or(shellmat::Ward::NONE, |l| l.ward) == ward && shell.wave.is_some() == wave
    {
        return;
    }
    if shell.border.map_or(shellmat::Ward::NONE, |l| l.ward) != ward {
        if let Some(gone) = shell.border.take() {
            commands.entity(gone.stand).despawn();
            commands.entity(gone.lie).despawn();
        }
        shell.border = spawn_border(commands, index, materials, card, ward, motion);
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

/// Builds the border of `ward` round `card`, hidden: what stands, on the
/// card's face, which is the plane its mask measures the print on, and the
/// ring it lies down to, which [`fit_the_shells`] lays on the felt. `None`
/// for no keyword.
fn spawn_border(
    commands: &mut Commands,
    index: &mut SceneIndex,
    materials: &mut Assets<ShellMaterial>,
    card: Entity,
    ward: shellmat::Ward,
    motion: f32,
) -> Option<Layer> {
    if ward.is_empty() {
        return None;
    }
    let layers = ward.layers();
    let stand_mesh = index.border_meshes.get(&layers).cloned()?;
    let lie_mesh = index.ring_meshes.get(&layers).cloned()?;
    let stand_material = shell_material(index, materials, ShellLook::border(ward), motion);
    let lie_material = shell_material(index, materials, ShellLook::ring(ward), motion);
    let [stand, lie] = [
        (ShellPart::Border, stand_mesh, stand_material, true),
        (ShellPart::Ring, lie_mesh, lie_material, false),
    ]
    .map(|(part, mesh, material, up)| {
        commands
            .spawn((
                part,
                Mesh3d(mesh),
                MeshMaterial3d(material),
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
    Some(Layer {
        ward,
        stand,
        lie,
        standing: false,
    })
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
    let material = shell_material(index, materials, ShellLook::wave(), motion);
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

/// Stands each permanent's border, or lays it on the felt as a ring, and
/// keeps the rings flat on the felt under their card; and draws each wave
/// that lands on no other print.
///
/// After the glide, because the question is about where every card *is* this
/// frame: a card gliding in, a hover lifting one, a flier on its bob. It asks
/// [`shellmat::border_stands`] for the border and
/// [`shellmat::wave_stands`] for the wave, against every card on the table,
/// departing ones included, from where the camera is. Either comes back
/// only with a flier's bob of headroom ([`shellmat::STAND_AGAIN`]).
#[allow(clippy::type_complexity)] // three disjoint views of one table
pub fn fit_the_shells(
    mut index: ResMut<SceneIndex>,
    cards: Query<
        (Entity, &Transform, &Visibility),
        (Or<(With<CardVisual>, With<Departing>)>, Without<ShellPart>),
    >,
    camera: Query<&Transform, (With<TableCamera>, Without<ShellPart>)>,
    mut parts: Query<(&mut Visibility, &mut Transform), (With<ShellPart>, Without<TableCamera>)>,
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
        ..
    } = &mut *index;
    let shown = |up: bool| {
        if up {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        }
    };
    for (object, shell) in shells.iter_mut() {
        let Some(&card) = drawn.get(object) else {
            continue;
        };
        let Ok((_, at, _)) = cards.get(card) else {
            continue;
        };
        let me = shellmat::Footprint::of(at);
        let others = faces
            .iter()
            .filter(move |(entity, _)| *entity != card)
            .map(|(_, face)| *face);
        let headroom = |standing: bool| {
            if standing { 0.0 } else { shellmat::STAND_AGAIN }
        };
        if let Some((wave, standing)) = &mut shell.wave {
            *standing = shellmat::wave_stands(&me, others.clone(), eye, headroom(*standing));
            if let Ok((mut seen, _)) = parts.get_mut(*wave) {
                seen.set_if_neq(shown(*standing));
            }
        }
        if let Some(layer) = &mut shell.border {
            layer.standing = shellmat::border_stands(
                layer.ward.layers(),
                &me,
                others,
                eye,
                headroom(layer.standing),
            );
            if let Ok((mut seen, _)) = parts.get_mut(layer.stand) {
                seen.set_if_neq(shown(layer.standing));
            }
            if let Ok((mut seen, mut transform)) = parts.get_mut(layer.lie) {
                seen.set_if_neq(shown(!layer.standing));
                transform.set_if_neq(on_the_felt(at));
            }
        }
    }
}

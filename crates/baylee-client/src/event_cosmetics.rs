//! Account-authorized table cosmetics. Materials belong to visible seats.
//! The engine, card identity and object IDs have no role in their appearance.
use baylee_core::ids::PlayerId;
use bevy::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
mod admission;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use admission::MatchLooks;
mod preview;
#[cfg(test)]
mod tests;
pub(crate) use preview::CosmeticPreview;

/// The baked image and the authored, UV-locked surface finish (0, 1, or 2).
#[derive(Clone, PartialEq)]
pub(crate) struct Paint {
    pub(crate) image: Handle<Image>,
    pub(crate) tier: u8,
}

#[derive(Default)]
struct Seat {
    sleeve: Option<(Paint, Handle<crate::cardmat::CardMaterial>)>,
    mat: Option<Paint>,
}

/// Only the current participant's authorized images, released on table change.
#[derive(Resource, Default)]
pub struct Worn([Seat; 8]);

impl Worn {
    pub(crate) fn sleeve(&self, seat: PlayerId) -> Option<&Handle<crate::cardmat::CardMaterial>> {
        self.0
            .get(usize::from(seat.get()))?
            .sleeve
            .as_ref()
            .map(|(_, m)| m)
    }
    pub(crate) fn mat(&self, seat: PlayerId) -> Option<&Paint> {
        self.0.get(usize::from(seat.get()))?.mat.as_ref()
    }
    pub(crate) fn sleeve_paint(&self, seat: PlayerId) -> Option<&Paint> {
        self.0
            .get(usize::from(seat.get()))?
            .sleeve
            .as_ref()
            .map(|(p, _)| p)
    }
}

/// Every slab in a library wears the same back, including its top.
#[derive(Component)]
pub(crate) struct LibrarySleeve(
    pub(crate) PlayerId,
    pub(crate) Handle<crate::cardmat::CardMaterial>,
);

/// An existing UI card displaying its physical reverse, with a public owner.
#[derive(Component)]
pub(crate) struct HiddenPreview(pub(crate) PlayerId);

#[derive(Component)]
struct Attached {
    face: Entity,
    paint: Paint,
}

fn apply_previews(
    mut commands: Commands,
    worn: Res<Worn>,
    slots: Query<(Entity, Ref<HiddenPreview>, Option<&Attached>)>,
    children: Query<&Children>,
    detached: Query<&Attached, Without<HiddenPreview>>,
    mut removed: RemovedComponents<HiddenPreview>,
) {
    for slot in removed.read() {
        if let Ok(old) = detached.get(slot) {
            commands.entity(old.face).despawn();
            commands.entity(slot).remove::<Attached>();
        }
    }
    for (slot, owner, current) in &slots {
        if !worn.is_changed() && !owner.is_changed() {
            continue;
        }
        let paint = worn.sleeve_paint(owner.0);
        if current.map(|c| &c.paint) == paint {
            continue;
        }
        match (paint, current) {
            (Some(p), Some(old)) => {
                commands.entity(old.face).insert(CosmeticPreview(p.clone()));
                commands.entity(slot).insert(Attached {
                    face: old.face,
                    paint: p.clone(),
                });
            }
            (Some(p), None) => {
                let face = commands
                    .spawn((
                        CosmeticPreview(p.clone()),
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(0),
                            top: px(0),
                            width: percent(100),
                            height: percent(100),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .id();
                // Cover the base print while keeping later public indicators on top.
                let above_base = usize::from(children.get(slot).is_ok_and(|c| !c.is_empty()));
                commands
                    .entity(slot)
                    .insert_children(above_base, &[face])
                    .insert(Attached {
                        face,
                        paint: p.clone(),
                    });
            }
            (None, Some(old)) => {
                commands.entity(old.face).despawn();
                commands.entity(slot).remove::<Attached>();
            }
            (None, None) => {}
        }
    }
}

/// A concealed object's already public owner. A missing printing on a
/// visible object is never interpreted as a hidden card.
pub(crate) fn concealed(object: &baylee_view::PublicObject) -> Option<PlayerId> {
    (object.card.is_none() && (object.status.is_face_down() || object.name == "Face-down"))
        .then_some(object.owner)
}

/// The reverse of a card already visible in a public zone or an expressly
/// shown hand. No hidden zone is indexed or reconstructed.
pub(crate) fn visible_owner(
    view: &baylee_view::PlayerView,
    id: baylee_core::ids::ObjectId,
) -> Option<PlayerId> {
    view.object(id)
        .map(|o| o.owner)
        .or_else(|| {
            view.hand
                .iter()
                .any(|card| card.id == id)
                .then_some(view.seat)
        })
        .or_else(|| {
            view.shared_hands
                .iter()
                .chain(&view.controlled_hands)
                .find(|hand| hand.cards.iter().any(|card| card.id == id))
                .map(|hand| hand.player)
        })
}

fn apply_surfaces(
    worn: Res<Worn>,
    mats: Query<(
        Ref<crate::table::SeatMat>,
        &MeshMaterial3d<crate::matmat::MatMaterial>,
    )>,
    mut assets: ResMut<Assets<crate::matmat::MatMaterial>>,
    mut libraries: Query<(
        Ref<LibrarySleeve>,
        &mut MeshMaterial3d<crate::cardmat::CardMaterial>,
    )>,
) {
    for (seat, handle) in &mats {
        if !worn.is_changed() && !seat.is_added() {
            continue;
        }
        let paint = worn.mat(seat.0);
        let image = paint.map(|p| &p.image);
        let decoration = UVec4::new(paint.map_or(0, |p| u32::from(p.tier) + 1), 0, 0, 0);
        if assets
            .get(handle)
            .is_some_and(|m| m.artwork.as_ref() != image || m.decoration != decoration)
            && let Some(mut m) = assets.get_mut(handle)
        {
            m.artwork = image.cloned();
            m.decoration = decoration;
        }
    }
    for (sleeve, mut material) in &mut libraries {
        if !worn.is_changed() && !sleeve.is_added() {
            continue;
        }
        let wanted = worn.sleeve(sleeve.0).unwrap_or(&sleeve.1);
        if material.0 != *wanted {
            material.0 = wanted.clone();
        }
    }
}

pub(crate) fn install(app: &mut App) {
    bevy::asset::embedded_asset!(app, "shaders/event_preview.wesl");
    app.init_resource::<Worn>().add_systems(
        Update,
        apply_surfaces
            .after(crate::table::sync_zones)
            .in_set(crate::DuelSet::Present),
    );
    #[cfg(not(target_arch = "wasm32"))]
    app.init_resource::<MatchLooks>()
        .add_systems(OnEnter(crate::DuelPhase::Closed), admission::leave)
        .add_systems(Update, admission::refresh.before(crate::DuelSet::Present));
    preview::install(app);
    app.add_systems(
        Update,
        apply_previews
            .after(crate::hud::sync_overlay)
            .in_set(crate::DuelSet::Present),
    );
}

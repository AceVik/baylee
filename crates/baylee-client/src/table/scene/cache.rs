//! Exact retained inputs for a visible card, separate from its animated pose.
//!
//! Never use a view sequence alone: replay seeks and perspective replacement
//! can reuse it. A face also compares the public object this seat actually saw.

use super::{
    Entity, FinishTreatment, HashMap, HashSet, ObjectId, Place, Placement, SceneIndex, Transform,
};

/// Reusable storage owned by the scene synchronization system.
#[derive(Default)]
pub struct SceneWork {
    pub(super) wanted: Vec<Placement>,
    pub(super) live: HashSet<ObjectId>,
    pub(super) hidden: HashSet<ObjectId>,
    pub(super) fanned: HashMap<ObjectId, Place>,
    pub(super) stamps: HashMap<ObjectId, CardStamp>,
}

/// Handles may arrive after the first card. Keep their presence in the key
/// so a skipped decoration is retried even when the public view is unchanged.
/// Read once per scene pass; fixed arrays neither allocate nor retain handles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct DecorationReady {
    assets: [bool; 8],
    shells: [[bool; 2]; crate::shellmat::MAX_LAYERS as usize],
}

impl DecorationReady {
    pub(super) fn of(index: &SceneIndex) -> Self {
        let mut ready = Self {
            assets: [
                index.marks_quad.is_some(),
                index.badge_quad.is_some(),
                index.plate_quad.is_some(),
                index.floor_quad.is_some(),
                index.wave_mesh.is_some(),
                index.shadow_quad.is_some(),
                index.shadow_material.is_some(),
                index.blank.is_some(),
            ],
            shells: [[false; 2]; crate::shellmat::MAX_LAYERS as usize],
        };
        for (layers, pair) in (1..=crate::shellmat::MAX_LAYERS).zip(&mut ready.shells) {
            *pair = [
                index.border_meshes.contains_key(&layers),
                index.ring_meshes.contains_key(&layers),
            ];
        }
        ready
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct CardInput {
    pub(super) decorations: DecorationReady,
    pub(super) placement: Placement,
    pub(super) eye: Transform,
    pub(super) still: bool,
    pub(super) finish: FinishTreatment,
    pub(super) glow: u32,
    pub(super) veil: crate::cardmat::Veil,
    pub(super) sweep: Option<crate::sheen::Sweep>,
    pub(super) show_face: bool,
    pub(super) texture_epoch: u64,
    pub(super) text_epoch: u64,
    pub(super) seq: u64,
}

pub(super) struct CardStamp {
    entity: Entity,
    input: CardInput,
    // Only a constructed face reads all the projected words. Art and its
    // decorations are fully described by CardInput and keep no object copy.
    face: Option<baylee_view::PublicObject>,
}

impl CardStamp {
    pub(super) fn new(
        entity: Entity,
        input: &CardInput,
        object: Option<&baylee_view::PublicObject>,
    ) -> Self {
        Self {
            entity,
            input: *input,
            face: object.filter(|_| input.show_face).cloned(),
        }
    }

    /// A camera, layout or asset change does not change the public object.
    /// Keep its owned buffers in that case, but drop them when the face or
    /// the seat's authorization to see the object goes away.
    pub(super) fn update(
        &mut self,
        entity: Entity,
        input: &CardInput,
        object: Option<&baylee_view::PublicObject>,
    ) {
        self.entity = entity;
        self.input = *input;
        let face = object.filter(|_| input.show_face);
        if self.face.as_ref() != face {
            self.face = face.cloned();
        }
    }

    pub(super) fn matches(
        &self,
        entity: Entity,
        input: &CardInput,
        object: Option<&baylee_view::PublicObject>,
    ) -> bool {
        self.entity == entity
            && self.input == *input
            && (!input.show_face || self.face.as_ref() == object)
    }

    pub(super) fn face_changed(
        &self,
        input: &CardInput,
        object: Option<&baylee_view::PublicObject>,
    ) -> bool {
        self.input.text_epoch != input.text_epoch || self.face.as_ref() != object
    }
}

#[cfg(test)]
mod tests;

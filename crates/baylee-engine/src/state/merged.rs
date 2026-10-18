//! Physical components and the visible top of a merged permanent.
use super::{GameObject, GameState, ObjectId};
use crate::object::Rider;

impl GameState {
    /// Physical objects that leave together when this permanent changes zones.
    #[must_use]
    pub fn merged_components(&self, id: ObjectId) -> smallvec::SmallVec<[ObjectId; 2]> {
        let mut cards = smallvec::smallvec![id];
        if let Some(object) = self.object(id) {
            cards.extend(object.riders.iter().filter_map(|r| match r {
                Rider::MergedComponent(component) => Some(*component),
                _ => None,
            }));
        }
        cards
    }

    /// The top's printing/token identity, without losing the root's physical identity.
    #[must_use]
    pub fn merged_top<'a>(&'a self, object: &'a GameObject) -> &'a GameObject {
        object
            .riders
            .iter()
            .find_map(|r| match r {
                Rider::MergedTop(top) => self.object(*top),
                _ => None,
            })
            .unwrap_or(object)
    }

    /// Every commander within this permanent, including a departure snapshot.
    #[must_use]
    pub fn merged_commanders(&self, object: &GameObject) -> smallvec::SmallVec<[ObjectId; 2]> {
        self.commanders
            .iter()
            .flatten()
            .filter(|commander| {
                commander.object == object.id
                    || object
                        .riders
                        .contains(&Rider::MergedComponent(commander.object))
            })
            .map(|commander| commander.object)
            .collect()
    }
}

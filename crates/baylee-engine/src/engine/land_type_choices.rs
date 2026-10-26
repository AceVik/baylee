//! Ordered basic-land-type entry choices use the ordinary subtype prompt.

use super::{CardLookup, Engine, ObjectId, Pending, PlanKind, PlayerId};
use baylee_core::{generated::subtypes::land, ids::SubtypeId};

impl<L: CardLookup> Engine<L> {
    pub(super) fn ask_two_land_types(&mut self, object: ObjectId, player: PlayerId) -> bool {
        self.pending_plan = Some(PlanKind::ChooseTwoBasicLandTypes {
            object,
            first: None,
        });
        self.pending = Pending::ChooseSubtype {
            player,
            options: vec![
                land::PLAINS,
                land::ISLAND,
                land::SWAMP,
                land::MOUNTAIN,
                land::FOREST,
            ],
        };
        self.awaiting_answer = true;
        true
    }

    pub(super) fn answer_two_land_types(&mut self, subtype: SubtypeId) -> bool {
        let Some(PlanKind::ChooseTwoBasicLandTypes { object, first }) = self.pending_plan.as_ref()
        else {
            return false;
        };
        let (object, first) = (*object, *first);
        if let Some(first) = first {
            if let Some(permanent) = self.state.object_mut(object) {
                permanent.chosen_subtype = Some(first);
                permanent
                    .riders
                    .retain(|rider| !matches!(rider, crate::object::Rider::SecondChosenSubtype(_)));
                permanent
                    .riders
                    .push(crate::object::Rider::SecondChosenSubtype(subtype));
            }
            self.pending_plan = None;
        } else {
            self.pending_plan = Some(PlanKind::ChooseTwoBasicLandTypes {
                object,
                first: Some(subtype),
            });
            if let Some(permanent) = self.state.object_mut(object) {
                permanent.chosen_subtype = Some(subtype);
            }
            // Both questions offer all five types. Choosing a type twice
            // is meaningful: setting a land to that type removes its other
            // land types and printed abilities (CR 305.7).
            self.awaiting_answer = true;
        }
        self.state.invalidate_projections();
        true
    }
}

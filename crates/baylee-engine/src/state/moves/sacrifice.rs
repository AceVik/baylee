//! Sacrifice provenance and the pre-event departure snapshot.
use super::{Cause, GameState, ObjectId, StateError, Zone, ZoneLocation, ZonePosition};

impl GameState {
    /// Sacrifice provenance is independent of whether the operation pays a cost.
    pub(crate) fn sacrifice_object(
        &mut self,
        id: ObjectId,
        cause: Cause,
    ) -> Result<ObjectId, StateError> {
        self.sacrifice_object_with_departure(id, cause, None)
    }

    /// Preserve the pre-event snapshot when several permanents are sacrificed together.
    pub(crate) fn sacrifice_object_with_departure(
        &mut self,
        id: ObjectId,
        cause: Cause,
        departure: Option<crate::damage_history::BattlefieldDeparture>,
    ) -> Result<ObjectId, StateError> {
        let object = self.object(id).ok_or(StateError::NoSuchObject(id))?;
        if object.zone != Zone::Battlefield {
            return Ok(id);
        }
        let owner = object.owner;
        let mut departure = departure.or_else(|| self.departure_snapshot(id));
        if let Some(departure) = &mut departure {
            departure.event.sacrificed = true;
        }
        self.move_object_with_departure(
            id,
            ZoneLocation::Graveyard(owner),
            ZonePosition::Top,
            cause,
            departure,
        )
    }
}

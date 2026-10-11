//! The prepared-permanent button opens the normal casting wizard.
use super::{CardLookup, Engine, EngineError, ObjectId, PlayerId, casting};

impl<L: CardLookup> Engine<L> {
    pub(super) fn start_prepared_cast(
        &mut self,
        player: PlayerId,
        source: ObjectId,
    ) -> Result<(), EngineError> {
        let copy = self
            .state
            .prepared_copy(source)
            .ok_or(EngineError::IllegalAction("no prepared spell"))?;
        if casting::can_cast(&self.state, &self.lookup, player, copy).is_err() {
            return Err(EngineError::IllegalAction(
                "the prepared spell cannot be cast right now",
            ));
        }
        self.start_cast_wizard(player, copy)
    }
}

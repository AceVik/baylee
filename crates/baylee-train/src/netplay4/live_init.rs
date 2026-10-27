//! Initialization for an authorized live seat, without a complete game preset.
use super::{MAX_SEATS, NetPlayer4, PlayerId, RECURRENT_SLOTS, SeatKnowledge4};

impl NetPlayer4 {
    /// Starts one live seat using only its own submitted cards and public teams.
    /// Call once per game, never for repeated roster/print-table frames.
    ///
    /// # Errors
    /// Returns an error if the roster size or viewing seat is invalid.
    pub fn init_own_seat(
        &mut self,
        viewer: PlayerId,
        own_cards: &[(u32, u8)],
        teams: &[Option<u8>],
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            !teams.is_empty() && teams.len() <= MAX_SEATS,
            "invalid live seat count"
        );
        anyhow::ensure!(
            usize::from(viewer.get()) < teams.len(),
            "live seat is outside the table"
        );
        self.reset_game();
        self.teams = teams.to_vec();
        self.knowledge.insert(
            viewer.get(),
            SeatKnowledge4::from_own_cards(viewer, teams.len(), own_cards),
        );
        self.h_states
            .insert(viewer.get(), vec![0.; RECURRENT_SLOTS * self.d_model]);
        Ok(())
    }

    /// Starts one live seat using its full [`baylee_seat::DeckList`] (`main`, `sideboard`, `commanders`).
    ///
    /// # Errors
    /// Returns an error if the roster size or viewing seat is invalid.
    #[cfg(feature = "live-seat")]
    pub fn init_own_deck_list(
        &mut self,
        viewer: PlayerId,
        deck: &baylee_seat::DeckList,
        teams: &[Option<u8>],
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            !teams.is_empty() && teams.len() <= MAX_SEATS,
            "invalid live seat count"
        );
        anyhow::ensure!(
            usize::from(viewer.get()) < teams.len(),
            "live seat is outside the table"
        );
        self.reset_game();
        self.teams = teams.to_vec();
        self.knowledge.insert(
            viewer.get(),
            SeatKnowledge4::from_deck_list(viewer, teams.len(), deck),
        );
        self.h_states
            .insert(viewer.get(), vec![0.; RECURRENT_SLOTS * self.d_model]);
        Ok(())
    }
}

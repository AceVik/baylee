//! The view-only boundary shared by ONNX and deterministic adapter tests.
use crate::features4::SourceKind4;
use crate::netplay4::NetPlayer4;
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_seat::DeckList;
use baylee_view::PlayerView;

/// One action and its actual origin, including explicit runtime fallbacks.
pub struct LiveAnswer4 {
    /// Legal engine action chosen from this seat's view and pending question.
    pub action: PlayerAction,
    /// Neural, forced, or fallback origin returned by the runtime.
    pub source: SourceKind4,
    /// Live-only draw rule audit, absent for ordinary learned policy answers.
    pub draw: Option<super::DrawDecision4>,
}

/// A synchronous seat brain. It never receives opponent decks or game state.
pub trait Brain4 {
    /// Initialize once from this seat's deck and public team assignments.
    ///
    /// # Errors
    /// Returns an error when the seat/roster cannot initialize the runtime.
    fn initialize(
        &mut self,
        seat: PlayerId,
        deck: &DeckList,
        teams: &[Option<u8>],
    ) -> anyhow::Result<()>;
    /// Consume a genuinely changed authorized observation before any action.
    ///
    /// # Errors
    /// Returns an error if inference fails; the caller must stop this trajectory.
    fn observe(&mut self, view: &PlayerView) -> anyhow::Result<()>;
    /// Answer this seat's offered question; no wake/standing filter intervenes.
    ///
    /// # Errors
    /// Returns an error when the offered question cannot be answered safely.
    fn answer(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> anyhow::Result<LiveAnswer4>;
}

impl Brain4 for NetPlayer4 {
    fn initialize(
        &mut self,
        seat: PlayerId,
        deck: &DeckList,
        teams: &[Option<u8>],
    ) -> anyhow::Result<()> {
        self.init_own_deck_list(seat, deck, teams)
    }
    fn observe(&mut self, view: &PlayerView) -> anyhow::Result<()> {
        Self::observe(self, view)
    }
    fn answer(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> anyhow::Result<LiveAnswer4> {
        let answer = self.act_detailed(view, pending, teams);
        let draw =
            super::draw::decide(pending, view, teams, answer.provenance.source, answer.value);
        if let Some(decision) = draw {
            return Ok(LiveAnswer4 {
                action: PlayerAction::YesNo(decision.accepted),
                source: SourceKind4::House,
                draw: Some(decision),
            });
        }
        Ok(LiveAnswer4 {
            action: answer.action,
            source: answer.provenance.source,
            draw: None,
        })
    }
}

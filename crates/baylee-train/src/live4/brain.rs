//! The view-only boundary shared by ONNX and deterministic adapter tests.
use crate::features4::{SourceKind4, StepProvenance4};
use crate::netplay4::{CriticSemantics4, NetPlayer4};
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_seat::DeckList;
use baylee_view::PlayerView;

/// One action and its actual origin, including explicit runtime fallbacks.
pub struct LiveAnswer4 {
    /// Legal engine action chosen from this seat's view and pending question.
    pub action: PlayerAction,
    /// Actual runtime origin and conditional probability, never inferred from a label.
    pub provenance: StepProvenance4,
    /// Live-only draw rule audit, absent for ordinary learned policy answers.
    pub draw: Option<super::DrawDecision4>,
}

/// A synchronous seat brain. It never receives opponent decks or game state.
pub trait Brain4 {
    /// Check this brain's inputs and value semantics against the live adapter.
    ///
    /// # Errors
    /// Rejects unsupported observation context or critic semantics.
    fn validate_live_contract(&self) -> anyhow::Result<()> {
        Ok(())
    }
    /// Whether the brain natively consumes live `casting`/`targeting` context.
    #[must_use]
    fn retains_live_context(&self) -> bool {
        false
    }
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

pub(super) fn validate_wire_encoder(encoder: u32) -> anyhow::Result<()> {
    anyhow::ensure!(
        matches!(encoder, 5..=7),
        "Encoder {encoder} live seats require a supported observation contract; Encoders 8 and 9 need payment-context wire support; use offline eval4"
    );
    Ok(())
}

impl Brain4 for NetPlayer4 {
    fn validate_live_contract(&self) -> anyhow::Result<()> {
        let identity = self.model_identity();
        validate_wire_encoder(identity.encoder_version)?;
        validate_live_critic(identity.critic_semantics.as_ref())
    }
    fn retains_live_context(&self) -> bool {
        self.is_encoder7()
    }
    fn initialize(
        &mut self,
        seat: PlayerId,
        deck: &DeckList,
        teams: &[Option<u8>],
    ) -> anyhow::Result<()> {
        self.validate_live_contract()?;
        self.init_own_deck_list(seat, deck, teams)
    }
    fn observe(&mut self, view: &PlayerView) -> anyhow::Result<()> {
        self.validate_live_contract()?;
        Self::observe(self, view)
    }
    fn answer(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> anyhow::Result<LiveAnswer4> {
        self.validate_live_contract()?;
        Ok(from_runtime(
            self.act_detailed(view, pending, teams),
            view,
            pending,
            teams,
        ))
    }
}

pub(super) fn validate_live_critic(semantics: Option<&CriticSemantics4>) -> anyhow::Result<()> {
    anyhow::ensure!(
        matches!(
            semantics,
            None | Some(CriticSemantics4::MtgSideOutcomeUtilityV1 {})
        ),
        "research utility critic is incompatible with the live MTG draw rule; use offline eval4"
    );
    Ok(())
}

pub(super) fn from_runtime(
    answer: crate::netplay4::Answer4,
    view: &PlayerView,
    pending: &Pending,
    teams: &[Option<u8>],
) -> LiveAnswer4 {
    let draw = super::draw::decide(pending, view, teams, answer.provenance.source, answer.value);
    if let Some(decision) = draw {
        return LiveAnswer4 {
            action: PlayerAction::YesNo(decision.accepted),
            provenance: StepProvenance4 {
                source: SourceKind4::House,
                behavior_logprob: 0.0,
                has_logprob: false,
                ..answer.provenance
            },
            draw: Some(decision),
        };
    }
    LiveAnswer4 {
        action: answer.action,
        provenance: answer.provenance,
        draw: None,
    }
}

//! Targets as candidates ([`SearchConfig4::with_targets`], v3).
//!
//! A root's answer is the announcement of a spell or an ability; its target
//! is chosen at the next question, where the engine cannot fork
//! (`ForkRefusal::Continuation`), so a v2 search never compares targets and
//! the policy's target head alone decides what a removal spell exiles. Here
//! a candidate that casts a spell or activates a non-mana ability is applied
//! in world 0, and when the seat's next question there is a single target,
//! the candidate becomes one per target the policy ranks at it (its own
//! answer first, then by raw logit), each tried at that target in every
//! world. Targets on the battlefield, on the stack and players are public,
//! so world 0 offers the same ones as every world and as the table.
//!
//! [`SearchConfig4::with_targets`]: super::SearchConfig4::with_targets

use super::rollout::{Stop, target_question, world};
use super::select::probabilities;
use super::{
    Candidate4, ForkSource4, PlayerAction, PlayerView, Ranked4, RootDecision4, SearchPlayer4,
    TargetFollow4, candidates,
};
use crate::netplay4::tactics::is_mana_ability_entry;

/// What reading the targets of a root's candidates starts from.
pub(super) struct Probe<'a, S> {
    pub(super) view: &'a PlayerView,
    pub(super) teams: &'a [Option<u8>],
    pub(super) decision: &'a RootDecision4,
    /// The player's state at the root.
    pub(super) root: &'a S,
    /// The most targets one candidate is tried at.
    pub(super) targets: usize,
    /// World 0's policy seed: the targets are read off the first world.
    pub(super) seed: u64,
}

impl<S> Probe<'_, S> {
    /// `ranked`, in order, with every candidate that announces a spell or
    /// an ability whose next question in world 0 is a single target with
    /// two or more answers replaced by one candidate per answer, up to
    /// `targets` of them; with `targets` 0, `ranked` as it is (v2). `player`
    /// ends in the root's state.
    pub(super) fn expand<P, T>(
        &self,
        player: &mut P,
        table: &T,
        ranked: Vec<Ranked4>,
    ) -> Result<Vec<Candidate4>, Stop>
    where
        P: SearchPlayer4<Snapshot = S>,
        T: ForkSource4,
    {
        if self.targets == 0 {
            return Ok(ranked.into_iter().map(Candidate4::plain).collect());
        }
        let mut expanded = Vec::with_capacity(ranked.len());
        for root in ranked {
            let targets = if announces(self.view, &root.action) {
                self.targets_of(player, table, &root.action)
            } else {
                Ok(Vec::new())
            };
            player.restore(self.root);
            let targets = targets?;
            if targets.len() < 2 {
                expanded.push(Candidate4::plain(root));
            } else {
                expanded.extend(targets.into_iter().map(|target| Candidate4 {
                    root: root.clone(),
                    target: Some(target),
                }));
            }
        }
        Ok(expanded)
    }

    /// The answers the policy ranks at the single-target question that
    /// follows `action` in world 0, its own first, with its probability of
    /// each; empty when another question follows or the policy does not
    /// rank the answers (one legal target, a question it does not choose).
    fn targets_of<P, T>(
        &self,
        player: &mut P,
        table: &T,
        action: &PlayerAction,
    ) -> Result<Vec<TargetFollow4>, Stop>
    where
        P: SearchPlayer4<Snapshot = S>,
        T: ForkSource4,
    {
        let seat = self.view.seat;
        let mut world = world(table, seat, self.seed).map_err(Stop::Refused)?;
        player.restore(self.root);
        player.conclude(self.view, self.decision, action);
        if world.act_slice(seat, action.clone(), 0).is_err() {
            return Ok(Vec::new());
        }
        let Some((pending, view)) =
            target_question(player, &mut world, seat).map_err(Stop::Failed)?
        else {
            return Ok(Vec::new());
        };
        let decision = player.decide(&view, &pending, self.teams);
        if decision.ranking.is_none() {
            return Ok(Vec::new());
        }
        let ranked = candidates(&decision, self.targets);
        let priors = probabilities(&decision, &ranked);
        Ok(ranked
            .into_iter()
            .zip(priors)
            .map(|(ranked, prior)| TargetFollow4 {
                action: ranked.action,
                prior,
            })
            .collect())
    }
}

/// Whether `action` announces a spell or an ability that may ask for a
/// target: a cast, or the activation of an ability that is not a mana
/// ability (CR 605.1; a mana ability takes no target, CR 605.1a).
fn announces(view: &PlayerView, action: &PlayerAction) -> bool {
    match action {
        PlayerAction::CastSpell { .. } => true,
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        } => !is_mana_ability_entry(view, *source, *ability_index),
        _ => false,
    }
}

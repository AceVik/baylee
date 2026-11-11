//! What restricted mana may pay for in an offer (CR 106.6):
//! `LegalActions::restricted_taps` and `LegalActions::restricted_floating`.
//!
//! "Spend this mana only to cast a creature spell of the chosen type" is a
//! question about the spell (its types, its face, the source's chosen type)
//! that a seat cannot answer from its view, and a planner that taps a Cavern
//! of Souls for a spell its mana may not pay for has spent the land on
//! nothing. The engine answers it here, with the reader the payment uses
//! (`casting::restriction_admits`, `casting::admitted_by_colour`), for every
//! cast, activation and open payment this offer names, and for the asked
//! seat's own sources and pool alone.
//!
//! Nothing here is asked unless the seat has a restricted mana ability on
//! offer or restricted mana floating, which is most offers: the two lists
//! are then empty and cost one pass over the abilities.

use super::{CardLookup, Engine, PaymentContinuation};
use crate::casting::{self, SpellForm, SpendFor};
use crate::choice::{CastModeKind, LegalActions, Pending, Spend};
use baylee_cards_dsl::{AbilityDef, Effect, ManaRestriction};
use baylee_core::ids::{ObjectId, PlayerId};

impl<L: CardLookup> Engine<L> {
    /// The priority a CR 605.3a window opens with: `legal`, already narrowed
    /// to mana (`narrow_to_mana`), stating what restricted mana may pay
    /// toward the payment the window is open for. Every opener sets
    /// `mana_window` first and publishes through here, since the offer it
    /// narrowed was built before the window was there to ask.
    pub(super) fn window_priority(&self, player: PlayerId, mut legal: LegalActions) -> Pending {
        self.offer_restricted_mana(player, &mut legal);
        Pending::Priority {
            player,
            legal: Box::new(legal),
        }
    }

    /// Fills `legal`'s restricted lists for `player`, after everything else
    /// in it is decided (the casts, the hints, a window's narrowing).
    pub(super) fn offer_restricted_mana(&self, player: PlayerId, legal: &mut LegalActions) {
        legal.restricted_taps.clear();
        legal.restricted_floating.clear();
        let taps: Vec<(ObjectId, u32, ManaRestriction)> = legal
            .abilities
            .iter()
            .filter_map(|&(source, index)| {
                self.restricted_mana_ability(player, source, index)
                    .map(|restriction| (source, index, restriction))
            })
            .collect();
        let floating = !self.state.players[usize::from(player.get())]
            .mana_pool
            .restricted()
            .is_empty();
        if taps.is_empty() && !floating {
            return;
        }
        for (spend, what) in self.spends(player, legal) {
            for &(source, index, restriction) in &taps {
                if casting::restriction_admits(&self.state, player, source, restriction, what) {
                    legal.restricted_taps.push((source, index, spend));
                }
            }
            if floating {
                let by_colour = casting::admitted_by_colour(&self.state, player, what);
                if by_colour.iter().any(|n| *n > 0) {
                    legal.restricted_floating.push((spend, by_colour));
                }
            }
        }
    }

    /// The restriction the mana of `player`'s mana ability `(source, index)`
    /// carries, when it is spent only on some payments: a rider alone
    /// restricts nothing (#232).
    fn restricted_mana_ability(
        &self,
        player: PlayerId,
        source: ObjectId,
        index: u32,
    ) -> Option<ManaRestriction> {
        if self.state.object(source)?.controller != player || !self.is_mana_offer(source, index) {
            return None;
        }
        let (AbilityDef::Activated { effects, .. }
        | AbilityDef::ActivatedConditional { effects, .. }) =
            self.activation_definition(source, index)?
        else {
            return None;
        };
        let mut found = None;
        Effect::walk(effects, &mut 0, &mut |effect| {
            if let Effect::AddMana {
                restriction: Some(restriction),
                ..
            } = effect
                && restriction.restricts
                && found.is_none()
            {
                found = Some(*restriction);
            }
        });
        found
    }

    /// Every payment this offer names, each as the spend a seat asks about
    /// and as the payment the engine makes for it: each way of casting each
    /// card it lists (a face or a form apart, every other way as the
    /// ordinary cast, `Spend::cast`), each source of an activation whose
    /// mana is not floating yet, and the payment a window is open for.
    fn spends(&self, player: PlayerId, legal: &LegalActions) -> Vec<(Spend, SpendFor)> {
        let mut out: Vec<(Spend, SpendFor)> = Vec::new();
        let mut add = |spend: Spend, what: SpendFor| {
            if !out.iter().any(|(s, _)| *s == spend) {
                out.push((spend, what));
            }
        };
        for &card in legal.castable.iter().chain(&legal.payable) {
            let ways = legal
                .cast_ways
                .iter()
                .filter(|(id, _)| *id == card)
                .map(|(_, way)| way.kind);
            for kind in std::iter::once(CastModeKind::Normal).chain(ways) {
                if let Some(what) = self.cast_spend(card, kind) {
                    add(Spend::cast(card, kind), what);
                }
            }
        }
        for &(source, ..) in &legal.unpaid_abilities {
            add(Spend::Activate(source), SpendFor::Ability(source));
        }
        if let Some(what) = self.window_spend(player) {
            add(Spend::Owed, what);
        }
        out
    }

    /// The payment casting `card` the way `kind` makes, as the cast wizard
    /// names it (`cast_wizard::spend_for`); `None` for playing a land face.
    fn cast_spend(&self, card: ObjectId, kind: CastModeKind) -> Option<SpendFor> {
        let face = || {
            self.state
                .object(card)
                .and_then(|o| o.card)
                .and_then(|c| self.lookup.card(c.index))
                .and_then(|def| def.faces.first())
        };
        match Spend::cast(card, kind) {
            Spend::Cast(_, CastModeKind::Face(i)) => Some(SpendFor::SpellOnFace(card, i)),
            Spend::Cast(_, CastModeKind::Prototype) => Some(SpendFor::SpellAs(
                card,
                SpellForm::Prototype(face()?.prototype?),
            )),
            Spend::Cast(_, CastModeKind::Disguise) => {
                Some(SpendFor::SpellAs(card, SpellForm::Disguise))
            }
            Spend::Cast(_, CastModeKind::Foretell) => Some(SpendFor::SpellAs(
                card,
                SpellForm::Foretell(face()?.foretell?),
            )),
            // Playing a land face is no cast (CR 305.1), and costs nothing.
            _ if matches!(kind, CastModeKind::PlayLandFace(_)) => None,
            _ => Some(SpendFor::Spell(card)),
        }
    }

    /// The payment `player`'s open CR 605.3a window is for, as its
    /// continuation pays it.
    fn window_spend(&self, player: PlayerId) -> Option<SpendFor> {
        let window = self.mana_window.as_ref().filter(|w| w.player == player)?;
        Some(match &window.suspended {
            PaymentContinuation::Miracle { wizard, .. } => {
                let face = self
                    .state
                    .object(wizard.card)
                    .and_then(|o| o.card)
                    .and_then(|c| self.lookup.card(c.index))
                    .and_then(|def| def.faces.first())?;
                super::cast_wizard::spend_for(wizard, face)
            }
            PaymentContinuation::Cast { card, .. } => SpendFor::Spell(*card),
            PaymentContinuation::Activation(payment) => SpendFor::Ability(payment.source()),
            PaymentContinuation::LandMana(_)
            | PaymentContinuation::Tax(_)
            | PaymentContinuation::GrantedAction { .. }
            | PaymentContinuation::AttackTax { .. }
            | PaymentContinuation::BlockTax { .. } => SpendFor::Other,
        })
    }
}

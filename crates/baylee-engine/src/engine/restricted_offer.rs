//! What restricted mana may pay for in an offer (CR 106.6):
//! `LegalActions::restricted_taps` and `LegalActions::restricted_floating`.
//!
//! "Spend this mana only to cast a creature spell of the chosen type" is a
//! question about the spell (its types, its face, the source's chosen type)
//! that a seat cannot answer from its view, and a planner that taps a Cavern
//! of Souls for a spell its mana may not pay for has spent the land on
//! nothing. The engine answers it here, with the reader the payment uses
//! (`casting::provenance_allows`), for every cast, activation and open
//! payment this offer names, and for the asked seat's own sources and pool
//! alone: by provenance class, so a board of Powerstones is asked as one.
//!
//! Nothing here is asked unless the seat has a restricted mana ability on
//! offer or restricted mana floating, which is most offers: the two lists
//! are then empty and cost one pass over the abilities.

use super::{CardLookup, Engine, PaymentContinuation};
use crate::casting::{self, SpellForm, SpendFor};
use crate::choice::{
    CastModeKind, LegalActions, Pending, RESTRICTED_ENTRIES, RestrictedTaps, Spend,
};
use crate::mana_provenance::{ManaProvenance, ProvenanceClass};
use baylee_cards_dsl::{AbilityDef, Effect, ManaRestriction};
use baylee_core::ids::{ObjectId, PlayerId};
use std::collections::{HashMap, HashSet};

/// The most times one offer asks whether a class of restricted mana may pay
/// a spend: sixteen times [`RESTRICTED_ENTRIES`]. Classes are few on any
/// board that is played, but a source whose restriction reads the source
/// itself is a class of its own, and a thousand of them beside a thousand
/// spells would be asked a million times.
pub(super) const RESTRICTED_ASKS: usize = 16 * RESTRICTED_ENTRIES;

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
    ///
    /// Linear in the board: the restricted taps are grouped into classes
    /// whose mana is admitted alike (`ManaProvenance::class`; every
    /// Powerstone is one), and so is the restricted mana floating, and each
    /// spend asks each class once, with the spend's object built once. What
    /// is named stops at [`RESTRICTED_ENTRIES`], and the asking at
    /// [`RESTRICTED_ASKS`] (a board of sources that each make a class of
    /// their own): a cut offers less, never more, and the payment asks
    /// again.
    pub(super) fn offer_restricted_mana(&self, player: PlayerId, legal: &mut LegalActions) {
        legal.restricted_taps.clear();
        legal.restricted_floating.clear();
        let mut classes: Vec<(ManaProvenance, RestrictedTaps)> = Vec::new();
        let mut class_of: HashMap<ProvenanceClass, usize> = HashMap::new();
        let mut budget = RESTRICTED_ENTRIES;
        for &(source, index) in &legal.abilities {
            let Some(restriction) = self.restricted_mana_ability(player, source, index) else {
                continue;
            };
            if budget == 0 {
                break;
            }
            let provenance =
                casting::restricted_provenance(&self.state, player, source, restriction);
            let at = *class_of.entry(provenance.class()).or_insert_with(|| {
                classes.push((provenance, RestrictedTaps::default()));
                classes.len() - 1
            });
            classes[at].1.taps.push((source, index));
            budget -= 1;
        }
        let floating = casting::floating_classes(&self.state, player);
        if classes.is_empty() && floating.is_empty() {
            return;
        }
        let mut asks = RESTRICTED_ASKS;
        'spends: for (spend, what) in self.spends(player, legal) {
            let spell = casting::spending_object(&self.state, what);
            for (provenance, class) in &mut classes {
                if budget == 0 {
                    break;
                }
                let Some(left) = asks.checked_sub(1) else {
                    break 'spends;
                };
                asks = left;
                if casting::provenance_allows(&self.state, provenance, what, spell.as_deref()) {
                    class.spends.push(spend);
                    budget -= 1;
                }
            }
            if !floating.is_empty() && legal.restricted_floating.len() < RESTRICTED_ENTRIES {
                let mut by_colour = [0_u32; 6];
                for (provenance, amounts) in &floating {
                    let Some(left) = asks.checked_sub(1) else {
                        break 'spends;
                    };
                    asks = left;
                    if casting::provenance_allows(&self.state, provenance, what, spell.as_deref()) {
                        for (sum, add) in by_colour.iter_mut().zip(amounts) {
                            *sum = sum.saturating_add(*add);
                        }
                    }
                }
                if by_colour.iter().any(|n| *n > 0) {
                    legal.restricted_floating.push((spend, by_colour));
                }
            }
        }
        legal.restricted_taps = classes
            .into_iter()
            .map(|(_, class)| class)
            .filter(|class| !class.spends.is_empty())
            .collect();
    }

    /// The restriction the mana of `player`'s mana ability `(source, index)`
    /// carries, when it is spent only on some payments: a rider alone
    /// restricts nothing (#232).
    pub(super) fn restricted_mana_ability(
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
    /// mana is not floating yet, and the payment a window is open for. Each
    /// spend once, in the offer's order, in time linear in the offer.
    fn spends(&self, player: PlayerId, legal: &LegalActions) -> Vec<(Spend, SpendFor)> {
        let mut ways: HashMap<ObjectId, Vec<CastModeKind>> = HashMap::new();
        for (card, way) in &legal.cast_ways {
            ways.entry(*card).or_default().push(way.kind);
        }
        let mut seen: HashSet<Spend> = HashSet::new();
        let mut out: Vec<(Spend, SpendFor)> = Vec::new();
        let mut add = |spend: Spend, what: SpendFor| {
            if seen.insert(spend) {
                out.push((spend, what));
            }
        };
        for &card in legal.castable.iter().chain(&legal.payable) {
            let kinds = ways.get(&card).map_or(&[][..], Vec::as_slice);
            for &kind in std::iter::once(&CastModeKind::Normal).chain(kinds) {
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

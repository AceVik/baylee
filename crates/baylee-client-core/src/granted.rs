//! Explicit, stable-ID drafts for temporary player actions.
use baylee_core::ids::GrantedActionId;
use baylee_engine::choice::{GrantedActionOffer, PlayerAction};

/// A menu cursor and a separately confirmed action; never an automatic payment.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Draft {
    /// Whether the player opened the menu.
    pub open: bool,
    /// The focused offer, preserved across reordering.
    pub focused: Option<GrantedActionId>,
    /// Exact offer explicitly selected by the player.
    pub selected: Option<GrantedActionOffer>,
}
impl Draft {
    /// Opens on the first offer without selecting or spending anything.
    pub fn show(&mut self, offers: &[GrantedActionOffer]) {
        self.open = !offers.is_empty();
        self.focused = offers.first().map(|o| o.id);
        self.selected = None;
    }
    /// Withdraws stale drafts, including changed costs or recipients.
    pub fn sync(&mut self, offers: &[GrantedActionOffer]) {
        if offers.is_empty() {
            *self = Self::default();
            return;
        }
        if self
            .selected
            .as_ref()
            .is_some_and(|old| !offers.contains(old))
        {
            self.selected = None;
        }
        if !offers.iter().any(|o| Some(o.id) == self.focused) {
            self.focused = offers.first().map(|o| o.id);
        }
    }
    /// Selects one current offer, without submitting it.
    pub fn select(&mut self, id: GrantedActionId, offers: &[GrantedActionOffer]) {
        self.sync(offers);
        if self.open {
            self.selected = offers.iter().find(|o| o.id == id).cloned();
            self.focused = self.selected.as_ref().map(|o| o.id);
        }
    }
    /// Confirms exactly the displayed offer once, rechecking current legality.
    pub fn confirm(&mut self, offers: &[GrantedActionOffer]) -> Option<PlayerAction> {
        self.sync(offers);
        let selected = self.selected.take()?;
        self.open = false;
        Some(PlayerAction::TakeGrantedAction { id: selected.id })
    }
}

/// Describe cancellation of one obligation, including its strict payment deadline.
#[must_use]
pub fn cancellation_label(
    effect: baylee_engine::choice::GrantedActionKind,
    lang: crate::i18n::Lang,
) -> Option<String> {
    let baylee_engine::choice::GrantedActionKind::CancelDelayedLifeLoss { amount } = effect else {
        return None;
    };
    Some(crate::i18n::Phrase::GrantedCancelLifeLoss.fill(lang, &[&amount.to_string()]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_cards_dsl::{SpecialActionCost, SpecialActionTiming};
    use baylee_core::{
        ids::{DamageSourceRef, ObjectId},
        mana::ManaColor,
    };
    use baylee_engine::choice::GrantedActionKind;
    fn offer(id: u64) -> GrantedActionOffer {
        GrantedActionOffer {
            id: GrantedActionId::new(id),
            source: DamageSourceRef {
                object: ObjectId::new(1, 0),
                version: 0,
            },
            ability: None,
            timing: SpecialActionTiming::ManaAbility,
            cost: SpecialActionCost::Life(1),
            effect: GrantedActionKind::AddMana {
                color: ManaColor::Colorless,
                amount: 1,
            },
        }
    }
    #[test]
    fn opening_focuses_but_never_pays_and_confirmation_is_one_shot() {
        let offers = [offer(1)];
        let mut draft = Draft::default();
        draft.show(&offers);
        assert_eq!(draft.focused, Some(offers[0].id));
        assert_eq!(draft.confirm(&offers), None);
        draft.select(offers[0].id, &offers);
        assert_eq!(
            draft.confirm(&offers),
            Some(PlayerAction::TakeGrantedAction { id: offers[0].id })
        );
        assert_eq!(draft.confirm(&offers), None);
    }
    #[test]
    fn reordering_preserves_identity_but_changed_or_expired_offers_clear_selection() {
        let offers = [offer(1), offer(2)];
        let mut draft = Draft::default();
        draft.show(&offers);
        draft.select(offers[1].id, &offers);
        draft.sync(&[offers[1].clone(), offers[0].clone()]);
        assert_eq!(draft.focused, Some(offers[1].id));
        let mut changed = offers[1].clone();
        changed.cost = SpecialActionCost::Life(2);
        assert_eq!(draft.confirm(&[changed]), None);
        draft.sync(&[]);
        assert!(!draft.open);
    }

    fn debt(id: u64) -> GrantedActionOffer {
        let mut debt = offer(id);
        debt.timing = SpecialActionTiming::Priority;
        debt.cost = SpecialActionCost::Mana(baylee_core::mana::ManaCost::from_symbol_generic(1));
        debt.effect = GrantedActionKind::CancelDelayedLifeLoss { amount: 1024 };
        debt
    }
    #[test]
    fn identical_debts_keep_independent_ids_and_expired_selection_cannot_pay_another() {
        let offers = [debt(41), debt(42)];
        let mut draft = Draft::default();
        draft.show(&offers);
        draft.select(offers[1].id, &offers);
        assert_eq!(
            draft.confirm(&[offers[1].clone(), offers[0].clone()]),
            Some(PlayerAction::TakeGrantedAction { id: offers[1].id })
        );
        draft.show(&offers);
        draft.select(offers[1].id, &offers);
        assert_eq!(draft.confirm(&offers[..1]), None);
        draft.select(offers[0].id, &offers[..1]);
        assert_eq!(
            draft.confirm(&offers[..1]),
            Some(PlayerAction::TakeGrantedAction { id: offers[0].id })
        );
        draft.show(&offers);
        draft.select(offers[0].id, &offers);
        let mut changed = offers[0].clone();
        changed.effect = GrantedActionKind::CancelDelayedLifeLoss { amount: 1 };
        assert_eq!(draft.confirm(&[changed]), None);
    }
    #[test]
    fn cancellation_label_names_life_loss_and_deadline_in_each_language() {
        let effect = debt(1).effect;
        assert_eq!(
            cancellation_label(effect, crate::i18n::Lang::En).unwrap(),
            "Cancel this pending 1024 life loss before your next draw step"
        );
        assert_eq!(
            cancellation_label(effect, crate::i18n::Lang::De).unwrap(),
            "Diesen bevorstehenden Verlust von 1024 Lebenspunkten vor deinem nächsten Ziehsegment aufheben"
        );
        assert!(cancellation_label(offer(1).effect, crate::i18n::Lang::En).is_none());
    }
    #[test]
    fn priority_cancellation_offers_roundtrip_without_collapsing_obligation_ids() {
        use baylee_engine::choice::{LegalActions, Pending};
        let pending = Pending::Priority {
            player: baylee_core::ids::PlayerId::new(0),
            legal: Box::new(LegalActions {
                granted_actions: vec![debt(51), debt(52)],
                can_pass: true,
                ..LegalActions::default()
            }),
        };
        let json = serde_json::to_string(&pending).unwrap();
        assert!(json.contains("CancelDelayedLifeLoss"));
        let decoded: Pending = serde_json::from_str(&json).unwrap();
        let Pending::Priority { legal, .. } = decoded else {
            panic!("priority roundtrip")
        };
        assert_eq!(legal.granted_actions, vec![debt(51), debt(52)]);
        let unknown = json.replace("CancelDelayedLifeLoss", "UnknownDebtAction");
        assert!(serde_json::from_str::<Pending>(&unknown).is_err());
    }
}

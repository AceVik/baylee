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
}

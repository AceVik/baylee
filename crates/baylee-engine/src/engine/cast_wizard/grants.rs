//! Casting permissions consumed only by a successfully completed cast.
use super::{
    CardLookup, CastModeKind, CastWizard, Engine, EngineError, ObjectId, PlayerId, WizardStage,
};
use baylee_cards_dsl::Modifier;

impl<L: CardLookup> Engine<L> {
    /// Ask after mana abilities, so their sacrificed cards are available too.
    pub(super) fn ask_escape_payment(&mut self, wizard: &CastWizard) -> Result<bool, EngineError> {
        let Some(count) = wizard.escape_count(self.wizard_face(wizard)) else {
            return Ok(false);
        };
        if wizard.escape_exiles.len() == usize::from(count) {
            return Ok(false);
        }
        let mut options =
            crate::casting::escape_exile_options(&self.state, wizard.player, wizard.card);
        options.retain(|card| !wizard.delve_exiles.contains(card));
        let mut cost_cards = wizard.cost_objects.to_vec();
        if self
            .wizard_face(wizard)
            .mandatory_additional_costs
            .contains(&baylee_cards_dsl::CostPart::DiscardHand)
        {
            cost_cards.extend(
                self.state
                    .zones
                    .list(crate::zone::ZoneLocation::Hand(wizard.player))
                    .iter()
                    .copied()
                    .filter(|card| *card != wizard.card),
            );
        }
        for &card in &cost_cards {
            if crate::casting::additional_card_enters_graveyard(&self.state, wizard.player, card)
                && !options.contains(&card)
            {
                options.push(card);
            }
        }
        if options.len() < usize::from(count) {
            return Err(EngineError::IllegalAction(
                "not enough other cards in the graveyard to escape",
            ));
        }
        let mut wizard = wizard.clone();
        wizard.stage = WizardStage::Escape;
        self.pending = crate::choice::Pending::ChooseCards {
            player: wizard.player,
            options,
            min: count,
            max: count,
            prompt: crate::choice::ChoicePrompt::CostExile,
            total: None,
        };
        self.cast_wizard = Some(wizard);
        self.awaiting_answer = true;
        Ok(true)
    }

    pub(super) fn choose_graveyard_permission(
        &mut self,
        mut wizard: CastWizard,
    ) -> Result<(), EngineError> {
        use crate::casting::GraveyardPermission;
        let own_cost_permission = wizard.escapes()
            || matches!(
                wizard.option,
                Some(CastModeKind::Flashback | CastModeKind::BeamMeUp)
            )
            || matches!(wizard.option, Some(CastModeKind::Face(_)))
                && self.wizard_face(&wizard).disturb;
        let in_graveyard = self
            .state
            .object(wizard.card)
            .is_some_and(|object| object.zone == crate::zone::Zone::Graveyard);
        if in_graveyard
            && !wizard.free
            && wizard.by_effect.is_none()
            && !own_cost_permission
            && crate::casting::play_permission(&self.state, wizard.player, wizard.card).is_none()
        {
            let face = match wizard.option {
                Some(CastModeKind::Face(face)) => face,
                _ => 0,
            };
            wizard.permissions = crate::casting::graveyard_face_permissions(
                &self.state,
                wizard.player,
                wizard.card,
                face,
            );
            let def = self
                .state
                .object(wizard.card)
                .and_then(|object| object.card)
                .and_then(|card| self.lookup.card(card.index))
                .expect("wizard card known");
            if face == 0
                && wizard
                    .option
                    .is_some_and(|option| crate::casting::mana_cost_mode(def, face, option))
                && crate::casting::flashback_granted(&self.state, wizard.card)
            {
                wizard.permissions.push(GraveyardPermission::Flashback);
            }
            match wizard.permissions.as_slice() {
                [] => {
                    return Err(EngineError::IllegalAction(
                        "no permission for this spell face and cost",
                    ));
                }
                [only] => wizard.permission = Some(*only),
                permissions => {
                    self.pending = crate::choice::Pending::ChooseCastMode {
                        player: wizard.player,
                        object: wizard.card,
                        options: permissions
                            .iter()
                            .enumerate()
                            .map(|(index, permission)| {
                                let source = match *permission {
                                    GraveyardPermission::Unlimited => None,
                                    GraveyardPermission::EachType { source, .. } => Some(source),
                                    GraveyardPermission::Once { source, .. } => source,
                                    GraveyardPermission::Flashback => Some(wizard.card),
                                };
                                crate::choice::CastModeDesc {
                                    index: index as u32,
                                    kind: CastModeKind::Permission(source),
                                    cost: baylee_core::mana::ManaCost::ZERO,
                                }
                            })
                            .collect(),
                    };
                    self.cast_wizard = Some(wizard);
                    self.awaiting_answer = true;
                    return Ok(());
                }
            }
        }
        wizard.stage = WizardStage::ManaPermission;
        self.cast_wizard = Some(wizard);
        self.advance_cast_wizard()
    }

    pub(super) fn apply_graveyard_cast_grants(&mut self, wizard: &CastWizard) {
        use crate::casting::GraveyardPermission;
        let Some(permission) = wizard.permission else {
            return;
        };
        let card = wizard.card;
        let types = self
            .state
            .object(card)
            .expect("cast spell exists")
            .characteristics()
            .types;
        crate::casting::note_graveyard_play(
            &mut self.state,
            wizard.player,
            Some(permission),
            types,
        );
        match permission {
            GraveyardPermission::Once { vampire: false, .. } => {
                self.state
                    .object_mut(card)
                    .expect("cast spell exists")
                    .riders
                    .push(crate::object::Rider::ExileInsteadOfGraveyard);
            }
            GraveyardPermission::Once { vampire: true, .. } => {
                self.state
                    .object_mut(card)
                    .expect("cast spell exists")
                    .riders
                    .push(crate::object::Rider::EntryFinality);
                let filter = crate::effects::EffectFilter::object(&self.state, card);
                let timestamp = self.state.next_timestamp();
                self.state
                    .effects
                    .register(crate::effects::ContinuousEffect {
                        id: baylee_core::ids::EffectId::new(0),
                        source: None,
                        controller: wizard.player,
                        origin: crate::effects::EffectOrigin::Resolution,
                        layer: baylee_cards_dsl::Layer::Type,
                        timestamp,
                        duration: baylee_cards_dsl::Duration::Indefinitely,
                        filter,
                        modifier: Modifier::AddSubtype(
                            baylee_core::generated::subtypes::creature::VAMPIRE,
                        ),
                    });
            }
            GraveyardPermission::Flashback
            | GraveyardPermission::Unlimited
            | GraveyardPermission::EachType { .. } => {}
        }
    }

    pub(super) fn apply_next_spell_grants(&mut self, player: PlayerId, card: ObjectId) {
        let applies = |effect: &crate::effects::ContinuousEffect| {
            effect.controller == player && effect.modifier == Modifier::NextSpellUncounterable
        };
        if !self.state.effects.iter().any(applies) {
            return;
        }
        self.state.effects.remove_where(applies);
        if let Some(object) = self.state.object_mut(card) {
            object.riders.push(crate::object::Rider::Uncounterable);
        }
    }
}

//! Immediate free plays include land faces, spell faces and additional costs.
#[allow(clippy::wildcard_imports)]
use super::*;

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    pub(crate) fn start_linked_free_play(
        &mut self,
        player: PlayerId,
        card: ObjectId,
    ) -> Result<(), EngineError> {
        self.start_immediate_free_play(player, card, false)
    }

    /// Immediate free casts can forbid land plays (Mad Wizard's Lair).
    pub(crate) fn start_immediate_free_play(
        &mut self,
        player: PlayerId,
        card: ObjectId,
        cast_only: bool,
    ) -> Result<(), EngineError> {
        let options = self.immediate_free_options(player, card, cast_only)?;
        self.start_free_cast_options(player, card, options, true, None)
    }

    pub(in crate::engine) fn immediate_free_options(
        &self,
        player: PlayerId,
        card: ObjectId,
        cast_only: bool,
    ) -> Result<Vec<CastModeDesc>, EngineError> {
        let def = self
            .state
            .object(card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
            .ok_or(EngineError::IllegalAction("linked card is gone"))?;
        let make_mana = self.free_cast_makes_mana(player);
        let mut options = Vec::new();
        if self.linked_spell_face_allowed(player, card, def, 0) {
            if casting::modes_are_the_only_way(def, 0) {
                options = self.free_modes_with_mana(player, card, def, make_mana);
            } else if casting::face_has_a_legal_target(&self.state, &self.lookup, player, card, 0) {
                let cost = self.front_spell_price(player, card, ManaCost::ZERO);
                if make_mana || self.free_cost_affordable(player, card, cost) {
                    options.push(CastModeDesc {
                        index: 0,
                        kind: CastModeKind::Normal,
                        cost,
                    });
                }
            }
        }
        for (face, _) in casting::castable_back_faces(def, false) {
            if !self.linked_spell_face_allowed(player, card, def, face)
                || !casting::face_has_a_legal_target(&self.state, &self.lookup, player, card, face)
            {
                continue;
            }
            let cost = casting::face_adjustment(&self.state, player, card, def, face, None).price(
                ManaCost::ZERO,
                casting::printed_reduction(&self.state, &def.faces[face], player, card),
            );
            let restricted = casting::planning_pool(
                &self.state,
                player,
                casting::SpendFor::SpellOnFace(card, face),
                0,
            );
            let pool = restricted
                .as_ref()
                .unwrap_or(&self.state.players[usize::from(player.get())].mana_pool);
            if make_mana
                || casting::source_affordable(
                    &self.state,
                    player,
                    casting::SpendFor::SpellOnFace(card, face),
                    pool,
                    &cost,
                    &[],
                    None,
                )
            {
                options.push(CastModeDesc {
                    index: 0,
                    kind: CastModeKind::Face(face),
                    cost,
                });
            }
        }
        if !cast_only
            && self
                .state
                .object(card)
                .is_some_and(crate::object::GameObject::is_card)
            && self.state.turn.active == player
            && casting::has_a_land_drop_left(&self.state, player)
        {
            options.extend(def.land_faces_from_hand().map(|face| CastModeDesc {
                index: 0,
                kind: CastModeKind::PlayLandFace(face),
                cost: ManaCost::ZERO,
            }));
        }
        for (index, option) in options.iter_mut().enumerate() {
            option.index = u32::try_from(index).expect("bounded cast options");
        }
        Ok(options)
    }

    pub(super) fn linked_spell_face_allowed(
        &self,
        player: PlayerId,
        card: ObjectId,
        def: &baylee_cards_dsl::CardDef,
        face: usize,
    ) -> bool {
        let Some(object) = self.state.object(card) else {
            return false;
        };
        if def.faces[face]
            .types
            .contains(baylee_core::types::TypeSet::LAND)
            || !casting::may_begin_casting(&self.state, player)
            // This permission bypasses timing derived from card types, but
            // not Teferi's explicit restriction. A parent is still resolving,
            // so this is never a time its opponent could cast a sorcery.
            || self.state.effects.iter().any(|effect| {
                effect.modifier == baylee_cards_dsl::Modifier::OpponentsCastAsSorcery
                    && self.state.is_opponent(effect.controller, player)
            })
            || !casting::spell_condition_allows(&self.state, player, card, def, face)
            || casting::additional_payment_capacity(
                &self.state,
                player,
                card,
                &def.faces[face],
                &[],
            )
            .is_none()
        {
            return false;
        }
        // A restriction sees the chosen spell face, not the card's front face.
        !casting::face_is_forbidden(&self.state, player, object, def, face)
    }

    pub(in crate::engine) fn effect_play_options(
        &self,
        player: PlayerId,
        card: ObjectId,
        cost_mode: baylee_cards_dsl::PlayCostMode,
        cast_only: bool,
    ) -> Result<Vec<CastModeDesc>, EngineError> {
        match cost_mode {
            baylee_cards_dsl::PlayCostMode::WithoutPayingManaCost => {
                self.immediate_free_options(player, card, cast_only)
            }
            baylee_cards_dsl::PlayCostMode::Normal => {
                self.immediate_paid_options(player, card, None, false, cast_only)
            }
            baylee_cards_dsl::PlayCostMode::AnyManaColor => {
                self.immediate_paid_options(player, card, None, true, cast_only)
            }
            baylee_cards_dsl::PlayCostMode::Alternative(alt) => {
                self.immediate_paid_options(player, card, Some(alt), false, cast_only)
            }
        }
    }

    pub(in crate::engine) fn start_effect_play_card(
        &mut self,
        player: PlayerId,
        card: ObjectId,
        cost_mode: baylee_cards_dsl::PlayCostMode,
        cast_only: bool,
        exile_instead_of_graveyard: bool,
    ) -> Result<(), EngineError> {
        let options = self.effect_play_options(player, card, cost_mode, cast_only)?;
        let (option, stage) = match options.as_slice() {
            [] => return Err(EngineError::IllegalAction("no legal play")),
            [only] => (Some(only.kind), WizardStage::XValue),
            _ => (None, WizardStage::ChooseMode),
        };
        let (free, override_mana_cost, effect_cast) = match cost_mode {
            baylee_cards_dsl::PlayCostMode::WithoutPayingManaCost => {
                (true, None, EffectCast::Plain)
            }
            baylee_cards_dsl::PlayCostMode::Normal => (false, None, EffectCast::Plain),
            baylee_cards_dsl::PlayCostMode::AnyManaColor => (false, None, EffectCast::AnyMana),
            baylee_cards_dsl::PlayCostMode::Alternative(alt) => {
                (false, Some(alt), EffectCast::Plain)
            }
        };
        let wizard = Box::new(CastWizard {
            option,
            free,
            override_mana_cost,
            exile_instead_of_graveyard,
            by_effect: Some(effect_cast),
            ..CastWizard::new(card, player, stage, options)
        });
        self.cast_wizard = Some(wizard);
        self.advance_cast_wizard()
    }

    fn immediate_paid_options(
        &self,
        player: PlayerId,
        card: ObjectId,
        override_cost: Option<ManaCost>,
        any_mana: bool,
        cast_only: bool,
    ) -> Result<Vec<CastModeDesc>, EngineError> {
        let def = self
            .state
            .object(card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
            .ok_or(EngineError::IllegalAction("card is gone"))?;
        let make_mana = self.free_cast_makes_mana(player);
        let mut options = Vec::new();
        if self.linked_spell_face_allowed(player, card, def, 0) {
            let base = override_cost.unwrap_or(def.faces[0].mana_cost);
            if casting::modes_are_the_only_way(def, 0) {
                options = self.modes_with_base(player, card, def, base, make_mana || any_mana);
                if !make_mana && any_mana {
                    options.retain(|opt| {
                        self.immediate_paid_cost_affordable(player, card, 0, opt.cost, true)
                    });
                }
            } else if casting::face_has_a_legal_target(&self.state, &self.lookup, player, card, 0) {
                let cost = self.front_spell_price(player, card, base);
                if make_mana
                    || self.immediate_paid_cost_affordable(
                        player,
                        card,
                        0,
                        cost.with_x(0),
                        any_mana,
                    )
                {
                    options.push(CastModeDesc {
                        index: 0,
                        kind: CastModeKind::Normal,
                        cost,
                    });
                }
            }
        }
        for (face, _) in casting::castable_back_faces(def, false) {
            if !self.linked_spell_face_allowed(player, card, def, face)
                || !casting::face_has_a_legal_target(&self.state, &self.lookup, player, card, face)
            {
                continue;
            }
            let base = override_cost.unwrap_or(def.faces[face].mana_cost);
            let cost = casting::face_adjustment(&self.state, player, card, def, face, None).price(
                base,
                casting::printed_reduction(&self.state, &def.faces[face], player, card),
            );
            if make_mana
                || self.immediate_paid_cost_affordable(player, card, face, cost.with_x(0), any_mana)
            {
                options.push(CastModeDesc {
                    index: 0,
                    kind: CastModeKind::Face(face),
                    cost,
                });
            }
        }
        if !cast_only
            && override_cost.is_none()
            && self
                .state
                .object(card)
                .is_some_and(crate::object::GameObject::is_card)
            && self.state.turn.active == player
            && casting::has_a_land_drop_left(&self.state, player)
        {
            options.extend(def.land_faces_from_hand().map(|face| CastModeDesc {
                index: 0,
                kind: CastModeKind::PlayLandFace(face),
                cost: ManaCost::ZERO,
            }));
        }
        for (index, option) in options.iter_mut().enumerate() {
            option.index = u32::try_from(index).expect("bounded cast options");
        }
        Ok(options)
    }

    fn immediate_paid_cost_affordable(
        &self,
        player: PlayerId,
        card: ObjectId,
        face: usize,
        cost: ManaCost,
        any_mana: bool,
    ) -> bool {
        let what = if face == 0 {
            casting::SpendFor::Spell(card)
        } else {
            casting::SpendFor::SpellOnFace(card, face)
        };
        let restricted = casting::planning_pool(&self.state, player, what, 0);
        let pool = restricted
            .as_ref()
            .unwrap_or(&self.state.players[usize::from(player.get())].mana_pool);
        if any_mana {
            let allowance = casting::base_allowance(&self.state, what);
            casting::component_affordable(
                &self.state,
                player,
                what,
                pool,
                &cost,
                &cost,
                &[],
                allowance,
            )
        } else {
            casting::source_affordable(&self.state, player, what, pool, &cost, &[], None)
        }
    }
}

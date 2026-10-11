//! Immediate free plays include land faces, spell faces and additional costs.
#[allow(clippy::wildcard_imports)]
use super::*;

impl<L: CardLookup> Engine<L> {
    pub(crate) fn start_linked_free_play(
        &mut self,
        player: PlayerId,
        card: ObjectId,
    ) -> Result<(), EngineError> {
        let def = self
            .state
            .object(card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
            .ok_or(EngineError::IllegalAction("linked card is gone"))?;
        let make_mana = self.compute_legal(player).has_mana_source();
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
            if make_mana || casting::spell_affordable(&self.state, player, card, pool, &cost) {
                options.push(CastModeDesc {
                    index: 0,
                    kind: CastModeKind::Face(face),
                    cost,
                });
            }
        }
        if self
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
        self.start_free_cast_options(player, card, options, true)
    }

    fn linked_spell_face_allowed(
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
            || additional_card_costs(&def.faces[face]).any(|part| {
                crate::engine::cost_wizard::options(&self.state, player, card, part).is_empty()
            })
        {
            return false;
        }
        // A restriction sees the chosen spell face, not the card's front face.
        let mut projected = object.clone();
        projected.base = std::sync::Arc::new(crate::object::Characteristics::from_face(
            def,
            face,
            object.base.name,
        ));
        projected.cache.clear();
        projected.face_index = u8::try_from(face).expect("bounded faces");
        !casting::cast_is_forbidden(&self.state, player, &projected)
    }
}

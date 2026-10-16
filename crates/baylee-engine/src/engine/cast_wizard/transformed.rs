//! A resolving instruction can cast only the physical card's transforming back.
#[cfg(test)]
mod tests;
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::ids::DamageSourceRef;

impl<L: CardLookup> Engine<L> {
    pub(crate) fn start_transformed_free_cast(
        &mut self,
        player: PlayerId,
        card: DamageSourceRef,
    ) -> Result<(), EngineError> {
        let object = self
            .state
            .object(card.object)
            .filter(|o| {
                o.version == card.version && o.zone == crate::zone::Zone::Exile && o.is_card()
            })
            .ok_or(EngineError::IllegalAction("the defeated card left exile"))?;
        let def = object
            .card
            .and_then(|c| self.lookup.card(c.index))
            .filter(|d| d.faces.len() == 2 && !d.faces[1].castable_from_hand)
            .ok_or(EngineError::IllegalAction(
                "the card has no transforming back face",
            ))?;
        if !self.linked_spell_face_allowed(player, card.object, def, 1) {
            return Err(EngineError::IllegalAction(
                "the transformed spell cannot be cast",
            ));
        }
        let make_mana = self.compute_legal(player).has_mana_source();
        let adjustment = casting::face_adjustment(&self.state, player, card.object, def, 1, None);
        let reduction = casting::printed_reduction(&self.state, &def.faces[1], player, card.object);
        let base = adjustment.price(ManaCost::ZERO, reduction);
        let affordable = |cost: &ManaCost| {
            let restricted = casting::planning_pool(
                &self.state,
                player,
                casting::SpendFor::SpellOnFace(card.object, 1),
                0,
            );
            let pool = restricted
                .as_ref()
                .unwrap_or(&self.state.players[usize::from(player.get())].mana_pool);
            make_mana
                || casting::source_affordable(
                    &self.state,
                    player,
                    casting::SpendFor::SpellOnFace(card.object, 1),
                    pool,
                    cost,
                    &[],
                    None,
                )
        };
        let mut options = Vec::new();
        for ability in def.abilities_for_face(1) {
            let AbilityDef::ModalSpell { modes, choose } = ability else {
                continue;
            };
            for set in casting::mode_sets(modes, *choose) {
                let cost = casting::mode_set_cost(base, modes, set);
                let wall_only = crate::eval::target_domain::modes_wall_only(modes, set);
                if affordable(&cost)
                    && casting::chosen_modes(modes, set).all(|(_, mode)| {
                        mode.cost_override.is_none()
                            && [mode.targets, mode.second_targets].into_iter().all(|req| {
                                casting::requirement_is_reachable_for(
                                    req,
                                    &self.state,
                                    player,
                                    card.object,
                                    Some((def, 1)),
                                    wall_only,
                                )
                            })
                    })
                {
                    options.push(CastModeDesc {
                        index: u32::try_from(options.len()).expect("bounded modes"),
                        kind: if choose.is_one() {
                            CastModeKind::Mode(set.trailing_zeros() as usize)
                        } else {
                            CastModeKind::Modes(set)
                        },
                        cost,
                    });
                }
            }
        }
        if !casting::modes_are_the_only_way(def, 1)
            && affordable(&base)
            && casting::face_has_a_legal_target(&self.state, &self.lookup, player, card.object, 1)
        {
            options.push(CastModeDesc {
                index: u32::try_from(options.len()).expect("bounded modes"),
                kind: CastModeKind::Face(1),
                cost: base,
            });
        }
        self.start_free_cast_options(player, card.object, options, true, Some(1))
    }
}

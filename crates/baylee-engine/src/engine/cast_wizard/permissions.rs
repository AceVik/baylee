//! Choices for a priority-time permission that waives a card's mana cost.
#[allow(clippy::wildcard_imports)]
use super::*;

impl<L: CardLookup> Engine<L> {
    pub(super) fn permitted_free_options(
        &self,
        player: PlayerId,
        card: ObjectId,
        def: &baylee_cards_dsl::CardDef,
    ) -> Vec<CastModeDesc> {
        let mut options = Vec::new();
        let front = &def.faces[0];
        if !front.types.contains(baylee_core::types::TypeSet::LAND)
            && casting::face_timing_allows(&self.state, player, card, def, 0)
            && casting::face_has_a_legal_target(&self.state, &self.lookup, player, card, 0)
        {
            options = if casting::modes_are_the_only_way(def, 0) {
                self.free_modes(player, card, def)
            } else {
                self.free_normal_options(player, card)
            };
        }
        let on_adventure = casting::adventure_only_permission(&self.state, player, card);
        for (face, _) in casting::castable_back_faces(def, on_adventure) {
            if casting::face_timing_allows(&self.state, player, card, def, face)
                && casting::face_has_a_legal_target(&self.state, &self.lookup, player, card, face)
                && casting::free_face_affordable(&self.state, player, card, def, face)
            {
                options.push(CastModeDesc {
                    index: u32::try_from(options.len()).unwrap_or(u32::MAX),
                    kind: CastModeKind::Face(face),
                    cost: casting::free_face_price(&self.state, player, card, def, face),
                });
            }
        }
        options
    }
}

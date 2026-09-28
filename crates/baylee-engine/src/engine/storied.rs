//! Storied is continuous, and its designation survives the source (CR 702.195).
use baylee_cards_dsl::KeywordSet;
use baylee_core::{
    generated::subtypes,
    types::{SupertypeSet, TypeSet},
};

impl crate::state::GameState {
    /// Reapply continuous effects after awarding a designation and before
    /// trigger detection (CR 702.195c). Each permanent counts only once.
    pub(crate) fn award_enduring_stories(&mut self) -> bool {
        let battlefield: Vec<_> = self.battlefield_seen().collect();
        let mut earned = Vec::new();
        for player in &self.players {
            if player.enduring_story || player.has_lost() {
                continue;
            }
            let mut has_storied = false;
            let mut count = 0;
            for id in &battlefield {
                let Some(o) = self.object(*id).filter(|o| o.controller == player.id) else {
                    continue;
                };
                let c = o.characteristics();
                has_storied |= c.keywords.contains(KeywordSet::STORIED);
                if c.types.contains(TypeSet::ARTIFACT)
                    || c.supertypes.contains(SupertypeSet::LEGENDARY)
                    || c.subtypes.contains(subtypes::enchantment::SAGA)
                {
                    count += 1;
                }
            }
            if has_storied && count >= 3 {
                earned.push(player.id);
            }
        }
        for id in &earned {
            self.players[usize::from(id.get())].enduring_story = true;
        }
        !earned.is_empty()
    }
}

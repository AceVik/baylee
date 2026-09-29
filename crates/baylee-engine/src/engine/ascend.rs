//! Ascend (CR 702.131): the city's blessing, a designation kept for the rest
//! of the game.
use baylee_cards_dsl::KeywordSet;

impl crate::state::GameState {
    /// Gives the city's blessing to every player who controls a permanent
    /// with ascend and ten or more permanents (CR 702.131b), and says
    /// whether anybody got it, so that continuous effects are reapplied
    /// before triggers are looked for (CR 702.131d).
    ///
    /// Asked where enduring stories are, which is every engine step and
    /// between the effects of a resolution: "any time" as far as anything
    /// can observe it. A phased-out permanent is not there to count.
    pub(crate) fn award_citys_blessings(&mut self) -> bool {
        let battlefield: Vec<_> = self.battlefield_seen().collect();
        let mut earned = Vec::new();
        for player in &self.players {
            if player.citys_blessing || player.has_lost() {
                continue;
            }
            let mut ascends = false;
            let mut count = 0_usize;
            for id in &battlefield {
                let Some(o) = self.object(*id).filter(|o| o.controller == player.id) else {
                    continue;
                };
                ascends |= o.characteristics().keywords.contains(KeywordSet::ASCEND);
                count += 1;
            }
            if ascends && count >= 10 {
                earned.push(player.id);
            }
        }
        for id in &earned {
            self.players[usize::from(id.get())].citys_blessing = true;
        }
        !earned.is_empty()
    }
}

//! What the groups say of an answer already given, for a reader of a journal.

use baylee_core::ids::ObjectId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::PlayerView;

use super::{GROUPS, RuleGroup, TacticRules4, attack, block, opening, struck_by, targets};

impl TacticRules4 {
    /// Each group that would have changed this answer, and how. An attack and a
    /// block are read as declared, with the logits they no longer have.
    #[must_use]
    pub fn audit(
        &self,
        view: &PlayerView,
        pending: &Pending,
        action: &PlayerAction,
    ) -> Vec<(&'static str, String)> {
        let name = |group: RuleGroup| GROUPS[group as usize].1;
        let mut found = Vec::new();
        match (pending, action) {
            (Pending::Priority { legal, .. }, _) => {
                if let Some((_, struck)) = self.strike_priority(view, legal)
                    && let Some(group) = struck_by(Some(action), &struck)
                {
                    found.push((name(group), String::new()));
                }
            }
            (Pending::Mulligan { .. }, PlayerAction::MulliganKeep)
                if self.on(RuleGroup::Keep) && opening::redraw(view, pending) =>
            {
                found.push((name(RuleGroup::Keep), String::new()));
            }
            (
                Pending::ChooseTargets {
                    options,
                    player_options,
                    ..
                },
                PlayerAction::ChooseObjects { objects }
                | PlayerAction::ChooseTargets { objects, .. },
            ) => {
                let named: Vec<targets::Named> = options
                    .iter()
                    .map(|id| targets::Named::Object(*id))
                    .chain(player_options.iter().map(|id| targets::Named::Player(*id)))
                    .collect();
                for (index, group) in self.refuse_named(&named, view, pending, None) {
                    if index < options.len() && objects.contains(&options[index]) {
                        found.push((name(group), format!("{:?}", options[index])));
                    }
                }
            }
            (Pending::ChooseAttackers { .. }, PlayerAction::DeclareAttackers { attackers }) => {
                let going: Vec<ObjectId> = attackers.iter().map(|pair| pair.0).collect();
                if self.on(RuleGroup::Press) {
                    let free = attack::free_swings(view, pending, &going);
                    if !free.is_empty() {
                        found.push((name(RuleGroup::Press), format!("{free:?}")));
                    }
                }
                if self.on(RuleGroup::Race) {
                    let staying = attack::decide(view, pending, going);
                    if !staying.is_empty() {
                        found.push((name(RuleGroup::Race), format!("{staying:?}")));
                    }
                }
            }
            (Pending::ChooseBlockers { .. }, PlayerAction::DeclareBlockers { blockers }) => {
                let tests: [(RuleGroup, block::BlockTest); 3] = [
                    (RuleGroup::Trip, block::stops_a_connection),
                    (RuleGroup::Trade, block::is_a_fair_trade),
                    (RuleGroup::Wall, block::lives),
                ];
                for (group, test) in tests {
                    if self.on(group) {
                        let open = block::left_open(view, pending, blockers, test);
                        if !open.is_empty() {
                            found.push((name(group), format!("{open:?}")));
                        }
                    }
                }
                if self.on(RuleGroup::Chump) {
                    let chumps = block::chumps(view, pending, blockers);
                    if !chumps.is_empty() {
                        found.push((name(RuleGroup::Chump), format!("{chumps:?}")));
                    }
                }
            }
            _ => {}
        }
        found
    }
}

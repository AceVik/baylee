//! Shared assertions on the real tokens produced by card resolutions.
#[allow(clippy::wildcard_imports)]
use super::*;
pub(super) fn named(e: &Engine<RegistryLookup>, name: &str) -> Vec<ObjectId> {
    e.state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            e.state()
                .object(*id)
                .is_some_and(|o| o.token.is_some_and(|token| token.name == name))
        })
        .collect()
}
pub(super) fn attack(e: &mut Engine<RegistryLookup>, token: ObjectId) {
    let p1 = PlayerId::new(1);
    let chars = e
        .dev_state_mut(PlayerId::new(0))
        .unwrap()
        .object_mut(token)
        .unwrap()
        .base_mut();
    chars.keywords = chars.keywords.union(baylee_cards_dsl::KeywordSet::HASTE);
    attack_and_collect_blocks(e, token, p1);
    e.apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(e, |e| e.state().turn.step == crate::turn::Step::CombatEnd);
}

//! Small fixtures shared by card regressions; all effects use engine rules.
#[allow(clippy::wildcard_imports)]
use super::*;
pub(super) fn give_mana(e: &mut Engine<RegistryLookup>, who: PlayerId, color: ManaColor, n: u32) {
    e.dev_state_mut(who).unwrap().players[who.get() as usize]
        .mana_pool
        .add(color, n);
    e.refresh_offer();
}
pub(super) fn attach_test_aura(e: &mut Engine<RegistryLookup>, who: PlayerId, target: ObjectId) {
    let aura = baylee_core::generated::index::HOLY_STRENGTH;
    give_mana(e, who, ManaColor::White, 1);
    let card = in_hand(e, who, aura).unwrap();
    e.apply(who, PlayerAction::CastSpell { card }).unwrap();
    aim_at(e, who, target);
    pass_until(e, stack_is_empty);
}
pub(super) fn deal_test_damage(
    e: &mut Engine<RegistryLookup>,
    source: ObjectId,
    target: ObjectId,
    combat: bool,
) -> u32 {
    let state = e.dev_state_mut(PlayerId::new(0)).unwrap();
    let source_version = Some(state.object(source).unwrap().version);
    let mut work = crate::damage::DamageWork::new(
        state,
        vec![crate::damage::Assignment {
            source,
            source_version,
            recipient: crate::event::DamageTarget::Object(target),
            amount: 1,
            is_combat: combat,
        }],
    );
    assert!(
        work.advance(state).is_none(),
        "at most one applicable prevention effect"
    );
    work.dealt()
}

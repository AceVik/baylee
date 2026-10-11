//! Damage fixture shared by history and triggered-damage regressions.
#[allow(clippy::wildcard_imports)] // shared card fixture vocabulary
use super::*;
pub(super) fn damage_player(
    e: &mut Engine<RegistryLookup>,
    source: ObjectId,
    player: PlayerId,
    amount: u32,
    combat: bool,
) {
    let state = e.dev_state_mut(PlayerId::new(0)).unwrap();
    let source_version = Some(state.object(source).unwrap().version);
    let mut work = crate::damage::DamageWork::new(
        state,
        vec![crate::damage::Assignment {
            source,
            source_version,
            recipient: crate::event::DamageTarget::Player(player),
            amount,
            is_combat: combat,
        }],
    );
    assert!(work.advance(state).is_none());
}

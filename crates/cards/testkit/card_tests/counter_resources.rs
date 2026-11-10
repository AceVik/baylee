//! Shared counter rules exercised by the real cards that depend on them.
#[allow(clippy::wildcard_imports)]
pub use super::*;
pub use baylee_core::generated::{index, subtypes};
pub use baylee_engine::event::GameEvent;

pub const P0: PlayerId = PlayerId::new(0);
pub const P1: PlayerId = PlayerId::new(1);

pub fn prepared(
    hand: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, hand)
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    for color in ManaColor::ALL {
        e.dev_state_mut(P0).unwrap().players[0]
            .mana_pool
            .add(color, 30);
    }
    e.refresh_offer();
    e
}

pub fn choose(e: &mut Engine<RegistryLookup>, cards: Vec<ObjectId>) {
    e.apply(P0, PlayerAction::ChooseObjects { objects: cards })
        .unwrap();
}

pub fn snake_count(e: &Engine<RegistryLookup>) -> usize {
    e.state()
        .battlefield_seen()
        .filter(|id| {
            e.state().object(*id).is_some_and(|o| {
                o.controller == P0
                    && o.characteristics()
                        .subtypes
                        .contains(subtypes::creature::SNAKE)
            })
        })
        .count()
}

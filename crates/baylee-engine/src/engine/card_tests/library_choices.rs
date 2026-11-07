//! Shared library inspection and search restrictions, using the real card definitions.
pub use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
pub use super::*;
pub use baylee_core::generated::index;
pub use baylee_engine::event::GameEvent;
pub const P0: PlayerId = PlayerId::new(0);
pub const P1: PlayerId = PlayerId::new(1);

pub fn choose(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    let player = e.pending().asked().unwrap();
    e.apply(player, PlayerAction::ChooseObjects { objects })
        .unwrap();
}
pub fn library(e: &Engine<RegistryLookup>, p: PlayerId) -> Vec<ObjectId> {
    e.state().zones.list(ZoneLocation::Library(p)).clone()
}
pub fn searched(e: &Engine<RegistryLookup>) -> bool {
    matches!(
        e.pending(),
        Pending::ChooseCards {
            prompt: ChoicePrompt::SearchLibrary,
            ..
        }
    )
}

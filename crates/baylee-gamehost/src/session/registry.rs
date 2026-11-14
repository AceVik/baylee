//! The session's compiled card registry.
use baylee_cards::dsl::CardDef;
use baylee_core::ids::CardIndex;
use baylee_engine::state::CardLookup;

/// Registry lookup backed by the compiled card pool.
///
/// A unit, so copying it copies nothing: a forked simulation
/// (`Engine::fork_for`) carries its own.
#[derive(Clone, Copy, Debug, Default)]
pub struct RegistryLookup;
impl CardLookup for RegistryLookup {
    fn card(&self, index: CardIndex) -> Option<&'static CardDef> {
        baylee_cards::by_index(index)
    }
    fn token_id(&self, token: &baylee_cards_dsl::TokenDef) -> Option<u16> {
        let id = baylee_cards::tokens::token_id(token);
        baylee_cards::tokens::by_token_id(id).map(|_| id)
    }
    fn tower_supply(
        &self,
        supply: baylee_core::preset::TowerSupply,
    ) -> Option<&'static baylee_cards_dsl::TokenDef> {
        Some(baylee_cards::tokens::tower::definition(supply))
    }
}

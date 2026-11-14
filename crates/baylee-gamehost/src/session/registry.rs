//! The session's compiled card registry.
use baylee_cards::dsl::CardDef;
use baylee_core::ids::CardIndex;
use baylee_engine::state::CardLookup;

/// A compiled registry retained by live sessions, replays and fair AI forks.
/// Implementations must preserve the shared card/token presentation identities.
pub trait SessionRegistry: CardLookup + Clone + Default + Send + Sync + 'static {
    /// Hosted variant supported by this compiled registry, independently of loop policy.
    fn supports_execution(variant: baylee_protocol::execution::Variant) -> bool;
}

impl SessionRegistry for RegistryLookup {
    fn supports_execution(variant: baylee_protocol::execution::Variant) -> bool {
        matches!(
            variant,
            baylee_protocol::execution::Variant::Standard
                | baylee_protocol::execution::Variant::StandardLoops
        )
    }
}
impl<P: baylee_engine::extensions::NativeRulePack> SessionRegistry
    for baylee_engine::extensions::NativeLookup<RegistryLookup, P>
{
    fn supports_execution(variant: baylee_protocol::execution::Variant) -> bool {
        variant == baylee_protocol::execution::Variant::Tower
    }
}

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

//! Compiled setup adapters. No event callbacks, mutable service API or interpreter.
//!
//! A pack validates an immutable preset and resolves finite definition IDs once.
//! After setup its sources use the ordinary rules pipeline. The lookup's type
//! fixes the admitted pack; neither the preset nor a feature-unified dependency
//! can switch an ordinary lookup into a different executor.
use crate::state::{CardLookup, SetupError};
use baylee_cards_dsl::{CardDef, TokenDef};
use baylee_core::{
    ids::CardIndex,
    preset::{GamePreset, TowerSupply},
};
use std::marker::PhantomData;

/// A native definition's setup representation. All operands are compiled data.
#[derive(Clone, Copy, Debug)]
pub enum NativeDefinition {
    /// A pack-validated modifier already represented in the ordinary preset.
    SetupOnly,
    /// An immutable command-zone source, with the normal emblem semantics.
    Emblem(&'static TokenDef),
    /// One starting token permanent, subject to ordinary removal and copy rules.
    Permanent(&'static TokenDef),
}

/// A statically selected native pack; implementations live outside the kernel.
pub trait NativeRulePack: Clone + Send + Sync + 'static {
    /// Stable nonzero namespace admitted by this compiled pack.
    const PACK: u16;
    /// Exact immutable definition revision.
    const REVISION: u16;
    /// Checks exact pack/revision, all definitions and derived setup resources.
    /// It must refuse an absent or different pack instead of downgrading it.
    ///
    /// # Errors
    /// The frozen preset does not match this compiled pack's supported contract.
    fn validate(preset: &GamePreset) -> Result<(), SetupError>;
    /// Resolves one admitted immutable definition, never caller-provided effects.
    fn definition(id: u16) -> Option<NativeDefinition>;
}

/// A registry plus one compile-time pack. The pack occupies no runtime storage.
#[derive(Clone, Debug)]
pub struct NativeLookup<L, P> {
    registry: L,
    pack: PhantomData<fn() -> P>,
}

impl<L, P> NativeLookup<L, P> {
    /// Binds the trusted registry to the pack selected by the executable's type.
    #[must_use]
    pub const fn new(registry: L) -> Self {
        Self {
            registry,
            pack: PhantomData,
        }
    }
}

impl<L: Default, P> Default for NativeLookup<L, P> {
    fn default() -> Self {
        Self::new(L::default())
    }
}

impl<L: CardLookup, P: NativeRulePack> CardLookup for NativeLookup<L, P> {
    fn card(&self, index: CardIndex) -> Option<&'static CardDef> {
        self.registry.card(index)
    }
    fn token_id(&self, token: &TokenDef) -> Option<u16> {
        self.registry.token_id(token)
    }
    fn tower_supply(&self, supply: TowerSupply) -> Option<&'static TokenDef> {
        self.registry.tower_supply(supply)
    }
    fn validate_rule_pack(&self, preset: &GamePreset) -> Result<(), SetupError> {
        let Some(setup) = &preset.house_rules.rule_pack else {
            return Err(SetupError::UnsupportedRulePack);
        };
        if setup.pack != P::PACK || setup.revision != P::REVISION {
            return Err(SetupError::UnsupportedRulePack);
        }
        P::validate(preset)?;
        // Resolve the entire manifest before allocating decks or native sources.
        // Unknown definitions and presentation identities fail atomically.
        for id in setup.seats.iter().flat_map(|seat| &seat.definitions) {
            match P::definition(*id).ok_or(SetupError::UnsupportedRulePack)? {
                NativeDefinition::SetupOnly => {}
                NativeDefinition::Emblem(token) | NativeDefinition::Permanent(token) => {
                    if self.registry.token_id(token).is_none() {
                        return Err(SetupError::UnsupportedRulePack);
                    }
                }
            }
        }
        Ok(())
    }
    fn native_definition(&self, id: u16) -> Option<NativeDefinition> {
        P::definition(id)
    }
}

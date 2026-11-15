//! Custom-mode opening supplies built from ordinary engine objects.

use super::{CardLookup, GameState, SetupError};
use crate::object::{CounterKind, ObjectKind};
use crate::zone::ZoneLocation;
use baylee_core::ids::PlayerId;
use baylee_core::preset::{SeatSupplies, TowerSupply};

impl GameState {
    pub(super) fn seed_supplies(
        &mut self,
        supplies: &[SeatSupplies],
        lookup: &impl CardLookup,
    ) -> Result<(), SetupError> {
        for supply in supplies {
            let player = PlayerId::new(supply.seat);
            for (kind, charges) in [
                (TowerSupply::HealingPotion, supply.healing),
                (TowerSupply::ManaPotion, supply.mana),
            ] {
                if charges == 0 && !supply.concealed {
                    continue;
                }
                let definition = lookup
                    .tower_supply(kind)
                    .ok_or(SetupError::UnsupportedSupply)?;
                let name = self.names.intern(definition.name);
                let id = self.create_bare(
                    player,
                    ObjectKind::Emblem,
                    name,
                    ZoneLocation::Command(player),
                );
                let object = self.object_mut(id).expect("new supply exists");
                // Registry identity is shared with tokens for views and all
                // seats; Emblem keeps the object outside permanent/token SBAs.
                object.token = Some(definition);
                object.counters.set(CounterKind::Charge, u16::from(charges));
                if supply.concealed {
                    object
                        .status
                        .insert(crate::object::Status::PRIVATE_INVENTORY);
                }
            }
            if supply.seedling {
                let definition = lookup
                    .tower_supply(TowerSupply::Seedling)
                    .ok_or(SetupError::UnsupportedSupply)?;
                crate::resolve::tokens::create_tokens(self, player, definition, None, 1);
            }
        }
        Ok(())
    }
}

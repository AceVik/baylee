//! One-time placement of admitted native sources; no artificial entry events.
use super::{CardLookup, GameState, SetupError};
use crate::{
    extensions::NativeDefinition,
    object::{CounterKind, ObjectKind},
    zone::ZoneLocation,
};
use baylee_core::{ids::PlayerId, preset::RulePackSetup};

impl GameState {
    pub(super) fn seed_extensions(
        &mut self,
        setup: Option<&RulePackSetup>,
        lookup: &impl CardLookup,
    ) -> Result<(), SetupError> {
        let Some(setup) = setup else {
            return Ok(());
        };
        for seat in &setup.seats {
            let player = PlayerId::new(seat.seat);
            for &id in &seat.definitions {
                let native = lookup
                    .native_definition(id)
                    .ok_or(SetupError::UnsupportedRulePack)?;
                let (definition, kind, zone) = match native {
                    NativeDefinition::SetupOnly => continue,
                    NativeDefinition::Emblem(definition) => (
                        definition,
                        ObjectKind::Emblem,
                        ZoneLocation::Command(player),
                    ),
                    NativeDefinition::Permanent(definition) => {
                        (definition, ObjectKind::Permanent, ZoneLocation::Battlefield)
                    }
                };
                // Every seat must be able to identify and describe these rules.
                if lookup.token_id(definition).is_none() {
                    return Err(SetupError::UnsupportedRulePack);
                }
                let name = self.names.intern(definition.name);
                let base =
                    (kind == ObjectKind::Permanent).then(|| self.token_base(definition, None));
                let source = self.create_bare(player, kind, name, zone);
                let object = self.object_mut(source).expect("new native source exists");
                object.token = Some(definition);
                if let Some(base) = base {
                    object.base = base;
                    if let Some(loyalty) = definition.loyalty {
                        object.counters.set(CounterKind::Loyalty, loyalty);
                    }
                    if let Some(defense) = definition.defense {
                        object.counters.set(CounterKind::Defense, defense);
                    }
                }
            }
        }
        Ok(())
    }
}

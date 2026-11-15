//! The same derivation at authority setup and native executor admission.
use super::{
    GENERATOR, PACK,
    catalog::{self, Definition},
    generation,
};
use crate::Refusal;
use baylee_core::preset::{
    FormatId, GamePreset, HouseRules, LoopPolicy, RulePackSetup, SeatCapabilities, SeatController,
    SeatExtensions, SeatOpening, SeatSupplies,
};

/// Public resources and immutable native definitions for one frozen seat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resources {
    /// Applied exactly to the seat's explicit starting life.
    pub life: i32,
    /// Bounded London/opening attributes.
    pub opening: SeatOpening,
    /// Paid table charges; never the crafting potion.
    pub supplies: SeatSupplies,
    /// Only active slots, in canonical slot order.
    pub extensions: SeatExtensions,
}

/// Derive a seat from active definitions and verified, unspent table charges.
/// # Errors
/// Unknown seat, excessive charges or combined active resources beyond bounds.
pub fn resources(active: &[Definition], charges: [u8; 2], seat: u8) -> Result<Resources, Refusal> {
    resources_for(active, charges, seat, GENERATOR)
}

/// Derive under the saved generation's equipment contract.
/// # Errors
/// Unknown generation, unsupported family, seat or combined resource bounds.
pub fn resources_for(
    active: &[Definition],
    charges: [u8; 2],
    seat: u8,
    generator: u16,
) -> Result<Resources, Refusal> {
    generation::contract(generator)?;
    for definition in active {
        generation::definition(generator, *definition)?;
    }
    if seat > 1 || charges.iter().map(|n| u16::from(*n)).sum::<u16>() > 2 {
        return Err(Refusal::Invalid);
    }
    let attributes = if generator == generation::RUN {
        catalog::run_attributes(active)?
    } else {
        catalog::attributes(active)?
    };
    Ok(Resources {
        life: attributes.life,
        opening: SeatOpening {
            seat,
            cards: attributes.cards,
            maximum_hand: attributes.maximum_hand,
            free_mulligans: attributes.free_mulligans,
        },
        supplies: SeatSupplies {
            seat,
            healing: charges[0],
            mana: charges[1],
            seedling: false,
            concealed: generator == generation::RUN,
        },
        extensions: SeatExtensions {
            seat,
            definitions: active.iter().map(|item| item.id()).collect(),
        },
    })
}

/// Freeze native Tower rules. The service also freezes decks, opponent and seeds.
/// # Errors
/// The resources do not represent exactly seats zero and one, in that order.
pub fn rules(seats: &[Resources; 2], policy_seed: u64) -> Result<HouseRules, Refusal> {
    rules_for(seats, policy_seed, GENERATOR)
}

/// Derive rules for a retained run without using the current native revision.
/// # Errors
/// Unknown generation, invalid seat order or a definition outside its frozen pool.
pub fn rules_for(
    seats: &[Resources; 2],
    policy_seed: u64,
    generator: u16,
) -> Result<HouseRules, Refusal> {
    let contract = generation::contract(generator)?;
    for (index, seat) in seats.iter().enumerate() {
        if usize::from(seat.extensions.seat) != index
            || seat.opening.seat != seat.extensions.seat
            || seat.supplies.seat != seat.extensions.seat
        {
            return Err(Refusal::Invalid);
        }
        for id in &seat.extensions.definitions {
            generation::definition(generator, Definition::from_id(*id).ok_or(Refusal::Invalid)?)?;
        }
    }
    Ok(HouseRules {
        loop_policy: LoopPolicy::Off,
        challenge_policy_seed: Some(policy_seed),
        decision_timeout_secs: 0,
        reconnect_window_secs: 0,
        time_extension_votes: false,
        opening: seats.iter().map(|seat| seat.opening).collect(),
        supplies: seats.iter().map(|seat| seat.supplies).collect(),
        rule_pack: Some(RulePackSetup {
            pack: PACK,
            revision: contract.revision,
            seats: seats.iter().map(|seat| seat.extensions.clone()).collect(),
        }),
        ..HouseRules::for_a_table()
    })
}

/// Refuse setup resources not justified by this native pack's exact definitions.
/// This validates rules, not account ownership or deck acquisition provenance.
/// # Errors
/// Any unsupported manifest, bonus, administrative capability or policy override.
pub fn validate(preset: &GamePreset) -> Result<(), Refusal> {
    validate_for(preset, GENERATOR)
}

/// Validate a saved generation at the authority; the executor admits its own pack only.
/// # Errors
/// Unknown generation or any setup not derived from that generation's definitions.
pub fn validate_for(preset: &GamePreset, generator: u16) -> Result<(), Refusal> {
    let contract = generation::contract(generator)?;
    preset.validate().map_err(|_| Refusal::Invalid)?;
    let house = &preset.house_rules;
    let pack = house.rule_pack.as_ref().ok_or(Refusal::Invalid)?;
    let policy_seed = house.challenge_policy_seed.ok_or(Refusal::Invalid)?;
    if preset.format != FormatId::Custom
        || preset.seats.len() != 2
        || !preset.modifiers.is_empty()
        || pack.pack != PACK
        || pack.revision != contract.revision
        || pack.seats.len() != 2
        || house.supplies.len() != 2
    {
        return Err(Refusal::Invalid);
    }
    let mut derived = Vec::with_capacity(2);
    for (index, seat) in preset.seats.iter().enumerate() {
        let entry = &pack.seats[index];
        let supplies = house.supplies[index];
        if usize::from(entry.seat) != index
            || supplies.seat != entry.seat
            || seat.capabilities != SeatCapabilities::default()
            || seat.team.is_some()
            || seat.starting_hand.is_some()
            || !seat.starting_battlefield.is_empty()
            || !seat.emblems.is_empty()
            || if generator == generation::RUN {
                seat.commanders.len() != 1 || seat.deck.len() != 39 || seat.sideboard.len() > 5
            } else {
                !seat.commanders.is_empty() || !seat.sideboard.is_empty()
            }
        {
            return Err(Refusal::Invalid);
        }
        let active = entry
            .definitions
            .iter()
            .map(|id| Definition::from_id(*id))
            .collect::<Option<Vec<_>>>()
            .ok_or(Refusal::Invalid)?;
        let resource = resources_for(
            &active,
            [supplies.healing, supplies.mana],
            entry.seat,
            generator,
        )?;
        if seat.starting_life != Some(resource.life) {
            return Err(Refusal::Invalid);
        }
        derived.push(resource);
    }
    if !matches!(
        preset.seats[0].controller,
        SeatController::Open | SeatController::Human { .. }
    ) || preset.seats[1].controller != SeatController::Ai(contract.expert())
        || *house
            != rules_for(
                &derived.try_into().map_err(|_| Refusal::Invalid)?,
                policy_seed,
                generator,
            )?
    {
        return Err(Refusal::Invalid);
    }
    Ok(())
}

#[cfg(test)]
mod tests;

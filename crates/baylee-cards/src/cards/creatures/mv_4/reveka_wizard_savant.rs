//! Reveka, Wizard Savant — {2}{U}{U} — Legendary Creature — Dwarf Wizard
//! Oracle: {T}: Reveka deals 2 damage to any target and doesn't untap during your next untap step.
//! Set: ME3 #49 — Masters Edition III | Scryfall ID: 27c32cc5-b4da-4345-9118-1fe46a5049cb | Oracle ID: 7085d5c6-da96-465e-90cf-8469fcacb474
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::REVEKA_WIZARD_SAVANT,
    oracle_id = "7085d5c6-da96-465e-90cf-8469fcacb474",
    scryfall_id = "27c32cc5-b4da-4345-9118-1fe46a5049cb",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Reveka, Wizard Savant",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::DWARF, subtypes::creature::WIZARD],
        power = Some(0),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[
            Effect::DealDamage {
                amount: Amount::Fixed(2),
                target: TargetSpec::AnyTarget,
            },
            Effect::ModifySource {
                modifier: Modifier::DoesNotUntap,
                duration: Duration::UntilYourNextUntapStep,
            },
        ],
        target = Some(TargetSpec::AnyTarget),
    )],
);

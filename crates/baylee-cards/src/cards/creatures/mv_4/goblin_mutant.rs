//! Goblin Mutant — {2}{R}{R} — Creature — Goblin Mutant
//! Oracle: Trample
//! Oracle: This creature can't attack if defending player controls an untapped creature with power 3 or greater.
//! Oracle: This creature can't block creatures with power 3 or greater.
//! Set: ME1 #96 — Masters Edition | Scryfall ID: 4540dbe7-5a4b-4603-93c7-5bb294d67686 | Oracle ID: 540e1371-baa9-412f-a17a-22ea833ea3e8
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const POWER_3_OR_MORE: Filter = Filter::And(&[Filter::CREATURE, Filter::PowerAtLeast(3)]);
const UNTAPPED_POWER_3_OR_MORE: Filter = Filter::And(&[POWER_3_OR_MORE, Filter::Untapped]);

card!(
    index = index::GOBLIN_MUTANT,
    oracle_id = "540e1371-baa9-412f-a17a-22ea833ea3e8",
    scryfall_id = "4540dbe7-5a4b-4603-93c7-5bb294d67686",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Mutant",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN, subtypes::creature::MUTANT],
        power = Some(5),
        toughness = Some(3),
    ),],
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::CantAttackIfDefenderControls(&UNTAPPED_POWER_3_OR_MORE),
        ),
        static_ability!(Filter::This, Modifier::CantBlock(&POWER_3_OR_MORE)),
    ],
);

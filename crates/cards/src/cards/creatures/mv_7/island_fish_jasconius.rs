//! Island Fish Jasconius — {4}{U}{U}{U} — Creature — Fish
//! Oracle: This creature doesn't untap during your untap step.
//! Oracle: At the beginning of your upkeep, you may pay {U}{U}{U}. If you do, untap this creature.
//! Oracle: This creature can't attack unless defending player controls an Island.
//! Oracle: When you control no Islands, sacrifice this creature.
//! Set: 4ED #78 — Fourth Edition | Scryfall ID: 84f18188-70fa-433e-a114-2f1dd49388ed | Oracle ID: bb217f12-532f-4833-a27a-99e290aa47d0
// IMPLEMENTED — static no-untap, an upkeep payment that untaps it, the
// Island-gated attack, and a state trigger that sacrifices it with no Islands.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static DEFENDING_ISLAND: Filter = Filter::HasSubtype(subtypes::land::ISLAND);

static YOUR_ISLAND: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::land::ISLAND),
    Filter::ControlledByYou,
]);

static NO_ISLANDS: Condition = Condition::ControlCountAtMost(&YOUR_ISLAND, 0);

card!(
    index = index::ISLAND_FISH_JASCONIUS,
    oracle_id = "bb217f12-532f-4833-a27a-99e290aa47d0",
    scryfall_id = "84f18188-70fa-433e-a114-2f1dd49388ed",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Island Fish Jasconius",
        mana_cost = mana!("{4}{U}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::FISH],
        power = Some(6),
        toughness = Some(8),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(Filter::This, Modifier::DoesNotUntap),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMayPayManaThen {
                player: PlayerRel::You,
                cost: mana!("{U}{U}{U}"),
                effects: &[Effect::UntapSelf],
            }],
        ),
        static_ability!(
            Filter::This,
            Modifier::CantAttackUnlessDefenderControls(&DEFENDING_ISLAND)
        ),
        triggered!(Trigger::State(&NO_ISLANDS), &[Effect::SacrificeSelf]),
    ],
);

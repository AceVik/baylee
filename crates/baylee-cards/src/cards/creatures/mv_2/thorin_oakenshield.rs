//! Thorin Oakenshield — {R}{W} — Legendary Creature — Dwarf Noble
//! Oracle: Trample
//! Oracle: Storied (If you control three or more artifacts, legendaries, and/or Sagas, you have an enduring story for the rest of the game.)
//! Oracle: As long as you have an enduring story, artifacts and creatures you control have ward {1}.
//! Set: HOB #165 — The Hobbit | Scryfall ID: c7e18609-d1ed-4829-be11-f2ce2cfcbc49 | Oracle ID: bdd41af0-bbd1-4ecd-a699-99f006f5e5ce
// IMPLEMENTED — storied permanently awards the designation; the ward grant
// follows Thorin and his current controller. Engine tests: ward_tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THORIN_OAKENSHIELD,
    oracle_id = "bdd41af0-bbd1-4ecd-a699-99f006f5e5ce",
    scryfall_id = "c7e18609-d1ed-4829-be11-f2ce2cfcbc49",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::TRAMPLE.union(KeywordSet::STORIED),
    coverage = Coverage::Implemented,
    abilities = &[static_ability!(
        Filter::And(&[
            Filter::ControlledByYou,
            Filter::HasType(TypeSet::ARTIFACT.union(TypeSet::CREATURE))
        ]),
        Modifier::GrantTriggered {
            trigger: Trigger::Ward,
            effects: &[Effect::PlayerMayPayOr {
                player: PlayerRel::ControllerOfTarget,
                mana: Amount::Fixed(1),
                effect: &Effect::CounterTargetSpellOrAbility,
            }],
            target: None
        },
        condition = Some(Condition::EnduringStory)
    )],
    faces = &[face!(
        name = "Thorin Oakenshield",
        mana_cost = mana!("{R}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::DWARF, subtypes::creature::NOBLE],
        power = Some(3),
        toughness = Some(2),
    ),],
);

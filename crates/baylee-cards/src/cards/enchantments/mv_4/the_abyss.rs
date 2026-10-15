//! The Abyss — {3}{B} — World Enchantment
//! Oracle: At the beginning of each player's upkeep, destroy target nonartifact creature that player controls of their choice. It can't be regenerated.
//! Set: ME3 #77 — Masters Edition III | Scryfall ID: f11db51c-bbbc-4890-960e-d8a3eacca1e5 | Oracle ID: a02a7816-c967-4503-bb08-f8db44915250
// IMPLEMENTED — each upkeep player chooses the target; destruction cannot regenerate.

use baylee_cards_dsl::prelude::*;

const NONARTIFACT_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::LacksType(TypeSet::ARTIFACT)]);

card!(
    index = index::THE_ABYSS,
    oracle_id = "a02a7816-c967-4503-bb08-f8db44915250",
    scryfall_id = "f11db51c-bbbc-4890-960e-d8a3eacca1e5",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "The Abyss",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    )],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::EachPlayer
        },
        &[Effect::Destroy {
            target: TargetSpec::ObjectOfEventPlayer(&NONARTIFACT_CREATURE),
            no_regen: true,
        }],
        targets = Some(
            TargetReq::one(TargetSpec::ObjectOfEventPlayer(&NONARTIFACT_CREATURE))
                .chosen_by_event_player()
        )
    )],
);

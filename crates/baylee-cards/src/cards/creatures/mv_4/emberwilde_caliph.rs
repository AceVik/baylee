//! Emberwilde Caliph — {2}{U}{R} — Creature — Djinn
//! Oracle: Flying, trample
//! Oracle: This creature attacks each combat if able.
//! Oracle: Whenever this creature deals damage, you lose that much life.
//! Set: MIR #262 — Mirage | Scryfall ID: 9598a9b4-15bb-4645-92ed-8eedef75dc24 | Oracle ID: 8434bdb1-286b-484e-904e-946d074614e7
// IMPLEMENTED — flying, trample, attacks each combat if able, and you lose life equal to damage it deals.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EMBERWILDE_CALIPH,
    oracle_id = "8434bdb1-286b-484e-904e-946d074614e7",
    scryfall_id = "9598a9b4-15bb-4645-92ed-8eedef75dc24",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Emberwilde Caliph",
        mana_cost = mana!("{2}{U}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DJINN],
        power = Some(4),
        toughness = Some(4),
        keywords = KeywordSet::FLYING.union(KeywordSet::TRAMPLE),
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::AttacksEachCombat),
        triggered!(
            Trigger::DealsDamage(&Filter::This),
            &[Effect::LoseLife {
                amount: Amount::EventAmount,
                target: PlayerRel::You,
            }],
        ),
    ],
);

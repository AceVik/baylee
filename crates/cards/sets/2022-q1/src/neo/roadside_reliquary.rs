//! Roadside Reliquary — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}, Sacrifice this land: Draw a card if you control an artifact. Draw a card if you control an enchantment.
//! Set: NEO #272 — Kamigawa: Neon Dynasty | Scryfall ID: 8002de90-93fb-48ea-a849-40fdad0aef5a | Oracle ID: 2fb13687-0518-4ba0-a5ae-dd609464b026
// IMPLEMENTED — both halves; the two draws are two independent branches,
// which is how the card prints them.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ROADSIDE_RELIQUARY,
    oracle_id = "2fb13687-0518-4ba0-a5ae-dd609464b026",
    scryfall_id = "8002de90-93fb-48ea-a849-40fdad0aef5a",
    faces = &[face!(name = "Roadside Reliquary", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // Two sentences and not one with two conditions: a board with an
        // artifact and an enchantment draws two, and the card is worth
        // playing for either half alone. The land itself is already in the
        // graveyard when these run — it paid for the ability — and is
        // neither an artifact nor an enchantment, so its departure changes
        // no answer here.
        activated!(
            cost!("{2}", TapSelf, SacrificeSelf),
            &[
                Effect::IfCondition {
                    condition: Condition::ControlCount(&Filter::ARTIFACT, 1),
                    then: &[Effect::draw(1)],
                    otherwise: &[],
                },
                Effect::IfCondition {
                    condition: Condition::ControlCount(&Filter::ENCHANTMENT, 1),
                    then: &[Effect::draw(1)],
                    otherwise: &[],
                },
            ]
        ),
    ],
);

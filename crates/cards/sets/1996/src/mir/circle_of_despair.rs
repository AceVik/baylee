//! Circle of Despair — {1}{W}{B} — Enchantment
//! Oracle: {1}, Sacrifice a creature: The next time a source of your choice would deal damage to any target this turn, prevent that damage.
//! Set: MIR #259 — Mirage | Scryfall ID: ef9d2c05-0d2f-4d02-aef3-e1078d78e5ff | Oracle ID: 4ad57fbf-69f3-4367-b061-ed8e0b83b9af
// PARTIAL — only its controller can be targeted; chosen-source prevention for other players and creatures is unsupported.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CIRCLE_OF_DESPAIR,
    oracle_id = "4ad57fbf-69f3-4367-b061-ed8e0b83b9af",
    scryfall_id = "ef9d2c05-0d2f-4d02-aef3-e1078d78e5ff",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::White]),
    coverage = Coverage::Partial(
        "Target yourself to prevent the next damage a chosen source would deal to you this turn; chosen-source prevention for other players and creatures is unsupported."
    ),
    faces = &[face!(
        name = "Circle of Despair",
        mana_cost = mana!("{1}{W}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{1}", Sacrifice(&Filter::CREATURE)),
        &[Effect::PreventNextFromChosenSource {
            sources: &Filter::Any,
            combat_only: false,
            all_but: 0,
            gain_life: false,
        }],
        targets = Some(TargetReq::one(TargetSpec::Player(PlayerRel::You))),
    )],
);

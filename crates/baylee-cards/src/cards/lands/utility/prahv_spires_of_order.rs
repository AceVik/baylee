//! Prahv, Spires of Order — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}{W}{U}, {T}: Prevent all damage a source of your choice would deal this turn.
//! Set: DIS #177 — Dissension | Scryfall ID: 2a315f63-96ad-4a5f-8eb4-d81361797348 | Oracle ID: 37ff5ba6-0763-4c73-85bf-66856e67b8f3
// PARTIAL — {T}: Add {C} is built; the {4}{W}{U}, {T} ability is not
// expressible yet (see the NOT SUPPORTED line beside the abilities).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PRAHV_SPIRES_OF_ORDER,
    oracle_id = "37ff5ba6-0763-4c73-85bf-66856e67b8f3",
    scryfall_id = "2a315f63-96ad-4a5f-8eb4-d81361797348",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[face!(
        name = "Prahv, Spires of Order",
        types = TypeSet::LAND,
    ),],
    coverage = Coverage::Partial(
        "the {4}{W}{U} ability chooses a source of your choice as it resolves; \
         no Effect takes an object chosen that way, so the nearest variant — \
         Effect::CreateContinuousEffect with Modifier::PreventDamageFromIt, \
         whose Filter::This binds to the first target — would ask on the stack \
         a question the card asks on resolution"
    ),
    // NOT SUPPORTED: "{4}{W}{U}, {T}: Prevent all damage a source of your
    // choice would deal this turn." — the source is chosen as the ability
    // resolves and is not a target (CR 115.1), and the DSL has no effect
    // that picks an arbitrary object at that moment: a TargetSpec would put
    // the choice on the stack, where hexproof and protection answer an
    // ability the printed card walks past them. Modifier::PreventDamageFromIt
    // itself exists, so only the choice is missing.
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);

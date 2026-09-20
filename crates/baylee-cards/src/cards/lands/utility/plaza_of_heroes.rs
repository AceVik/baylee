//! Plaza of Heroes — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a legendary spell.
//! Oracle: {T}: Add one mana of any color among legendary permanents you control.
//! Oracle: {3}, {T}, Exile this land: Target legendary creature gains hexproof and indestructible until end of turn.
//! Set: MSC #255 — Marvel Super Heroes Commander | Scryfall ID: 96fe4b9b-d766-463b-a6df-345ebebfc17c | Oracle ID: 9c58d241-4d9f-4b46-b8ee-f4587f9acfd6
// PARTIAL — the {C} line, the restricted any-color line and the {3} exile
// activation are built; the third mana line needs a ManaSource that reads a
// colour set off a filter.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PLAZA_OF_HEROES,
    oracle_id = "9c58d241-4d9f-4b46-b8ee-f4587f9acfd6",
    scryfall_id = "96fe4b9b-d766-463b-a6df-345ebebfc17c",
    faces = &[face!(name = "Plaza of Heroes", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "{T}: Add one mana of any color among legendary permanents you control — ManaSource has no variant for a colour set computed from a filter",
    ),
    abilities = &[
        // {T}: Add {C}.
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // {T}: Add one mana of any color. Spend this mana only to cast a
        // legendary spell.
        mana_ability!(&[Effect::mana_of_any_color().restricted(
            &Filter::HasSupertype(SupertypeSet::LEGENDARY),
            SpendRider::None,
        )]),
        // NOT SUPPORTED: {T}: Add one mana of any color among legendary
        // permanents you control.
        //
        // {3}, {T}, Exile this land: Target legendary creature gains
        // hexproof and indestructible until end of turn.
        activated!(
            cost!("{3}", TapSelf, ExileSelf),
            &[Effect::continuous(
                &Filter::This,
                Modifier::AddKeyword(KeywordSet::HEXPROOF.union(KeywordSet::INDESTRUCTIBLE)),
                Duration::UntilEndOfTurn,
            )],
            target = Some(TargetSpec::Object(&Filter::LEGENDARY_CREATURE)),
        ),
    ],
);

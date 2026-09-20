//! Emergence Zone — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}, Sacrifice this land: You may cast spells this turn as though they had flash.
//! Set: WAR #245 — War of the Spark | Scryfall ID: ab95f6e7-b806-47fe-a071-6c38b3176d94 | Oracle ID: 7536eb66-959d-4dca-9b75-895572ef733c
// PARTIAL — {C} mana ability, plus the {1}, {T}, Sacrifice flash grant built on
// Modifier::SorceriesHaveFlash, the DSL's only timing grant (see coverage).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EMERGENCE_ZONE,
    oracle_id = "7536eb66-959d-4dca-9b75-895572ef733c",
    scryfall_id = "ab95f6e7-b806-47fe-a071-6c38b3176d94",
    faces = &[face!(name = "Emergence Zone", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the second ability says \"cast spells\", and the DSL's only timing grant is \
         Modifier::SorceriesHaveFlash, which reaches sorcery spells alone — a creature, \
         artifact, enchantment or planeswalker spell would stay at sorcery speed"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "You may cast spells this turn as though they had flash" —
        // Modifier::SorceriesHaveFlash is the nearest variant and covers sorceries only.
        activated!(
            cost!("{1}", TapSelf, SacrificeSelf),
            &[Effect::continuous(
                &Filter::This,
                Modifier::SorceriesHaveFlash,
                Duration::UntilEndOfTurn,
            )]
        ),
    ],
);

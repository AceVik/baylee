//! Guildmages' Forum — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add one mana of any color. If that mana is spent on a multicolored creature spell, that creature enters with an additional +1/+1 counter on it.
//! Set: GRN #250 — Guilds of Ravnica | Scryfall ID: cba12bda-d460-4206-8469-4357c967b9b8 | Oracle ID: ace6403d-9fac-4d0f-a6ea-eb2ff3da259d
// PARTIAL — {T}: Add {C} and {1}, {T}: Add one mana of any color are built; the +1/+1 counter rider on multicolored creature cast is unsupported.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GUILDMAGES_FORUM,
    oracle_id = "ace6403d-9fac-4d0f-a6ea-eb2ff3da259d",
    scryfall_id = "cba12bda-d460-4206-8469-4357c967b9b8",
    faces = &[face!(name = "Guildmages' Forum", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "SpendRider has no variant for entering with an additional +1/+1 counter when mana is spent on a multicolored creature spell"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(cost!("{1}", TapSelf), &[Effect::mana_of_any_color()]),
        // NOT SUPPORTED: "If that mana is spent on a multicolored creature spell, that creature enters with an additional +1/+1 counter on it."
    ],
);

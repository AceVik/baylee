//! Secluded Courtyard — (no cost) — Land
//! Oracle: As this land enters, choose a creature type.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a creature spell of the chosen type or activate an ability of a creature source of the chosen type.
//! Set: MSC #265 — Marvel Super Heroes Commander | Scryfall ID: c0d17d04-cf0b-4918-bfec-b34b0d98a602 | Oracle ID: 79ba18fd-f184-43c1-86df-56ee18ce806c
// PARTIAL — the choose-a-type arrival and both mana abilities are built; the
// printed spend restriction on the any-color mana is not enforced.

use baylee_cards_dsl::prelude::*;

/// "a creature spell of the chosen type" — the half of the spend restriction
/// a `Filter` can name: the answer the land stored on itself as it entered,
/// read back against the spell the mana is being spent on.
static CHOSEN_TYPE_CREATURE_SPELL: Filter =
    Filter::And(&[Filter::CREATURE, Filter::MatchesChosenTypeOfSource]);

// NOT SUPPORTED: "Spend this mana only to cast a creature spell of the chosen
// type or activate an ability of a creature source of the chosen type." The
// restriction is written down as a `ManaRestriction`, but mana in the pool
// carries no provenance, so nothing enforces it — the {T} ability hands out
// one mana of any color for anything (docs/card-dsl.md, "Explicitly not
// supported yet": mana-source tracking / restricted mana riders). The second
// half names an *ability* on the stack, which no filter reaches at all.

card!(
    index = index::SECLUDED_COURTYARD,
    oracle_id = "79ba18fd-f184-43c1-86df-56ee18ce806c",
    scryfall_id = "c0d17d04-cf0b-4918-bfec-b34b0d98a602",
    faces = &[face!(
        name = "Secluded Courtyard",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::ChooseSubtype],
    )],
    coverage = Coverage::Partial(
        "the spend restriction on the any-color mana is not enforced by the engine"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[
            Effect::mana_of_any_color().restricted(&CHOSEN_TYPE_CREATURE_SPELL, SpendRider::None)
        ]),
    ],
);

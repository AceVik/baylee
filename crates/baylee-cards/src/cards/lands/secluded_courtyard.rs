//! Secluded Courtyard — (no cost) — Land
//! Oracle: As this land enters, choose a creature type.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a creature spell of the chosen type or activate an ability of a creature source of the chosen type.
//! Set: MSC #265 — Marvel Super Heroes Commander | Scryfall ID: c0d17d04-cf0b-4918-bfec-b34b0d98a602 | Oracle ID: 79ba18fd-f184-43c1-86df-56ee18ce806c
// PARTIAL — the choose-a-type arrival and both mana abilities are built; the
// any-color mana's spend restriction is enforced for spells, and its
// activated-ability half has no shape.

use baylee_cards_dsl::prelude::*;

/// "a creature spell of the chosen type" — the half of the spend restriction
/// a `Filter` can name: the answer the land stored on itself as it entered,
/// read back against the spell the mana is being spent on.
static CHOSEN_TYPE_CREATURE_SPELL: Filter =
    Filter::And(&[Filter::CREATURE, Filter::MatchesChosenTypeOfSource]);

// NOT SUPPORTED: "Spend this mana only to cast a creature spell of the chosen
// type or activate an ability of a creature source of the chosen type." The
// first half is a `ManaRestriction`, enforced when the mana pays for a spell.
// The second half names an *ability*, and a restriction is asked only of a
// spell being cast: restricted mana never pays for an activated ability, so
// the land's any-color mana is narrower than printed.

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
        "the any-color mana can pay only for creature spells of the chosen type: \
         \"or activate an ability of a creature source of the chosen type\" has no \
         shape, because a ManaRestriction's filter is asked only of a spell being \
         cast and restricted mana never pays for an activated ability"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[
            Effect::mana_of_any_color().restricted(&CHOSEN_TYPE_CREATURE_SPELL, SpendRider::None)
        ]),
    ],
);

//! Haven of the Spirit Dragon — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a Dragon creature spell.
//! Oracle: {2}, {T}, Sacrifice this land: Return target Dragon creature card or Ugin planeswalker card from your graveyard to your hand.
//! Set: TDC #370 — Tarkir: Dragonstorm Commander | Scryfall ID: d0c552da-874e-49eb-b221-47220b8b7786 | Oracle ID: acc9c16a-5e72-43bd-87e1-56a16aa892f5
// IMPLEMENTED — {C}; any color, restricted to Dragon creature spells; and
// {2}, {T}, Sacrifice this land to return a Dragon creature card or Ugin
// planeswalker card from your graveyard to your hand.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::{creature, planeswalker};

/// "a Dragon creature spell" — what the restricted mana may be spent on.
static DRAGON_SPELL: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasSubtype(creature::DRAGON)]);

/// "a Dragon creature card or Ugin planeswalker card" — what the activated
/// ability reaches in the graveyard.
///
/// Ugin is a planeswalker **subtype** (CR 205.3j), which is how a printing
/// names a character, so both halves of the disjunction are readable without
/// a name filter the `Filter` vocabulary does not have.
static GRAVEYARD_TARGET: Filter = Filter::Or(&[
    Filter::And(&[Filter::CREATURE, Filter::HasSubtype(creature::DRAGON)]),
    Filter::And(&[Filter::PLANESWALKER, Filter::HasSubtype(planeswalker::UGIN)]),
]);

card!(
    index = index::HAVEN_OF_THE_SPIRIT_DRAGON,
    oracle_id = "acc9c16a-5e72-43bd-87e1-56a16aa892f5",
    scryfall_id = "d0c552da-874e-49eb-b221-47220b8b7786",
    faces = &[face!(
        name = "Haven of the Spirit Dragon",
        types = TypeSet::LAND,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[Effect::mana_of_any_color().restricted(&DRAGON_SPELL, SpendRider::None)]),
        activated!(
            cost!("{2}", TapSelf, SacrificeSelf),
            &[Effect::GraveyardToHand {
                target: TargetSpec::CardInGraveyard(&GRAVEYARD_TARGET, PlayerRel::You),
            }],
            target = Some(TargetSpec::CardInGraveyard(
                &GRAVEYARD_TARGET,
                PlayerRel::You,
            )),
        ),
    ],
);

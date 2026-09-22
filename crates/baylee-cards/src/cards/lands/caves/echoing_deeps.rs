//! Echoing Deeps — (no cost) — Land — Cave
//! Oracle: You may have this land enter tapped as a copy of any land card in a graveyard, except it's a Cave in addition to its other types.
//! Oracle: {T}: Add {C}.
//! Set: LCI #271 — The Lost Caverns of Ixalan | Scryfall ID: 244c06b3-532d-426e-8bee-ee9461d092a6 | Oracle ID: 2ef88214-f46d-473e-a55b-795a647e2f03
// PARTIAL — {T}: Add {C}, plus the optional enter-as-a-copy of any land card
// in a graveyard with the printed Cave exception. The "enter tapped" half of
// that choice has no shape in the DSL; see the NOT SUPPORTED line below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ECHOING_DEEPS,
    oracle_id = "2ef88214-f46d-473e-a55b-795a647e2f03",
    scryfall_id = "244c06b3-532d-426e-8bee-ee9461d092a6",
    faces = &[face!(
        name = "Echoing Deeps",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::CAVE],
    ),],
    coverage = Coverage::Partial(
        "the copy arrives untapped: a CopyOnEnter choice has no enter modifier \
         that can ride it, so the printed \"enter tapped\" is dropped"
    ),
    abilities = &[
        // NOT SUPPORTED: "enter tapped as a copy of any land card in a
        // graveyard" — the choice, the copy and the Cave exception are all
        // sayable (`CopyOnEnter` + `CopyMod::AddSubtype`), but the tapped
        // arrival is not: `CopyMod` carries types, supertypes, subtypes,
        // keywords and counters, and an unconditional
        // `EnterModifier::Tapped` on the face would also tap the land when
        // the controller declines to copy anything.
        AbilityDef::CopyOnEnter {
            target: TargetSpec::CardInGraveyard(&Filter::LAND, PlayerRel::EachPlayer),
            mods: &[CopyMod::AddSubtype(subtypes::land::CAVE)],
        },
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
    ],
);

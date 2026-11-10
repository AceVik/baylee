//! Forgotten Monument — (no cost) — Land — Cave
//! Oracle: {T}: Add {C}.
//! Oracle: Other Caves you control have "{T}, Pay 1 life: Add one mana of any color."
//! Set: LCI #272 — The Lost Caverns of Ixalan | Scryfall ID: de8c1c02-e533-46b2-a3eb-91dff561854b | Oracle ID: 71393988-ad6f-43fd-9978-c0de15ae8e87
// IMPLEMENTED — {T}: Add {C}, and the static that gives every other Cave
// you control "{T}, Pay 1 life: Add one mana of any color".

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// `Filter::Another` and not a bare subtype: the card prints *other* Caves,
/// and this land is one. `effects::applies_to` resolves the filter against
/// `fx.source`, so "another" is measured from this monument rather than from
/// whatever object is being asked about.
///
/// The grant is `mana_ability: true` under CR 605.1 — it could add mana, it
/// has no target, and it is not a loyalty ability. `lints::mana_ability_fault`
/// reads the flag on a grant as well as on a printed ability.
static OTHER_CAVES: Filter = Filter::And(&[
    Filter::Another,
    Filter::ControlledByYou,
    Filter::HasSubtype(subtypes::land::CAVE),
]);

card!(
    index = index::FORGOTTEN_MONUMENT,
    oracle_id = "71393988-ad6f-43fd-9978-c0de15ae8e87",
    scryfall_id = "de8c1c02-e533-46b2-a3eb-91dff561854b",
    faces = &[face!(
        name = "Forgotten Monument",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::CAVE],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        static_ability!(
            OTHER_CAVES,
            Modifier::GrantActivated {
                cost: cost!(TapSelf, PayLife(1)),
                effects: ANY_COLOR_MANA,
                mana_ability: true,
            }
        ),
    ],
);

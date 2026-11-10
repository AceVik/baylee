//! Waterveil Cavern — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {U} or {B}. This land doesn't untap during your next untap step.
//! Set: CHK #286 — Champions of Kamigawa | Scryfall ID: 405805e0-07f6-420f-86f9-5bde822caa5c | Oracle ID: 3debfa0d-9945-4a85-a714-7c3d3d74de4e
// IMPLEMENTED — a colorless mana ability, and a second one making {U} or {B}
// that also creates Modifier::DoesNotUntap for Duration::UntilYourNextUntapStep.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WATERVEIL_CAVERN,
    oracle_id = "3debfa0d-9945-4a85-a714-7c3d3d74de4e",
    scryfall_id = "405805e0-07f6-420f-86f9-5bde822caa5c",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(name = "Waterveil Cavern", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            Cost::TAP,
            &[
                Effect::mana_choice(&[ManaColor::Blue, ManaColor::Black]),
                Effect::continuous(
                    &Filter::This,
                    Modifier::DoesNotUntap,
                    Duration::UntilYourNextUntapStep,
                ),
            ]
        ),
    ],
);

//! Rootwater Depths — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {U} or {B}. This land doesn't untap during your next untap step.
//! Set: TPR #241 — Tempest Remastered | Scryfall ID: 7186d86c-679b-4520-9cf0-5b25f6d98312 | Oracle ID: 2d28c83a-7415-4eb0-95a6-6245f2169d17
// IMPLEMENTED — {C} always; {U}/{B} on the second ability, which also creates
// the suppression it prints as a continuous effect with
// Duration::UntilYourNextUntapStep (the duration that ends after the step it
// suppresses).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ROOTWATER_DEPTHS,
    oracle_id = "2d28c83a-7415-4eb0-95a6-6245f2169d17",
    scryfall_id = "7186d86c-679b-4520-9cf0-5b25f6d98312",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Rootwater Depths", types = TypeSet::LAND,),],
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

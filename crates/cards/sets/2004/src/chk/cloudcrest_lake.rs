//! Cloudcrest Lake — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {W} or {U}. This land doesn't untap during your next untap step.
//! Set: CHK #274 — Champions of Kamigawa | Scryfall ID: 9bcbd030-762e-4754-ae95-685d59ccdfb9 | Oracle ID: 8df14d53-472c-416e-93c6-6c0b7f9b614e
// IMPLEMENTED — {C} on demand; {W} or {U} with the printed suppression, which
// is the continuous effect `Duration::UntilYourNextUntapStep` exists for.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CLOUDCREST_LAKE,
    oracle_id = "8df14d53-472c-416e-93c6-6c0b7f9b614e",
    scryfall_id = "9bcbd030-762e-4754-ae95-685d59ccdfb9",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Cloudcrest Lake", types = TypeSet::LAND,),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            Cost::TAP,
            &[
                Effect::mana_choice(&[ManaColor::White, ManaColor::Blue]),
                Effect::continuous(
                    &Filter::This,
                    Modifier::DoesNotUntap,
                    Duration::UntilYourNextUntapStep,
                ),
            ],
        ),
    ],
);

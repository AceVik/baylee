//! Lantern-Lit Graveyard — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {B} or {R}. This land doesn't untap during your next untap step.
//! Set: CHK #278 — Champions of Kamigawa | Scryfall ID: 484a7675-787f-49be-9b44-edd0a7d73812 | Oracle ID: 73a39a1b-2fb7-4328-8718-18569ae28e9e
// IMPLEMENTED — {C} always; the coloured ability registers the printed rider as
// Modifier::DoesNotUntap for Duration::UntilYourNextUntapStep on the source.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LANTERN_LIT_GRAVEYARD,
    oracle_id = "73a39a1b-2fb7-4328-8718-18569ae28e9e",
    scryfall_id = "484a7675-787f-49be-9b44-edd0a7d73812",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[face!(name = "Lantern-Lit Graveyard", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[
            Effect::mana_choice(&[ManaColor::Black, ManaColor::Red]),
            Effect::continuous(
                &Filter::This,
                Modifier::DoesNotUntap,
                Duration::UntilYourNextUntapStep,
            ),
        ]),
    ],
);

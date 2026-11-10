//! Pinecrest Ridge — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {R} or {G}. This land doesn't untap during your next untap step.
//! Set: CHK #281 — Champions of Kamigawa | Scryfall ID: 7900552f-6147-49ec-8c02-f253e4896c4d | Oracle ID: d8ef7c7b-0201-4978-ac73-fd376a19830f
// IMPLEMENTED — {C} always; the coloured half adds {R} or {G} and creates the
// suppression that ends at the controller's next untap step
// (Duration::UntilYourNextUntapStep).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PINECREST_RIDGE,
    oracle_id = "d8ef7c7b-0201-4978-ac73-fd376a19830f",
    scryfall_id = "7900552f-6147-49ec-8c02-f253e4896c4d",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces = &[face!(name = "Pinecrest Ridge", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[
            Effect::mana_choice(&[ManaColor::Red, ManaColor::Green]),
            Effect::continuous(
                &Filter::This,
                Modifier::DoesNotUntap,
                Duration::UntilYourNextUntapStep,
            ),
        ]),
    ],
);

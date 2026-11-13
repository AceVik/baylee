//! Tranquil Garden — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {G} or {W}. This land doesn't untap during your next untap step.
//! Set: CHK #284 — Champions of Kamigawa | Scryfall ID: 112b6577-fbd6-46dd-b77d-37df0abe9845 | Oracle ID: d9dfef08-b824-4d56-a0e9-3dcefb7e4612
// IMPLEMENTED — {C} always; {G} or {W} with the printed tap-down rider,
// created when the land is tapped (Duration::UntilYourNextUntapStep).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TRANQUIL_GARDEN,
    oracle_id = "d9dfef08-b824-4d56-a0e9-3dcefb7e4612",
    scryfall_id = "112b6577-fbd6-46dd-b77d-37df0abe9845",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Tranquil Garden", types = TypeSet::LAND,),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            Cost::TAP,
            &[
                Effect::mana_choice(&[ManaColor::Green, ManaColor::White]),
                Effect::continuous(
                    &Filter::This,
                    Modifier::DoesNotUntap,
                    Duration::UntilYourNextUntapStep
                ),
            ]
        ),
    ],
);

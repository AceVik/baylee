//! Vec Townships — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {G} or {W}. This land doesn't untap during your next untap step.
//! Set: TPR #247 — Tempest Remastered | Scryfall ID: 17579dbd-e8dd-49f0-83df-25dd78691c4a | Oracle ID: b0a4680f-9707-431c-b5d5-7d4424783602
// IMPLEMENTED — {C} from the first mana ability; {G} or {W} from the second,
// which also creates a DoesNotUntap effect on this land lasting through the
// controller's next untap step (Duration::UntilYourNextUntapStep).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VEC_TOWNSHIPS,
    oracle_id = "b0a4680f-9707-431c-b5d5-7d4424783602",
    scryfall_id = "17579dbd-e8dd-49f0-83df-25dd78691c4a",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(name = "Vec Townships", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[
            Effect::mana_choice(&[ManaColor::Green, ManaColor::White]),
            Effect::continuous(
                &Filter::This,
                Modifier::DoesNotUntap,
                Duration::UntilYourNextUntapStep,
            ),
        ]),
    ],
);

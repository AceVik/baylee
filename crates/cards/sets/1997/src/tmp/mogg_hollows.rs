//! Mogg Hollows — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {R} or {G}. This land doesn't untap during your next untap step.
//! Set: TPR #239 — Tempest Remastered | Scryfall ID: ee7f2031-1a91-4f9d-aae9-e4b1b3206211 | Oracle ID: 1745fd57-467c-45f9-a46e-b9a2af87ec87
// IMPLEMENTED — {C} always, and {R} or {G} at the price of staying tapped
// through the controller's next untap step (a created DoesNotUntap effect).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MOGG_HOLLOWS,
    oracle_id = "1745fd57-467c-45f9-a46e-b9a2af87ec87",
    scryfall_id = "ee7f2031-1a91-4f9d-aae9-e4b1b3206211",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces = &[face!(name = "Mogg Hollows", types = TypeSet::LAND,),],
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

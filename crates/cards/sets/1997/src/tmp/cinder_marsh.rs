//! Cinder Marsh — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {B} or {R}. This land doesn't untap during your next untap step.
//! Set: TPR #236 — Tempest Remastered | Scryfall ID: 8c6dc9af-ea7e-41f8-8c1e-22c588312053 | Oracle ID: 6f8cc374-e76c-4bfa-bf20-28dea0bfefbe
// IMPLEMENTED — a colorless mana ability and a {B}/{R} choice that also
// registers "doesn't untap during your next untap step" on itself.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CINDER_MARSH,
    oracle_id = "6f8cc374-e76c-4bfa-bf20-28dea0bfefbe",
    scryfall_id = "8c6dc9af-ea7e-41f8-8c1e-22c588312053",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Cinder Marsh", types = TypeSet::LAND,),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            Cost::TAP,
            &[
                Effect::mana_choice(&[ManaColor::Black, ManaColor::Red]),
                Effect::continuous(
                    &Filter::This,
                    Modifier::DoesNotUntap,
                    Duration::UntilYourNextUntapStep,
                ),
            ],
        ),
    ],
);

//! Thalakos Lowlands — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {W} or {U}. This land doesn't untap during your next untap step.
//! Set: TPR #246 — Tempest Remastered | Scryfall ID: 8522c51f-9126-41eb-84b5-f8489c7ad7ff | Oracle ID: 5a54d6a3-b1d0-42fe-9531-604b34d197f1
// IMPLEMENTED — {C} always; {W} or {U} on an activation that also keeps
// this land from untapping through the controller's next untap step.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THALAKOS_LOWLANDS,
    oracle_id = "5a54d6a3-b1d0-42fe-9531-604b34d197f1",
    scryfall_id = "8522c51f-9126-41eb-84b5-f8489c7ad7ff",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Thalakos Lowlands", types = TypeSet::LAND,),],
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

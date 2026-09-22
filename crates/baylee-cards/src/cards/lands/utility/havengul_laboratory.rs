//! Havengul Laboratory // Havengul Mystery — (no cost) — Legendary Land // Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}: Investigate. (Create a colorless Clue artifact token with "{2}, Sacrifice this artifact: Draw a card.")
//! Oracle: At the beginning of your end step, if you sacrificed three or more Clues this turn, transform Havengul Laboratory.
//! Oracle: When this land transforms into Havengul Mystery, return target creature card from your graveyard to the battlefield.
//! Oracle: When the creature put onto the battlefield with Havengul Mystery leaves the battlefield, transform Havengul Mystery.
//! Oracle: {T}, Pay 1 life: Add {B}.
//! Set: SLX #9 — Universes Within | Scryfall ID: 823b019e-10c0-4712-8167-d4f37a71e782 | Oracle ID: e71ac446-02a4-4468-8d29-f28b21617665
//! Face: Havengul Laboratory —  — Legendary Land
//! Face: Havengul Mystery —  — Legendary Land
// PARTIAL — {T}: Add {C} and {4}, {T}: Investigate on the front, {T}, Pay 1
// life: Add {B} on the back; the transform loop is NOT SUPPORTED, see the
// lines beside the abilities.

use crate::tokens::CLUE;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::HAVENGUL_LABORATORY,
    oracle_id = "e71ac446-02a4-4468-8d29-f28b21617665",
    scryfall_id = "823b019e-10c0-4712-8167-d4f37a71e782",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[
        face!(
            name = "Havengul Laboratory",
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
        ),
        face!(
            name = "Havengul Mystery",
            // CR 712.8c: a nonmodal double-faced card is cast as its front
            // face and reaches this one only by transforming.
            castable_from_hand = false,
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            abilities = &[mana_ability!(
                cost!(TapSelf, PayLife(1)),
                &[Effect::mana(ManaColor::Black, 1)]
            )],
        ),
    ],
    coverage = Coverage::Partial(
        "the transform loop: no Condition for \"you sacrificed three or more \
         Clues this turn\", no Effect that transforms a permanent, and no \
         Trigger for \"transforms into\""
    ),
    abilities = &[
        // {T}: Add {C}.
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // {4}, {T}: Investigate — one Clue.
        activated!(
            cost!("{4}", TapSelf),
            &[Effect::CreateToken { token: &CLUE }]
        ),
        // NOT SUPPORTED: "At the beginning of your end step, if you
        // sacrificed three or more Clues this turn, transform Havengul
        // Laboratory." — Condition counts control, graveyards, counters and
        // a filter on the source, never a sacrificed-this-turn tally, and no
        // Effect transforms a permanent.
        // NOT SUPPORTED (back face): "When this land transforms into
        // Havengul Mystery, return target creature card from your graveyard
        // to the battlefield." — Trigger has no transform event, so the
        // reanimation has nothing to hang on.
        // NOT SUPPORTED (back face): "When the creature put onto the
        // battlefield with Havengul Mystery leaves the battlefield,
        // transform Havengul Mystery." — nothing links "the creature put
        // onto the battlefield with this" back to the source.
    ],
);

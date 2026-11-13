//! Grindstone — {1} — Artifact
//! Oracle: {3}, {T}: Target player mills two cards. If two cards that share a color were milled this way, repeat this process.
//! Set: TPR #223 — Tempest Remastered | Scryfall ID: 0f15505d-b32f-4b71-8b99-f27bc964c5a8 | Oracle ID: 97560adc-814e-450c-9ca7-f9364e910a0b
// PARTIAL — Pays {3}, {T} for target player to mill two cards; repeating the process when two milled cards share a color is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GRINDSTONE,
    oracle_id = "97560adc-814e-450c-9ca7-f9364e910a0b",
    scryfall_id = "0f15505d-b32f-4b71-8b99-f27bc964c5a8",
    coverage = Coverage::Partial(
        "Pays {3}, {T} for target player to mill two cards; repeating the process when two milled cards share a color is not yet modeled."
    ),
    faces = &[face!(
        name = "Grindstone",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{3}", TapSelf),
        &[Effect::Mill {
            amount: Amount::Fixed(2),
            target: PlayerRel::Chosen,
        }],
        target = Some(TargetSpec::AnyPlayer),
    )],
);

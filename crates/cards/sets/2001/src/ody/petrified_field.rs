//! Petrified Field — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Sacrifice this land: Return target land card from your graveyard to your hand.
//! Set: ODY #323 — Odyssey | Scryfall ID: eaeaf9f2-d196-4607-a704-06f2315d8cc5 | Oracle ID: c4bc5bc4-e589-42c5-91fa-2ebc96448e85
// IMPLEMENTED — the {C} mana ability, plus the sacrifice ability returning a target
// land card from your graveyard to your hand (the target is the ability's own
// CardInGraveyard spec, which the effect reads back).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PETRIFIED_FIELD,
    oracle_id = "c4bc5bc4-e589-42c5-91fa-2ebc96448e85",
    scryfall_id = "eaeaf9f2-d196-4607-a704-06f2315d8cc5",
    faces = &[face!(name = "Petrified Field", types = TypeSet::LAND,)],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!(TapSelf, SacrificeSelf),
            &[Effect::GraveyardToHand {
                target: TargetSpec::CardInGraveyard(&Filter::LAND, PlayerRel::You),
            }],
            target = Some(TargetSpec::CardInGraveyard(&Filter::LAND, PlayerRel::You)),
        ),
    ],
);

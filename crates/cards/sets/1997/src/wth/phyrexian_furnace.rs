//! Phyrexian Furnace — {1} — Artifact
//! Oracle: {T}: Exile the bottom card of target player's graveyard.
//! Oracle: {1}, Sacrifice this artifact: Exile target card from a graveyard. Draw a card.
//! Set: WTH #155 — Weatherlight | Scryfall ID: e98bca31-8c05-430b-b5d7-331bdc55710a | Oracle ID: 4aafb7e7-b9fc-44ae-9c54-da11fd9dcc17
// PARTIAL — only "{1}, Sacrifice this artifact: Exile target card from a graveyard. Draw a card." NOT SUPPORTED: "{T}: Exile the bottom card of target player's graveyard." — a graveyard's bottom card cannot be named, and exiling a chosen card instead would be a different, stronger ability.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PHYREXIAN_FURNACE,
    oracle_id = "4aafb7e7-b9fc-44ae-9c54-da11fd9dcc17",
    scryfall_id = "e98bca31-8c05-430b-b5d7-331bdc55710a",
    coverage = Coverage::Partial(
        "only the sacrifice ability; the tap ability exiling the bottom card of target player's graveyard is not supported (a graveyard's bottom card cannot be named, and exiling a chosen card would be stronger than printed)",
    ),
    faces = &[face!(
        name = "Phyrexian Furnace",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        // NOT SUPPORTED: "{T}: Exile the bottom card of target player's graveyard." —
        // no way to name the bottom card of a graveyard.
        activated!(
            cost!("{1}", SacrificeSelf),
            &[
                Effect::Exile {
                    target: TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::EachPlayer),
                },
                Effect::draw(1),
            ],
            targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
                &Filter::Any,
                PlayerRel::EachPlayer,
            ))),
        ),
    ],
);

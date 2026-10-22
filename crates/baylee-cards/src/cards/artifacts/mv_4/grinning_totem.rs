//! Grinning Totem — {4} — Artifact
//! Oracle: {2}, {T}, Sacrifice this artifact: Search target opponent's library for a card and exile it. Then that player shuffles. Until the beginning of your next upkeep, you may play that card. At the beginning of your next upkeep, if you haven't played it, put it into its owner's graveyard.
//! Set: TSB #110 — Time Spiral Timeshifted | Scryfall ID: 1d8c8ecc-7637-424a-bae2-ed3b8678f97d | Oracle ID: fa798a3d-7797-4795-b251-a45f0429d169
// NOT SUPPORTED: searching target opponent's library to exile a card, letting you play it until your next upkeep, and putting it into its owner's graveyard if unplayed.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GRINNING_TOTEM,
    oracle_id = "fa798a3d-7797-4795-b251-a45f0429d169",
    scryfall_id = "1d8c8ecc-7637-424a-bae2-ed3b8678f97d",
    coverage = Coverage::Partial(
        "pays {2}, {T}, sacrifices Grinning Totem, and shuffles target opponent's library; searching that library to exile a card, letting you play it until your next upkeep, and putting it into its owner's graveyard if unplayed are not supported",
    ),
    faces = &[face!(
        name = "Grinning Totem",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{2}", TapSelf, SacrificeSelf),
        &[Effect::ShuffleLibrary {
            who: PlayerRel::Chosen,
        }],
        target = Some(TargetSpec::AnyOpponent),
    )],
);

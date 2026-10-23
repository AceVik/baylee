//! Storm Cauldron — {5} — Artifact
//! Oracle: Each player may play an additional land during each of their turns.
//! Oracle: Whenever a land is tapped for mana, return it to its owner's hand.
//! Set: 7ED #320 — Seventh Edition | Scryfall ID: 0bb5bdd3-6ecd-49cd-bfa2-e7da1ee85d88 | Oracle ID: 5a51b168-02d1-4eb4-8ccc-614ab6f6cffa

// PARTIAL — "Whenever a land is tapped for mana, return it to its owner's hand." for every player's lands; no additional land play.
// NOT SUPPORTED: "Each player may play an additional land during each of their turns." Modifier::ExtraLandDrops counts for the effect's controller only, so the extra land play would be Storm Cauldron's controller's alone while the printed card gives it to each player (the symmetry is missing and the card would play stronger than printed).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STORM_CAULDRON,
    oracle_id = "5a51b168-02d1-4eb4-8ccc-614ab6f6cffa",
    scryfall_id = "0bb5bdd3-6ecd-49cd-bfa2-e7da1ee85d88",
    faces = &[face!(
        name = "Storm Cauldron",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "Returns any land tapped for mana to its owner's hand; the additional land play of each player is not played, because Modifier::ExtraLandDrops counts for Storm Cauldron's controller only and would play stronger than printed."
    ),
    abilities = &[triggered!(
        Trigger::TappedForMana {
            by: PlayerRel::EachPlayer,
            filter: &Filter::LAND,
        },
        &[Effect::ReturnToHand {
            target: TargetSpec::EventObject,
        }]
    ),],
);

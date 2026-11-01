//! Lodestone Bauble — {0} — Artifact
//! Oracle: {1}, {T}, Sacrifice this artifact: Put up to four target basic land cards from a player's graveyard on top of their library in any order. That player draws a card at the beginning of the next turn's upkeep.
//! Set: ME2 #213 — Masters Edition II | Scryfall ID: 91e9021d-57fb-4d07-a7d3-45c435e8b3ef | Oracle ID: 6048b9ac-7d5e-4486-a89a-30adb45aab27
// PARTIAL — one target basic land card, from your own graveyard only, goes on top of your library and you draw at the next turn's upkeep.
// NOT SUPPORTED: "up to four target basic land cards from a player's graveyard ... in any order" moves only one card, and "that player draws a card" cannot name a player other than the ability's controller (a delayed trigger carries no chosen player), so a graveyard of another player is not offered.

use baylee_cards_dsl::prelude::*;

static BASIC_LAND: Filter = Filter::And(&[Filter::LAND, Filter::HasSupertype(SupertypeSet::BASIC)]);

card!(
    index = index::LODESTONE_BAUBLE,
    oracle_id = "6048b9ac-7d5e-4486-a89a-30adb45aab27",
    scryfall_id = "91e9021d-57fb-4d07-a7d3-45c435e8b3ef",
    coverage = Coverage::Partial(
        "Targeting up to four basic land cards from a player's graveyard and putting them on top in any order is not said (one card moves), and \"that player draws\" cannot name anyone but the controller, so only the controller's own graveyard is offered; in play, Lodestone Bauble puts one target basic land card from your graveyard on top of your library and you draw a card at the beginning of the next turn's upkeep.",
    ),
    faces = &[face!(
        name = "Lodestone Bauble",
        mana_cost = mana!("{0}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{1}", TapSelf, SacrificeSelf),
        &[
            Effect::GraveyardToTop {
                target: TargetSpec::CardInGraveyard(&BASIC_LAND, PlayerRel::You),
            },
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        target = Some(TargetSpec::CardInGraveyard(&BASIC_LAND, PlayerRel::You)),
    )],
);

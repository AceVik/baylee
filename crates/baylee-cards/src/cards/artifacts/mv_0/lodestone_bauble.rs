//! Lodestone Bauble — {0} — Artifact
//! Oracle: {1}, {T}, Sacrifice this artifact: Put up to four target basic land cards from a player's graveyard on top of their library in any order. That player draws a card at the beginning of the next turn's upkeep.
//! Set: ME2 #213 — Masters Edition II | Scryfall ID: 91e9021d-57fb-4d07-a7d3-45c435e8b3ef | Oracle ID: 6048b9ac-7d5e-4486-a89a-30adb45aab27
// NOT SUPPORTED: targeting up to four basic land cards from a single player's graveyard, ordering them on top of their library, and having that graveyard's owner draw a card at the next turn's upkeep has no single-ability DSL representation.

use baylee_cards_dsl::prelude::*;

static BASIC_LAND: Filter = Filter::And(&[Filter::LAND, Filter::HasSupertype(SupertypeSet::BASIC)]);

card!(
    index = index::LODESTONE_BAUBLE,
    oracle_id = "6048b9ac-7d5e-4486-a89a-30adb45aab27",
    scryfall_id = "91e9021d-57fb-4d07-a7d3-45c435e8b3ef",
    coverage = Coverage::Partial(
        "Targeting up to four basic land cards from a single player's graveyard, ordering them on top of their library, and having that graveyard's owner draw a card at the next turn's upkeep has no single-ability DSL representation; in play, Lodestone Bauble puts one target basic land card from a graveyard on top of its owner's library and its controller draws a card at the beginning of the next turn's upkeep.",
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
                target: TargetSpec::CardInGraveyard(&BASIC_LAND, PlayerRel::EachPlayer),
            },
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        target = Some(TargetSpec::CardInGraveyard(
            &BASIC_LAND,
            PlayerRel::EachPlayer
        )),
    )],
);

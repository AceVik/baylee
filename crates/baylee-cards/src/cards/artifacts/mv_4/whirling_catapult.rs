//! Whirling Catapult — {4} — Artifact
//! Oracle: {2}, Exile the top two cards of your library: This artifact deals 1 damage to each creature with flying and each player.
//! Set: ME2 #224 — Masters Edition II | Scryfall ID: 5a405387-3018-40c8-9836-c8f5ef7a1986 | Oracle ID: 0987b5e9-0012-4189-8745-45f10e9557f3
// NOT SUPPORTED: CostPart has no top-of-library exile cost, so the {2} ability deals 1 damage to each creature with flying and each player without exiling two cards from your library.

use baylee_cards_dsl::prelude::*;

static FLYING_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasKeyword(KeywordSet::FLYING),
]);

card!(
    index = index::WHIRLING_CATAPULT,
    oracle_id = "0987b5e9-0012-4189-8745-45f10e9557f3",
    scryfall_id = "5a405387-3018-40c8-9836-c8f5ef7a1986",
    coverage = Coverage::Partial(
        "The 'Exile the top two cards of your library' activation cost is not implemented; activating the {2} ability deals 1 damage to each creature with flying and each player.",
    ),
    faces = &[face!(
        name = "Whirling Catapult",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{2}"),
        &[
            Effect::DealDamageEach {
                amount: Amount::Fixed(1),
                filter: &FLYING_CREATURE,
            },
            Effect::DealDamage {
                target: TargetSpec::Player(PlayerRel::EachPlayer),
                amount: Amount::Fixed(1),
            },
        ],
    )],
);

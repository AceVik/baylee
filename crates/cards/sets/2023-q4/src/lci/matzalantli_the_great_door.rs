//! Matzalantli, the Great Door // The Core — {3} — Legendary Artifact // Legendary Land
//! Oracle: {T}: Draw a card, then discard a card.
//! Oracle: {4}, {T}: Transform Matzalantli. Activate only if there are four or more permanent types among cards in your graveyard. (Artifact, battle, creature, enchantment, land, and planeswalker are permanent types.)
//! Oracle: (Transforms from Matzalantli.)
//! Oracle: Fathomless descent — {T}: Add X mana of any one color, where X is the number of permanent cards in your graveyard.
//! Set: LCI #256 — The Lost Caverns of Ixalan | Scryfall ID: b4c31b29-06ba-436d-a3d9-18f4796c39be | Oracle ID: 16182e01-22ff-4786-985d-919b47c4aa4d
//! Face: Matzalantli, the Great Door — {3} — Legendary Artifact
//! Face: The Core —  — Legendary Land

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

static FATHOMLESS_DESCENT: &[AbilityDef] = &[mana_ability!(&[Effect::mana_choice_dynamic(
    ALL_MANA_COLORS,
    Amount::CountOf {
        filter: &Filter::HasType(
            TypeSet::ARTIFACT
                .union(TypeSet::BATTLE)
                .union(TypeSet::CREATURE)
                .union(TypeSet::ENCHANTMENT)
                .union(TypeSet::LAND)
                .union(TypeSet::PLANESWALKER),
        ),
        zone: ZoneSel::GraveyardYou,
    },
)])];

card!(
    index = index::MATZALANTLI_THE_GREAT_DOOR,
    oracle_id = "16182e01-22ff-4786-985d-919b47c4aa4d",
    scryfall_id = "b4c31b29-06ba-436d-a3d9-18f4796c39be",
    faces = &[
        face!(
            name = "Matzalantli, the Great Door",
            mana_cost = mana!("{3}"),
            types = TypeSet::ARTIFACT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
        face!(
            name = "The Core",
            castable_from_hand = false,
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            abilities = FATHOMLESS_DESCENT,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[
        activated!(
            Cost::TAP,
            &[
                Effect::draw(1),
                Effect::DiscardForPlayers {
                    who: PlayerRel::You,
                    count: 1,
                },
            ],
        ),
        activated!(
            cost!("{4}", TapSelf),
            &[Effect::TransformSource],
            condition = Some(Condition::GraveyardTypesAmongAtLeast {
                types: TypeSet::ARTIFACT
                    .union(TypeSet::BATTLE)
                    .union(TypeSet::CREATURE)
                    .union(TypeSet::ENCHANTMENT)
                    .union(TypeSet::LAND)
                    .union(TypeSet::PLANESWALKER),
                minimum: 4,
            })
        ),
    ],
);

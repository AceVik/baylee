//! Eden, Seat of the Sanctum — (no cost) — Land — Town
//! Oracle: {T}: Add {C}.
//! Oracle: {5}, {T}: Mill two cards. Then you may sacrifice this land. When you do, return another target permanent card from your graveyard to your hand.
//! Set: FIN #277 — Final Fantasy | Scryfall ID: e28eac1e-adc7-4f8d-b206-bef09ba07d38 | Oracle ID: 84856b92-5ce8-47f3-9a1c-78d6a3e26aca
// IMPLEMENTED — `{T}: Add {C}`, and a `{5}, {T}` ability that mills two and
// may sacrifice the land. The return is a reflexive triggered ability
// (CR 603.12): it exists only if the land was sacrificed, and it chooses its
// target as it goes on the stack, after the mill, so a card the mill just put
// into the graveyard can be the one returned.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// "another target permanent card from your graveyard": a permanent card is
// an artifact, battle, creature, enchantment, land or planeswalker card
// (CR 110.4a), and "another" leaves Eden itself out, which is in the
// graveyard by then.
static ANOTHER_PERMANENT_CARD: Filter = Filter::And(&[
    Filter::HasType(
        TypeSet::ARTIFACT
            .union(TypeSet::BATTLE)
            .union(TypeSet::CREATURE)
            .union(TypeSet::ENCHANTMENT)
            .union(TypeSet::LAND)
            .union(TypeSet::PLANESWALKER),
    ),
    Filter::Another,
]);
static RETURNED: TargetSpec = TargetSpec::CardInGraveyard(&ANOTHER_PERMANENT_CARD, PlayerRel::You);

card!(
    index = index::EDEN_SEAT_OF_THE_SANCTUM,
    oracle_id = "84856b92-5ce8-47f3-9a1c-78d6a3e26aca",
    scryfall_id = "e28eac1e-adc7-4f8d-b206-bef09ba07d38",
    faces = &[face!(
        name = "Eden, Seat of the Sanctum",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::TOWN],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{5}", TapSelf),
            &[
                Effect::Mill {
                    amount: Amount::Fixed(2),
                    target: PlayerRel::You,
                },
                Effect::MayDo {
                    effects: &[Effect::SacrificeSelf],
                },
                Effect::Reflexive {
                    when: ReflexiveEvent::SacrificedThis,
                    effects: &[Effect::GraveyardToHand { target: RETURNED }],
                    target: Some(RETURNED),
                },
            ]
        ),
    ],
);

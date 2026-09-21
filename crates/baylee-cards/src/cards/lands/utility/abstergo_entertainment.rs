//! Abstergo Entertainment — (no cost) — Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add one mana of any color.
//! Oracle: {3}, {T}, Exile Abstergo Entertainment: Return up to one target historic card from your graveyard to your hand, then exile all graveyards. (Artifacts, legendaries, and Sagas are historic.)
//! Set: ACR #79 — Assassin's Creed | Scryfall ID: 4d197866-7633-493c-80dd-ec3a09165934 | Oracle ID: d06a8026-1657-4404-8dff-64e44f1a14f8
// PARTIAL — {T}: Add {C}; {1}, {T}: Add one mana of any color; and the {3}
// ability (exile the land as a cost, return a historic card, then exile every
// graveyard), written with the one target it needs.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::enchantment;

/// "Historic" (CR 700.6): an artifact, a legendary, or a Saga.
static HISTORIC: Filter = Filter::Or(&[
    Filter::HasType(TypeSet::ARTIFACT),
    Filter::HasSupertype(SupertypeSet::LEGENDARY),
    Filter::HasSubtype(enchantment::SAGA),
]);

card!(
    index = index::ABSTERGO_ENTERTAINMENT,
    oracle_id = "d06a8026-1657-4404-8dff-64e44f1a14f8",
    scryfall_id = "4d197866-7633-493c-80dd-ec3a09165934",
    faces = &[face!(
        name = "Abstergo Entertainment",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Partial(
        "AbilityDef::Activated carries a bare TargetSpec, which the engine \
         reads as exactly one target, so \"return up to one target historic \
         card from your graveyard\" is offered only while a historic card is \
         there",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(cost!("{1}", TapSelf), &[Effect::mana_of_any_color()]),
        // NOT SUPPORTED: "up to one target" — an activated ability's target
        // is exactly one (a minimum of zero needs a `TargetReq`, which only
        // spells, triggers and loyalty abilities take), so this ability
        // cannot be activated on an empty graveyard.
        activated!(
            cost!("{3}", TapSelf, ExileSelf),
            &[
                Effect::GraveyardToHand {
                    target: TargetSpec::CardInGraveyard(&HISTORIC, PlayerRel::You),
                },
                Effect::ExileGraveyard {
                    player: PlayerRel::EachPlayer,
                },
            ],
            target = Some(TargetSpec::CardInGraveyard(&HISTORIC, PlayerRel::You)),
        ),
    ],
);

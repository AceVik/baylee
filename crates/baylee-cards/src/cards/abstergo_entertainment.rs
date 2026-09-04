//! Abstergo Entertainment — (no cost) — Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add one mana of any color.
//! Oracle: {3}, {T}, Exile Abstergo Entertainment: Return up to one target historic card from your graveyard to your hand, then exile all graveyards. (Artifacts, legendaries, and Sagas are historic.)
//! Set: ACR #79 — Assassin's Creed | Scryfall ID: 4d197866-7633-493c-80dd-ec3a09165934 | Oracle ID: d06a8026-1657-4404-8dff-64e44f1a14f8
// IMPLEMENTED — colorless and filter mana + {3}, tap, exile self to return historic card from graveyard to hand and exile all graveyards.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::enchantment;

static HISTORIC: Filter = Filter::Or(&[
    Filter::ARTIFACT,
    Filter::HasSupertype(SupertypeSet::LEGENDARY),
    Filter::HasSubtype(enchantment::SAGA),
]);

card! {
    index: 200,
    oracle_id: "d06a8026-1657-4404-8dff-64e44f1a14f8",
    scryfall_id: "4d197866-7633-493c-80dd-ec3a09165934",
    coverage: Coverage::Implemented,
    faces: &[
    face! {
        name: "Abstergo Entertainment",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
    },
    ],
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            Cost {
                mana: baylee_core::mana!("{1}"),
                parts: &[CostPart::TapSelf],
            },
            &[Effect::mana_of_any_color()],
        ),
        activated!(
            Cost {
                mana: baylee_core::mana!("{3}"),
                parts: &[CostPart::TapSelf, CostPart::ExileSelf],
            },
            &[
                Effect::GraveyardToHand {
                    target: TargetSpec::CardInGraveyard(&HISTORIC, PlayerRel::You),
                },
                Effect::ExileGraveyard {
                    player: PlayerRel::EachPlayer,
                },
            ],
            target: Some(TargetSpec::CardInGraveyard(&HISTORIC, PlayerRel::You)),
        ),
    ],
}

// Engine-level test lives in baylee-engine: activation pays {3}, tap and
// exiles self, targets a historic card in your graveyard to return to hand, and exiles all graveyards.

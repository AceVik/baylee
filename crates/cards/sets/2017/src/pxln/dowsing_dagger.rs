//! Dowsing Dagger // Lost Vale — {2} — Artifact — Equipment // Land
//! Oracle: When this Equipment enters, target opponent creates two 0/2 green Plant creature tokens with defender.
//! Oracle: Equipped creature gets +2/+1.
//! Oracle: Whenever equipped creature deals combat damage to a player, you may transform this Equipment.
//! Oracle: Equip {2}
//! Oracle: (Transforms from Dowsing Dagger.)
//! Oracle: {T}: Add three mana of any one color.
//! Set: XLN #235 — Ixalan | Scryfall ID: 514d53be-6ade-4f73-a844-e9ae2dafd6ce | Oracle ID: df34a6ad-ae1c-4470-8c9e-49815bba1973
//! Face: Dowsing Dagger — {2} — Artifact — Equipment
//! Face: Lost Vale —  — Land

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana_choice_dynamic(
    ALL_MANA_COLORS,
    Amount::Fixed(3)
)])];

card!(
    index = index::DOWSING_DAGGER,
    oracle_id = "df34a6ad-ae1c-4470-8c9e-49815bba1973",
    scryfall_id = "514d53be-6ade-4f73-a844-e9ae2dafd6ce",
    faces = &[
        face!(
            name = "Dowsing Dagger",
            mana_cost = mana!("{2}"),
            types = TypeSet::ARTIFACT,
            subtypes = &[subtypes::artifact::EQUIPMENT],
        ),
        face!(
            name = "Lost Vale",
            castable_from_hand = false,
            types = TypeSet::LAND,
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(2, 1)),
        triggered!(
            Trigger::DealsCombatDamageToPlayer(&Filter::AttachedToBySource),
            &[Effect::MayDo {
                effects: &[Effect::TransformSource],
            }]
        ),
        equip!("{2}"),
        triggered!(
            Trigger::ETB,
            &[Effect::CreateTokensFor {
                player: PlayerRel::Chosen,
                token: &generated_tokens::PLANT_0_2_GREEN_DEFENDER,
                amount: Amount::Fixed(2)
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyOpponent))
        ),
    ],
);

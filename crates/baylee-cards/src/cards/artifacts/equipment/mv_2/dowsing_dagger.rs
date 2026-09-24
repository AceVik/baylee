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
// PARTIAL — the equip bonus and Lost Vale's three mana are built. The
// combat-damage trigger is written as an exile-and-return, not a transform
// (#206), and the enters-trigger is not written (see NOT SUPPORTED).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// Lost Vale's `{T}: Add three mana of any one color.` — one pick of a
/// colour, three mana of it, which is the shape `mana_choice_dynamic` is
/// for and not the "in any combination" one.
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
            // CR 712.8c: a nonmodal double-faced card is cast as its front
            // face and reaches this one only by transforming.
            castable_from_hand = false,
            types = TypeSet::LAND,
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Partial(
        "enters-trigger: no effect creates tokens under a target opponent (CreateTokenForTargetController reads an object target's controller, not a chosen player); and the transform to Lost Vale is an exile-and-return, not a transform (#206)",
    ),
    // NOT SUPPORTED: When this Equipment enters, target opponent creates two 0/2 green Plant creature tokens with defender. — the token exists (`PLANT_0_2_GREEN_DEFENDER`), but `CreateTokenForTargetController` reads an object target's controller rather than a target player, so the Plants would be yours.
    abilities = &[
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(2, 1)),
        // NOT SUPPORTED (#206): "you may transform it" — ExileSelfReturnAsFace
        // exiles the Equipment and returns Lost Vale as a new object, which
        // enters; a transform turns the same permanent over (CR 701.27a).
        triggered!(
            Trigger::DealsCombatDamageToPlayer(&Filter::AttachedToBySource),
            &[Effect::MayDo {
                effects: &[Effect::ExileSelfReturnAsFace { face: 1 }],
            }]
        ),
        equip!("{2}"),
    ],
);

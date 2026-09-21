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
// PARTIAL — the equip bonus, the combat-damage transform trigger and Lost
// Vale's three mana are built; the enters-trigger is not (see NOT SUPPORTED).

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
            types = TypeSet::LAND,
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Partial(
        "enters-trigger: no 0/2 green Plant token with defender is in the token registry, and no effect creates tokens under a chosen player",
    ),
    // NOT SUPPORTED: When this Equipment enters, target opponent creates two 0/2 green Plant creature tokens with defender. — the token does not exist in `crate::tokens`/`generated_tokens`, and `CreateTokenForTargetController` reads an object target's controller rather than the `AnyOpponent` seat.
    abilities = &[
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(2, 1)),
        triggered!(
            Trigger::DealsCombatDamageToPlayer(&Filter::AttachedToBySource),
            &[Effect::MayDo {
                effects: &[Effect::ExileSelfReturnAsFace { face: 1 }],
            }]
        ),
        equip!("{2}"),
    ],
);

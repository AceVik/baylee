//! Soul Burn — {X}{2}{B} — Sorcery
//! Oracle: Spend only black and/or red mana on X.
//! Oracle: Soul Burn deals X damage to any target. You gain life equal to the damage dealt, but not more than the amount of {B} spent on X, the player's life total before the damage was dealt, the planeswalker's loyalty before the damage was dealt, or the creature's toughness.
//! Set: INV #124 — Invasion | Scryfall ID: 70515cd2-97d5-4491-a758-bc7188fdc6dc | Oracle ID: 063b0f5d-af27-4681-87b9-b553f6887061
// PARTIAL — X can be paid with black mana only (printed black and/or red); with black mana only, the life gain "not more than the amount of {B} spent on X" never binds.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SOUL_BURN,
    oracle_id = "063b0f5d-af27-4681-87b9-b553f6887061",
    scryfall_id = "70515cd2-97d5-4491-a758-bc7188fdc6dc",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "X can be paid with black mana only (printed black and/or red), so the {B}-spent-on-X life-gain cap never binds; red mana cannot pay X",
    ),
    faces = &[face!(
        name = "Soul Burn",
        mana_cost = mana!("{X}{2}{B}"),
        x_mana_color = Some(Color::Black),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[
        // PARTIAL: "Spend only black and/or red mana on X" is read as black only
        // (`x_mana_color` names one colour), which is a subset of the printed
        // payments. Every {B} on X then equals X, so the printed life-gain cap
        // "not more than the amount of {B} spent on X" never binds before the
        // damage-based caps do.
        // CR 120.3 / CR 119.1: Deals X damage to any target and gains life capped by the target's prior life/loyalty/toughness.
        spell!(
            &[Effect::DealDamageWithCappedLifeGain { amount: Amount::X }],
            targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
        ),
    ],
);

//! Soul Burn — {X}{2}{B} — Sorcery
//! Oracle: Spend only black and/or red mana on X.
//! Oracle: Soul Burn deals X damage to any target. You gain life equal to the damage dealt, but not more than the amount of {B} spent on X, the player's life total before the damage was dealt, the planeswalker's loyalty before the damage was dealt, or the creature's toughness.
//! Set: INV #124 — Invasion | Scryfall ID: 70515cd2-97d5-4491-a758-bc7188fdc6dc | Oracle ID: 063b0f5d-af27-4681-87b9-b553f6887061

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SOUL_BURN,
    oracle_id = "063b0f5d-af27-4681-87b9-b553f6887061",
    scryfall_id = "70515cd2-97d5-4491-a758-bc7188fdc6dc",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Soul Burn",
        mana_cost = mana!("{X}{2}{B}"),
        x_mana_colors = Some(ColorSet::from_slice(&[Color::Black, Color::Red])),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[
        // Cap by black mana paid for X and actual damage, then by current
        // creature toughness or the target's prior life or loyalty.
        spell!(
            &[Effect::DealDamageWithLimitedLifeGain {
                amount: Amount::X,
                life_limit: Amount::ManaTypeSpentOnX(ManaColor::Black),
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
        ),
    ],
);

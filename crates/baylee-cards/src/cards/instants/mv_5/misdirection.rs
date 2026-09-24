//! Misdirection — {3}{U}{U} — Instant
//! Oracle: You may exile a blue card from your hand rather than pay this spell's mana cost.
//! Oracle: Change the target of target spell with a single target.
//! Set: DDT #15 — Duel Decks: Merfolk vs. Goblins | Scryfall ID: c96763d6-0cea-40ed-afb2-886bfebe50a0 | Oracle ID: c39e5fb0-6de3-4105-ad3c-0ecb8951a1d5
// PARTIAL — pitch cast + target redirection (CR 115.7a).
// NOT SUPPORTED: "with a single target" (CR 115.9a, #249). No `Filter` asks a
// spell how many targets it has, so a spell with two is a legal target too;
// `Effect::ChangeTarget` then moves both or neither.

static BLUE_CARD: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Blue]));

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MISDIRECTION,
    oracle_id = "c39e5fb0-6de3-4105-ad3c-0ecb8951a1d5",
    scryfall_id = "c96763d6-0cea-40ed-afb2-886bfebe50a0",
    faces = &[face!(
        name = "Misdirection",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::INSTANT,
        alternative_costs = &[AlternativeCost {
            cost: cost!(ExileFromHand(&BLUE_CARD)),
            condition: AltCondition::Always,
        }],
    )],
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial("cannot restrict the target to a spell with a single target"),
    abilities = &[spell!(
        &[Effect::ChangeTarget { to: &Filter::Any }],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
    )],
);

//! Firestorm — {R} — Instant
//! Oracle: As an additional cost to cast this spell, discard X cards.
//! Oracle: Firestorm deals X damage to each of X targets.
//! Set: WTH #101 — Weatherlight | Scryfall ID: e674aa8a-668a-4345-95ee-73a0b87bbcb1 | Oracle ID: 157813c8-af83-41b5-9ffb-4abd167c1bfc
// NOT SUPPORTED: announcing X without {X} in the mana cost to discard X cards and deal X damage to each of X targets requires a CostPart::DiscardX variant.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FIRESTORM,
    oracle_id = "157813c8-af83-41b5-9ffb-4abd167c1bfc",
    scryfall_id = "e674aa8a-668a-4345-95ee-73a0b87bbcb1",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "discards a card as an additional cost to cast and deals 1 damage to any target; announcing X without {X} in the mana cost to discard X cards and deal X damage to each of X targets requires a CostPart::DiscardX variant",
    ),
    faces = &[face!(
        name = "Firestorm",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
        mandatory_additional_costs = &[CostPart::Discard(&Filter::Any)],
    ),],
    abilities = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )],
);

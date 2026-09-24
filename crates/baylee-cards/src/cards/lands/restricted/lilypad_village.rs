//! Lilypad Village — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {U}. Spend this mana only to cast a creature spell.
//! Oracle: {U}, {T}: Surveil 2. Activate only if a Bird, Frog, Otter, or Rat entered the battlefield under your control this turn.
//! Set: BLB #255 — Bloomburrow | Scryfall ID: 7e95a7cc-ed77-4ca4-80db-61c0fc68bf50 | Oracle ID: 5bb06e6f-e3af-4caa-b66d-77248ad46b61
// PARTIAL — both mana abilities, the {U} one restricted to creature spells; the surveil ability is omitted.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LILYPAD_VILLAGE,
    oracle_id = "5bb06e6f-e3af-4caa-b66d-77248ad46b61",
    scryfall_id = "7e95a7cc-ed77-4ca4-80db-61c0fc68bf50",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(name = "Lilypad Village", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the {U}, {T} ability's activation condition — \"a Bird, Frog, Otter, or Rat entered the battlefield under your control this turn\" — is a record of a past event no `Condition` reads, and ControlCount over EnteredThisTurn would miss a creature that entered under your control and has since left the battlefield, so the ability is left off"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[
            Effect::mana(ManaColor::Blue, 1).restricted(&Filter::CREATURE, SpendRider::None),
        ]),
        // NOT SUPPORTED: {U}, {T}: Surveil 2. Activate only if a Bird, Frog, Otter, or Rat entered the battlefield under your control this turn.
    ],
);

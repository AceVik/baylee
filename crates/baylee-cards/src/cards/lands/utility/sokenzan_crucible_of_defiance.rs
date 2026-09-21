//! Sokenzan, Crucible of Defiance — (no cost) — Legendary Land
//! Oracle: {T}: Add {R}.
//! Oracle: Channel — {3}{R}, Discard this card: Create two 1/1 colorless Spirit creature tokens. They gain haste until end of turn. This ability costs {1} less to activate for each legendary creature you control.
//! Set: NEO #276 — Kamigawa: Neon Dynasty | Scryfall ID: aa548dcd-c1dd-492d-a69f-c65dfeef0633 | Oracle ID: c5ee72d5-3a9e-4fe5-8802-3286ee612055
// PARTIAL — the mana ability only; the channel ability is not expressible.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: Channel — {3}{R}, Discard this card: Create two 1/1 colorless
// Spirit creature tokens. They gain haste until end of turn. This ability costs
// {1} less to activate for each legendary creature you control.
// Two clauses have no variant. The reduction is counted and `CostReduction`
// prints only `NotStartingPlayer`, so the ability cannot be charged what it
// costs. And "They gain haste" reaches the two tokens this resolution made and
// nothing else, which no filter can name — a `PumpFilter` would have to say
// `Filter::YOUR_CREATURE` and hand haste to the whole board. The ability comes
// off the card rather than being written at its printed {3}{R}, which would tax
// a board of legendary creatures that is supposed to pay less.

card!(
    index = index::SOKENZAN_CRUCIBLE_OF_DEFIANCE,
    oracle_id = "c5ee72d5-3a9e-4fe5-8802-3286ee612055",
    scryfall_id = "aa548dcd-c1dd-492d-a69f-c65dfeef0633",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Sokenzan, Crucible of Defiance",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Partial(
        "the channel ability: its {1}-less reduction per legendary creature you control, and the haste it grants the tokens it creates, are both inexpressible"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])],
);

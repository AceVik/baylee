//! Northampton Farm — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Exile target creature you own.
//! Oracle: {2}, {T}, Sacrifice this land: Return a creature card exiled with this land to the battlefield under your control. Return each other card exiled with this land to its owner's hand.
//! Set: TMT #188 — Teenage Mutant Ninja Turtles | Scryfall ID: dbca168e-095f-4fbc-88f8-3048d83caf94 | Oracle ID: e05f1a43-16ce-4e88-b0ad-2202efb25516
// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

static OWNED_CREATURE: Filter = f!(owned CREATURE);

card!(
    index = index::NORTHAMPTON_FARM,
    oracle_id = "e05f1a43-16ce-4e88-b0ad-2202efb25516",
    scryfall_id = "dbca168e-095f-4fbc-88f8-3048d83caf94",
    faces = &[face!(name = "Northampton Farm", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{1}", TapSelf),
            &[Effect::exile_linked(TargetSpec::Object(&OWNED_CREATURE))],
            target = Some(TargetSpec::Object(&OWNED_CREATURE)),
        ),
        activated!(
            cost!("{2}", TapSelf, SacrificeSelf),
            &[Effect::ReturnChosenLinkedToBattlefield {
                filter: &Filter::CREATURE,
                other_to_hand: true,
            }],
        ),
    ],
);

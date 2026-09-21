//! Northampton Farm — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Exile target creature you own.
//! Oracle: {2}, {T}, Sacrifice this land: Return a creature card exiled with this land to the battlefield under your control. Return each other card exiled with this land to its owner's hand.
//! Set: TMT #188 — Teenage Mutant Ninja Turtles | Scryfall ID: dbca168e-095f-4fbc-88f8-3048d83caf94 | Oracle ID: e05f1a43-16ce-4e88-b0ad-2202efb25516
// PARTIAL — {T}: Add {C} and the {1}, {T} linked exile are built; the return
// clause is not expressible in the DSL, see NOT SUPPORTED below.

use baylee_cards_dsl::prelude::*;

static OWNED_CREATURE: Filter = f!(owned CREATURE);

card!(
    index = index::NORTHAMPTON_FARM,
    oracle_id = "e05f1a43-16ce-4e88-b0ad-2202efb25516",
    scryfall_id = "dbca168e-095f-4fbc-88f8-3048d83caf94",
    faces = &[face!(name = "Northampton Farm", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the return clause puts one creature card exiled with this land onto \
         the battlefield under your control and each other card exiled with \
         it into its owner's hand; Effect::ReturnLinkedToBattlefield returns \
         every linked card to the battlefield under its owner's control, and \
         there is no variant that picks one for the battlefield and sends \
         the rest to hand"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{1}", TapSelf),
            &[Effect::ExileLinked {
                target: TargetSpec::Object(&OWNED_CREATURE),
            }],
            target = Some(TargetSpec::Object(&OWNED_CREATURE)),
        ),
        // NOT SUPPORTED: "{2}, {T}, Sacrifice this land: Return a creature
        // card exiled with this land to the battlefield under your control.
        // Return each other card exiled with this land to its owner's hand."
        // — the nearest variant (Effect::ReturnLinkedToBattlefield) returns
        // every card exiled with the source to the battlefield under its
        // owner's control instead of one creature card under this ability's
        // controller and each other card to its owner's hand.
    ],
);

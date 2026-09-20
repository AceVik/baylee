//! Avengers Tower — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a Hero spell or to activate an ability of a Hero source.
//! Oracle: {4}, {T}: Look at the top three cards of your library. You may reveal a Hero card from among them and put it into your hand. Put the rest on the bottom of your library in any order.
//! Set: MSH #260 — Marvel Super Heroes | Scryfall ID: 88f0d9c9-8a1f-4b5a-b6f9-821ddd658d27 | Oracle ID: c5fc8e7c-a87e-4586-a13c-d30e0a3aafbf
// IMPLEMENTED — {T}: Add {C}. Both Hero clauses are NOT SUPPORTED below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AVENGERS_TOWER,
    oracle_id = "c5fc8e7c-a87e-4586-a13c-d30e0a3aafbf",
    scryfall_id = "88f0d9c9-8a1f-4b5a-b6f9-821ddd658d27",
    faces = &[face!(name = "Avengers Tower", types = TypeSet::LAND,)],
    coverage = Coverage::Partial(
        "the Hero spend rider on the any-color mana, and the Hero-only pick in the {4} ability",
    ),
    abilities = &[
        // NOT SUPPORTED: "{T}: Add one mana of any color. Spend this mana only
        // to cast a Hero spell or to activate an ability of a Hero source." —
        // `ManaRestriction` restricts *spells* ("the mana is spendable only on
        // spells matching this"), so the printed clause's second half — "or to
        // activate an ability of a Hero source" — has no shape at all; and
        // pool mana carries no provenance, so the mana would leave the
        // restriction unenforced. The ability is left off rather than written
        // as unrestricted any-color mana, which would be strictly stronger
        // than the printed card.
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{4}, {T}: Look at the top three cards of your
        // library. You may reveal a Hero card from among them and put it into
        // your hand. Put the rest on the bottom of your library in any
        // order." — `Effect::LookAtTopPick { count, pick }` keeps `pick` of
        // the cards looked at and carries no filter, so the pick could not be
        // held to the Hero type: it would hand over any of the three.
    ],
);

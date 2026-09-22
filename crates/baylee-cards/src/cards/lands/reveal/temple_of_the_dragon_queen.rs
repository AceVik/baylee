//! Temple of the Dragon Queen — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Dragon card from your hand. This land enters tapped unless you revealed a Dragon card this way or you control a Dragon.
//! Oracle: As this land enters, choose a color.
//! Oracle: {T}: Add one mana of the chosen color.
//! Set: TDC #104 — Tarkir: Dragonstorm Commander | Scryfall ID: 91658f56-12c9-4173-94ad-dfd186b1dbae | Oracle ID: 169a26d2-7bc9-4403-9c92-98d4bd5ca4f3
// PARTIAL — the chosen colour (EnterModifier::ChooseColor plus
// Effect::mana_chosen) and the enters-tapped check against a Dragon you
// control, which is the half of the entry condition the DSL can say.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

// NOT SUPPORTED: "tapped unless you revealed a Dragon card this way **or**
// you control a Dragon." `EnterModifier::TappedUnlessReveal` can now say the
// reveal, and `TappedUnless` says the control half, but the clause is a
// *disjunction* and a face carries a list: every arm of the entry scan only
// ever inserts `TAPPED`, so two modifiers side by side are an `and` — this
// land would come down tapped unless both were satisfied, which is stricter
// than the card. The second obstacle is in the same sentence: the reveal
// asks a question and so does `ChooseColor`, and the scan answers the first
// one only.

card!(
    index = index::TEMPLE_OF_THE_DRAGON_QUEEN,
    oracle_id = "169a26d2-7bc9-4403-9c92-98d4bd5ca4f3",
    scryfall_id = "91658f56-12c9-4173-94ad-dfd186b1dbae",
    faces = &[face!(
        name = "Temple of the Dragon Queen",
        types = TypeSet::LAND,
        enter_modifiers = &[
            EnterModifier::TappedUnless(&f!(your Filter::HasSubtype(creature::DRAGON))),
            EnterModifier::ChooseColor,
        ],
    ),],
    coverage = Coverage::Partial(
        "the entry clause is a disjunction (revealed a Dragon or control one) and a face's modifier list is a conjunction",
    ),
    abilities = &[mana_ability!(&[Effect::mana_chosen()])],
);

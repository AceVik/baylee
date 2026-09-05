//! Castle Ardenvale — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Plains.
//! Oracle: {T}: Add {W}.
//! Oracle: {2}{W}{W}, {T}: Create a 1/1 white Human creature token.
//! Set: TDC #346 — Tarkir: Dragonstorm Commander | Scryfall ID: 65e4de2e-47d2-4967-be31-9df0057a9c74 | Oracle ID: f8f4fc60-725d-46d8-8e8f-e68e00d20589
// PARTIAL — TappedUnless(Plains) + {T}: Add {W} implemented; Human token
// inexpressible: no HUMAN_1_1_WHITE token in crate::tokens and the
// one-file constraint prevents adding it.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static CHECK: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::LAND,
    Filter::HasSubtype(subtypes::land::PLAINS),
]);

card! {
    index: 332,
    oracle_id: "f8f4fc60-725d-46d8-8e8f-e68e00d20589",
    scryfall_id: "65e4de2e-47d2-4967-be31-9df0057a9c74",
    color_identity: ColorSet::from_slice(&[Color::White]),
    coverage: Coverage::Partial("no HUMAN_1_1_WHITE token in crate::tokens; {2}{W}{W},{T}: Create a 1/1 white Human creature token is unimplemented"),
    faces: &[
    face! {
        name: "Castle Ardenvale",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::TappedUnless(&CHECK)],
    },
    ],
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        // NOT SUPPORTED: no HUMAN_1_1_WHITE token defined in crate::tokens;
        // add `pub static HUMAN_1_1_WHITE: TokenDef` there, then replace this
        // comment with:
        //   activated!(Cost { mana: baylee_core::mana!("{2}{W}{W}"),
        //       parts: &[CostPart::TapSelf] },
        //       &[Effect::CreateToken { token: &crate::tokens::HUMAN_1_1_WHITE }])
    ],
}

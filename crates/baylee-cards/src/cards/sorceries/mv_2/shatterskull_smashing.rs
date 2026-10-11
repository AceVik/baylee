//! Shatterskull Smashing // Shatterskull, the Hammer Pass — {X}{R}{R} — Sorcery // Land
//! Oracle: Shatterskull Smashing deals X damage divided as you choose among up to two target creatures and/or planeswalkers. If X is 6 or more, Shatterskull Smashing deals twice X damage divided as you choose among them instead.
//! Oracle: As this land enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {R}.
//! Set: ZNR #161 — Zendikar Rising | Scryfall ID: bc7239ea-f8aa-4a6f-87bd-c35359635673 | Oracle ID: 78301998-fd9b-4cd5-afad-dbcb43cac2a7
//! Face: Shatterskull Smashing — {X}{R}{R} — Sorcery
//! Face: Shatterskull, the Hammer Pass —  — Land

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHATTERSKULL_SMASHING,
    oracle_id = "78301998-fd9b-4cd5-afad-dbcb43cac2a7",
    scryfall_id = "bc7239ea-f8aa-4a6f-87bd-c35359635673",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[
        face!(
            name = "Shatterskull Smashing",
            mana_cost = mana!("{X}{R}{R}"),
            types = TypeSet::SORCERY,
            abilities = &[spell!(
                &[Effect::DealDamageDivided {
                    amount: DivisionAmount::XDoubleFrom(6)
                }],
                targets = Some(TargetReq::up_to(
                    TargetSpec::Object(&Filter::CREATURE_OR_PLANESWALKER),
                    2
                )),
            )],
        ),
        face!(
            name = "Shatterskull, the Hammer Pass",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::TappedOrPayLife(3)],
            abilities = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])],
        ),
    ],
    coverage = Coverage::Implemented,
);

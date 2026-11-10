//! Sokenzan, Crucible of Defiance — (no cost) — Legendary Land
//! Oracle: {T}: Add {R}.
//! Oracle: Channel — {3}{R}, Discard this card: Create two 1/1 colorless Spirit creature tokens. They gain haste until end of turn. This ability costs {1} less to activate for each legendary creature you control.
//! Set: NEO #276 — Kamigawa: Neon Dynasty | Scryfall ID: aa548dcd-c1dd-492d-a69f-c65dfeef0633 | Oracle ID: c5ee72d5-3a9e-4fe5-8802-3286ee612055

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

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
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
        activated!(
            cost!("{3}{R}", DiscardSelf),
            &[Effect::CreateTokenModified {
                token: &crate::tokens::SPIRIT_1_1_COLORLESS,
                amount: Amount::Fixed(2),
                tapped: false,
                haste: true,
            }],
            zone = ActivationZone::Hand,
            cost_reduction = Some(CostReduction::PerCount {
                amount: Amount::CountOf {
                    filter: &Filter::And(&[Filter::LEGENDARY_CREATURE, Filter::ControlledByYou]),
                    zone: ZoneSel::Battlefield
                },
                each: 1
            })
        ),
    ],
);

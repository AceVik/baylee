//! Otawara, Soaring City — (no cost) — Legendary Land
//! Oracle: {T}: Add {U}.
//! Oracle: Channel — {3}{U}, Discard this card: Return target artifact, creature, enchantment, or planeswalker to its owner's hand. This ability costs {1} less to activate for each legendary creature you control.
//! Set: NEO #271 — Kamigawa: Neon Dynasty | Scryfall ID: 486d7edc-d983-41f0-8b78-c99aecd72996 | Oracle ID: e9b6a394-691c-425a-9307-76d8edc7375e

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

static CHANNEL_TARGET: Filter = Filter::Or(&[
    Filter::ARTIFACT,
    Filter::CREATURE,
    Filter::ENCHANTMENT,
    Filter::PLANESWALKER,
]);

card!(
    index = index::OTAWARA_SOARING_CITY,
    oracle_id = "e9b6a394-691c-425a-9307-76d8edc7375e",
    scryfall_id = "486d7edc-d983-41f0-8b78-c99aecd72996",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Otawara, Soaring City",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
        activated!(
            cost!("{3}{U}", DiscardSelf),
            &[Effect::bounce(TargetSpec::Object(&CHANNEL_TARGET))],
            target = Some(TargetSpec::Object(&CHANNEL_TARGET)),
            zone = ActivationZone::Hand,
            cost_reduction = Some(CostReduction::PerCount {
                amount: Amount::CountOf {
                    filter: &Filter::And(&[Filter::LEGENDARY_CREATURE, Filter::ControlledByYou]),
                    zone: ZoneSel::Battlefield
                },
                each: 1
            }),
        ),
    ],
);

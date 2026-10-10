//! Eiganjo, Seat of the Empire — (no cost) — Legendary Land
//! Oracle: {T}: Add {W}.
//! Oracle: Channel — {2}{W}, Discard this card: It deals 4 damage to target attacking or blocking creature. This ability costs {1} less to activate for each legendary creature you control.
//! Set: NEO #268 — Kamigawa: Neon Dynasty | Scryfall ID: c375a022-5b57-496d-a802-e4ea8376e9e4 | Oracle ID: 7edb3d15-4f70-4ebe-8c5e-caf6a225076d

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EIGANJO_SEAT_OF_THE_EMPIRE,
    oracle_id = "7edb3d15-4f70-4ebe-8c5e-caf6a225076d",
    scryfall_id = "c375a022-5b57-496d-a802-e4ea8376e9e4",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Eiganjo, Seat of the Empire",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        activated!(
            cost!("{2}{W}", DiscardSelf),
            &[Effect::DealDamage {
                amount: Amount::Fixed(4),
                target: TargetSpec::Object(&Filter::And(&[
                    Filter::CREATURE,
                    Filter::Or(&[Filter::Attacking, Filter::Blocking]),
                ])),
            }],
            target = Some(TargetSpec::Object(&Filter::And(&[
                Filter::CREATURE,
                Filter::Or(&[Filter::Attacking, Filter::Blocking]),
            ]))),
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

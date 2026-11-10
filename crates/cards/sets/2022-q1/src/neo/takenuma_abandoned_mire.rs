//! Takenuma, Abandoned Mire — (no cost) — Legendary Land
//! Oracle: {T}: Add {B}.
//! Oracle: Channel — {3}{B}, Discard this card: Mill three cards, then return a creature or planeswalker card from your graveyard to your hand. This ability costs {1} less to activate for each legendary creature you control.
//! Set: NEO #278 — Kamigawa: Neon Dynasty | Scryfall ID: 499037cc-a577-41cb-8ca2-5e117945634f | Oracle ID: ac2dd694-d2f1-4025-8400-12332bdc882a

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TAKENUMA_ABANDONED_MIRE,
    oracle_id = "ac2dd694-d2f1-4025-8400-12332bdc882a",
    scryfall_id = "499037cc-a577-41cb-8ca2-5e117945634f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Takenuma, Abandoned Mire",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        activated!(
            cost!("{3}{B}", DiscardSelf),
            &[
                Effect::Mill {
                    amount: Amount::Fixed(3),
                    target: PlayerRel::You
                },
                Effect::ChooseFromYourGraveyard {
                    filter: &Filter::HasType(TypeSet::CREATURE.union(TypeSet::PLANESWALKER)),
                    destination: SearchDest::Hand,
                    tapped: false,
                    optional: false
                }
            ],
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

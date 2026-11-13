//! Tomb of Urami — (no cost) — Legendary Land
//! Oracle: {T}: Add {B}. Tomb of Urami deals 1 damage to you if you don't control an Ogre.
//! Oracle: {2}{B}{B}, {T}, Sacrifice all lands you control: Create Urami, a legendary 5/5 black Demon Spirit creature token with flying.
//! Set: SOK #165 — Saviors of Kamigawa | Scryfall ID: 90fedf90-825c-4814-8f0d-170f537db44c | Oracle ID: f002be6a-e459-49c4-b765-062e30107439
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TOMB_OF_URAMI,
    oracle_id = "f002be6a-e459-49c4-b765-062e30107439",
    scryfall_id = "90fedf90-825c-4814-8f0d-170f537db44c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Tomb of Urami",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[
            Effect::mana(ManaColor::Black, 1),
            Effect::IfCondition {
                condition: Condition::ControlCountAtMost(
                    &Filter::HasSubtype(subtypes::creature::OGRE),
                    0,
                ),
                then: &[Effect::DealDamage {
                    amount: Amount::Fixed(1),
                    target: TargetSpec::Player(PlayerRel::You),
                }],
                otherwise: &[],
            },
        ]),
        activated!(
            cost!("{2}{B}{B}", TapSelf, SacrificeAll(&Filter::YOUR_LAND)),
            &[Effect::CreateToken {
                token: &crate::generated_tokens::URAMI_LEGENDARY_5_5_BLACK_FLYING
            }],
        )
    ],
);

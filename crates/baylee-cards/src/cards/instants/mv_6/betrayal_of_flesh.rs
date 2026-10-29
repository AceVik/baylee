//! Betrayal of Flesh — {5}{B} — Instant
//! Oracle: Choose one —
//! Oracle: • Destroy target creature.
//! Oracle: • Return target creature card from your graveyard to the battlefield.
//! Oracle: Entwine—Sacrifice three lands. (Choose both if you pay the entwine cost.)
//! Set: MRD #58 — Mirrodin | Scryfall ID: a9e2e107-0277-4e5c-81a7-258bb2998f3e | Oracle ID: 43cee9b6-9853-4337-869a-45540c9db10f
use baylee_cards_dsl::prelude::*;

card!(
    index = index::BETRAYAL_OF_FLESH,
    oracle_id = "43cee9b6-9853-4337-869a-45540c9db10f",
    scryfall_id = "a9e2e107-0277-4e5c-81a7-258bb2998f3e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Betrayal of Flesh",
        mana_cost = mana!("{5}{B}"),
        types = TypeSet::INSTANT,
        entwine = Some(Cost {
            mana: ManaCost::ZERO,
            parts: &[
                CostPart::Sacrifice(&Filter::YOUR_LAND),
                CostPart::Sacrifice(&Filter::YOUR_LAND),
                CostPart::Sacrifice(&Filter::YOUR_LAND),
            ],
        }),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[AbilityDef::ModalSpell {
        choose: ModeCount::ONE,
        modes: &[
            mode!(
                &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))],
                targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
            ),
            mode!(
                &[Effect::reanimate(TargetSpec::CardInGraveyard(
                    &Filter::CREATURE,
                    PlayerRel::You,
                ))],
                targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
                    &Filter::CREATURE,
                    PlayerRel::You,
                ))),
            ),
        ],
    }],
);

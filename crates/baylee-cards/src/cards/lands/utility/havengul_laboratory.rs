//! Havengul Laboratory // Havengul Mystery — (no cost) — Legendary Land // Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}: Investigate. (Create a colorless Clue artifact token with "{2}, Sacrifice this artifact: Draw a card.")
//! Oracle: At the beginning of your end step, if you sacrificed three or more Clues this turn, transform Havengul Laboratory.
//! Oracle: When this land transforms into Havengul Mystery, return target creature card from your graveyard to the battlefield.
//! Oracle: When the creature put onto the battlefield with Havengul Mystery leaves the battlefield, transform Havengul Mystery.
//! Oracle: {T}, Pay 1 life: Add {B}.
//! Set: SLX #9 — Universes Within | Scryfall ID: 823b019e-10c0-4712-8167-d4f37a71e782 | Oracle ID: e71ac446-02a4-4468-8d29-f28b21617665
//! Face: Havengul Laboratory —  — Legendary Land
//! Face: Havengul Mystery —  — Legendary Land

use crate::tokens::CLUE;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::artifact;
static BACK_ABILITIES: &[AbilityDef] = &[
    triggered!(
        Trigger::TransformsIntoThis,
        &[Effect::GraveyardToBattlefieldLinked {
            target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You)
        }],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You
        )))
    ),
    triggered!(Trigger::LinkedCreatureLeaves, &[Effect::TransformSource]),
    mana_ability!(
        cost!(TapSelf, PayLife(1)),
        &[Effect::mana(ManaColor::Black, 1)]
    ),
];

card!(
    index = index::HAVENGUL_LABORATORY,
    oracle_id = "e71ac446-02a4-4468-8d29-f28b21617665",
    scryfall_id = "823b019e-10c0-4712-8167-d4f37a71e782",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[
        face!(
            name = "Havengul Laboratory",
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY
        ),
        face!(
            name = "Havengul Mystery",
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            castable_from_hand = false,
            abilities = BACK_ABILITIES
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{4}", TapSelf),
            &[Effect::CreateToken { token: &CLUE }]
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::You
            },
            &[Effect::TransformSource],
            condition = Some(Condition::SacrificedSubtypeThisTurn {
                subtype: artifact::CLUE,
                count: 3
            })
        ),
    ],
);

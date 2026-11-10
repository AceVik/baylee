//! Sidequest: Catch a Fish // Cooking Campsite — {2}{W} — Enchantment // Land
//! Oracle: At the beginning of your upkeep, look at the top card of your library. If it's an artifact or creature card, you may reveal it and put it into your hand. If you put a card into your hand this way, create a Food token and transform this enchantment.
//! Oracle: {T}: Add {W}.
//! Oracle: {3}, {T}, Sacrifice an artifact: Put a +1/+1 counter on each creature you control. Activate only as a sorcery.
//! Set: FIN #31 — Final Fantasy | Scryfall ID: bdb5452e-d97f-409b-91d0-2664f39b09b8 | Oracle ID: bd7c328e-0380-46f8-bb85-7bf4e201b7ac
//! Face: Sidequest: Catch a Fish — {2}{W} — Enchantment
//! Face: Cooking Campsite —  — Land
use baylee_cards_dsl::prelude::*;

/// Cooking Campsite's abilities, which are the back face's: a mana ability,
/// and a sorcery-speed artifact sacrifice that grows the team.
static BACK_ABILITIES: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
    activated!(
        cost!("{3}", TapSelf, Sacrifice(&Filter::ARTIFACT)),
        &[Effect::AddCounterFilter {
            filter: &Filter::YOUR_CREATURE,
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }],
        timing = ActivationTiming::SorcerySpeed,
    ),
];

card!(
    index = index::SIDEQUEST_CATCH_A_FISH,
    oracle_id = "bd7c328e-0380-46f8-bb85-7bf4e201b7ac",
    scryfall_id = "bdb5452e-d97f-409b-91d0-2664f39b09b8",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[
        face!(
            name = "Sidequest: Catch a Fish",
            mana_cost = mana!("{2}{W}"),
            types = TypeSet::ENCHANTMENT,
        ),
        face!(
            name = "Cooking Campsite",
            types = TypeSet::LAND,
            castable_from_hand = false,
            abilities = BACK_ABILITIES,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[Effect::LookAtTopMayTake {
            filter: &Filter::Or(&[Filter::ARTIFACT, Filter::CREATURE]),
            rest: LookRest::Keep,
            then: &[
                Effect::CreateToken {
                    token: &crate::tokens::FOOD
                },
                Effect::TransformSource,
            ],
        }]
    )],
);

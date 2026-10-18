//! Hostile Hostel // Creeping Inn — (no cost) — Land // Artifact Creature — Horror Construct
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}, Sacrifice a creature: Put a soul counter on this land. Then if there are three or more soul counters on it, remove those counters, transform it, then untap it. Activate only as a sorcery.
//! Oracle: Whenever this creature attacks, you may exile a creature card from your graveyard. If you do, each opponent loses X life and you gain X life, where X is the number of creature cards exiled with this creature.
//! Oracle: {4}: This creature phases out.
//! Set: MID #264 — Innistrad: Midnight Hunt | Scryfall ID: ac83c27f-55d6-4e5a-93a4-febb0c183289 | Oracle ID: 1b340f71-502f-48e9-85ed-9af62f356115
//! Face: Hostile Hostel —  — Land
//! Face: Creeping Inn —  — Artifact Creature — Horror Construct
// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BACK_ABILITIES: &[AbilityDef] = &[
    triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::MayExileFromYourGraveyardThen {
            count: 1,
            filter: &Filter::CREATURE,
            then: &[
                Effect::LoseLife {
                    target: PlayerRel::EachOpponent,
                    amount: Amount::CountOf {
                        filter: &Filter::CREATURE,
                        zone: ZoneSel::LinkedExile,
                    },
                },
                Effect::GainLife {
                    amount: Amount::CountOf {
                        filter: &Filter::CREATURE,
                        zone: ZoneSel::LinkedExile,
                    },
                },
            ],
        }],
    ),
    activated!(cost!("{4}"), &[Effect::PhaseOut { target: None }]),
];

card!(
    index = index::HOSTILE_HOSTEL,
    oracle_id = "1b340f71-502f-48e9-85ed-9af62f356115",
    scryfall_id = "ac83c27f-55d6-4e5a-93a4-febb0c183289",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[
        face!(name = "Hostile Hostel", types = TypeSet::LAND,),
        face!(
            name = "Creeping Inn",
            types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
            subtypes = &[subtypes::creature::HORROR, subtypes::creature::CONSTRUCT],
            power = Some(3),
            toughness = Some(7),
            color_indicator = ColorSet::from_slice(&[Color::Black]),
            castable_from_hand = false,
            abilities = BACK_ABILITIES,
        ),
    ],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{1}", TapSelf, Sacrifice(&Filter::CREATURE)),
            &[
                Effect::AddCounter {
                    kind: counters::SOUL,
                    amount: Amount::Fixed(1),
                },
                Effect::IfCondition {
                    condition: Condition::CountersOnSelf(counters::SOUL, 3),
                    then: &[
                        Effect::RemoveAllCountersSelf {
                            kind: counters::SOUL,
                        },
                        Effect::TransformSource,
                        Effect::UntapSelf,
                    ],
                    otherwise: &[],
                },
            ],
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);

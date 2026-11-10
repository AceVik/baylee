//! Professor Dellian Fel — {2}{B}{G} — Legendary Planeswalker — Dellian
//! Oracle: +2: You gain 3 life.
//! Oracle: 0: You draw a card and lose 1 life.
//! Oracle: −3: Destroy target creature.
//! Oracle: −6: You get an emblem with "Whenever you gain life, target opponent loses that much life."
//! Set: SOS #214 — Secrets of Strixhaven | Scryfall ID: 6ff3b4d8-1271-4c5d-8834-7662244f173d | Oracle ID: 8c5fe985-69e1-4a03-99fc-a6fd2314a9a7
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static EMBLEM_ABILITIES: &[AbilityDef] = &[triggered!(
    Trigger::GainsLife(PlayerRel::You),
    &[Effect::LoseLife {
        amount: Amount::EventAmount,
        target: PlayerRel::TargetPlayerOrController,
    }],
    targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
)];

card!(
    index = index::PROFESSOR_DELLIAN_FEL,
    oracle_id = "8c5fe985-69e1-4a03-99fc-a6fd2314a9a7",
    scryfall_id = "6ff3b4d8-1271-4c5d-8834-7662244f173d",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Professor Dellian Fel",
        mana_cost = mana!("{2}{B}{G}"),
        types = TypeSet::PLANESWALKER,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::planeswalker::DELLIAN],
        loyalty = Some(5),
    ),],
    abilities = &[
        loyalty!(2, &[Effect::gain_life(3)]),
        loyalty!(
            0,
            &[
                Effect::draw(1),
                Effect::LoseLife {
                    amount: Amount::Fixed(1),
                    target: PlayerRel::You,
                },
            ],
        ),
        loyalty!(
            -3,
            &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        loyalty!(
            -6,
            &[Effect::CreateEmblem {
                abilities: EMBLEM_ABILITIES,
            }],
        ),
    ],
);

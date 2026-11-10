//! Welcome to . . . // Jurassic Park — {1}{G}{G} — Enchantment — Saga // Legendary Land
//! Oracle: (As this Saga enters and after your draw step, add a lore counter.)
//! Oracle: I — For each opponent, up to one target noncreature artifact they control becomes a 0/4 Wall artifact creature with defender for as long as you control this Saga.
//! Oracle: II — Create a 3/3 green Dinosaur creature token with trample. It gains haste until end of turn.
//! Oracle: III — Destroy all Walls. Exile this Saga, then return it to the battlefield transformed under your control.
//! Oracle: (Transforms from Welcome to ....)
//! Oracle: Each Dinosaur card in your graveyard has escape. The escape cost is equal to the card's mana cost plus exile three other cards from your graveyard. (You may cast cards from your graveyard for their escape cost.)
//! Oracle: {T}: Add {G} for each Dinosaur you control.
//! Set: REX #7 — Jurassic World Collection | Scryfall ID: 6d84e2d4-38bf-4d46-99a6-37c2dda66b16 | Oracle ID: a4b37d16-95b3-4143-a0b2-ad9f2aba91f8
//! Face: Welcome to . . . — {1}{G}{G} — Enchantment — Saga
//! Face: Jurassic Park —  — Legendary Land

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;
use baylee_core::generated::subtypes::creature;

/// "Each Dinosaur you control" — the count Jurassic Park's mana ability
/// reads. A Dinosaur an opponent controls makes no mana.
static YOUR_DINOSAURS: Filter = Filter::And(&[
    Filter::HasSubtype(creature::DINOSAUR),
    Filter::ControlledByYou,
]);

static NONCREATURE_ARTIFACT: Filter = Filter::And(&[Filter::ARTIFACT, Filter::NONCREATURE]);

static JURASSIC_PARK_ABILITIES: &[AbilityDef] = &[
    static_ability!(
        Filter::And(&[
            Filter::HasSubtype(creature::DINOSAUR),
            Filter::InZone(ZoneRef::Graveyard),
            Filter::OwnedByYou,
        ]),
        Modifier::GrantsEscape(3)
    ),
    mana_ability!(&[Effect::mana_dynamic(
        ManaColor::Green,
        Amount::CountOf {
            filter: &YOUR_DINOSAURS,
            zone: ZoneSel::Battlefield
        },
    )]),
];

card!(
    index = index::WELCOME_TO,
    oracle_id = "a4b37d16-95b3-4143-a0b2-ad9f2aba91f8",
    scryfall_id = "6d84e2d4-38bf-4d46-99a6-37c2dda66b16",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[
        face!(
            name = "Welcome to . . .",
            mana_cost = mana!("{1}{G}{G}"),
            types = TypeSet::ENCHANTMENT,
            subtypes = &[subtypes::enchantment::SAGA],
        ),
        face!(
            name = "Jurassic Park",
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            // The back of a transforming card is reached by turning the card
            // over and never by casting it (CR 712.2). The stub reader leaves
            // this line off every *land* back, because an MDFC's land back is
            // the one a player does play — so this is the case it cannot tell
            // apart and a person has to.
            castable_from_hand = false,
            abilities = JURASSIC_PARK_ABILITIES,
        ),
    ],
    abilities = &[
        chapter!(
            1,
            &[
                Effect::ModifyTargets {
                    modifier: Modifier::AddType(TypeSet::CREATURE),
                    duration: Duration::WhileYouControlSource
                },
                Effect::ModifyTargets {
                    modifier: Modifier::AddSubtype(creature::WALL),
                    duration: Duration::WhileYouControlSource
                },
                Effect::ModifyTargets {
                    modifier: Modifier::SetPT(0, 4),
                    duration: Duration::WhileYouControlSource
                },
                Effect::ModifyTargets {
                    modifier: Modifier::AddKeyword(KeywordSet::DEFENDER),
                    duration: Duration::WhileYouControlSource
                },
            ],
            targets = Some(TargetReq::up_to(
                TargetSpec::ObjectOfEachOpponent(&NONCREATURE_ARTIFACT),
                u8::MAX
            )),
        ),
        chapter!(
            2,
            &[Effect::CreateTokenModified {
                token: &crate::generated_tokens::DINOSAUR_3_3_GREEN_TRAMPLE,
                amount: Amount::Fixed(1),
                tapped: false,
                haste: true,
            }]
        ),
        chapter!(
            3,
            &[
                Effect::destroy_all(&Filter::HasSubtype(creature::WALL)),
                // "…transformed under your control."
                Effect::ExileSelfReturnAsFace {
                    face: 1,
                    owner_control: false,
                },
            ]
        ),
    ],
);

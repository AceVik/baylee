//! Tamiyo, Inquisitive Student // Tamiyo, Seasoned Scholar — {U} — Legendary Creature — Moonfolk Wizard // Legendary Planeswalker — Tamiyo
//! Oracle: Flying
//! Oracle: Whenever Tamiyo attacks, investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
//! Oracle: When you draw your third card in a turn, exile Tamiyo, then return her to the battlefield transformed under her owner's control.
//! Oracle: +2: Until your next turn, whenever a creature attacks you or a planeswalker you control, it gets -1/-0 until end of turn.
//! Oracle: −3: Return target instant or sorcery card from your graveyard to your hand. If it's a green card, add one mana of any color.
//! Oracle: −7: Draw cards equal to half the number of cards in your library, rounded up. You get an emblem with "You have no maximum hand size."
//! Set: MH3 #242 — Modern Horizons 3 | Scryfall ID: 2a717b98-cdac-416d-bf6c-f6b6638e65d1 | Oracle ID: 1ed9e351-0000-4b75-8ecb-d15bd88df68a
//! Face: Tamiyo, Inquisitive Student — {U} — Legendary Creature — Moonfolk Wizard
//! Face: Tamiyo, Seasoned Scholar —  — Legendary Planeswalker — Tamiyo
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static INSTANT_OR_SORCERY: Filter = Filter::HasType(TypeSet::INSTANT.union(TypeSet::SORCERY));
static GREEN_CARD: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Green]));

static BACK_ABILITIES: &[AbilityDef] = &[
    loyalty!(
        2,
        &[
            Effect::WheneverCreatureAttacksYouOrYourPlaneswalkerUntilYourNextTurn {
                effects: &[Effect::ModifyEvent {
                    modifier: Modifier::ModifyPT(-1, 0),
                    duration: Duration::UntilEndOfTurn,
                }],
            },
        ],
    ),
    loyalty!(
        -3,
        &[
            Effect::IfTargetMatches {
                filter: &GREEN_CARD,
                then: &[Effect::mana_of_any_color()],
            },
            Effect::GraveyardToHand {
                target: TargetSpec::CardInGraveyard(&INSTANT_OR_SORCERY, PlayerRel::You),
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &INSTANT_OR_SORCERY,
            PlayerRel::You,
        ))),
    ),
    loyalty!(
        -7,
        &[
            Effect::DrawCards {
                amount: Amount::Half {
                    base: &Amount::LibrarySize(PlayerRel::You),
                    round_up: true,
                },
            },
            Effect::CreateEmblem {
                abilities: &[static_ability!(Filter::Any, Modifier::NoMaxHandSize)],
            },
        ],
    ),
];

card!(
    index = index::TAMIYO_INQUISITIVE_STUDENT,
    oracle_id = "1ed9e351-0000-4b75-8ecb-d15bd88df68a",
    scryfall_id = "2a717b98-cdac-416d-bf6c-f6b6638e65d1",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[
        face!(
            name = "Tamiyo, Inquisitive Student",
            mana_cost = mana!("{U}"),
            types = TypeSet::CREATURE,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::creature::MOONFOLK, subtypes::creature::WIZARD],
            power = Some(0),
            toughness = Some(3),
        ),
        face!(
            name = "Tamiyo, Seasoned Scholar",
            types = TypeSet::PLANESWALKER,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::planeswalker::TAMIYO],
            loyalty = Some(2),
            color_indicator = ColorSet::from_slice(&[Color::Green, Color::Blue]),
            castable_from_hand = false,
            abilities = BACK_ABILITIES,
        ),
    ],
    abilities = &[
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[Effect::CreateToken {
                token: &crate::tokens::CLUE,
            }],
        ),
        triggered!(
            Trigger::NthCardDrawnInTurn {
                who: PlayerRel::You,
                n: 3,
            },
            &[Effect::ExileSelfReturnAsFace {
                face: 1,
                owner_control: true,
            }],
        ),
    ],
);

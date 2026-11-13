//! Grist, Voracious Larva // Grist, the Plague Swarm — {G} — Legendary Creature — Insect // Legendary Planeswalker — Grist
//! Oracle: Deathtouch
//! Oracle: Whenever Grist or another creature you control enters, if it entered from your graveyard or you cast it from your graveyard, you may pay {G}. If you do, exile Grist, then return it to the battlefield transformed under its owner's control.
//! Oracle: +1: Create a 1/1 black and green Insect creature token, then mill two cards. Put a deathtouch counter on the token if a black card was milled this way.
//! Oracle: −2: Destroy target artifact or enchantment.
//! Oracle: −6: For each creature card in your graveyard, create a token that's a copy of it, except it's a 1/1 black and green Insect.
//! Set: MH3 #251 — Modern Horizons 3 | Scryfall ID: 68239b41-b7db-4044-b672-6808c2c342ec | Oracle ID: 196f8f13-d4df-4740-8746-8fed3bc71840
//! Face: Grist, Voracious Larva — {G} — Legendary Creature — Insect
//! Face: Grist, the Plague Swarm —  — Legendary Planeswalker — Grist
// IMPLEMENTED — deathtouch, the graveyard-entry/cast transformation, and all
// three loyalty abilities, including exact token identity and copy exceptions.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLACK_CARD: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Black]));
static BLACK_MILLED: Condition =
    Condition::AmountAtLeast(Amount::ThisWay(ThisWay::Milled(&BLACK_CARD)), 1);

static BACK_ABILITIES: &[AbilityDef] = &[
    loyalty!(
        1,
        &[
            Effect::CreateToken {
                token: &crate::generated_tokens::INSECT_1_1_BLACK_GREEN,
            },
            Effect::Mill {
                amount: Amount::Fixed(2),
                target: PlayerRel::You,
            },
            Effect::IfCondition {
                condition: BLACK_MILLED,
                then: &[Effect::AddCounterCreatedTokens {
                    kind: CounterKind::Deathtouch,
                    amount: Amount::Fixed(1),
                }],
                otherwise: &[],
            },
        ]
    ),
    loyalty!(
        -2,
        &[Effect::destroy(TargetSpec::Object(
            &Filter::ARTIFACT_OR_ENCHANTMENT,
        ))],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &Filter::ARTIFACT_OR_ENCHANTMENT,
        ))),
    ),
    loyalty!(
        -6,
        &[Effect::CreateTokenCopiesOfGraveyard {
            player: PlayerRel::You,
            filter: &Filter::CREATURE,
            mods: &[
                CopyMod::SetPT(1, 1),
                CopyMod::SetColor(ColorSet::from_slice(&[Color::Black, Color::Green])),
                CopyMod::SetCreatureTypes(&[subtypes::creature::INSECT]),
            ],
        }]
    ),
];

card!(
    index = index::GRIST_VORACIOUS_LARVA,
    oracle_id = "196f8f13-d4df-4740-8746-8fed3bc71840",
    scryfall_id = "68239b41-b7db-4044-b672-6808c2c342ec",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::DEATHTOUCH,
    coverage = Coverage::Implemented,
    faces = &[
        face!(
            name = "Grist, Voracious Larva",
            mana_cost = mana!("{G}"),
            types = TypeSet::CREATURE,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::creature::INSECT],
            power = Some(1),
            toughness = Some(2),
        ),
        face!(
            name = "Grist, the Plague Swarm",
            types = TypeSet::PLANESWALKER,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::planeswalker::GRIST],
            loyalty = Some(3),
            color_indicator = ColorSet::from_slice(&[Color::Black, Color::Green]),
            castable_from_hand = false,
            abilities = BACK_ABILITIES,
        ),
    ],
    abilities = &[triggered!(
        Trigger::EntersBattlefieldFromGraveyardOrCast(&Filter::YOUR_CREATURE),
        &[Effect::PlayerMayPayManaThen {
            player: PlayerRel::You,
            cost: mana!("{G}"),
            effects: &[Effect::ExileSelfReturnAsFace {
                face: 1,
                owner_control: true,
            }],
        }],
    ),],
);

//! Aphelia, Viper Whisperer — {1}{B} — Legendary Creature — Gorgon Assassin
//! Oracle: Deathtouch
//! Oracle: Whenever Aphelia attacks, you may pay {1}{B/G}. If you do, create a 1/1 black Snake creature token with deathtouch.
//! Oracle: {4}{B}: Until end of turn, whenever one or more Gorgons and/or Snakes you control deal combat damage to a player, that player loses half their life, rounded up.
//! Set: J25 #40 — Foundations Jumpstart | Scryfall ID: 1a0867be-a861-4fb4-b8ff-cb2966193755 | Oracle ID: 191e003f-1a78-4d85-9c70-2ae132a5f9cc
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static YOUR_GORGON_OR_SNAKE: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::Or(&[
        Filter::HasSubtype(subtypes::creature::GORGON),
        Filter::HasSubtype(subtypes::creature::SNAKE),
    ]),
]);

card!(
    index = index::APHELIA_VIPER_WHISPERER,
    oracle_id = "191e003f-1a78-4d85-9c70-2ae132a5f9cc",
    scryfall_id = "1a0867be-a861-4fb4-b8ff-cb2966193755",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::DEATHTOUCH,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Aphelia, Viper Whisperer",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::GORGON, subtypes::creature::ASSASSIN],
        power = Some(1),
        toughness = Some(3),
    ),],
    abilities = &[
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[Effect::PlayerMayPayManaThen {
                player: PlayerRel::You,
                cost: mana!("{1}{B/G}"),
                effects: &[Effect::CreateToken {
                    token: &crate::generated_tokens::SNAKE_1_1_BLACK_DEATHTOUCH,
                }],
            }],
        ),
        activated!(
            cost!("{4}{B}"),
            &[Effect::WheneverOneOrMoreDealCombatDamageToPlayerThisTurn {
                filter: &YOUR_GORGON_OR_SNAKE,
                effects: &[Effect::LoseHalfLife {
                    player: PlayerRel::DamagedPlayer,
                }],
            }],
        ),
    ],
);

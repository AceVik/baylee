//! The Serpent Society — {1}{B}{G} — Legendary Creature — Human Snake Villain
//! Oracle: Deathtouch
//! Oracle: Ward—Get five poison counters. (A player with ten or more poison counters loses the game.)
//! Oracle: Whenever another creature you control with deathtouch dies, each opponent sacrifices a nontoken creature of their choice.
//! Set: MSH #226 — Marvel Super Heroes | Scryfall ID: 209855ee-d531-4b58-926b-8da171d46619 | Oracle ID: 3688ad0d-653b-4d5d-a57e-444db679d83e
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ANOTHER_YOUR_DEATHTOUCH_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Another,
    Filter::ControlledByYou,
    Filter::HasKeyword(KeywordSet::DEATHTOUCH),
]);

static NONTOKEN_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::IsToken)]);

card!(
    index = index::THE_SERPENT_SOCIETY,
    oracle_id = "3688ad0d-653b-4d5d-a57e-444db679d83e",
    scryfall_id = "209855ee-d531-4b58-926b-8da171d46619",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::DEATHTOUCH,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "The Serpent Society",
        mana_cost = mana!("{1}{B}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::SNAKE,
            subtypes::creature::VILLAIN
        ],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[
        triggered!(
            Trigger::Ward,
            &[Effect::PlayerMayGetPoisonOr {
                player: PlayerRel::ControllerOfTarget,
                poison: Amount::Fixed(5),
                effect: &Effect::CounterTargetSpellOrAbility,
            }],
        ),
        triggered!(
            Trigger::Dies(&ANOTHER_YOUR_DEATHTOUCH_CREATURE),
            &[Effect::SacrificeFilter {
                who: PlayerRel::EachOpponent,
                filter: &NONTOKEN_CREATURE,
            }],
        ),
    ],
);

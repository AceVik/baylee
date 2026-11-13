//! Tony Stark // The Invincible Iron Man — {1}{U} — Legendary Creature — Human Artificer Hero // Legendary Artifact Creature — Human Hero
//! Oracle: {1}, {T}: Look at the top four cards of your library. You may reveal an artifact card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.
//! Oracle: {4}{U}{R}: Transform Tony Stark. Activate only as a sorcery.
//! Oracle: Flying, haste
//! Oracle: At the beginning of combat on your turn, you may put an artifact card from your hand onto the battlefield. If it's an Equipment, attach it to The Invincible Iron Man.
//! Set: MSH #80 — Marvel Super Heroes | Scryfall ID: 4cea42fd-035e-4b8f-8b1d-ff363b694f14 | Oracle ID: 4a46ea59-6c5d-4bed-9951-b5590f9a1bdb
//! Face: Tony Stark — {1}{U} — Legendary Creature — Human Artificer Hero
//! Face: The Invincible Iron Man — {4}{U}{R} — Legendary Artifact Creature — Human Hero
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BACK_ABILITIES: &[AbilityDef] = &[triggered!(
    Trigger::StepBegin {
        step: StepKind::CombatBegin,
        whose: PlayerRel::You,
    },
    &[Effect::MayPutFromHandOntoBattlefieldAttachIfEquipment {
        filter: &Filter::ARTIFACT,
    }],
)];

card!(
    index = index::TONY_STARK,
    oracle_id = "4a46ea59-6c5d-4bed-9951-b5590f9a1bdb",
    scryfall_id = "4cea42fd-035e-4b8f-8b1d-ff363b694f14",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[
        face!(
            name = "Tony Stark",
            mana_cost = mana!("{1}{U}"),
            types = TypeSet::CREATURE,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[
                subtypes::creature::HUMAN,
                subtypes::creature::ARTIFICER,
                subtypes::creature::HERO
            ],
            power = Some(1),
            toughness = Some(3),
        ),
        face!(
            name = "The Invincible Iron Man",
            mana_cost = mana!("{4}{U}{R}"),
            types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::creature::HUMAN, subtypes::creature::HERO],
            power = Some(5),
            toughness = Some(5),
            keywords = KeywordSet::FLYING.union(KeywordSet::HASTE),
            abilities = BACK_ABILITIES,
        ),
    ],
    abilities = &[
        activated!(
            cost!("{1}", TapSelf),
            &[Effect::LookAtTopFind {
                found: Find::HAND,
                count: 4,
                filter: &Filter::ARTIFACT,
                random: true,
            }],
        ),
        activated!(
            cost!("{4}{U}{R}"),
            &[Effect::TransformSource],
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);

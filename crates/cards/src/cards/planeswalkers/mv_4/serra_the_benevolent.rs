//! Serra the Benevolent — {2}{W}{W} — Legendary Planeswalker — Serra
//! Oracle: +2: Creatures you control with flying get +1/+1 until end of turn.
//! Oracle: −3: Create a 4/4 white Angel creature token with flying and vigilance.
//! Oracle: −6: You get an emblem with "If you control a creature, damage that would reduce your life total to less than 1 reduces it to 1 instead."
//! Set: MH1 #26 — Modern Horizons | Scryfall ID: 88bf4af9-4b14-43e7-9d50-0e6a895cece1 | Oracle ID: 8b0e2382-dd91-42e6-aba3-5e8a15418a49
// IMPLEMENTED — both loyalty effects and the conditional life-floor emblem.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static YOUR_FLYING_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::ControlledByYou,
    Filter::HasKeyword(KeywordSet::FLYING),
]);
static EMBLEM_ABILITIES: &[AbilityDef] = &[static_ability!(
    Filter::Any,
    Modifier::DamageLifeLossFloor {
        players: PlayerRel::You,
        minimum: 1,
    },
    condition = Some(Condition::ControlCount(&Filter::YOUR_CREATURE, 1))
)];

card!(
    index = index::SERRA_THE_BENEVOLENT,
    oracle_id = "8b0e2382-dd91-42e6-aba3-5e8a15418a49",
    scryfall_id = "88bf4af9-4b14-43e7-9d50-0e6a895cece1",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Serra the Benevolent",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::PLANESWALKER,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::planeswalker::SERRA],
        loyalty = Some(4),
    ),],
    abilities = &[
        loyalty!(
            2,
            &[Effect::PumpFilter {
                filter: &YOUR_FLYING_CREATURE,
                controlled_by: Some(PlayerRel::You),
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(1),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }]
        ),
        loyalty!(
            -3,
            &[Effect::CreateToken {
                token: &generated_tokens::ANGEL_4_4_WHITE_FLYING_VIGILANCE,
            }]
        ),
        loyalty!(
            -6,
            &[Effect::CreateEmblem {
                abilities: EMBLEM_ABILITIES,
            }]
        ),
    ],
);

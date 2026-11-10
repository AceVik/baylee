//! Virtue of Persistence // Locthwain Scorn — {5}{B}{B} — Enchantment // Sorcery — Adventure
//! Oracle: At the beginning of your upkeep, put target creature card from a graveyard onto the battlefield under your control.
//! Oracle: Target creature gets -3/-3 until end of turn. You gain 2 life.
//! Set: WOE #115 — Wilds of Eldraine | Scryfall ID: f1e5cafb-b0e6-4ee5-8c58-6f8e5ef2b9da | Oracle ID: fae36da0-bf9c-484f-b2fe-dd8ab2effc5c
//! Face: Virtue of Persistence — {5}{B}{B} — Enchantment
//! Face: Locthwain Scorn — {1}{B} — Sorcery — Adventure
// IMPLEMENTED — upkeep graveyard reanimation from any graveyard + Locthwain Scorn Adventure.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static GRAVEYARD_CREATURE: TargetSpec =
    TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::EachPlayer);

static ADVENTURE_ABILITIES: &[AbilityDef] = &[spell!(
    &[
        Effect::PumpTarget {
            power: Amount::NegXFixed(3),
            toughness: Amount::NegXFixed(3),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        },
        Effect::gain_life(2),
    ],
    targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
)];

card!(
    index = index::VIRTUE_OF_PERSISTENCE,
    oracle_id = "fae36da0-bf9c-484f-b2fe-dd8ab2effc5c",
    scryfall_id = "f1e5cafb-b0e6-4ee5-8c58-6f8e5ef2b9da",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[
        face!(
            name = "Virtue of Persistence",
            mana_cost = mana!("{5}{B}{B}"),
            types = TypeSet::ENCHANTMENT,
        ),
        face!(
            name = "Locthwain Scorn",
            mana_cost = mana!("{1}{B}"),
            types = TypeSet::SORCERY,
            subtypes = &[subtypes::spell::ADVENTURE],
            adventure = true,
            abilities = ADVENTURE_ABILITIES,
        ),
    ],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::GraveyardToBattlefield {
            target: GRAVEYARD_CREATURE,
            owner_control: false,
            counters: None,
        }],
        targets = Some(TargetReq::one(GRAVEYARD_CREATURE)),
    )],
);

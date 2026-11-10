//! Vertigo — {R} — Instant
//! Oracle: Vertigo deals 2 damage to target creature with flying. That creature loses flying until end of turn.
//! Set: 6ED #212 — Classic Sixth Edition | Scryfall ID: 0a58ac6a-095e-47d7-ba42-56d148e3c6b9 | Oracle ID: 1dabfade-79e5-44e2-afa6-b20498c6bb8b

use baylee_cards_dsl::prelude::*;

static CREATURE_WITH_FLYING: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::FLYING)]);

card!(
    index = index::VERTIGO,
    oracle_id = "1dabfade-79e5-44e2-afa6-b20498c6bb8b",
    scryfall_id = "0a58ac6a-095e-47d7-ba42-56d148e3c6b9",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Vertigo",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::DealDamage {
                amount: Amount::Fixed(2),
                target: TargetSpec::Object(&CREATURE_WITH_FLYING),
            },
            Effect::ModifyTargets {
                modifier: Modifier::RemoveKeyword(KeywordSet::FLYING),
                duration: Duration::UntilEndOfTurn,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&CREATURE_WITH_FLYING)))
    )],
);

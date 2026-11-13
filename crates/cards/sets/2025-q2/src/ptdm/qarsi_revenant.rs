//! Qarsi Revenant — {1}{B}{B} — Creature — Vampire
//! Oracle: Flying, deathtouch, lifelink
//! Oracle: Renew — {2}{B}, Exile this card from your graveyard: Put a flying counter, a deathtouch counter, and a lifelink counter on target creature. Activate only as a sorcery.
//! Set: TDM #86 — Tarkir: Dragonstorm | Scryfall ID: 8c93a0f6-5e50-4dda-9ff6-da741fb839ff | Oracle ID: bbea79bc-71d4-4100-92f4-783c2c4b46da
// IMPLEMENTED — combat keywords and sorcery-speed Renew with three keyword counters.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::QARSI_REVENANT,
    oracle_id = "bbea79bc-71d4-4100-92f4-783c2c4b46da",
    scryfall_id = "8c93a0f6-5e50-4dda-9ff6-da741fb839ff",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Qarsi Revenant",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::VAMPIRE],
        power = Some(3),
        toughness = Some(3),
    ),],
    keywords = KeywordSet::FLYING
        .union(KeywordSet::DEATHTOUCH)
        .union(KeywordSet::LIFELINK),
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{2}{B}", ExileSelf),
        &[
            Effect::AddCounter {
                kind: CounterKind::Flying,
                amount: Amount::Fixed(1)
            },
            Effect::AddCounter {
                kind: CounterKind::Deathtouch,
                amount: Amount::Fixed(1)
            },
            Effect::AddCounter {
                kind: CounterKind::Lifelink,
                amount: Amount::Fixed(1)
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        zone = ActivationZone::Graveyard,
        timing = ActivationTiming::SorcerySpeed,
    )],
);

//! Unlikely Alliance — {1}{W} — Enchantment
//! Oracle: {1}{W}: Target nonattacking, nonblocking creature gets +0/+2 until end of turn.
//! Set: ALL #20 — Alliances | Scryfall ID: c14d2c73-1934-4504-bbfb-62ba82e0a0e3 | Oracle ID: f7ac18bc-ea63-4808-b709-92c1e69cf541
// IMPLEMENTED — {1}{W}: Target nonattacking, nonblocking creature gets +0/+2 until end of turn.

use baylee_cards_dsl::prelude::*;

static NONCOMBAT_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::Attacking),
    Filter::Not(&Filter::Blocking),
]);

card!(
    index = index::UNLIKELY_ALLIANCE,
    oracle_id = "f7ac18bc-ea63-4808-b709-92c1e69cf541",
    scryfall_id = "c14d2c73-1934-4504-bbfb-62ba82e0a0e3",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Unlikely Alliance",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{1}{W}"),
        &[Effect::PumpTarget {
            power: Amount::Fixed(0),
            toughness: Amount::Fixed(2),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&NONCOMBAT_CREATURE)),
    )],
);

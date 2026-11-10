//! Magnetic Web — {2} — Artifact
//! Oracle: If a creature with a magnet counter on it attacks, all creatures with magnet counters on them attack if able.
//! Oracle: Whenever a creature with a magnet counter on it attacks, all creatures with magnet counters on them block that creature this turn if able.
//! Oracle: {1}, {T}: Put a magnet counter on target creature.
//! Set: TMP #295 — Tempest | Scryfall ID: 9f3c7309-4efb-49ce-a9cc-a8f7b04c1a15 | Oracle ID: c62c8006-5650-43cb-8247-da097aa42376
// PARTIAL: taps with {1} to put a magnet counter on target creature; forcing all creatures with magnet counters to attack or block whenever a creature with a magnet counter attacks is not supported.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MAGNETIC_WEB,
    oracle_id = "c62c8006-5650-43cb-8247-da097aa42376",
    scryfall_id = "9f3c7309-4efb-49ce-a9cc-a8f7b04c1a15",
    coverage = Coverage::Partial(
        "taps with {1} to put a magnet counter on target creature; forcing all creatures with magnet counters to attack or block whenever a creature with a magnet counter attacks is not supported",
    ),
    faces = &[face!(
        name = "Magnetic Web",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{1}", TapSelf),
        &[Effect::AddCounter {
            kind: counters::MAGNET,
            amount: Amount::Fixed(1),
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);

//! Winter Blast — {X}{G} — Sorcery
//! Oracle: Tap X target creatures. Winter Blast deals 2 damage to each of those creatures with flying.
//! Set: ME1 #138 — Masters Edition | Scryfall ID: d806c7c8-3aac-4385-b937-f66e360ffd28 | Oracle ID: 594827de-e85f-4aac-b0e6-d92878f0f26c
// PARTIAL — the tap is written; the damage to the fliers among those targets is not.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WINTER_BLAST,
    oracle_id = "594827de-e85f-4aac-b0e6-d92878f0f26c",
    scryfall_id = "d806c7c8-3aac-4385-b937-f66e360ffd28",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "no effect deals damage to a filter applied to the ability's own \
         targets: DealDamageEach sweeps the whole battlefield"
    ),
    faces = &[face!(
        name = "Winter Blast",
        mana_cost = mana!("{X}{G}"),
        types = TypeSet::SORCERY,
    ),],
    // NOT SUPPORTED: "Winter Blast deals 2 damage to each of those creatures
    // with flying." — `Effect::DealDamageEach` takes a filter over all
    // permanents, so it would also damage flying creatures that were never
    // targeted; `Effect::DealDamage` names a single target; no effect reads
    // the ability's own target list through a filter, and no Filter can say
    // "one of this ability's targets".
    abilities = &[spell!(
        &[Effect::TapTarget],
        targets = Some(TargetReq::x_targets(TargetSpec::Object(&Filter::CREATURE)))
    )],
);

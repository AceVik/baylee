//! Nova Pentacle — {4} — Artifact
//! Oracle: {3}, {T}: The next time a source of your choice would deal damage to you this turn, that damage is dealt to target creature of an opponent's choice instead.
//! Set: ME3 #200 — Masters Edition III | Scryfall ID: 0e627164-da4e-4860-b516-7304fc41161e | Oracle ID: db121503-8a34-498a-829c-72c33798369b
// IMPLEMENTED — an opponent selects the target before costs; the activator
// chooses the source on resolution and redirects its next damage to the target.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NOVA_PENTACLE,
    oracle_id = "db121503-8a34-498a-829c-72c33798369b",
    scryfall_id = "0e627164-da4e-4860-b516-7304fc41161e",
    faces = &[face!(
        name = "Nova Pentacle",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{3}", TapSelf),
        &[Effect::RedirectNextChosenSourceDamageToTarget {
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)).chosen_by_opponent()),
    )],
);

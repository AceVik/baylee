//! Glacial Chasm — (no cost) — Land
//! Oracle: Cumulative upkeep—Pay 2 life. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: When this land enters, sacrifice a land.
//! Oracle: Creatures you control can't attack.
//! Oracle: Prevent all damage that would be dealt to you.
//! Set: ME2 #229 — Masters Edition II | Scryfall ID: 0c008129-daba-46bc-829c-d2c0c13ecdd3 | Oracle ID: 73e7a2ad-d11c-4867-b97d-f971809da778
// PARTIAL — the enter trigger sacrifices a land; the other three clauses have
// no vocabulary (see the NOT SUPPORTED lines below).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GLACIAL_CHASM,
    oracle_id = "73e7a2ad-d11c-4867-b97d-f971809da778",
    scryfall_id = "0c008129-daba-46bc-829c-d2c0c13ecdd3",
    faces = &[face!(name = "Glacial Chasm", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "no cumulative upkeep; no attack prohibition; no damage prevention to a player",
    ),
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::SacrificeFilter {
            who: PlayerRel::You,
            filter: &Filter::LAND,
        }]
    )],
);

// NOT SUPPORTED: Cumulative upkeep—Pay 2 life. (no ability kind for cumulative upkeep; `Echo` and `Prepared` are fixed one-shots and nothing models age counters)
// NOT SUPPORTED: Creatures you control can't attack. (no `Modifier` forbids attacking)
// NOT SUPPORTED: Prevent all damage that would be dealt to you. (`PreventDamageToIt`/`PreventDamageFromIt` reach an object; no `Modifier` prevents damage to a player)

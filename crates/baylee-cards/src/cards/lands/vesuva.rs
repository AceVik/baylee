//! Vesuva — (no cost) — Land
//! Oracle: You may have this land enter tapped as a copy of any land on the battlefield.
//! Set: TSR #289 — Time Spiral Remastered | Scryfall ID: 0726f70a-c1c4-4edb-86fb-9be280d9ea73 | Oracle ID: 4001b868-ada1-43f4-92e2-27ab0e80c913
// PARTIAL — the copy choice is built: this land enters as a copy of any land
// on the battlefield (CopyOnEnter, CR 707.9). The "tapped" half of the same
// clause is not writable, because it is tied to that choice.
// NOT SUPPORTED: "enter tapped as a copy of any land" — CopyOnEnter's `mods`
// are the "except …" clauses (types, supertypes, subtypes, keywords, entry
// counters) and `EnterModifier::Tapped` is unconditional, which would also tap
// the land on the branch where the copy is declined.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VESUVA,
    oracle_id = "4001b868-ada1-43f4-92e2-27ab0e80c913",
    scryfall_id = "0726f70a-c1c4-4edb-86fb-9be280d9ea73",
    faces = &[face!(name = "Vesuva", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "enters tapped as a copy: no CopyMod for it, and EnterModifier::Tapped is unconditional"
    ),
    abilities = &[AbilityDef::CopyOnEnter {
        target: TargetSpec::Object(&Filter::LAND),
        mods: &[],
    }],
);

//! Swarmyard — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Regenerate target Insect, Rat, Spider, or Squirrel. (The next time it would be destroyed this turn, instead tap it, remove it from combat, and heal all damage on it.)
//! Set: TSR #284 — Time Spiral Remastered | Scryfall ID: b89329f2-d386-40a7-9098-6d80beeb8843 | Oracle ID: 4b508087-99da-4eb1-8b12-29162f2ec85d
// IMPLEMENTED — {T}: Add {C}, and the regeneration shield the second line
// hands to one of four creature types.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

/// "Target Insect, Rat, Spider, or Squirrel" — the four the card prints,
/// in the card's own order.
static VERMIN: Filter = Filter::Or(&[
    Filter::HasSubtype(creature::INSECT),
    Filter::HasSubtype(creature::RAT),
    Filter::HasSubtype(creature::SPIDER),
    Filter::HasSubtype(creature::SQUIRREL),
]);

card!(
    index = index::SWARMYARD,
    oracle_id = "4b508087-99da-4eb1-8b12-29162f2ec85d",
    scryfall_id = "b89329f2-d386-40a7-9098-6d80beeb8843",
    faces = &[face!(name = "Swarmyard", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            Cost::TAP,
            &[Effect::regenerate(TargetSpec::Object(&VERMIN))],
            target = Some(TargetSpec::Object(&VERMIN))
        ),
    ],
);

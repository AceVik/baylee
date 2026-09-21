//! Swarmyard — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Regenerate target Insect, Rat, Spider, or Squirrel. (The next time it would be destroyed this turn, instead tap it, remove it from combat, and heal all damage on it.)
//! Set: TSR #284 — Time Spiral Remastered | Scryfall ID: b89329f2-d386-40a7-9098-6d80beeb8843 | Oracle ID: 4b508087-99da-4eb1-8b12-29162f2ec85d
// PARTIAL — {T}: Add {C} is built; the regenerate ability has no effect
// variant and no keyword bit in the DSL.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SWARMYARD,
    oracle_id = "4b508087-99da-4eb1-8b12-29162f2ec85d",
    scryfall_id = "b89329f2-d386-40a7-9098-6d80beeb8843",
    faces = &[face!(name = "Swarmyard", types = TypeSet::LAND,),],
    coverage =
        Coverage::Partial("regenerate is neither an Effect variant nor a keyword the engine reads"),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{T}: Regenerate target Insect, Rat, Spider, or
        // Squirrel." — there is no `Effect::Regenerate`, and the readable
        // keyword list has no regenerate bit; nothing here replaces a
        // destruction (`Modifier::PreventDamageToIt` is damage, not
        // destruction) and the target's phenotype set (Insect/Rat/Spider/
        // Squirrel) has no bearing on that.
    ],
);

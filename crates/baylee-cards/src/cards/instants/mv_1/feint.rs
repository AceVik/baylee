//! Feint — {R} — Instant
//! Oracle: Tap all creatures blocking target attacking creature. Prevent all combat damage that would be dealt this turn by that creature and each creature blocking it.
//! Set: LEG #146 — Legends | Scryfall ID: 95b265bc-a94d-403b-8232-8fdfa0f8d9d5 | Oracle ID: 1bb8fe05-abb3-40a8-9e80-5d99ed0e4284
// PARTIAL — the whole spell is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FEINT,
    oracle_id = "1bb8fe05-abb3-40a8-9e80-5d99ed0e4284",
    scryfall_id = "95b265bc-a94d-403b-8232-8fdfa0f8d9d5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "no Filter can name the creatures blocking a chosen attacker or the \
         creatures blocked by one: the only blocking predicate is \
         Filter::Blocking, which asks an object about itself, and no filter \
         references the spell's target"
    ),
    faces = &[face!(
        name = "Feint",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
    // NOT SUPPORTED: "Tap all creatures blocking target attacking creature."
    // — `Effect::TapAll` takes a `Filter` and no filter can say "blocking
    // [the target]": `Filter::Blocking` is a creature's own state and nothing
    // in the vocabulary reads the block relation, let alone one pointed at a
    // chosen attacker.
    // NOT SUPPORTED: "Prevent all combat damage that would be dealt this turn
    // by that creature and each creature blocking it." —
    // `Modifier::PreventDamageFromIt` is the right prevention but it is a
    // filter-wide layer effect (`Effect::continuous`), and neither "the
    // target" nor "each creature blocking it" is a filter; the spell comes
    // off the card rather than tapping or shielding the wrong creatures.
    abilities = &[],
);

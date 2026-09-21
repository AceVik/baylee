//! Tyrite Sanctum — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: Target legendary creature becomes a God in addition to its other types. Put a +1/+1 counter on it.
//! Oracle: {4}, {T}, Sacrifice this land: Put an indestructible counter on target God.
//! Set: CMM #1049 — Commander Masters | Scryfall ID: 27906645-7d43-4c18-a20b-8d683069351e | Oracle ID: 38c83332-2d21-450c-ae97-34109b565f59
// IMPLEMENTED — {C} mana, and {2}, {T}: the target legendary creature gains
// the God subtype for good and gets a +1/+1 counter. The {4} ability is a
// Partial: see the NOT SUPPORTED note beside the ability list.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::TYRITE_SANCTUM,
    oracle_id = "38c83332-2d21-450c-ae97-34109b565f59",
    scryfall_id = "27906645-7d43-4c18-a20b-8d683069351e",
    faces = &[face!(name = "Tyrite Sanctum", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the {4}, {T}, Sacrifice ability's \"indestructible counter\": CounterKind has no such variant and counters assigns no id for one"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}", TapSelf),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(creature::GOD),
                    Duration::Indefinitely,
                ),
                Effect::AddCounter {
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(1),
                },
            ],
            target = Some(TargetSpec::Object(&Filter::LEGENDARY_CREATURE)),
        ),
        // NOT SUPPORTED: "{4}, {T}, Sacrifice this land: Put an indestructible
        // counter on target God." — there is nothing to put on the God:
        // `CounterKind` has no indestructible counter and `counters` assigns
        // no id for one, and an id invented in a card file is the collision
        // that module exists to prevent. Writing it as a granted
        // `KeywordSet::INDESTRUCTIBLE` instead would be a different card —
        // proliferate, counter removal and every "counters on it" sentence
        // would read it wrongly — so the ability comes off rather than
        // shipping as a keyword grant.
    ],
);

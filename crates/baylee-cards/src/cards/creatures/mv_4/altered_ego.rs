//! Altered Ego — {X}{2}{G}{U} — Creature — Shapeshifter
//! Oracle: This spell can't be countered.
//! Oracle: You may have this creature enter as a copy of any creature on the battlefield, except it enters with X additional +1/+1 counters on it.
//! Set: SOC #292 — Secrets of Strixhaven Commander | Scryfall ID: d51e076a-be33-4b2b-b52f-fe7b5bc56206 | Oracle ID: 7c35f3fd-c64e-4944-a4d5-37ce916d23c3
// PARTIAL — "can't be countered" is the uncounterable keyword bit and the
// optional copy is AbilityDef::CopyOnEnter; the X counters that copy clause
// adds have no variant, so the ability below carries a NOT SUPPORTED line.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ALTERED_EGO,
    oracle_id = "7c35f3fd-c64e-4944-a4d5-37ce916d23c3",
    scryfall_id = "d51e076a-be33-4b2b-b52f-fe7b5bc56206",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces = &[face!(
        name = "Altered Ego",
        mana_cost = mana!("{X}{2}{G}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(0),
        toughness = Some(0),
    ),],
    keywords = KeywordSet::UNCOUNTERABLE,
    coverage = Coverage::Partial(
        "\"except it enters with X additional +1/+1 counters on it\": CopyOnEnter's \
         mods carry CopyMod::AddCounter(CounterKind, u16) — a fixed count (it is \
         Spark Double's single counter) — and no Amount, so the X this card adds to \
         the copy cannot be said"
    ),
    // NOT SUPPORTED: "except it enters with X additional +1/+1 counters on it."
    // The copy itself is exact; the counters are the part no variant reaches.
    // EnterModifier::WithCounters does carry an Amount, but it is
    // unconditional, and the printed "except" ties those counters to the copy
    // being made — an Altered Ego that declined the copy would still be an X/X.
    abilities = &[AbilityDef::CopyOnEnter {
        target: TargetSpec::Object(&Filter::CREATURE),
        mods: &[],
    }],
);

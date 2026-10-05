//! Rapid Fire — {3}{W} — Instant
//! Oracle: Cast this spell only before blockers are declared.
//! Oracle: Target creature gains first strike until end of turn. If it doesn't have rampage, that creature gains rampage 2 until end of turn. (Whenever the creature becomes blocked, it gets +2/+2 until end of turn for each creature blocking it beyond the first.)
//! Set: LEG #32 — Legends | Scryfall ID: e26e7c9c-e6de-47f4-8394-7e853408f84c | Oracle ID: 63c7248e-a7a2-4e32-b4d2-55dbc63e51e5
// PARTIAL — the cast restriction and the first-strike grant are built; the
// conditional rampage grant is not expressible and is left off.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RAPID_FIRE,
    oracle_id = "63c7248e-a7a2-4e32-b4d2-55dbc63e51e5",
    scryfall_id = "e26e7c9c-e6de-47f4-8394-7e853408f84c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "rampage is a keyword with a number: it has no KeywordSet bit, no \
         AbilityDef carries it, and no Amount counts the creatures blocking \
         beyond the first, so \"If it doesn't have rampage, that creature \
         gains rampage 2 until end of turn\" is not written"
    ),
    faces = &[face!(
        name = "Rapid Fire",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::INSTANT,
    ),],
    // NOT SUPPORTED: "If it doesn't have rampage, that creature gains
    // rampage 2 until end of turn." — rampage is a number-carrying keyword
    // (CR 702.23) on neither KeywordSet nor AbilityDef, and the engine reads
    // it nowhere; Filter::HasKeyword can therefore not ask whether the target
    // already has it either. The unconditional first-strike half is written
    // below, without rampage.
    abilities = &[spell!(
        &[Effect::PumpTarget {
            power: Amount::Fixed(0),
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::FIRST_STRIKE,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        condition = Some(Condition::BeforeStep(StepKind::DeclareBlockers))
    )],
);

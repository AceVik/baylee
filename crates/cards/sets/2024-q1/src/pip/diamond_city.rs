//! Diamond City — (no cost) — Land
//! Oracle: This land enters with a shield counter on it. (If it would be dealt damage or destroyed, remove a shield counter from it instead.)
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Move a shield counter from this land onto target creature. Activate only if two or more creatures entered the battlefield under your control this turn.
//! Set: PIP #147 — Fallout | Scryfall ID: 3e9bd49a-e9f1-4543-b04a-777a9e5a55ec | Oracle ID: 88e29d50-1680-495d-be84-b92b4c9e636f
// IMPLEMENTED — intrinsic shield counter on entry, colorless mana, and shield
// transfer gated by historical creature arrivals under your control.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DIAMOND_CITY,
    oracle_id = "88e29d50-1680-495d-be84-b92b4c9e636f",
    scryfall_id = "3e9bd49a-e9f1-4543-b04a-777a9e5a55ec",
    faces = &[face!(
        name = "Diamond City",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::Shield,
            amount: Amount::Fixed(1)
        }],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!(TapSelf),
            &[Effect::MoveCounter {
                from: TargetSlot::This,
                to: TargetSlot::First,
                kind: Some(CounterKind::Shield)
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
            condition = Some(Condition::AmountAtLeast(
                Amount::CreaturesEnteredUnderYourControlThisTurn,
                2
            )),
        ),
    ],
);

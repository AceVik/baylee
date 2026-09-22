//! Fortified Beachhead — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Soldier card from your hand. This land enters tapped unless you revealed a Soldier card this way or you control a Soldier.
//! Oracle: {T}: Add {W} or {U}.
//! Oracle: {5}, {T}: Soldiers you control get +1/+1 until end of turn.
//! Set: BRO #262 — The Brothers' War | Scryfall ID: e248204c-865d-42d9-b745-8ff73225b4a1 | Oracle ID: 387fe395-e4a0-4fb2-8d6c-88a1a21d2ed8
// PARTIAL — {W}/{U} and the Soldier pump are built; of the enter clause only
// "unless you control a Soldier" is sayable, so the land is written as
// entering tapped unless you control a Soldier.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

/// "A Soldier you control" — the land's untap condition and the pump's subject.
static SOLDIERS_YOU_CONTROL: Filter = f!(your Filter::HasSubtype(creature::SOLDIER));

card!(
    index = index::FORTIFIED_BEACHHEAD,
    oracle_id = "387fe395-e4a0-4fb2-8d6c-88a1a21d2ed8",
    scryfall_id = "e248204c-865d-42d9-b745-8ff73225b4a1",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    coverage = Coverage::Partial(
        "the entry clause is a disjunction (revealed a Soldier or control one) and a face's modifier list is a conjunction",
    ),
    faces = &[face!(
        name = "Fortified Beachhead",
        types = TypeSet::LAND,
        // NOT SUPPORTED: "tapped unless you revealed a Soldier card this way
        // **or** you control a Soldier". Both halves are sayable on their own
        // — `TappedUnlessReveal` and this `TappedUnless` — and putting both
        // here would be wrong: the entry scan only ever *inserts* `TAPPED`,
        // so a list of modifiers is an `and`. The control half alone is the
        // stricter of the two readings a single modifier can give, which is
        // the one to be wrong in the safe direction.
        enter_modifiers = &[EnterModifier::TappedUnless(&SOLDIERS_YOU_CONTROL)],
    )],
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::White, ManaColor::Blue])]),
        activated!(
            cost!("{5}", TapSelf),
            &[Effect::PumpFilter {
                filter: &SOLDIERS_YOU_CONTROL,
                controlled_by: None,
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(1),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }]
        ),
    ],
);

//! Kry Shield — {2} — Artifact
//! Oracle: {2}, {T}: Prevent all damage that would be dealt this turn by target creature you control. That creature gets +0/+X until end of turn, where X is its mana value.
//! Set: LEG #282 — Legends | Scryfall ID: a558f23c-c2ce-40d0-b894-f8ccbff8f622 | Oracle ID: 2c4bd475-b8af-4916-b7a0-68abb8994138
// PARTIAL — the +0/+X half is built; the prevention is not.
// NOT SUPPORTED: "Prevent all damage that would be dealt this turn by
// target creature you control." — `Modifier::PreventDamageFromIt` prevents
// combat damage only, and no shield or modifier prevents all damage from one
// named source for the turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KRY_SHIELD,
    oracle_id = "2c4bd475-b8af-4916-b7a0-68abb8994138",
    scryfall_id = "a558f23c-c2ce-40d0-b894-f8ccbff8f622",
    faces = &[face!(
        name = "Kry Shield",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "the prevention half: PreventDamageFromIt covers combat damage only, \
         and no shield prevents all damage from one named source for a turn",
    ),
    abilities = &[activated!(
        cost!("{2}", TapSelf),
        &[Effect::PumpTarget {
            power: Amount::Fixed(0),
            toughness: Amount::TargetCmc,
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE)),
    )],
);

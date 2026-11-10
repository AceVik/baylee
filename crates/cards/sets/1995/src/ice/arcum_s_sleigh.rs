//! Arcum's Sleigh — {1} — Artifact
//! Oracle: {2}, {T}: Target creature gains vigilance until end of turn. Activate only during combat and only if defending player controls a snow land.
//! Set: ICE #309 — Ice Age | Scryfall ID: e9780ce2-756c-48e5-9936-45f6a224f61d | Oracle ID: 40960ec8-e3be-45f2-8d27-316d82c69123

use baylee_cards_dsl::prelude::*;

// "Defending player controls a snow land": the Sleigh is no attacker, so
// `ControlledByDefendingPlayer` reads the active player's opponents during
// combat (CR 506.2, 802.2) and nobody outside it.
static DEFENDING_PLAYERS_SNOW_LAND: Filter = Filter::And(&[
    Filter::LAND,
    Filter::HasSupertype(SupertypeSet::SNOW),
    Filter::ControlledByDefendingPlayer,
]);

card!(
    index = index::ARCUM_S_SLEIGH,
    oracle_id = "40960ec8-e3be-45f2-8d27-316d82c69123",
    scryfall_id = "e9780ce2-756c-48e5-9936-45f6a224f61d",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Arcum's Sleigh",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{2}", TapSelf),
        &[Effect::PumpTarget {
            power: Amount::Fixed(0),
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::VIGILANCE,
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
        condition = Some(Condition::All(&[
            Condition::DuringCombat,
            Condition::BattlefieldCount(&DEFENDING_PLAYERS_SNOW_LAND, 1),
        ]))
    )],
);

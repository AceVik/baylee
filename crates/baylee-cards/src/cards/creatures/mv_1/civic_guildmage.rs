//! Civic Guildmage — {W} — Creature — Human Wizard
//! Oracle: {G}, {T}: Target creature gets +0/+1 until end of turn.
//! Oracle: {U}, {T}: Put target creature you control on top of its owner's library.
//! Set: MIR #7 — Mirage | Scryfall ID: a9319039-db2f-47bf-9ef0-8d3a381d54fb | Oracle ID: 6e93cb9a-44c8-49fb-b6a7-bfdf7bd725fe
// NOT SUPPORTED: "{U}, {T}: Put target creature you control on top of its owner's library" — no effect puts a permanent on top of its owner's library with no choice; OwnerPutsOnTopOrBottom lets the owner choose the bottom, which is not printed, so the ability is left off.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CIVIC_GUILDMAGE,
    oracle_id = "6e93cb9a-44c8-49fb-b6a7-bfdf7bd725fe",
    scryfall_id = "a9319039-db2f-47bf-9ef0-8d3a381d54fb",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue, Color::White]),
    coverage = Coverage::Partial(
        "gives target creature +0/+1 until end of turn for {G}, {T}; the {U}, {T} ability that puts a creature you control on top of its owner's library is left off because OwnerPutsOnTopOrBottom lets the owner choose top or bottom, which is not printed",
    ),
    faces = &[face!(
        name = "Civic Guildmage",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        // NOT SUPPORTED: {U}, {T}: Put target creature you control on top of its owner's library.
        activated!(
            cost!("{G}", TapSelf),
            &[Effect::continuous(
                &Filter::This,
                Modifier::ModifyPT(0, 1),
                Duration::UntilEndOfTurn,
            )],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);

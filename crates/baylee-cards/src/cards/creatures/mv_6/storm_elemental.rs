//! Storm Elemental — {5}{U} — Creature — Elemental
//! Oracle: Flying
//! Oracle: {U}, Exile the top card of your library: Tap target creature with flying.
//! Oracle: {U}, Exile the top card of your library: If the exiled card is a snow land, this creature gets +1/+1 until end of turn.
//! Set: ME2 #68 — Masters Edition II | Scryfall ID: 2903c66e-056d-4ab6-9dfe-81b18a8da454 | Oracle ID: f1fee486-660e-4356-896e-671e8b675ad8

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static CREATURE_WITH_FLYING: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::FLYING)]);

card!(
    index = index::STORM_ELEMENTAL,
    oracle_id = "f1fee486-660e-4356-896e-671e8b675ad8",
    scryfall_id = "2903c66e-056d-4ab6-9dfe-81b18a8da454",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Storm Elemental",
        mana_cost = mana!("{5}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        keywords = KeywordSet::FLYING,
        power = Some(3),
        toughness = Some(4),
    ),],
    coverage = Coverage::Partial(
        "Neither {U} activated ability exiles the top card of your library as part of its cost, and the second {U} ability gives +1/+1 until end of turn unconditionally rather than checking whether the exiled card is a snow land."
    ),
    abilities = &[
        activated!(
            cost!("{U}"),
            &[Effect::TapTarget],
            target = Some(TargetSpec::Object(&CREATURE_WITH_FLYING))
        ),
        activated!(
            cost!("{U}"),
            &[Effect::continuous(
                &Filter::This,
                Modifier::ModifyPT(1, 1),
                Duration::UntilEndOfTurn,
            )]
        ),
    ],
);

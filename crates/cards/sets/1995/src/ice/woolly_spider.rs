//! Woolly Spider — {1}{G}{G} — Creature — Spider
//! Oracle: Reach (This creature can block creatures with flying.)
//! Oracle: Whenever this creature blocks a creature with flying, this creature gets +0/+2 until end of turn.
//! Set: ME2 #189 — Masters Edition II | Scryfall ID: 2496a40a-1d67-44ea-900d-9c5fbd5034a7 | Oracle ID: 811c6f52-03cb-49a9-a3c7-7f4ae40e7155

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static CREATURE_WITH_FLYING: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::FLYING)]);

card!(
    index = index::WOOLLY_SPIDER,
    oracle_id = "811c6f52-03cb-49a9-a3c7-7f4ae40e7155",
    scryfall_id = "2496a40a-1d67-44ea-900d-9c5fbd5034a7",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    keywords = KeywordSet::REACH,
    faces = &[face!(
        name = "Woolly Spider",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIDER],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::BlocksCreature(&CREATURE_WITH_FLYING),
        &[Effect::continuous(
            &Filter::This,
            Modifier::ModifyPT(0, 2),
            Duration::UntilEndOfTurn,
        )]
    )],
);

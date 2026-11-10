//! Avatar of Fury — {6}{R}{R} — Creature — Avatar
//! Oracle: If an opponent controls seven or more lands, this spell costs {6} less to cast.
//! Oracle: Flying
//! Oracle: {R}: This creature gets +1/+0 until end of turn.
//! Set: E01 #39 — Archenemy: Nicol Bolas | Scryfall ID: f137e17e-6856-49c5-9878-8c5c8f3c1518 | Oracle ID: 4854f836-2247-43c3-adbc-1d1ee5eaf120
// PARTIAL — 6/6 flying with {R}: +1/+0 until end of turn.
// NOT SUPPORTED: conditional {6} self-cost reduction when an opponent controls seven or more lands.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AVATAR_OF_FURY,
    oracle_id = "4854f836-2247-43c3-adbc-1d1ee5eaf120",
    scryfall_id = "f137e17e-6856-49c5-9878-8c5c8f3c1518",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial(
        "self-cost reduction when an opponent controls seven or more lands is not supported",
    ),
    faces = &[face!(
        name = "Avatar of Fury",
        mana_cost = mana!("{6}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::AVATAR],
        power = Some(6),
        toughness = Some(6),
    ),],
    abilities = &[activated!(
        cost!("{R}"),
        &[Effect::continuous(
            &Filter::This,
            Modifier::ModifyPT(1, 0),
            Duration::UntilEndOfTurn
        )]
    ),],
);
